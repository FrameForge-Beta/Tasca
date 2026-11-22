//! O nó da Tasca: um endpoint iroh que hospeda grupos e se liga a grupos de outros.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow, bail, ensure};
use iroh::{
    Endpoint,
    endpoint::{Connection, SendStream, presets},
    protocol::{AcceptError, ProtocolHandler, Router},
};
use tokio::sync::{broadcast, watch};
use tracing::{debug, warn};

use crate::{
    ALPN,
    event::{EventKind, GroupId, SignedEvent},
    identity::Identity,
    invite::Invite,
    protocol::{ClientMsg, HostMsg, Presence, read_frame, write_frame},
    store::Store,
};

/// Evento novo num grupo, com o seu número de sequência.
pub type Update = (u64, SignedEvent);

#[derive(Debug, Clone, Default)]
pub struct NodeOptions {
    /// Só ligações diretas por IP (sem relays nem descoberta). Útil para testes e rede local.
    pub local_only: bool,
    /// Relays próprios (por exemplo, o teu servidor). Vazio usa os relays públicos do iroh.
    pub relay_urls: Vec<String>,
    /// Servidor onde ficam as caixas cifradas dos grupos. Sem ele, tudo é só P2P.
    #[cfg(feature = "servidor")]
    pub servidor: Option<crate::servidor::AccountServer>,
    /// Chave própria deste dispositivo (telemóvel). Sem ela, o nó usa a chave da conta.
    /// Ver [`crate::dispositivo`].
    pub dispositivo: Option<iroh::SecretKey>,
    /// Pasta dos anexos (ficheiros e imagens). Sem ela, não se enviam nem servem anexos.
    pub ficheiros: Option<std::path::PathBuf>,
}

pub(crate) struct Inner {
    identity: Identity,
    #[cfg(feature = "servidor")]
    servidor: Option<crate::servidor::AccountServer>,
    store: Mutex<Store>,
    feeds: Mutex<HashMap<GroupId, broadcast::Sender<Update>>>,
    /// Quem está online em cada grupo (no anfitrião: quem está ligado; nos membros: o que
    /// o anfitrião disse por último).
    presence: Mutex<HashMap<GroupId, watch::Sender<Presence>>>,
    /// Anfitrião: ligações abertas por (grupo, membro), para saber quando alguém sai mesmo.
    conns: Mutex<HashMap<(GroupId, [u8; 32]), usize>>,
    /// Lista de amigos, o meu estado e os perfis deles.
    amigos: Arc<crate::amigos::Amigos>,
    /// Certificado deste dispositivo, quando não usa a chave da conta.
    certificado: Option<crate::dispositivo::Certificado>,
    /// Quem anda a alojar salas de voz noutros dispositivos (o último anúncio de cada sala).
    anuncios: Mutex<HashMap<(GroupId, String), crate::voz::Anuncio>>,
    /// Chave iroh deste dispositivo.
    endpoint_id: [u8; 32],
    endpoint: iroh::Endpoint,
    /// Chamadas privadas a chegar.
    pub(crate) toques: broadcast::Sender<crate::voz::Toque>,
    /// Chamadas privadas conhecidas: grupo → (conta do amigo, onde a chamada pode estar).
    privadas: Mutex<HashMap<GroupId, (iroh::PublicKey, Option<iroh::EndpointAddr>)>>,
    pasta: Option<crate::ficheiros::Pasta>,
}

impl Inner {
    fn feed(&self, group: GroupId) -> broadcast::Sender<Update> {
        self.feeds
            .lock()
            .unwrap()
            .entry(group)
            .or_insert_with(|| broadcast::channel(1024).0)
            .clone()
    }

    pub(crate) fn presence(&self, group: GroupId) -> watch::Sender<Presence> {
        self.presence
            .lock()
            .unwrap()
            .entry(group)
            .or_insert_with(|| watch::channel(Presence::new()).0)
            .clone()
    }

    /// Anfitrião: marca a sala de voz de alguém na presença (`None` ao sair da voz).
    /// Quem sai da voz sem ligação ao grupo deixa de aparecer online.
    pub(crate) fn set_voice_presence(&self, group: GroupId, who: [u8; 32], room: Option<String>) {
        let ligado = who == *self.identity.public().as_bytes()
            || self.conns.lock().unwrap().contains_key(&(group, who));
        self.presence(group).send_if_modified(|p| match (&room, ligado) {
            (None, false) => p.remove(&who).is_some(),
            _ => {
                if p.get(&who) == Some(&room) {
                    return false;
                }
                p.insert(who, room.clone());
                true
            }
        });
    }

    pub(crate) fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap()
    }

    /// A conta de quem está do outro lado (o telemóvel apresenta-se com certificado).
    pub(crate) fn conta_de(&self, remoto: &iroh::PublicKey) -> iroh::PublicKey {
        self.amigos.dispositivos.conta_de(remoto)
    }

    /// Guarda um anúncio de voz de outro dispositivo, se for válido e mais recente.
    /// Devolve `true` se entrou.
    pub(crate) fn guardar_anuncio(&self, a: crate::voz::Anuncio) -> bool {
        if a.endpoint == self.endpoint_id || a.verificar().is_err() {
            return false;
        }
        let Ok(conta) = iroh::PublicKey::from_bytes(&a.conta) else { return false };
        if !self.store().is_member(a.group, &conta).unwrap_or(false) {
            return false;
        }
        let mut m = self.anuncios.lock().unwrap();
        let chave = (a.group, a.room.clone());
        if let Some(antigo) = m.get(&chave) {
            // Um anúncio mais antigo do mesmo dispositivo não volta atrás.
            if antigo.endpoint == a.endpoint && antigo.ts >= a.ts {
                return false;
            }
            // Outro dispositivo com a sala ainda ativa: fica o que começou primeiro.
            if antigo.endpoint != a.endpoint && antigo.ativo() && a.ativo() && (antigo.desde, antigo.endpoint) < (a.desde, a.endpoint) {
                return false;
            }
        }
        m.insert(chave, a);
        true
    }

    pub(crate) fn identity(&self) -> &Identity {
        &self.identity
    }

    pub(crate) fn meus_enderecos(&self) -> Vec<iroh::TransportAddr> {
        self.endpoint.addr().addrs.into_iter().collect()
    }

    /// Anúncios conhecidos de um grupo (os nossos e os dos outros que ainda valem).
    pub(crate) fn anuncios_de(&self, group: GroupId, hub: &crate::voz::Hub) -> Vec<crate::voz::Anuncio> {
        let mut out: Vec<_> = hub
            .anuncios(&self.identity, &self.meus_enderecos())
            .into_iter()
            .filter(|a| a.group == group)
            .collect();
        let m = self.anuncios.lock().unwrap();
        for a in m.values() {
            if a.group == group && a.ativo() && !out.iter().any(|o| o.room == a.room) {
                out.push(a.clone());
            }
        }
        out
    }
}

