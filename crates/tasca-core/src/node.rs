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
        let endpoint = builder
            .secret_key(identity.secret().clone())
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
        });
        let voz = Arc::new(crate::voz::Hub::new(Arc::downgrade(&inner)));
        let router = Router::builder(endpoint)
            .accept(
                ALPN,
                HostProtocol {
                    inner: inner.clone(),
                },
            )
            .accept(crate::amigos::ALPN, crate::amigos::Protocolo(inner.amigos.clone()))
            .accept(
                crate::voz::VOZ_ALPN,
                crate::voz::HostVoz { inner: Arc::downgrade(&inner), hub: voz.clone() },
            )
            .spawn();
        Ok(Self {
            inner,
            router,
            local_only: opts.local_only,
            voz,
        })
    }

    /// Amigos: lista, o meu estado e o que sei deles.
    pub fn amigos(&self) -> &crate::amigos::Amigos {
        &self.inner.amigos
    }

    /// Pergunta ao PC de um amigo como está (e troca perfis). `None` = offline ou invisível.
    /// Aceita a chave pública do amigo (o iroh descobre onde está) ou um endereço completo.
    pub async fn perguntar_amigo(&self, quem: impl Into<iroh::EndpointAddr>) -> Result<Option<crate::amigos::Estado>> {
        self.inner.amigos.perguntar(self.router.endpoint(), quem.into()).await
    }

    /// Endereço deste nó (para testes e ligações na rede local).
    pub fn addr(&self) -> iroh::EndpointAddr {
        self.router.endpoint().addr()
    }

    pub fn identity(&self) -> &Identity {
        &self.inner.identity
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

    /// Entra numa sala de voz de um grupo. Como anfitrião fica neste PC; como membro liga-se
    /// ao anfitrião, que reencaminha o áudio entre os participantes.
    pub async fn entrar_voz(&self, group: GroupId, room: &str) -> Result<crate::voz::Chamada> {
        let info = self
            .inner
            .store()
            .group(group)?
            .ok_or_else(|| anyhow!("grupo desconhecido"))?;
        if info.hosted {
            let me = *self.inner.identity.public().as_bytes();
            return Ok(crate::voz::Chamada::local(self.voz.clone(), group, room, me));
        }
        let conn = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            self.router.endpoint().connect(iroh::EndpointAddr::new(info.owner), crate::voz::VOZ_ALPN),
        )
        .await
        .map_err(|_| anyhow!("o anfitrião não respondeu"))?
        .map_err(|e| anyhow!("não consegui ligar ao anfitrião: {e}"))?;
        crate::voz::Chamada::remota(conn, group, room).await
    }

    /// Anfitrião: as salas de voz de um grupo e quem está em cada uma.
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
        // A chave da ligação QUIC é a identidade do membro: não dá para fingir ser outro.
        let remote = conn.remote_id();
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
