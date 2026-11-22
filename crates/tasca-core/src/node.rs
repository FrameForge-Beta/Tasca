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
    /// Quantas dessas ligações vêm de um telemóvel (dispositivo com chave própria).
    conns_tel: Mutex<HashMap<(GroupId, [u8; 32]), usize>>,
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
    /// Quem um moderador silenciou ou ensurdeceu, por (grupo, conta). A sala de voz usa isto
    /// para não deixar passar o som, mesmo que o cliente da pessoa não obedeça.
    pub(crate) moderados: Arc<Mutex<HashMap<(GroupId, [u8; 32]), Moderado>>>,
    /// Eventos que o anfitrião recusou (por exemplo, o AutoMod): (grupo, motivo).
    recusas: broadcast::Sender<(GroupId, String)>,
    /// Última vez que a caixa de cada grupo foi lida do servidor. Com a leitura em dia,
    /// os anúncios das chamadas (que também vão para a caixa) estão todos cá.
    caixa_lida: Mutex<HashMap<GroupId, std::time::Instant>>,
    /// Grupo na caixa: a sala de voz onde estou em cada grupo (vai na presença).
    minhas_salas: Mutex<HashMap<GroupId, Option<String>>>,
    /// Convites com que estou a entrar (para abrir a chave nova se ela rodar entretanto).
    convites: Mutex<HashMap<GroupId, Vec<Identity>>>,
    /// Eventos meus que as regras do grupo recusaram: id → motivo.
    rejeitados: Mutex<HashMap<[u8; 32], String>>,
    /// Em que época mandei cada evento meu (para o reenviar se a chave mudar entretanto).
    enviados: Mutex<HashMap<[u8; 32], (GroupId, u32)>>,
    /// Quando rodei a chave de cada grupo pela última vez (para não repetir).
    rodadas: Mutex<HashMap<GroupId, std::time::Instant>>,
    /// Uma leitura da caixa de cada vez, por grupo.
    leituras: Mutex<HashMap<GroupId, Arc<tokio::sync::Mutex<()>>>>,
    /// Passagem dos grupos antigos para a caixa (uma de cada vez).
    migracao: tokio::sync::Mutex<()>,
}

/// O que um moderador fez a alguém nas chamadas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Moderado {
    pub muted: bool,
    pub deaf: bool,
    /// Foi tirado da chamada: não fala nem ouve até voltar a entrar.
    pub fora: bool,
}

impl Inner {
    /// Um evento novo entrou na base de dados: avisa quem está à escuta e aplica o que for
    /// preciso nas chamadas.
    fn evento_novo(&self, group: GroupId, seq: u64, ev: SignedEvent) {
        if let EventKind::VoiceMod { member, action } = &ev.body.kind {
            use crate::cargos::AcaoVoz;
            let mut m = self.moderados.lock().unwrap();
            let e = m.entry((group, *member)).or_default();
            match action {
                AcaoVoz::Silenciar(on) => e.muted = *on,
                AcaoVoz::Ensurdecer(on) => e.deaf = *on,
                AcaoVoz::Desligar | AcaoVoz::Mover(_) => e.fora = true,
                AcaoVoz::PararVideo => {}
            }
        }
        let _ = self.feed(group).send((seq, ev));
    }

    /// Anfitrião: valida e guarda um evento, e trata do que vem a seguir (AutoMod, troca da
    /// chave da caixa quando alguém é expulso ou banido).
    fn aceitar(&self, ev: SignedEvent, by_invite: Option<[u8; 16]>) -> Result<Option<u64>> {
        let group = ev.group_id();
        let r = self.store().append_as_host(&ev, by_invite.is_some());
        let caso = self.store().take_automod();
        if let Some(c) = caso {
            self.castigo_automod(group, c);
        }
        let seq = r?;
        if let Some(seq) = seq {
            if let (Some(t), EventKind::MemberJoined { .. }) = (by_invite, &ev.body.kind) {
                let _ = self.store().use_invite(group, &t);
            }
            let tirou = matches!(ev.body.kind, EventKind::Kick { .. } | EventKind::Ban { .. });
            self.evento_novo(group, seq, ev);
            if tirou {
                // quem saiu ainda tem a chave antiga: os eventos seguintes vão para uma caixa nova
                let store = self.store();
                if let Ok(Some((epoch, _, mirrored, _))) = store.group_key(group) {
                    if let Ok(k) = crate::caixa::ChaveCaixa::generate() {
                        let _ = store.set_group_key(group, epoch + 1, &k, mirrored);
                    }
                }
            }
        }
        Ok(seq)
    }