#[derive(Clone)]
pub struct Node {
    inner: Arc<Inner>,
    router: Router,
    local_only: bool,
    voz: Arc<crate::voz::Hub>,
}

impl Node {
    pub async fn spawn(identity: Identity, store: Store, opts: NodeOptions) -> Result<Self> {
        let builder = if opts.local_only {
            Endpoint::builder(presets::Minimal)
        } else if !opts.relay_urls.is_empty() {
            let urls = opts
                .relay_urls
                .iter()
                .map(|u| u.parse())
                .collect::<Result<Vec<iroh::RelayUrl>, _>>()
                .context("URL de relay inválido")?;
            Endpoint::builder(presets::N0).relay_mode(iroh::RelayMode::custom(urls))
        } else {
            Endpoint::builder(presets::N0)
        };
        let chave = opts.dispositivo.clone().unwrap_or_else(|| identity.secret().clone());
        let certificado = opts
            .dispositivo
            .as_ref()
            .map(|d| crate::dispositivo::Certificado::emitir(&identity, &d.public()));
        let endpoint = builder
            .secret_key(chave)
            .bind()
            .await
            .map_err(|e| anyhow!("não consegui abrir o endpoint: {e}"))?;
        let inner = Arc::new(Inner {
            identity,
            #[cfg(feature = "servidor")]
            servidor: opts.servidor.clone(),
            store: Mutex::new(store),
            feeds: Default::default(),
            presence: Default::default(),
            conns: Default::default(),
            amigos: Default::default(),
            certificado,
            anuncios: Default::default(),
            endpoint_id: *endpoint.id().as_bytes(),
            endpoint: endpoint.clone(),
            toques: broadcast::channel(16).0,
            privadas: Default::default(),
            pasta: opts.ficheiros.clone().map(crate::ficheiros::Pasta),
        });
        let voz = Arc::new(crate::voz::Hub::new(Arc::downgrade(&inner), *endpoint.id().as_bytes()));
        let router = Router::builder(endpoint)
            .accept(
                ALPN,
                HostProtocol {
                    inner: inner.clone(),
                },
            )
            .accept(crate::amigos::ALPN, crate::amigos::Protocolo(inner.amigos.clone()))
            .accept(
                crate::dispositivo::ALPN,
                crate::dispositivo::Apresentacao(inner.amigos.dispositivos.clone()),
            )
            .accept(
                crate::ficheiros::ALPN,
                crate::ficheiros::Servir { inner: Arc::downgrade(&inner), pasta: opts.ficheiros.clone().map(crate::ficheiros::Pasta) },
            )
            .accept(
                crate::voz::VOZ_ALPN,
                crate::voz::HostVoz { inner: Arc::downgrade(&inner), hub: voz.clone() },
            )
            .spawn();
        let node = Self {
            inner,
            router,
            local_only: opts.local_only,
            voz,
        };
        node.anunciar_voz();
        Ok(node)
    }

    /// Enquanto este dispositivo alojar salas de voz, publica o anúncio na caixa de cada
    /// grupo: quando muda e a cada dois minutos. Quando a sala esvazia, publica-a vazia.
    fn anunciar_voz(&self) {
        let inner = Arc::downgrade(&self.inner);
        let hub = Arc::downgrade(&self.voz);
        let mut mudou = self.voz.watch();
        tokio::spawn(async move {
            let mut publicados: HashMap<(GroupId, String), (Vec<[u8; 32]>, std::time::Instant, u64)> = HashMap::new();
            loop {
                tokio::select! {
                    r = mudou.changed() => if r.is_err() { break },
                    _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
                }
                // Junta alterações seguidas (várias pessoas a entrar ao mesmo tempo).
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                let (Some(inner), Some(hub)) = (inner.upgrade(), hub.upgrade()) else { break };
                let agora: Vec<_> = hub.anuncios(&inner.identity, &inner.meus_enderecos());
                let mut enviar = Vec::new();
                for a in &agora {
                    let k = (a.group, a.room.clone());
                    let novo = match publicados.get(&k) {
                        Some((p, t, d)) => *p != a.pessoas || *d != a.desde || t.elapsed() > std::time::Duration::from_secs(120),
                        None => true,
                    };
                    if novo {
                        publicados.insert(k, (a.pessoas.clone(), std::time::Instant::now(), a.desde));
                        enviar.push(a.clone());
                    }
                }
                let acabadas: Vec<_> = publicados
                    .keys()
                    .filter(|k| !agora.iter().any(|a| a.group == k.0 && a.room == k.1))
                    .cloned()
                    .collect();
                for k in acabadas {
                    let (_, _, desde) = publicados.remove(&k).unwrap();
                    enviar.push(crate::voz::Anuncio::assinar(&inner.identity, k.0, &k.1, inner.endpoint_id, Vec::new(), Vec::new(), desde));
                }
                drop(hub);
                for a in enviar {
                    // Ao dono do grupo, diretamente (se estiver ligado responde logo a quem
                    // perguntar), e na caixa do grupo.
                    let dono = inner.store().group(a.group).ok().flatten().map(|g| g.owner);
                    if let Some(dono) = dono.filter(|d| *d != inner.identity.public() && *d.as_bytes() != inner.endpoint_id) {
                        let (ep, a2, cert) = (inner.endpoint.clone(), a.clone(), inner.certificado.clone());
                        tokio::spawn(async move {
                            let r: Result<()> = async {
                                let addr = iroh::EndpointAddr::new(dono);
                                if let Some(c) = &cert {
                                    crate::dispositivo::apresentar(&ep, addr.clone(), c).await?;
                                }
                                let conn = tokio::time::timeout(
                                    std::time::Duration::from_secs(8),
                                    ep.connect(addr, crate::voz::VOZ_ALPN),
                                )
                                .await??;
                                let (mut send, mut recv) = conn.open_bi().await?;
                                write_frame(&mut send, &crate::voz::VozCliente::Anunciar(a2)).await?;
                                send.finish()?;
                                let _ = tokio::time::timeout(std::time::Duration::from_secs(3), recv.read_to_end(16)).await;
                                conn.close(0u32.into(), b"ok");
                                Ok(())
                            }
                            .await;
                            if let Err(e) = r {
                                debug!("anúncio de voz não chegou ao dono: {e:#}");
                            }
                        });
                    }
                    if let Err(e) = inner.publicar_anuncio(a).await {
                        debug!("anúncio de voz não publicado: {e:#}");
                    }
                }
            }
        });
    }

    /// Amigos: lista, o meu estado e o que sei deles.
    pub fn amigos(&self) -> &crate::amigos::Amigos {
        &self.inner.amigos
    }

    /// Pergunta ao PC de um amigo como está (e troca perfis). `None` = offline ou invisível.
    /// Aceita a chave pública do amigo (o iroh descobre onde está) ou um endereço completo.
    pub async fn perguntar_amigo(&self, quem: impl Into<iroh::EndpointAddr>) -> Result<Option<crate::amigos::Estado>> {
        let quem = quem.into();
        if let Err(e) = self.apresentar(&quem).await {
            debug!("apresentação ao amigo falhou: {e:#}");
        }
        self.inner.amigos.perguntar(self.router.endpoint(), quem).await
    }

    /// Telemóvel: apresenta-se a outro nó como dispositivo da conta (ver
    /// [`crate::dispositivo`]). No PC (chave da conta) não faz nada.
    async fn apresentar(&self, a: &iroh::EndpointAddr) -> Result<()> {
        match &self.inner.certificado {
            Some(c) => crate::dispositivo::apresentar(self.router.endpoint(), a.clone(), c).await,
            None => Ok(()),
        }
    }

    /// `true` se este nó é um dispositivo extra da conta (não usa a chave da conta).
    pub fn e_dispositivo(&self) -> bool {
        self.inner.certificado.is_some()
    }

    /// Endereço deste nó (para testes e ligações na rede local).
    pub fn addr(&self) -> iroh::EndpointAddr {
        self.router.endpoint().addr()
    }

    pub fn identity(&self) -> &Identity {
        &self.inner.identity
    }

    /// Esquece um grupo neste dispositivo (sair do grupo, ou apagá-lo se fores o dono).
    pub fn forget_group(&self, group: GroupId) -> Result<()> {
        self.inner.store().forget_group(group)
    }

    /// Acesso à base de dados local (para ler histórico, nomes, etc.).
    pub fn with_store<T>(&self, f: impl FnOnce(&Store) -> T) -> T {
        f(&self.inner.store())
    }

    /// Cria um grupo novo hospedado neste PC.
    pub fn create_group(&self, name: &str, owner_name: &str) -> Result<GroupId> {
        let ev = SignedEvent::sign(
            &self.inner.identity,
            None,
            EventKind::GroupCreated {
                name: name.trim().to_string(),
                owner_name: owner_name.trim().to_string(),
                nonce: rand_bytes(),
            },
        );
        let mut store = self.inner.store();
        let group = store.create_hosted_group(&ev)?;
        store.set_group_key(group, 0, &crate::caixa::ChaveCaixa::generate()?, 0)?;
        Ok(group)
    }

    /// Anfitrião: troca a chave da caixa do grupo (por exemplo, depois de expulsar alguém).
    /// Os eventos seguintes vão para uma caixa nova; os membros recebem a chave nova quando
    /// se voltarem a ligar.
    pub fn rotate_group_key(&self, group: GroupId) -> Result<u32> {
        let store = self.inner.store();
        let info = store.group(group)?.ok_or_else(|| anyhow!("grupo desconhecido"))?;
        ensure!(info.hosted, "só o anfitrião pode trocar a chave");
        let (epoch, _, mirrored, _) = store.group_key(group)?.ok_or_else(|| anyhow!("grupo sem chave"))?;
        let next = epoch + 1;
        store.set_group_key(group, next, &crate::caixa::ChaveCaixa::generate()?, mirrored)?;
        Ok(next)
    }

    /// Abre um grupo de que somos membro sem ligar ao anfitrião (por exemplo, com ele
    /// desligado). As mensagens vão para a caixa do servidor e o anfitrião aceita-as quando voltar.
    pub fn open_offline(&self, group: GroupId) -> Result<GroupHandle> {
        ensure!(self.inner.store().group(group)?.is_some(), "grupo desconhecido");
        Ok(GroupHandle { group, inner: self.inner.clone(), remote: None, alive: None })
    }

    /// Gera um convite novo para um grupo que este PC hospeda.
    pub async fn create_invite(&self, group: GroupId) -> Result<Invite> {
        let info = self
            .inner
            .store()
            .group(group)?
            .ok_or_else(|| anyhow!("grupo desconhecido"))?;
        if !info.hosted {
            bail!("só o anfitrião pode criar convites");
        }
        if !self.local_only {
            // Espera até termos um relay e endereços públicos para pôr no convite.
            tokio::time::timeout(
                std::time::Duration::from_secs(15),
                self.router.endpoint().online(),
            )
            .await
            .ok();
        }
        let token = rand_bytes();
        self.inner.store().add_invite(group, token)?;
        Ok(Invite {
            host: self.router.endpoint().addr(),
            group,
            token,
        })
    }

    /// Quem está online num grupo e em que sala de voz. Muda quando alguém liga, desliga
    /// ou entra numa sala; vazio enquanto um membro não está ligado ao anfitrião.
    pub fn presence(&self, group: GroupId) -> watch::Receiver<Presence> {
        self.inner.presence(group).subscribe()
    }

    /// Recebe os eventos novos de um grupo (quer sejamos anfitrião, quer membro).
    pub fn subscribe(&self, group: GroupId) -> broadcast::Receiver<Update> {
        self.inner.feed(group).subscribe()
    }

    /// Abre um grupo que este PC hospeda, para enviar mensagens como dono.
    pub fn open_hosted(&self, group: GroupId) -> Result<GroupHandle> {
        let info = self
            .inner
            .store()
            .group(group)?
            .ok_or_else(|| anyhow!("grupo desconhecido"))?;
        if !info.hosted {
            bail!("este grupo não é hospedado aqui");
        }
        let me = *self.inner.identity.public().as_bytes();
        self.inner.presence(group).send_if_modified(|p| {
            if p.contains_key(&me) {
                return false;
            }
            p.insert(me, None);
            true
        });
        Ok(GroupHandle {
            group,
            inner: self.inner.clone(),
            remote: None,
            alive: None,
        })
    }