    /// AutoMod: castigo e alerta, assinados pelo anfitrião.
    fn castigo_automod(&self, group: GroupId, c: crate::store::AutoModCaso) {
        let nome = self
            .store()
            .display_name(group, &match iroh::PublicKey::from_bytes(&c.autor) {
                Ok(k) => k,
                Err(_) => return,
            })
            .ok()
            .flatten()
            .unwrap_or_else(|| "alguém".into());
        let mut novos = Vec::new();
        if c.bloqueio.timeout_secs > 0 {
            novos.push(EventKind::Timeout {
                member: c.autor,
                until_ms: crate::event::now_ms() + c.bloqueio.timeout_secs as u64 * 1000,
                reason: format!("AutoMod: {}", c.bloqueio.regra),
            });
        }
        if let Some(canal) = c.bloqueio.alert_channel.clone() {
            let texto: String = c.texto.chars().take(300).collect();
            novos.push(EventKind::Message {
                channel: canal,
                text: format!(
                    "🛡️ AutoMod ({}): {} de {nome} em #{}: «{texto}»",
                    c.bloqueio.regra,
                    if c.bloqueio.block { "bloqueei uma mensagem" } else { "mensagem marcada" },
                    c.canal
                ),
            });
        }
        for k in novos {
            let ev = SignedEvent::sign(&self.identity, Some(group), k);
            let r = self.store().append_as_host(&ev, false);
            match r {
                Ok(Some(seq)) => self.evento_novo(group, seq, ev),
                Ok(None) => {}
                Err(e) => warn!("AutoMod: {e:#}"),
            }
        }
    }

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
            conns_tel: Default::default(),
            amigos: Default::default(),
            certificado,
            anuncios: Default::default(),
            endpoint_id: *endpoint.id().as_bytes(),
            endpoint: endpoint.clone(),
            toques: broadcast::channel(16).0,
            privadas: Default::default(),
            pasta: opts.ficheiros.clone().map(crate::ficheiros::Pasta),
            moderados: Default::default(),
            recusas: broadcast::channel(16).0,
            caixa_lida: Default::default(),
            minhas_salas: Default::default(),
            convites: Default::default(),
            rejeitados: Default::default(),
            enviados: Default::default(),
            rodadas: Default::default(),
            leituras: Default::default(),
            migracao: Default::default(),
        });
        {
            // quem já estava silenciado por um moderador continua
            let store = inner.store();
            let mut m = inner.moderados.lock().unwrap();
            for g in store.groups().unwrap_or_default() {
                for (k, (muted, deaf)) in store.voice_mods(g.id).unwrap_or_default() {
                    m.insert((g.id, k), Moderado { muted, deaf, fora: false });
                }
            }
        }
        let voz = Arc::new(crate::voz::Hub::new(Arc::downgrade(&inner), *endpoint.id().as_bytes(), inner.moderados.clone()));
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
        if self.inner.has_server() {
            // o grupo vive na caixa: estou sempre "ligado"
            let me = *self.inner.identity.public().as_bytes();
            let sala = self.inner.minhas_salas.lock().unwrap().get(&group).cloned().flatten();
            self.inner.presence(group).send_if_modified(|p| {
                if p.get(&me) == Some(&sala) {
                    return false;
                }
                p.insert(me, sala);
                true
            });
        }
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
        ensure!(!self.inner.has_server(), "este convite é antigo; pede um convite novo");
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
        ensure!(!self.inner.has_server(), "o grupo vive na caixa do servidor");
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
        tokio::spawn(async move {
            let _conn = conn; // mantém a ligação viva enquanto lemos
            loop {
                match read_frame::<HostMsg>(&mut recv).await {
                    Ok(Some(HostMsg::Event { seq, event })) => {
                        let stored = inner.store().insert_replica(group, seq, &event);
                        match stored {
                            Ok(true) => {
                                inner.evento_novo(group, seq, event);
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
                        warn!("o anfitrião rejeitou a mensagem: {why}");
                        let _ = inner.recusas.send((group, why));
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
        // 1. Quem já se sabe que aloja a sala: os anúncios das chamadas chegam pelo servidor
        // (caixa do grupo), por isso com a caixa em dia não há nada a perguntar a ninguém.
        // 2. Sem servidor em dia: pergunta só ao dono do grupo, que recebe o anúncio de todas
        // as chamadas. 3. Dono desligado: pergunta aos membros.
        let eu_dono = self.inner.store().group(group)?.is_some_and(|g| g.owner == self.inner.identity.public());
        let em_dia = self
            .inner
            .caixa_lida
            .lock()
            .unwrap()
            .get(&group)
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(45));
        let mut tentados = Vec::new();
        for passo in 0..3 {
            match passo {
                0 => {}
                1 if eu_dono || em_dia => break,
                1 => {
                    if self.perguntar_dono(group).await {
                        let ha = self.salas_conhecidas(group).iter().any(|a| a.room == room && a.endpoint != self.inner.endpoint_id && !tentados.contains(&a.endpoint));
                        if !ha {
                            break;
                        }
                    }
                }
                _ => self.perguntar_salas(group, room).await,
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
    /// Eventos teus que o anfitrião recusou (sem permissão, AutoMod, castigo...).
    pub fn recusas(&self) -> broadcast::Receiver<(GroupId, String)> {
        self.inner.recusas.subscribe()
    }

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
                // Quem aloja uma chamada está online: se não responde depressa, já não está lá.
                let conn = tokio::time::timeout(
                    std::time::Duration::from_secs(4),
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

    /// Pergunta ao dono do grupo que salas de voz estão a ser alojadas e por quem.
    /// `true` se ele respondeu (então o que se sabe das salas está certo).
    async fn perguntar_dono(&self, group: GroupId) -> bool {
        let Ok(Some(info)) = self.inner.store().group(group) else { return false };
        let addr = iroh::EndpointAddr::new(info.owner);
        let r = tokio::time::timeout(std::time::Duration::from_millis(1500), async {
            let _ = self.apresentar(&addr).await;
            let conn = self.router.endpoint().connect(addr, crate::voz::VOZ_ALPN).await?;
            let (mut send, mut recv) = conn.open_bi().await?;
            write_frame(&mut send, &crate::voz::VozCliente::Quem { group }).await?;
            let r = read_frame::<crate::voz::VozAnfitriao>(&mut recv).await?;
            conn.close(0u32.into(), b"obrigado");
            anyhow::Ok(r)
        })
        .await;
        match r {
            Ok(Ok(Some(crate::voz::VozAnfitriao::Conhecidos(l)))) => {
                for a in l {
                    if a.group == group {
                        self.inner.guardar_anuncio(a);
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Pergunta ao dono do grupo e a alguns membros quem está a alojar salas de voz.
    /// Acaba logo que alguém diga quem aloja `room`, ou ao fim de pouco tempo: numa sala
    /// vazia ninguém responde com ela e quem entra fica a alojá-la sem esperar muito.
    async fn perguntar_salas(&self, group: GroupId, room: &str) {
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
        let _ = tokio::time::timeout(std::time::Duration::from_millis(1500), async {
            while let Some(r) = tarefas.join_next().await {
                let mut achou = false;
                for a in r.unwrap_or_default() {
                    if a.group == group {
                        achou |= a.room == room;
                        self.inner.guardar_anuncio(a);
                    }
                }
                if achou {
                    break;
                }
            }
        })
        .await;
        tarefas.abort_all();
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
        let online: Vec<[u8; 32]> = self.inner.presence(group).borrow().keys().copied().collect();
        for k in online {
            // (a presença também tem marcas de "só no telemóvel", que não são pessoas)
            if let Ok(k) = iroh::PublicKey::from_bytes(&k) {
                if self.inner.store().is_member(group, &k).unwrap_or(false) {
                    alvos.push(k);
                }
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
        #[cfg(feature = "servidor")]
        let _ = tokio::time::timeout(std::time::Duration::from_secs(3), self.inner.sair_presenca()).await;
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
        #[cfg(feature = "servidor")]
        if self.inner.has_server() {
            let channel = channel.filter(|c| c.chars().count() <= 64);
            self.inner.minhas_salas.lock().unwrap().insert(self.group, channel.clone());
            let me = *self.inner.identity.public().as_bytes();
            self.inner.presence(self.group).send_modify(|p| {
                p.insert(me, channel);
            });
            let (inner, g) = (self.inner.clone(), self.group);
            tokio::spawn(async move {
                if let Err(e) = inner.presenca(g).await {
                    debug!("presença não enviada: {e:#}");
                }
            });
            return Ok(());
        }
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

    // ---------------- cargos e moderação ----------------

    pub async fn set_roles(&self, roles: Vec<crate::cargos::Cargo>) -> Result<[u8; 32]> {
        self.publish(EventKind::Roles { roles }).await
    }

    pub async fn set_member_roles(&self, member: [u8; 32], roles: Vec<String>) -> Result<[u8; 32]> {
        self.publish(EventKind::MemberRoles { member, roles }).await
    }

    pub async fn set_channel_perms(&self, channel: String, overrides: Vec<crate::cargos::Excecao>) -> Result<[u8; 32]> {
        self.publish(EventKind::ChannelPerms { channel, overrides }).await
    }

    pub async fn kick(&self, member: [u8; 32], reason: String) -> Result<[u8; 32]> {
        self.publish(EventKind::Kick { member, reason }).await
    }

    pub async fn ban(&self, member: [u8; 32], reason: String, delete_secs: u64) -> Result<[u8; 32]> {
        self.publish(EventKind::Ban { member, reason, delete_secs }).await
    }

    pub async fn unban(&self, member: [u8; 32]) -> Result<[u8; 32]> {
        self.publish(EventKind::Unban { member }).await
    }

    pub async fn timeout(&self, member: [u8; 32], until_ms: u64, reason: String) -> Result<[u8; 32]> {
        self.publish(EventKind::Timeout { member, until_ms, reason }).await
    }

    pub async fn set_nick(&self, member: [u8; 32], nick: Option<String>) -> Result<[u8; 32]> {
        self.publish(EventKind::Nick { member, nick }).await
    }

    pub async fn pin(&self, target: [u8; 32], on: bool) -> Result<[u8; 32]> {
        self.publish(EventKind::Pin { target, on }).await
    }

    pub async fn set_settings(&self, d: crate::cargos::Definicoes) -> Result<[u8; 32]> {
        self.publish(EventKind::Settings(d)).await
    }

    pub async fn voice_mod(&self, member: [u8; 32], action: crate::cargos::AcaoVoz) -> Result<[u8; 32]> {
        self.publish(EventKind::VoiceMod { member, action }).await
    }

    pub async fn report(&self, target: [u8; 32], reason: String) -> Result<[u8; 32]> {
        self.publish(EventKind::Report { target, reason }).await
    }

    /// Sai do grupo (os outros deixam de te ver como membro).
    pub async fn leave(&self) -> Result<[u8; 32]> {
        self.publish(EventKind::Leave).await
    }

    /// Cria um convite e publica-o no grupo (só o hash do código). Quem tem permissão cria
    /// convites mesmo sem ser o anfitrião: o convite aponta para o PC do anfitrião.
    /// `max_uses` 0 = sem limite; `expires_ms` 0 = não expira.
    pub async fn create_invite(&self, max_uses: u32, expires_ms: u64) -> Result<Invite> {
        let info = self.inner.store().group(self.group)?.ok_or_else(|| anyhow!("grupo desconhecido"))?;
        let token: [u8; 16] = rand_bytes();
        let hash = *blake3::hash(&token).as_bytes();
        self.publish(EventKind::InviteCreated { hash, max_uses, expires_ms }).await?;
        let host = if info.hosted {
            self.inner.endpoint.addr()
        } else {
            iroh::EndpointAddr::new(info.owner)
        };
        Ok(Invite { host, group: self.group, token })
    }

    /// Cria um convite que funciona sem o dono ligado (o grupo vive na caixa do servidor).
    /// `max_uses` 0 = sem limite; `expires_ms` 0 = não expira.
    pub async fn criar_convite(&self, max_uses: u32, expires_ms: u64) -> Result<crate::invite::Convite> {
        let (epoch, chave, _, _) = self
            .inner
            .store()
            .group_key(self.group)?
            .ok_or_else(|| anyhow!("ainda não tenho a chave deste grupo"))?;
        let c = crate::invite::Convite { group: self.group, token: rand_bytes(), epoch, chave: *chave.as_bytes() };
        self.publish(EventKind::InviteCreated { hash: c.publica(), max_uses, expires_ms }).await?;
        Ok(c)
    }

    pub async fn revoke_invite(&self, hash: [u8; 32]) -> Result<[u8; 32]> {
        self.publish(EventKind::InviteRevoked { hash }).await
    }

    /// Põe ou tira uma reação.
    pub async fn react(&self, target: [u8; 32], emoji: &str, on: bool) -> Result<[u8; 32]> {
        self.publish(EventKind::React { target, emoji: emoji.into(), on }).await
    }

    async fn publish(&self, kind: EventKind) -> Result<[u8; 32]> {
        let ev = SignedEvent::sign(&self.inner.identity, Some(self.group), kind);
        let id = ev.id();
        #[cfg(feature = "servidor")]
        if self.inner.has_server() {
            // O grupo vive na caixa: o evento vale quando voltar de lá (todos aplicam as
            // mesmas regras pela mesma ordem). Até lá fica à vista como "a enviar".
            self.inner.migrar(self.group).await?;
            self.inner.enviar_item(self.group, crate::caixa::ItemGrupo::Pedido { event: ev.clone() }, Some(id)).await?;
            if let Ok(true) = self.inner.store().add_pending(self.group, &ev) {
                self.inner.evento_novo(self.group, 0, ev);
            }
            return Ok(id);
        }
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
            self.inner.aceitar(ev, None)?;
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
        if self.inner.has_server() {
            // Os grupos vivem na caixa do servidor: já não há anfitrião.
            write_frame(&mut send, &HostMsg::Rejected("atualiza a Tasca".into())).await?;
            send.finish()?;
            conn.closed().await;
            return Ok(());
        }

        let admitted = {
            let store = self.inner.store();
            match store.group(group)? {
                Some(info) if info.hosted => {
                    let by_invite = match &invite_token {
                        Some(t) => store.invite_valid(group, t)?,
                        None => false,
                    };
                    if store.is_banned(group, remote.as_bytes())? {
                        None
                    } else if by_invite || store.is_member(group, &remote)? {
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
        let telemovel = conn.remote_id() != remote;
        let _online = Online::new(self.inner.clone(), group, *remote.as_bytes(), telemovel);
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
        let mut epoca = None;
        if let Some((epoch, key, _, _)) = key {
            write_frame(&mut send, &HostMsg::GroupKey { epoch, key: *key.as_bytes() }).await?;
            epoca = Some(epoch);
        }
        let me = *remote.as_bytes();
        let atual = pres_rx.borrow_and_update().clone();
        write_frame(&mut send, &HostMsg::Presence(atual)).await?;

        loop {
            tokio::select! {
                update = feed.recv() => match update {
                    Ok((seq, event)) => {
                        let saiu = match &event.body.kind {
                            EventKind::Kick { member, .. } | EventKind::Ban { member, .. } => *member == me,
                            EventKind::Leave => event.body.author == me,
                            _ => false,
                        };
                        if seq > last_sent {
                            write_frame(&mut send, &HostMsg::Event { seq, event }).await?;
                            last_sent = seq;
                        }
                        if saiu {
                            break;
                        }
                        // chave nova (alguém foi expulso): manda-a a quem continua
                        let key = self.inner.store().group_key(group)?;
                        if let Some((epoch, key, _, _)) = key {
                            if epoca != Some(epoch) {
                                write_frame(&mut send, &HostMsg::GroupKey { epoch, key: *key.as_bytes() }).await?;
                                epoca = Some(epoch);
                            }
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
                        let result = self.inner.aceitar(ev, if by_invite { invite_token } else { None });
                        match result {
                            Ok(_) => {}
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
    telemovel: bool,
}

impl Online {
    fn new(inner: Arc<Inner>, group: GroupId, who: [u8; 32], telemovel: bool) -> Self {
        *inner.conns.lock().unwrap().entry((group, who)).or_default() += 1;
        if telemovel {
            *inner.conns_tel.lock().unwrap().entry((group, who)).or_default() += 1;
        }
        inner.presence(group).send_if_modified(|p| {
            if p.contains_key(&who) {
                return false;
            }
            p.insert(who, None);
            true
        });
        let o = Self { inner, group, who, telemovel };
        o.marcar_telemovel();
        o
    }

    /// Marca na presença quem está ligado só pelo telemóvel (ver
    /// [`crate::protocol::marca_telemovel`]). O anfitrião está sempre no PC.
    fn marcar_telemovel(&self) {
        let chave = (self.group, self.who);
        let total = self.inner.conns.lock().unwrap().get(&chave).copied().unwrap_or(0);
        let tel = self.inner.conns_tel.lock().unwrap().get(&chave).copied().unwrap_or(0);
        let so_tel = total > 0 && tel == total && self.who != *self.inner.identity.public().as_bytes();
        let marca = crate::protocol::marca_telemovel(&self.who);
        self.inner.presence(self.group).send_if_modified(|p| {
            if so_tel == p.contains_key(&marca) {
                return false;
            }
            if so_tel {
                p.insert(marca, None);
            } else {
                p.remove(&marca);
            }
            true
        });
    }
}

impl Drop for Online {
    fn drop(&mut self) {
        let chave = (self.group, self.who);
        if self.telemovel {
            let mut t = self.inner.conns_tel.lock().unwrap();
            let n = t.entry(chave).or_default();
            *n = n.saturating_sub(1);
            if *n == 0 {
                t.remove(&chave);
            }
        }
        let acabou = {
            let mut conns = self.inner.conns.lock().unwrap();
            let n = conns.entry(chave).or_default();
            *n = n.saturating_sub(1);
            let acabou = *n == 0;
            if acabou {
                conns.remove(&chave);
            }
            acabou
        };
        self.marcar_telemovel();
        // o telemóvel do próprio anfitrião saiu: o anfitrião continua online
        if acabou && self.who != *self.inner.identity.public().as_bytes() {
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
            let bloco = key.seal_item(&crate::caixa::ItemGrupo::Pedido { event: ev.clone() })?;
            srv.caixa_enviar_bloco(&key, bloco).await?;
            // fica à vista neste dispositivo até o anfitrião a aceitar
            if let Ok(true) = self.store().add_pending(group, &ev) {
                self.evento_novo(group, 0, ev);
            }
            return Ok(());
        }
        let _ = (group, ev);
        bail!("o anfitrião do grupo está desligado")
    }
}

/// Presença num grupo (vai cifrada para o servidor, que a guarda só uns minutos).
#[cfg(feature = "servidor")]
#[derive(serde::Serialize, serde::Deserialize)]
struct Presenca {
    conta: [u8; 32],
    sala: Option<String>,
    online: bool,
}

/// Entradas devolvidas pelo servidor numa leitura (se vierem tantas, há mais).
#[cfg(feature = "servidor")]
const LEITURA_CHEIA: usize = 500;

#[cfg(feature = "servidor")]
impl Inner {
    fn srv(&self) -> Result<crate::servidor::AccountServer> {
        self.servidor.clone().ok_or_else(|| anyhow!("sem servidor configurado"))
    }

    /// Põe um item na caixa atual do grupo (a da época mais alta).
    async fn enviar_item(&self, group: GroupId, item: crate::caixa::ItemGrupo, id: Option<[u8; 32]>) -> Result<()> {
        let srv = self.srv()?;
        let (epoch, key, _, _) = self
            .store()
            .group_key(group)?
            .ok_or_else(|| anyhow!("ainda não tenho a chave deste grupo"))?;
        let bloco = key.seal_item(&item)?;
        srv.caixa_enviar_bloco(&key, bloco).await?;
        if let Some(id) = id {
            let mut m = self.enviados.lock().unwrap();
            if m.len() > 5000 {
                m.clear();
            }
            m.insert(id, (group, epoch));
        }
        Ok(())
    }

    /// Passa um grupo antigo (que vivia no PC do dono) para a caixa, uma vez:
    /// - no PC do dono, copia para a caixa o que ainda lá não estava e guarda nas caixas
    ///   novas as chaves antigas (para quem entrar ler o histórico todo);
    /// - em todos, volta a ler a caixa desde o início para aplicar o que ficou à espera.
    async fn migrar(&self, group: GroupId) -> Result<()> {
        use crate::caixa::ItemGrupo;
        let _vez = self.migracao.lock().await;
        if !self.store().migrar_para_caixa(group)? {
            return Ok(());
        }
        let hosted = self.store().group(group)?.is_some_and(|g| g.hosted);
        if hosted {
            let srv = self.srv()?;
            let atual = self.store().group_key(group)?;
            if let Some((epoch, key, mut mirrored, _)) = atual {
                let falta = self.store().events_since(group, mirrored)?;
                for (seq, event) in falta {
                    let item = ItemGrupo::registo(&self.identity, &group, seq, event);
                    srv.caixa_enviar_bloco(&key, key.seal_item(&item)?).await?;
                    mirrored = seq;
                    self.store().set_key_cursors(group, epoch, mirrored, 0)?;
                }
            }
            let eps = self.store().epocas(group)?;
            for par in eps.windows(2) {
                let (antes, depois) = (&par[0], &par[1]);
                let item = ItemGrupo::Anterior { epoch: antes.0, chave: *antes.1.as_bytes() };
                srv.caixa_enviar_bloco(&depois.1, depois.1.seal_item(&item)?).await?;
            }
        }
        self.store().migracao_feita(group)
    }

    fn lugar_presenca(&self, key: &crate::caixa::ChaveCaixa) -> [u8; 16] {
        let mut h = blake3::Hasher::new_keyed(key.as_bytes());
        h.update(b"presenca");
        h.update(self.identity.public().as_bytes());
        // cada dispositivo da conta tem o seu lugar (o PC a fechar não tira o telemóvel)
        h.update(&self.endpoint_id);
        h.finalize().as_bytes()[..16].try_into().expect("16 bytes")
    }

    /// Diz ao grupo que estou online (e em que sala de voz) e lê quem mais está.
    pub(crate) async fn presenca(&self, group: GroupId) -> Result<()> {
        let srv = self.srv()?;
        let Some((_, key, _, _)) = self.store().group_key(group)? else { return Ok(()) };
        let me = *self.identity.public().as_bytes();
        let sala = self.minhas_salas.lock().unwrap().get(&group).cloned().flatten();
        let p = Presenca { conta: me, sala: sala.clone(), online: true };
        srv.presenca_enviar(&key, &self.lugar_presenca(&key), key.selar(&postcard::to_stdvec(&p)?)?).await?;
        let lista = srv.presenca_ler(&key).await?;
        let mut novo = Presence::new();
        novo.insert(me, sala);
        {
            let store = self.store();
            for (b, _) in lista {
                let Ok(d) = key.abrir(&b) else { continue };
                let Ok(p) = postcard::from_bytes::<Presenca>(&d) else { continue };
                let Ok(pk) = iroh::PublicKey::from_bytes(&p.conta) else { continue };
                if p.online && p.conta != me && store.is_member(group, &pk).unwrap_or(false) {
                    // a mesma conta em dois dispositivos: vale o que estiver numa sala
                    let e = novo.entry(p.conta).or_insert(None);
                    if let Some(sala) = p.sala.filter(|s| s.chars().count() <= 64) {
                        *e = Some(sala);
                    }
                }
            }
        }
        self.presence(group).send_if_modified(|m| {
            if *m == novo {
                return false;
            }
            *m = novo;
            true
        });
        Ok(())
    }

    /// Ao fechar a app: diz a todos os grupos que saí.
    async fn sair_presenca(&self) {
        let Ok(srv) = self.srv() else { return };
        let grupos = self.store().groups().unwrap_or_default();
        for g in grupos {
            let Ok(Some((_, key, _, _))) = self.store().group_key(g.id) else { continue };
            let p = Presenca { conta: *self.identity.public().as_bytes(), sala: None, online: false };
            let Ok(bloco) = postcard::to_stdvec(&p).map_err(anyhow::Error::from).and_then(|b| key.selar(&b)) else { continue };
            let _ = srv.presenca_enviar(&key, &self.lugar_presenca(&key), bloco).await;
        }
    }
}

#[cfg(feature = "servidor")]
impl Node {
    /// Porque é que as regras do grupo recusaram um evento meu (se recusaram).
    pub fn motivo_recusa(&self, id: &[u8; 32]) -> Option<String> {
        self.inner.rejeitados.lock().unwrap().remove(id)
    }

    /// Diz ao grupo que estou online e atualiza quem mais está (ver [`Node::presence`]).
    /// Chamar de 30 em 30 segundos enquanto a app estiver aberta.
    pub async fn presenca(&self, group: GroupId) -> Result<()> {
        self.inner.presenca(group).await
    }

    /// Entra num grupo com um convite novo. Não precisa do dono ligado: o convite traz a
    /// chave da caixa, o histórico vem do servidor e todos os membros confirmam a entrada.
    pub async fn entrar_por_convite(&self, c: &crate::invite::Convite, display_name: &str) -> Result<()> {
        let g = c.group;
        self.inner.convites.lock().unwrap().entry(g).or_default().push(c.identidade());
        self.inner
            .store()
            .set_group_key(g, c.epoch, &crate::caixa::ChaveCaixa::from_bytes(c.chave), 0)?;
        self.sync_group(g, 0).await?;
        ensure!(self.inner.store().group(g)?.is_some(), "não encontrei o grupo deste convite");
        let eps = self.inner.store().epocas(g)?;
        ensure!(eps.last().is_some_and(|e| !e.3), "este convite já não funciona; pede um convite novo");
        let me = self.inner.identity.public();
        if self.inner.store().is_member(g, &me)? {
            return Ok(());
        }
        ensure!(!self.inner.store().is_banned(g, me.as_bytes())?, "foste banido deste grupo");
        let ev = SignedEvent::sign(
            &self.inner.identity,
            Some(g),
            EventKind::MemberJoined { display_name: display_name.trim().to_string() },
        );
        let id = ev.id();
        let item = crate::caixa::ItemGrupo::Entrada { prova: c.prova(&ev), convite: c.publica(), event: ev };
        self.inner.enviar_item(g, item, None).await?;
        let ate = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            self.sync_group(g, 2).await?;
            if self.inner.store().is_member(g, &me)? {
                return Ok(());
            }
            if let Some(motivo) = self.inner.rejeitados.lock().unwrap().remove(&id) {
                bail!("{motivo}");
            }
            ensure!(std::time::Instant::now() < ate, "o grupo não respondeu a tempo; tenta outra vez");
        }
    }

    /// Lê a caixa do grupo e aplica o que for novo. Devolve quantos eventos novos entraram.
    ///
    /// A caixa é o registo do grupo: o servidor dá a ordem e cada dispositivo aplica as
    /// regras do grupo (permissões, castigos, convites, AutoMod) por essa ordem, com a hora
    /// do servidor. Assim todos chegam ao mesmo estado sem precisar do PC do dono. Registos
    /// antigos (de quando o dono aceitava tudo) entram tal como estão.
    ///
    /// `espera` (segundos) deixa o pedido de leitura à espera de novidades.
    pub async fn sync_group(&self, group: GroupId, espera: u32) -> Result<usize> {
        use crate::caixa::ItemGrupo;
        let srv = self.inner.srv()?;
        self.inner.migrar(group).await?;
        if self.inner.store().last_seq(group)? == 0 {
            self.descobrir_anteriores(group, &srv).await?;
        }
        let vez = self.inner.leituras.lock().unwrap().entry(group).or_default().clone();
        let me = *self.inner.identity.public().as_bytes();
        let mut novos = 0;
        let mut esperou = false;
        loop {
            let eps = self.inner.store().epocas(group)?;
            let Some(maior) = eps.last().map(|e| e.0) else { return Ok(novos) };
            let Some((epoch, key, lido, _)) = eps.iter().find(|e| !e.3).cloned() else { return Ok(novos) };
            let ultima = epoch == maior;
            let blocos = srv
                .caixa_ler_entradas(&key, lido, if ultima && !esperou { espera } else { 0 })
                .await?;
            if ultima {
                esperou = true;
                self.inner.caixa_lida.lock().unwrap().insert(group, std::time::Instant::now());
            }
            let cheio = blocos.len() >= LEITURA_CHEIA;
            let _vez = vez.lock().await;
            // outra leitura passou à frente entretanto: volta a ler daí
            let agora = self.inner.store().epocas(group)?;
            if agora.iter().find(|e| e.0 == epoch).map(|e| (e.2, e.3)) != Some((lido, false)) {
                continue;
            }
            let mut read = lido;
            let mut fechou = false;
            for (pos, bloco, em) in blocos {
                read = pos;
                let item = match key.open_item(&bloco) {
                    Ok(i) => i,
                    Err(e) => {
                        warn!("entrada {pos} da caixa ignorada: {e:#}");
                        continue;
                    }
                };
                match item {
                    ItemGrupo::Pedido { event } => {
                        if event.group_id() == group {
                            novos += self.aplicar(group, event, em, None);
                        }
                    }
                    ItemGrupo::Entrada { event, convite, prova } => {
                        let ok = event.body.group == Some(group)
                            && matches!(event.body.kind, EventKind::MemberJoined { .. })
                            && crate::invite::verificar_prova(&convite, &group, &event.id(), &prova);
                        if ok {
                            novos += self.aplicar(group, event, em, Some(convite));
                        }
                    }
                    item @ ItemGrupo::Registo { .. } => {
                        let ItemGrupo::Registo { seq, event, .. } = &item else { unreachable!() };
                        // O dono vem do histórico; no primeiro evento, do próprio GroupCreated.
                        let dono = match self.inner.store().group(group)? {
                            Some(g) => g.owner,
                            None if *seq == 1 && event.group_id() == group => event.author()?,
                            None => continue,
                        };
                        if event.group_id() != group {
                            continue;
                        }
                        if let Err(e) = item.verify_registo(&group, &dono) {
                            warn!("{e:#}");
                            continue;
                        }
                        let r = self.inner.store().inserir_registo(event);
                        match r {
                            Ok(Some(n)) => {
                                novos += 1;
                                self.inner.evento_novo(group, n, event.clone());
                            }
                            Ok(None) => {}
                            Err(e) => warn!("registo {seq} não aplicado: {e:#}"),
                        }
                    }
                    ItemGrupo::Voz(a) => {
                        if a.group == group && self.inner.guardar_anuncio(a.clone()) {
                            self.juntar_chamadas(&a);
                            self.voz.tocar();
                        }
                    }
                    ItemGrupo::Chave(c) => {
                        let valida = c.group == group
                            && c.epoch == epoch + 1
                            && self.inner.store().chave_nova_valida(group, &c.autor, em)?;
                        if !valida {
                            continue;
                        }
                        let nova = c.abrir(&self.inner.identity).or_else(|| {
                            let conv = self.inner.convites.lock().unwrap();
                            conv.get(&group).and_then(|l| l.iter().find_map(|i| c.abrir(i)))
                        });
                        {
                            let store = self.inner.store();
                            store.fechar_epoca(group, epoch)?;
                            store.set_lido(group, epoch, read)?;
                            if let Some(k) = &nova {
                                store.set_group_key(group, c.epoch, k, 0)?;
                            }
                        }
                        if let (Some(k), true) = (&nova, c.autor == me) {
                            // quem rodou guarda a chave antiga na caixa nova (para o histórico)
                            let item = ItemGrupo::Anterior { epoch, chave: *key.as_bytes() };
                            if let Err(e) = srv.caixa_enviar_bloco(k, k.seal_item(&item)?).await {
                                warn!("chave antiga não guardada na caixa nova: {e:#}");
                            }
                        }
                        fechou = true;
                        break;
                    }
                    ItemGrupo::Anterior { .. } => {}
                }
            }
            self.inner.store().set_lido(group, epoch, read)?;
            if fechou || cheio {
                continue;
            }
            if !ultima {
                // caixa antiga lida até ao fim sem chave nova lá dentro: a chave foi rodada
                // pelo anfitrião antigo; passa à seguinte
                self.inner.store().fechar_epoca(group, epoch)?;
                continue;
            }
            break;
        }
        self.depois_de_ler(group, &srv).await;
        Ok(novos)
    }

    /// Aplica um evento da caixa e avisa quem está à escuta. Devolve 1 se entrou.
    fn aplicar(&self, group: GroupId, ev: SignedEvent, em: u64, convite: Option<[u8; 32]>) -> usize {
        let me = *self.inner.identity.public().as_bytes();
        let meu = ev.body.author == me;
        let id = ev.id();
        let r = self.inner.store().aplicar_caixa(&ev, em, convite);
        let caso = self.inner.store().take_automod();
        let n = match r {
            Ok(Some(seq)) => {
                self.inner.evento_novo(group, seq, ev);
                1
            }
            Ok(None) => 0,
            Err(e) => {
                debug!("evento recusado: {e:#}");
                if meu {
                    let motivo = format!("{e:#}");
                    {
                        let mut m = self.inner.rejeitados.lock().unwrap();
                        if m.len() > 1000 {
                            m.clear();
                        }
                        m.insert(id, motivo.clone());
                    }
                    // só se avisa do que foi recusado agora (não do histórico)
                    if crate::event::now_ms().saturating_sub(em) < 5 * 60_000 {
                        let _ = self.inner.recusas.send((group, motivo));
                    }
                }
                0
            }
        };
        // Alerta do AutoMod: escreve-o o dono, se estiver ligado.
        if let Some(c) = caso {
            let dono = self.inner.store().group(group).ok().flatten().is_some_and(|g| *g.owner.as_bytes() == me);
            if let (true, Some(canal)) = (dono && crate::event::now_ms().saturating_sub(em) < 5 * 60_000, c.bloqueio.alert_channel.clone()) {
                let nome = self.inner.store().display_name(group, &match iroh::PublicKey::from_bytes(&c.autor) {
                    Ok(k) => k,
                    Err(_) => return n,
                });
                let nome = nome.ok().flatten().unwrap_or_else(|| "alguém".into());
                let texto: String = c.texto.chars().take(300).collect();
                let k = EventKind::Message {
                    channel: canal,
                    text: format!(
                        "🛡️ AutoMod ({}): {} de {nome} em #{}: «{texto}»",
                        c.bloqueio.regra,
                        if c.bloqueio.block { "bloqueei uma mensagem" } else { "mensagem marcada" },
                        c.canal
                    ),
                };
                let ev = SignedEvent::sign(&self.inner.identity, Some(group), k);
                let inner = self.inner.clone();
                tokio::spawn(async move {
                    let id = ev.id();
                    if let Err(e) = inner.enviar_item(group, crate::caixa::ItemGrupo::Pedido { event: ev }, Some(id)).await {
                        warn!("alerta do AutoMod não enviado: {e:#}");
                    }
                });
            }
        }
        n
    }

    /// Depois de ler tudo: roda a chave se alguém foi expulso e reenvia o que ficou numa
    /// caixa que entretanto fechou.
    async fn depois_de_ler(&self, group: GroupId, srv: &crate::servidor::AccountServer) {
        let me = *self.inner.identity.public().as_bytes();
        let Ok(Some((epoch, key, _, _))) = self.inner.store().group_key(group) else { return };
        // ainda não li tudo (por exemplo, a entrar no grupo): não rodo nem reenvio
        let em_dia = self.inner.store().epocas(group).ok().is_some_and(|e| e.iter().filter(|x| !x.3).count() == 1);
        if !em_dia {
            return;
        }
        let pendente = self.inner.store().rodar_pendente(group);
        if let Ok(Some((quando, quem, banido))) = pendente {
            let pode = {
                let store = self.inner.store();
                let dono = store.group(group).ok().flatten().is_some_and(|g| *g.owner.as_bytes() == me);
                let perms = store.perms(group, &me, None).unwrap_or(0);
                quem == me
                    || (crate::event::now_ms().saturating_sub(quando) > 90_000
                        && (dono || perms & (crate::cargos::EXPULSAR | crate::cargos::BANIR) != 0))
            };
            let recente = self
                .inner
                .rodadas
                .lock()
                .unwrap()
                .get(&group)
                .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(60));
            if pode && !recente {
                self.inner.rodadas.lock().unwrap().insert(group, std::time::Instant::now());
                let destinos: Vec<[u8; 32]> = {
                    let store = self.inner.store();
                    let mut d: Vec<[u8; 32]> =
                        store.members(group).unwrap_or_default().into_iter().map(|(k, _)| *k.as_bytes()).collect();
                    // depois de expulsar, os convites abertos continuam a servir (quem foi
                    // expulso pode voltar); depois de banir, não
                    if !banido {
                        d.extend(store.convites_abertos(group).unwrap_or_default());
                    }
                    d
                };
                let r: Result<()> = async {
                    let nova = crate::caixa::ChaveCaixa::generate()?;
                    let c = crate::caixa::NovaChave::criar(&self.inner.identity, group, epoch + 1, &nova, &destinos);
                    srv.caixa_enviar_bloco(&key, key.seal_item(&crate::caixa::ItemGrupo::Chave(c))?).await?;
                    Ok(())
                }
                .await;
                if let Err(e) = r {
                    warn!("não consegui trocar a chave do grupo: {e:#}");
                }
            }
        }
        // o que mandei para uma caixa que entretanto fechou vai outra vez, para a nova
        let pendentes = self.inner.store().meus_pendentes(group, &me).unwrap_or_default();
        for ev in pendentes {
            let id = ev.id();
            let antiga = self.inner.enviados.lock().unwrap().get(&id).is_some_and(|(g, e)| *g == group && *e < epoch);
            if antiga {
                let _ = self.inner.enviar_item(group, crate::caixa::ItemGrupo::Pedido { event: ev }, Some(id)).await;
            }
        }
    }

    /// Quem entra só com a chave mais recente procura, em cada caixa, a chave da anterior
    /// (o histórico de antes de a chave mudar).
    async fn descobrir_anteriores(&self, group: GroupId, srv: &crate::servidor::AccountServer) -> Result<()> {
        loop {
            let eps = self.inner.store().epocas(group)?;
            let Some((menor, key, _, _)) = eps.first().cloned() else { return Ok(()) };
            if menor == 0 {
                return Ok(());
            }
            let mut desde = 0;
            let mut achou = None;
            'ler: loop {
                let blocos = srv.caixa_ler_entradas(&key, desde, 0).await?;
                let n = blocos.len();
                for (pos, bloco, _) in blocos {
                    desde = pos;
                    if let Ok(crate::caixa::ItemGrupo::Anterior { epoch, chave }) = key.open_item(&bloco) {
                        if epoch + 1 == menor {
                            achou = Some(chave);
                            break 'ler;
                        }
                    }
                }
                if n < LEITURA_CHEIA {
                    break;
                }
            }
            match achou {
                Some(k) => self.inner.store().set_group_key(group, menor - 1, &crate::caixa::ChaveCaixa::from_bytes(k), 0)?,
                None => return Ok(()),
            }
        }
    }
}

fn rand_bytes<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    getrandom::fill(&mut out).expect("gerador aleatório do sistema");
    out
}