    /// Liga-se ao anfitrião de um grupo usando um convite, e entra se ainda não for membro.
    pub async fn join(&self, invite: &Invite, display_name: &str) -> Result<GroupHandle> {
        let handle = self
            .connect(invite.host.clone(), invite.group, Some(invite.token))
            .await?;
        let me = self.inner.identity.public();
        if !self.inner.store().is_member(invite.group, &me)? {
            handle
                .publish(EventKind::MemberJoined {
                    display_name: display_name.trim().to_string(),
                })
                .await?;
        }
        Ok(handle)
    }

    /// Volta a ligar-se ao anfitrião de um grupo de que já somos membro (sem convite).
    /// O anfitrião é encontrado pela chave do dono, que é o id do nó dele no iroh.
    pub async fn reconnect(&self, group: GroupId) -> Result<GroupHandle> {
        let info = self
            .inner
            .store()
            .group(group)?
            .ok_or_else(|| anyhow!("grupo desconhecido"))?;
        ensure!(!info.hosted, "este grupo é hospedado aqui");
        self.connect(iroh::EndpointAddr::new(info.owner), group, None)
            .await
    }

    async fn connect(
        &self,
        addr: iroh::EndpointAddr,
        group: GroupId,
        invite_token: Option<[u8; 16]>,
    ) -> Result<GroupHandle> {
        self.apresentar(&addr).await?;
        let conn = self
            .router
            .endpoint()
            .connect(addr, ALPN)
            .await
            .map_err(|e| anyhow!("não consegui ligar ao anfitrião: {e}"))?;
        let (mut send, mut recv) = conn.open_bi().await?;
        let since = self.inner.store().last_seq(group)?;
        write_frame(
            &mut send,
            &ClientMsg::Hello {
                group,
                invite_token,
                since,
            },
        )
        .await?;
        match read_frame::<HostMsg>(&mut recv).await? {
            Some(HostMsg::Welcome) => {}
            Some(HostMsg::Rejected(why)) => bail!("o anfitrião recusou: {why}"),
            other => bail!("resposta inesperada do anfitrião: {other:?}"),
        }

        // `true` enquanto a ligação ao anfitrião estiver viva.
        let (alive_tx, alive_rx) = watch::channel(true);

        // Tarefa que recebe eventos do anfitrião e os guarda na réplica local.
        let inner = self.inner.clone();
        let feed = inner.feed(group);
        tokio::spawn(async move {
            let _conn = conn; // mantém a ligação viva enquanto lemos
            loop {
                match read_frame::<HostMsg>(&mut recv).await {
                    Ok(Some(HostMsg::Event { seq, event })) => {
                        let stored = inner.store().insert_replica(group, seq, &event);
                        match stored {
                            Ok(true) => {
                                let _ = feed.send((seq, event));
                            }
                            Ok(false) => {}
                            Err(e) => warn!("evento {seq} rejeitado: {e:#}"),
                        }
                    }
                    Ok(Some(HostMsg::GroupKey { epoch, key })) => {
                        let key = crate::caixa::ChaveCaixa::from_bytes(key);
                        if let Err(e) = inner.store().set_group_key(group, epoch, &key, 0) {
                            warn!("não consegui guardar a chave do grupo: {e:#}");
                        }
                    }
                    Ok(Some(HostMsg::PublishFailed(why))) => {
                        warn!("o anfitrião rejeitou a mensagem: {why}")
                    }
                    Ok(Some(HostMsg::Presence(p))) => {
                        inner.presence(group).send_replace(p);
                    }
                    Ok(Some(other)) => debug!("mensagem ignorada: {other:?}"),
                    Ok(None) => break,
                    Err(e) => {
                        warn!("ligação ao anfitrião caiu: {e:#}");
                        break;
                    }
                }
            }
            let _ = alive_tx.send(false);
            inner.presence(group).send_replace(Presence::new());
        });

        Ok(GroupHandle {
            group,
            inner: self.inner.clone(),
            remote: Some(Arc::new(tokio::sync::Mutex::new(send))),
            alive: Some(alive_rx),
        })
    }

    /// Entra numa sala de voz de um grupo. Se alguém já está a alojar a sala, liga-se a
    /// essa pessoa; se não, passa a alojá-la este dispositivo (quem entra primeiro aloja).
    pub async fn entrar_voz(&self, group: GroupId, room: &str) -> Result<crate::voz::Chamada> {
        if self.voz.hospeda(group, room) {
            return Ok(self.chamada_local(group, room));
        }
        let privada = self.inner.privadas.lock().unwrap().get(&group).cloned();
        if let Some((amigo, onde)) = privada {
            // Chamada privada: a chamada está no dispositivo de onde nos ligaram ou no PC
            // do amigo; se não estiver em nenhum, fica aqui.
            let alvos = onde
                .into_iter()
                .map(|a| (a, std::time::Duration::from_secs(3)))
                .chain(std::iter::once((iroh::EndpointAddr::new(amigo), std::time::Duration::ZERO)));
            for (alvo, espera) in alvos {
                if let Ok(c) = self.entrar_voz_em(group, room, alvo, espera).await {
                    return Ok(c);
                }
            }
            return Ok(self.chamada_local(group, room));
        }
        self.inner
            .store()
            .group(group)?
            .ok_or_else(|| anyhow!("grupo desconhecido"))?;
        let mut tentados = Vec::new();
        for perguntar in [false, true] {
            if perguntar {
                self.perguntar_salas(group).await;
            }
            let candidatos: Vec<_> = self
                .salas_conhecidas(group)
                .into_iter()
                .filter(|a| a.room == room && a.endpoint != self.inner.endpoint_id && !tentados.contains(&a.endpoint))
                .collect();
            for a in candidatos {
                tentados.push(a.endpoint);
                let Ok(addr) = a.addr() else { continue };
                match self.entrar_voz_em(group, room, addr, std::time::Duration::from_secs(4)).await {
                    Ok(c) => return Ok(c),
                    Err(e) => {
                        debug!("voz: {} não respondeu: {e:#}", hex::encode(&a.endpoint[..4]));
                        // Já não aloja: esquece o anúncio.
                        self.inner.anuncios.lock().unwrap().retain(|_, x| x.endpoint != a.endpoint || x.room != room);
                    }
                }
            }
        }
        Ok(self.chamada_local(group, room))
    }

    fn chamada_local(&self, group: GroupId, room: &str) -> crate::voz::Chamada {
        let me = *self.inner.identity.public().as_bytes();
        crate::voz::Chamada::local(self.voz.clone(), group, room, me)
    }

    /// Chamada privada com um amigo. `onde`: dispositivo de onde ele nos ligou (de um
    /// [`crate::voz::Toque`]). Se a chamada ainda não existir, fica a ser alojada aqui e o
    /// PC do amigo recebe um toque.
    pub async fn ligar_privado(&self, amigo: iroh::PublicKey, onde: Option<iroh::EndpointAddr>) -> Result<crate::voz::Chamada> {
        let eu = self.inner.identity.public();
        let g = crate::voz::grupo_privado(&eu, &amigo);
        self.inner.privadas.lock().unwrap().insert(g, (amigo, onde));
        let c = self.entrar_voz(g, "privada").await?;
        if c.e_anfitriao() {
            let node = self.clone();
            tokio::spawn(async move {
                let r: Result<()> = async {
                    let addr = iroh::EndpointAddr::new(amigo);
                    let _ = node.apresentar(&addr).await;
                    let conn = tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        node.router.endpoint().connect(addr, crate::voz::VOZ_ALPN),
                    )
                    .await??;
                    let (mut send, mut recv) = conn.open_bi().await?;
                    write_frame(&mut send, &crate::voz::VozCliente::Tocar { group: g }).await?;
                    send.finish()?;
                    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), recv.read_to_end(16)).await;
                    conn.close(0u32.into(), b"ok");
                    Ok(())
                }
                .await;
                if let Err(e) = r {
                    debug!("toque não chegou: {e:#}");
                }
            });
        }
        Ok(c)
    }

    /// Chamadas privadas a chegar.
    pub fn toques(&self) -> broadcast::Receiver<crate::voz::Toque> {
        self.inner.toques.subscribe()
    }

    /// Liga-se à chamada alojada num dispositivo concreto (por exemplo depois de um
    /// [`crate::voz::NovoHost`]). Se for este dispositivo, passa a alojá-la. Enquanto o
    /// outro lado ainda não estiver pronto, tenta de novo até `espera`.
    pub async fn entrar_voz_em(
        &self,
        group: GroupId,
        room: &str,
        alvo: iroh::EndpointAddr,
        espera: std::time::Duration,
    ) -> Result<crate::voz::Chamada> {
        let limite = tokio::time::Instant::now() + espera;
        let mut alvo = alvo;
        let mut saltos = 0;
        loop {
            if *alvo.id.as_bytes() == self.inner.endpoint_id {
                return Ok(self.chamada_local(group, room));
            }
            let r: Result<crate::voz::Chamada> = async {
                let addr = alvo.clone();
                self.apresentar(&addr).await?;
                let conn = tokio::time::timeout(
                    std::time::Duration::from_secs(8),
                    self.router.endpoint().connect(addr, crate::voz::VOZ_ALPN),
                )
                .await
                .map_err(|_| anyhow!("quem aloja a chamada não respondeu"))?
                .map_err(|e| anyhow!("não consegui ligar a quem aloja a chamada: {e}"))?;
                crate::voz::Chamada::remota(conn, group, room).await
            }
            .await;
            match r {
                Ok(c) => return Ok(c),
                Err(e) => {
                    if let Some(crate::voz::Redirecionar(n)) = e.downcast_ref::<crate::voz::Redirecionar>() {
                        saltos += 1;
                        if saltos > 4 {
                            bail!("a chamada mudou de anfitrião demasiadas vezes");
                        }
                        alvo = n.addr()?;
                        continue;
                    }
                    if tokio::time::Instant::now() >= limite {
                        return Err(e);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                }
            }
        }
    }

    /// Pergunta ao dono do grupo e a alguns membros quem está a alojar salas de voz.
    async fn perguntar_salas(&self, group: GroupId) {
        let Ok(Some(info)) = self.inner.store().group(group) else { return };
        let eu = self.inner.identity.public();
        let mut alvos = vec![info.owner];
        for (m, _) in self.inner.store().members(group).unwrap_or_default() {
            if !alvos.contains(&m) {
                alvos.push(m);
            }
        }
        alvos.retain(|k| *k != eu && *k.as_bytes() != self.inner.endpoint_id);
        alvos.truncate(16);
        let mut tarefas = tokio::task::JoinSet::new();
        for alvo in alvos {
            let node = self.clone();
            tarefas.spawn(async move {
                let r: Result<Vec<crate::voz::Anuncio>> = async {
                    let addr = iroh::EndpointAddr::new(alvo);
                    let _ = node.apresentar(&addr).await;
                    let conn = node.router.endpoint().connect(addr, crate::voz::VOZ_ALPN).await?;
                    let (mut send, mut recv) = conn.open_bi().await?;
                    write_frame(&mut send, &crate::voz::VozCliente::Quem { group }).await?;
                    let r = read_frame::<crate::voz::VozAnfitriao>(&mut recv).await?;
                    conn.close(0u32.into(), b"obrigado");
                    match r {
                        Some(crate::voz::VozAnfitriao::Conhecidos(l)) => Ok(l),
                        _ => Ok(Vec::new()),
                    }
                }
                .await;
                r.unwrap_or_default()
            });
        }
        let _ = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while let Some(r) = tarefas.join_next().await {
                for a in r.unwrap_or_default() {
                    if a.group == group {
                        self.inner.guardar_anuncio(a);
                    }
                }
            }
        })
        .await;
    }

    /// A pasta dos anexos deste dispositivo.
    pub fn pasta(&self) -> Option<&crate::ficheiros::Pasta> {
        self.inner.pasta.as_ref()
    }

    /// Caminho local de um anexo, pedindo-o a quem o tiver se ainda não está cá: primeiro
    /// a quem o enviou, depois ao dono do grupo e a quem está online.
    pub async fn obter_ficheiro(&self, group: GroupId, anexo: &crate::event::Anexo) -> Result<std::path::PathBuf> {
        let pasta = self.inner.pasta.clone().ok_or_else(|| anyhow!("sem pasta de ficheiros"))?;
        if pasta.tem(&anexo.hash) {
            return Ok(pasta.caminho(&anexo.hash));
        }
        ensure!(anexo.size <= crate::ficheiros::MAX_FICHEIRO, "ficheiro demasiado grande");
        let mut alvos: Vec<iroh::PublicKey> = Vec::new();
        if let Ok(k) = iroh::PublicKey::from_bytes(&anexo.de) {
            alvos.push(k);
        }
        if let Ok(Some(info)) = self.inner.store().group(group) {
            alvos.push(info.owner);
        }
        for k in self.inner.presence(group).borrow().keys() {
            if let Ok(k) = iroh::PublicKey::from_bytes(k) {
                alvos.push(k);
            }
        }
        let mut vistos = Vec::new();
        let mut ultimo = anyhow!("ninguém online tem este ficheiro");
        for k in alvos {
            if vistos.contains(&k) || *k.as_bytes() == self.inner.endpoint_id || vistos.len() >= 8 {
                continue;
            }
            vistos.push(k);
            let addr = iroh::EndpointAddr::new(k);
            let _ = self.apresentar(&addr).await;
            match crate::ficheiros::pedir(self.router.endpoint(), addr, &pasta, group, anexo.hash).await {
                Ok(p) => return Ok(p),
                Err(e) => ultimo = e,
            }
        }
        Err(ultimo)
    }

    /// Chave iroh deste dispositivo (vai nos anexos, para pedirem o ficheiro aqui).
    pub fn endpoint_id(&self) -> [u8; 32] {
        self.inner.endpoint_id
    }

    /// As salas de voz que se sabe estarem ativas num grupo, com quem aloja cada uma e
    /// quem lá está (as deste dispositivo e as que outros anunciaram).
    pub fn salas_conhecidas(&self, group: GroupId) -> Vec<crate::voz::Anuncio> {
        self.inner.anuncios_de(group, &self.voz)
    }

    /// Passa a chamada que este dispositivo aloja para outro (o mesmo que
    /// [`crate::voz::Chamada::passar_host`], mas para fundir chamadas partidas).
    fn juntar_chamadas(&self, a: &crate::voz::Anuncio) {
        if !a.ativo() || !self.voz.hospeda(a.group, &a.room) {
            return;
        }
        let nossa = self
            .voz
            .anuncios(&self.inner.identity, &[])
            .into_iter()
            .find(|x| x.group == a.group && x.room == a.room);
        if let Some(n) = nossa {
            // Duas chamadas na mesma sala: fica a que começou primeiro.
            if (a.desde, a.endpoint) < (n.desde, n.endpoint) {
                self.voz.mudar_para(
                    a.group,
                    &a.room,
                    crate::voz::NovoHost { endpoint: a.endpoint, conta: a.conta, enderecos: a.enderecos.clone() },
                );
            }
        }
    }

    /// As salas de voz que este dispositivo aloja num grupo e quem está em cada uma.
    pub fn salas_voz(&self, group: GroupId) -> Vec<(String, Vec<crate::voz::Participante>)> {
        self.voz.salas(group)
    }

    /// Anfitrião: muda sempre que alguém entra, sai ou muda de estado numa sala de voz.
    pub fn watch_voz(&self) -> watch::Receiver<u64> {
        self.voz.watch()
    }

    pub async fn shutdown(self) -> Result<()> {
        self.router.shutdown().await.map_err(|e| anyhow!("{e}"))
    }
}

/// Um grupo aberto, onde se pode publicar.
#[derive(Clone)]
pub struct GroupHandle {
    pub group: GroupId,
    inner: Arc<Inner>,
    /// `None` quando somos o anfitrião (ou o grupo foi aberto sem anfitrião).
    remote: Option<Arc<tokio::sync::Mutex<SendStream>>>,
    /// Estado da ligação ao anfitrião (só quando `remote` existe).
    alive: Option<watch::Receiver<bool>>,
}

impl GroupHandle {
    /// Está ligado diretamente ao anfitrião (ou é o próprio anfitrião).
    pub fn is_connected(&self) -> bool {
        match &self.alive {
            Some(rx) => *rx.borrow(),
            None => self.remote.is_none() && self.is_hosted(),
        }
    }

    /// Este PC é o anfitrião do grupo.
    pub fn is_hosted(&self) -> bool {
        self.inner
            .store()
            .group(self.group)
            .ok()
            .flatten()
            .is_some_and(|g| g.hosted)
    }

    /// Espera até a ligação ao anfitrião cair. Sem ligação (anfitrião ou grupo aberto
    /// sem anfitrião), volta logo.
    pub async fn disconnected(&self) {
        if let Some(rx) = &self.alive {
            let mut rx = rx.clone();
            let _ = rx.wait_for(|v| !*v).await;
        }
    }

    /// Diz ao grupo em que sala de voz estás (`None` = saíste).
    pub async fn set_voice(&self, channel: Option<String>) -> Result<()> {
        if let Some(send) = &self.remote {
            return write_frame(&mut *send.lock().await, &ClientMsg::Voice(channel)).await;
        }
        if self.is_hosted() {
            let me = *self.inner.identity.public().as_bytes();
            self.inner.presence(self.group).send_modify(|p| {
                p.insert(me, channel);
            });
        }
        Ok(())
    }

    /// Publica o teu perfil neste grupo (nome, cor e foto).
    pub async fn set_profile(&self, name: &str, color: &str, photo: Option<String>) -> Result<()> {
        self.publish(EventKind::Profile {
            name: name.trim().to_string(),
            color: color.to_string(),
            photo,
        })
        .await?;
        Ok(())
    }

    /// Publica o teu cartão de perfil neste grupo (@nome, biografia e banner).
    pub async fn set_profile_card(&self, card: crate::store::ProfileCard) -> Result<()> {
        self.publish(EventKind::ProfileCard { username: card.username, bio: card.bio, banner: card.banner })
            .await?;
        Ok(())
    }

    /// Dono: publica a disposição nova de canais e categorias.
    pub async fn set_layout(
        &self,
        channels: Vec<crate::event::ChannelDef>,
        categories: Vec<String>,
    ) -> Result<()> {
        crate::store::check_layout(&channels, &categories)?;
        self.publish(EventKind::Layout { channels, categories }).await?;
        Ok(())
    }

    /// Envia uma mensagem. Devolve o id do evento (o mesmo que chega depois pelo `subscribe`).
    pub async fn send_message(&self, channel: &str, text: &str) -> Result<[u8; 32]> {
        self.publish(EventKind::Message {
            channel: channel.into(),
            text: text.into(),
        })
        .await
    }

    /// Envia uma mensagem com resposta e/ou anexos.
    pub async fn post(
        &self,
        channel: &str,
        text: &str,
        reply_to: Option<[u8; 32]>,
        files: Vec<crate::event::Anexo>,
    ) -> Result<[u8; 32]> {
        if reply_to.is_none() && files.is_empty() {
            return self.send_message(channel, text).await;
        }
        self.publish(EventKind::Post { channel: channel.into(), text: text.into(), reply_to, files }).await
    }

    /// Muda o texto de uma mensagem tua.
    pub async fn edit_message(&self, target: [u8; 32], text: &str) -> Result<[u8; 32]> {
        self.publish(EventKind::Edit { target, text: text.into() }).await
    }

    /// Apaga uma mensagem (tua, ou de qualquer pessoa se fores o dono).
    pub async fn delete_message(&self, target: [u8; 32]) -> Result<[u8; 32]> {
        self.publish(EventKind::Delete { target }).await
    }

    /// Põe ou tira uma reação.
    pub async fn react(&self, target: [u8; 32], emoji: &str, on: bool) -> Result<[u8; 32]> {
        self.publish(EventKind::React { target, emoji: emoji.into(), on }).await
    }

    async fn publish(&self, kind: EventKind) -> Result<[u8; 32]> {
        let ev = SignedEvent::sign(&self.inner.identity, Some(self.group), kind);
        let id = ev.id();
        if let Some(send) = &self.remote {
            let sent = write_frame(&mut *send.lock().await, &ClientMsg::Publish(ev.clone())).await;
            match sent {
                Ok(()) => return Ok(id),
                // Anfitrião caiu: tenta pela caixa do servidor.
                Err(e) if !self.inner.has_server() => return Err(e),
                Err(e) => warn!("anfitrião indisponível ({e:#}); a enviar pelo servidor"),
            }
        }
        let hosted = self.inner.store().group(self.group)?.is_some_and(|g| g.hosted);
        if hosted {
            let seq = self.inner.store().append_as_host(&ev, false)?;
            if let Some(seq) = seq {
                let _ = self.inner.feed(self.group).send((seq, ev));
            }
            return Ok(id);
        }
        self.inner.send_request(self.group, ev).await?;
        Ok(id)
    }
}

/// Lado do anfitrião: atende membros que se ligam.
#[derive(Clone)]
struct HostProtocol {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for HostProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostProtocol")
    }
}

impl ProtocolHandler for HostProtocol {
    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        self.serve(conn)
            .await
            .map_err(|e| AcceptError::from_boxed(e.into()))
    }
}

impl HostProtocol {
    async fn serve(&self, conn: Connection) -> Result<()> {
        // A chave da ligação QUIC é a identidade do membro (ou de um dispositivo dele que se
        // apresentou com certificado): não dá para fingir ser outro.
        let remote = self.inner.conta_de(&conn.remote_id());
        let (mut send, mut recv) = conn.accept_bi().await?;
        let Some(ClientMsg::Hello {
            group,
            invite_token,
            since,
        }) = read_frame(&mut recv).await?
        else {
            bail!("esperava Hello");
        };

        let admitted = {
            let store = self.inner.store();
            match store.group(group)? {
                Some(info) if info.hosted => {
                    let by_invite = match &invite_token {
                        Some(t) => store.invite_valid(group, t)?,
                        None => false,
                    };
                    if by_invite || store.is_member(group, &remote)? {
                        Some(by_invite)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        };
        let Some(by_invite) = admitted else {
            write_frame(
                &mut send,
                &HostMsg::Rejected("convite inválido ou grupo inexistente".into()),
            )
            .await?;
            send.finish()?;
            conn.closed().await;
            return Ok(());
        };
        write_frame(&mut send, &HostMsg::Welcome).await?;

        // Fica online enquanto esta ligação durar.
        let presence = self.inner.presence(group);
        let _online = Online::new(self.inner.clone(), group, *remote.as_bytes());
        let mut pres_rx = presence.subscribe();

        // Subscreve antes de ler o histórico, para não perder nada entretanto.
        let mut feed = self.inner.feed(group).subscribe();
        let mut last_sent = since;
        let backlog = self.inner.store().events_since(group, since)?;
        for (seq, event) in backlog {
            write_frame(&mut send, &HostMsg::Event { seq, event }).await?;
            last_sent = seq;
        }
        let key = self.inner.store().group_key(group)?;
        if let Some((epoch, key, _, _)) = key {
            write_frame(&mut send, &HostMsg::GroupKey { epoch, key: *key.as_bytes() }).await?;
        }
        let atual = pres_rx.borrow_and_update().clone();
        write_frame(&mut send, &HostMsg::Presence(atual)).await?;

        loop {
            tokio::select! {
                update = feed.recv() => match update {
                    Ok((seq, event)) => {
                        if seq > last_sent {
                            write_frame(&mut send, &HostMsg::Event { seq, event }).await?;
                            last_sent = seq;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let missed = self.inner.store().events_since(group, last_sent)?;
                        for (seq, event) in missed {
                            write_frame(&mut send, &HostMsg::Event { seq, event }).await?;
                            last_sent = seq;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                changed = pres_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let atual = pres_rx.borrow_and_update().clone();
                    write_frame(&mut send, &HostMsg::Presence(atual)).await?;
                }
                msg = read_frame::<ClientMsg>(&mut recv) => match msg? {
                    Some(ClientMsg::Publish(ev)) => {
                        if ev.body.author != *remote.as_bytes() {
                            write_frame(&mut send, &HostMsg::PublishFailed("autor não corresponde à ligação".into())).await?;
                            continue;
                        }
                        if ev.body.group != Some(group) {
                            write_frame(&mut send, &HostMsg::PublishFailed("grupo errado".into())).await?;
                            continue;
                        }
                        let result = self.inner.store().append_as_host(&ev, by_invite);
                        match result {
                            Ok(Some(seq)) => { let _ = self.inner.feed(group).send((seq, ev)); }
                            Ok(None) => {}
                            Err(e) => write_frame(&mut send, &HostMsg::PublishFailed(format!("{e:#}"))).await?,
                        }
                    }
                    Some(ClientMsg::Voice(channel)) => {
                        let who = *remote.as_bytes();
                        presence.send_if_modified(|p| {
                            let channel = channel.filter(|c| c.chars().count() <= 64);
                            if p.get(&who) == Some(&channel) {
                                return false;
                            }
                            p.insert(who, channel);
                            true
                        });
                    }
                    Some(ClientMsg::Hello { .. }) => bail!("Hello repetido"),
                    None => break,
                },
            }
        }
        Ok(())
    }
}

/// Anfitrião: marca um membro como online enquanto uma ligação dele estiver aberta.
struct Online {
    inner: Arc<Inner>,
    group: GroupId,
    who: [u8; 32],
}

impl Online {
    fn new(inner: Arc<Inner>, group: GroupId, who: [u8; 32]) -> Self {
        *inner.conns.lock().unwrap().entry((group, who)).or_default() += 1;
        inner.presence(group).send_if_modified(|p| {
            if p.contains_key(&who) {
                return false;
            }
            p.insert(who, None);
            true
        });
        Self { inner, group, who }
    }
}

impl Drop for Online {
    fn drop(&mut self) {
        let mut conns = self.inner.conns.lock().unwrap();
        let n = conns.entry((self.group, self.who)).or_default();
        *n = n.saturating_sub(1);
        if *n == 0 {
            conns.remove(&(self.group, self.who));
            // o telemóvel do próprio anfitrião saiu: o anfitrião continua online
            if self.who == *self.inner.identity.public().as_bytes() {
                return;
            }
            self.inner.presence(self.group).send_modify(|p| {
                p.remove(&self.who);
            });
        }
    }
}

impl Inner {
    fn has_server(&self) -> bool {
        #[cfg(feature = "servidor")]
        return self.servidor.is_some();
        #[cfg(not(feature = "servidor"))]
        false
    }

    /// Põe um anúncio de voz na caixa do grupo (sem servidor, não faz nada).
    async fn publicar_anuncio(&self, a: crate::voz::Anuncio) -> Result<()> {
        #[cfg(feature = "servidor")]
        if let Some(srv) = &self.servidor {
            let Some((_, key, _, _)) = self.store().group_key(a.group)? else { return Ok(()) };
            let bloco = key.seal_item(&crate::caixa::ItemGrupo::Voz(a))?;
            srv.caixa_enviar_bloco(&key, bloco).await?;
        }
        #[cfg(not(feature = "servidor"))]
        let _ = a;
        Ok(())
    }

    /// Membro com o anfitrião desligado: deixa o evento na caixa como pedido.
    async fn send_request(&self, group: GroupId, ev: SignedEvent) -> Result<()> {
        #[cfg(feature = "servidor")]
        if let Some(srv) = &self.servidor {
            let (_, key, _, _) = self
                .store()
                .group_key(group)?
                .ok_or_else(|| anyhow!("ainda não tenho a chave deste grupo; liga-te uma vez ao anfitrião"))?;
            let bloco = key.seal_item(&crate::caixa::ItemGrupo::Pedido { event: ev })?;
            srv.caixa_enviar_bloco(&key, bloco).await?;
            return Ok(());
        }
        let _ = (group, ev);
        bail!("o anfitrião do grupo está desligado")
    }
}

#[cfg(feature = "servidor")]
impl Node {
    /// Sincroniza um grupo com a caixa no servidor. Devolve quantos eventos novos entraram
    /// na base de dados local.
    ///
    /// - Anfitrião: aceita os pedidos que os membros deixaram e copia para a caixa os
    ///   eventos que ainda lá não estão (assinados por ele, com o número de ordem).
    /// - Membro: lê da caixa os eventos que o anfitrião já aceitou.
    ///
    /// `espera` (segundos) deixa o pedido de leitura à espera de novidades.
    pub async fn sync_group(&self, group: GroupId, espera: u32) -> Result<usize> {
        use crate::caixa::ItemGrupo;
        let srv = self.inner.servidor.clone().ok_or_else(|| anyhow!("sem servidor configurado"))?;
        let Some((epoch, key, mut mirrored, mut read)) = self.inner.store().group_key(group)? else {
            return Ok(0);
        };
        let info = self.inner.store().group(group)?;
        let hosted = info.as_ref().is_some_and(|g| g.hosted);
        let mut novos = 0;

        for (pos, bloco) in srv.caixa_ler_blocos(&key, read, if hosted { 0 } else { espera }).await? {
            read = pos;
            let item = match key.open_item(&bloco) {
                Ok(i) => i,
                Err(e) => {
                    warn!("entrada {pos} da caixa ignorada: {e:#}");
                    continue;
                }
            };
            match (hosted, item) {
                (true, ItemGrupo::Pedido { event }) => {
                    if event.body.group != Some(group) {
                        continue;
                    }
                    let r = self.inner.store().append_as_host(&event, false);
                    match r {
                        Ok(Some(seq)) => {
                            novos += 1;
                            let _ = self.inner.feed(group).send((seq, event));
                        }
                        Ok(None) => {}
                        Err(e) => warn!("pedido recusado: {e:#}"),
                    }
                }
                (_, ItemGrupo::Voz(a)) => {
                    if a.group == group && self.inner.guardar_anuncio(a.clone()) {
                        self.juntar_chamadas(&a);
                        self.voz.tocar();
                    }
                }
                (false, item @ ItemGrupo::Registo { .. }) => {
                    let ItemGrupo::Registo { seq, event, .. } = &item else { unreachable!() };
                    // O dono vem do histórico; no primeiro evento, do próprio GroupCreated.
                    let dono = match self.inner.store().group(group)? {
                        Some(g) => g.owner,
                        None if *seq == 1 && event.group_id() == group => event.author()?,
                        None => continue,
                    };
                    if let Err(e) = item.verify_registo(&group, &dono) {
                        warn!("{e:#}");
                        continue;
                    }
                    let r = self.inner.store().insert_replica(group, *seq, event);
                    match r {
                        Ok(true) => {
                            novos += 1;
                            let _ = self.inner.feed(group).send((*seq, event.clone()));
                        }
                        Ok(false) => {}
                        Err(e) => warn!("registo {seq} não aplicado: {e:#}"),
                    }
                }
                _ => {}
            }
        }

        if hosted {
            let pendentes = self.inner.store().events_since(group, mirrored)?;
            for (seq, event) in pendentes {
                let item = ItemGrupo::registo(&self.inner.identity, &group, seq, event);
                srv.caixa_enviar_bloco(&key, key.seal_item(&item)?).await?;
                mirrored = seq;
                // Guarda o progresso a cada evento, para não repetir se a ligação cair.
                self.inner.store().set_key_cursors(group, epoch, mirrored, read)?;
            }
        }
        self.inner.store().set_key_cursors(group, epoch, mirrored, read)?;
        Ok(novos)
    }
}

fn rand_bytes<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    getrandom::fill(&mut out).expect("gerador aleatório do sistema");
    out
}
