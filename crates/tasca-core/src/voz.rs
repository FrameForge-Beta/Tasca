//! Chamadas de voz: o anfitrião do grupo reencaminha o áudio entre quem está na mesma sala.
//!
//! O áudio já vem codificado (Opus) e o anfitrião não o descodifica: só o copia para os
//! outros participantes. Cada membro abre uma ligação iroh com o ALPN [`VOZ_ALPN`] ao
//! anfitrião; o controlo (entrar, estado, quem está na sala) vai num stream e o áudio vai
//! em datagramas QUIC, que não esperam por pacotes perdidos.
//!
//! Datagramas:
//! - membro → anfitrião: `seq: u16` + pacote Opus;
//! - anfitrião → membro: `id do autor: u16` + `seq: u16` + pacote Opus.
//!
//! Vídeo (câmara e partilha de ecrã): cada imagem codificada vai num stream QUIC
//! unidirecional próprio, para chegar inteira sem atrasar as outras:
//! - membro → anfitrião: a imagem (bytes opacos, ver [`Chamada::enviar_video`]);
//! - anfitrião → membro: `id do autor: u16` + a imagem.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use anyhow::{Result, anyhow, bail, ensure};
use bytes::{BufMut, Bytes, BytesMut};
use iroh::{
    endpoint::{Connection, SendStream},
    protocol::{AcceptError, ProtocolHandler},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};
use tracing::{debug, warn};

use crate::{
    event::GroupId,
    node::Inner,
    protocol::{read_frame, write_frame},
};

/// Identificador do protocolo de voz na negociação QUIC (ALPN).
pub const VOZ_ALPN: &[u8] = b"tasca/voz/0";

/// Tamanho máximo do nome de uma sala.
const MAX_SALA: usize = 64;

/// Quem está numa sala de voz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participante {
    /// Número curto que o anfitrião dá a cada participante (vai em cada pacote de áudio).
    pub id: u16,
    /// Chave pública da pessoa.
    pub key: [u8; 32],
    pub muted: bool,
    pub deaf: bool,
    /// Câmara ligada.
    pub camera: bool,
    /// A partilhar o ecrã.
    pub screen: bool,
}

/// Tamanho máximo de uma imagem de vídeo (8 MiB).
const MAX_IMAGEM: usize = 8 * 1024 * 1024;
/// Imagens a caminho de uma pessoa a partir das quais se deitam fora as que não são chave.
const FILA_VIDEO: usize = 6;
/// A partir daqui deitam-se fora todas.
const FILA_VIDEO_MAX: usize = 24;

/// Uma imagem de vídeo de outro participante.
#[derive(Debug, Clone)]
pub struct Imagem {
    pub from: [u8; 32],
    pub data: Bytes,
}

/// Um pacote de áudio de outro participante.
#[derive(Debug, Clone)]
pub struct Pacote {
    pub from: [u8; 32],
    pub seq: u16,
    pub data: Bytes,
}

#[derive(Debug, Serialize, Deserialize)]
enum VozCliente {
    Entrar { group: GroupId, room: String },
    Estado { muted: bool, deaf: bool },
    Video { camera: bool, screen: bool },
}

#[derive(Debug, Serialize, Deserialize)]
enum VozAnfitriao {
    Aceite { id: u16 },
    Recusado(String),
    Sala(Vec<Participante>),
}

/// As salas de voz que este PC hospeda.
pub(crate) struct Hub {
    inner: Weak<Inner>,
    estado: Mutex<HubEstado>,
    /// Muda sempre que alguém entra, sai ou muda o estado numa sala.
    mudou: watch::Sender<u64>,
}

#[derive(Default)]
struct HubEstado {
    salas: HashMap<(GroupId, String), HashMap<u16, Membro>>,
    proximo: u16,
}

struct Membro {
    key: [u8; 32],
    muted: bool,
    deaf: bool,
    camera: bool,
    screen: bool,
    saida: Saida,
    /// Imagens de vídeo a caminho desta pessoa.
    fila: Arc<std::sync::atomic::AtomicUsize>,
    sala: watch::Sender<Vec<Participante>>,
}

enum Saida {
    Rede(Connection),
    Local(mpsc::Sender<Pacote>, mpsc::Sender<Imagem>),
}

impl Membro {
    fn participante(&self, id: u16) -> Participante {
        Participante {
            id,
            key: self.key,
            muted: self.muted,
            deaf: self.deaf,
            camera: self.camera,
            screen: self.screen,
        }
    }
}

/// O primeiro byte de uma imagem diz se é chave (bit 0), por convenção de quem a envia.
fn e_chave(data: &[u8]) -> bool {
    data.first().is_some_and(|b| b & 1 == 1)
}

impl Hub {
    pub(crate) fn new(inner: Weak<Inner>) -> Self {
        Hub { inner, estado: Default::default(), mudou: watch::channel(0).0 }
    }

    /// Põe a sala de voz na presença do grupo, que o anfitrião manda a todos os membros.
    fn presenca(&self, group: GroupId, key: [u8; 32], room: Option<String>) {
        if let Some(inner) = self.inner.upgrade() {
            inner.set_voice_presence(group, key, room);
        }
    }

    fn entrar(&self, group: GroupId, room: &str, key: [u8; 32], saida: Saida) -> (u16, watch::Receiver<Vec<Participante>>) {
        let mut e = self.estado.lock().unwrap();
        let sala_chave = (group, room.to_string());
        // A mesma pessoa só fica uma vez em cada grupo (por exemplo, se voltar a ligar).
        let mut antigas = Vec::new();
        for ((g, r), membros) in e.salas.iter_mut() {
            if *g == group {
                membros.retain(|_, m| {
                    let fica = m.key != key;
                    if !fica {
                        if let Saida::Rede(c) = &m.saida {
                            c.close(1u32.into(), b"entrou noutra vez");
                        }
                    }
                    fica
                });
                antigas.push(r.clone());
            }
        }
        e.salas.retain(|_, m| !m.is_empty());
        let id = loop {
            e.proximo = e.proximo.wrapping_add(1);
            let id = e.proximo;
            if !e.salas.values().any(|m| m.contains_key(&id)) {
                break id;
            }
        };
        let (tx, rx) = watch::channel(Vec::new());
        e.salas.entry(sala_chave).or_default().insert(
            id,
            Membro {
                key,
                muted: false,
                deaf: false,
                camera: false,
                screen: false,
                saida,
                fila: Default::default(),
                sala: tx,
            },
        );
        for r in antigas {
            Self::anunciar(&mut e, group, &r);
        }
        Self::anunciar(&mut e, group, room);
        drop(e);
        self.presenca(group, key, Some(room.to_string()));
        self.mudou.send_modify(|v| *v += 1);
        (id, rx)
    }

    fn sair(&self, group: GroupId, room: &str, id: u16) {
        let mut e = self.estado.lock().unwrap();
        let chave = (group, room.to_string());
        let Some(membros) = e.salas.get_mut(&chave) else { return };
        let Some(saiu) = membros.remove(&id) else { return };
        if membros.is_empty() {
            e.salas.remove(&chave);
        } else {
            Self::anunciar(&mut e, group, room);
        }
        drop(e);
        self.presenca(group, saiu.key, None);
        self.mudou.send_modify(|v| *v += 1);
    }

    fn set_estado(&self, group: GroupId, room: &str, id: u16, muted: bool, deaf: bool) {
        self.mudar(group, room, id, |m| {
            m.muted = muted;
            m.deaf = deaf;
        });
    }

    fn set_video(&self, group: GroupId, room: &str, id: u16, camera: bool, screen: bool) {
        self.mudar(group, room, id, |m| {
            m.camera = camera;
            m.screen = screen;
        });
    }

    fn mudar(&self, group: GroupId, room: &str, id: u16, f: impl FnOnce(&mut Membro)) {
        let mut e = self.estado.lock().unwrap();
        let Some(m) = e.salas.get_mut(&(group, room.to_string())).and_then(|s| s.get_mut(&id)) else { return };
        let antes = (m.muted, m.deaf, m.camera, m.screen);
        f(m);
        if antes == (m.muted, m.deaf, m.camera, m.screen) {
            return;
        }
        Self::anunciar(&mut e, group, room);
        drop(e);
        self.mudou.send_modify(|v| *v += 1);
    }

    /// Manda a lista nova da sala a todos os que lá estão.
    fn anunciar(e: &mut HubEstado, group: GroupId, room: &str) {
        let Some(membros) = e.salas.get(&(group, room.to_string())) else { return };
        let mut lista: Vec<Participante> = membros
            .iter()
            .map(|(id, m)| m.participante(*id))
            .collect();
        lista.sort_by_key(|p| p.id);
        for m in membros.values() {
            m.sala.send_replace(lista.clone());
        }
    }

    /// Copia um pacote de áudio para os outros participantes da sala.
    fn encaminhar(&self, group: GroupId, room: &str, from: u16, seq: u16, data: &[u8]) {
        let e = self.estado.lock().unwrap();
        let Some(membros) = e.salas.get(&(group, room.to_string())) else { return };
        let Some(autor) = membros.get(&from) else { return };
        if autor.muted {
            return;
        }
        let mut dg = BytesMut::with_capacity(4 + data.len());
        dg.put_u16(from);
        dg.put_u16(seq);
        dg.put_slice(data);
        let dg = dg.freeze();
        for (id, m) in membros {
            if *id == from || m.deaf {
                continue;
            }
            match &m.saida {
                Saida::Rede(c) => {
                    if let Err(err) = c.send_datagram(dg.clone()) {
                        debug!("datagrama de voz não seguiu: {err}");
                    }
                }
                Saida::Local(tx, _) => {
                    let _ = tx.try_send(Pacote { from: autor.key, seq, data: dg.slice(4..) });
                }
            }
        }
    }

    /// Copia uma imagem de vídeo para os outros participantes da sala. `chave` diz se é uma
    /// imagem chave (as outras dependem dela): quando alguém está atrasado, só essas seguem.
    fn encaminhar_video(&self, group: GroupId, room: &str, from: u16, chave: bool, data: Bytes) {
        use std::sync::atomic::Ordering;
        let e = self.estado.lock().unwrap();
        let Some(membros) = e.salas.get(&(group, room.to_string())) else { return };
        let Some(autor) = membros.get(&from) else { return };
        for (id, m) in membros {
            if *id == from {
                continue;
            }
            let fila = m.fila.load(Ordering::Relaxed);
            if fila >= FILA_VIDEO_MAX || (fila >= FILA_VIDEO && !chave) {
                continue;
            }
            match &m.saida {
                Saida::Rede(c) => {
                    let (c, data, fila) = (c.clone(), data.clone(), m.fila.clone());
                    fila.fetch_add(1, Ordering::Relaxed);
                    tokio::spawn(async move {
                        let r: Result<()> = async {
                            let mut s = c.open_uni().await?;
                            s.write_all(&from.to_be_bytes()).await?;
                            s.write_all(&data).await?;
                            s.finish()?;
                            s.stopped().await?;
                            Ok(())
                        }
                        .await;
                        if let Err(e) = r {
                            debug!("imagem de vídeo não seguiu: {e:#}");
                        }
                        fila.fetch_sub(1, Ordering::Relaxed);
                    });
                }
                Saida::Local(_, tx) => {
                    let _ = tx.try_send(Imagem { from: autor.key, data: data.clone() });
                }
            }
        }
    }

    /// As salas de um grupo com quem lá está.
    pub(crate) fn salas(&self, group: GroupId) -> Vec<(String, Vec<Participante>)> {
        let e = self.estado.lock().unwrap();
        let mut out: Vec<_> = e
            .salas
            .iter()
            .filter(|((g, _), _)| *g == group)
            .map(|((_, r), membros)| {
                let mut l: Vec<_> = membros
                    .iter()
                    .map(|(id, m)| m.participante(*id))
                    .collect();
                l.sort_by_key(|p| p.id);
                (r.clone(), l)
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    pub(crate) fn watch(&self) -> watch::Receiver<u64> {
        self.mudou.subscribe()
    }
}

/// Lado do anfitrião: atende membros que entram numa sala de voz.
#[derive(Clone)]
pub(crate) struct HostVoz {
    pub(crate) inner: Weak<Inner>,
    pub(crate) hub: Arc<Hub>,
}

impl std::fmt::Debug for HostVoz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostVoz")
    }
}

impl ProtocolHandler for HostVoz {
    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        self.serve(conn).await.map_err(|e| AcceptError::from_boxed(e.into()))
    }
}

impl HostVoz {
    async fn serve(&self, conn: Connection) -> Result<()> {
        let inner_ = self.inner.upgrade().ok_or_else(|| anyhow!("nó desligado"))?;
        let remote = inner_.conta_de(&conn.remote_id());
        drop(inner_);
        let key = *remote.as_bytes();
        let (mut send, mut recv) = conn.accept_bi().await?;
        let Some(VozCliente::Entrar { group, room }) = read_frame(&mut recv).await? else {
            bail!("esperava Entrar");
        };
        let admitido = (|| -> Result<bool> {
            let inner = self.inner.upgrade().ok_or_else(|| anyhow!("nó desligado"))?;
            let store = inner.store();
            Ok(room.len() <= MAX_SALA
                && store.group(group)?.is_some_and(|g| g.hosted)
                && store.is_member(group, &remote)?)
        })()?;
        if !admitido {
            write_frame(&mut send, &VozAnfitriao::Recusado("não és membro deste grupo".into())).await?;
            send.finish()?;
            conn.closed().await;
            return Ok(());
        }

        let (id, mut sala) = self.hub.entrar(group, &room, key, Saida::Rede(conn.clone()));
        let _sai = SaiAoAcabar { hub: self.hub.clone(), group, room: room.clone(), id };
        write_frame(&mut send, &VozAnfitriao::Aceite { id }).await?;

        // Áudio que chega deste membro.
        let hub = self.hub.clone();
        let dg_conn = conn.clone();
        let dg_room = room.clone();
        let audio = AbortOnDrop(tokio::spawn(async move {
            while let Ok(dg) = dg_conn.read_datagram().await {
                if dg.len() < 3 {
                    continue;
                }
                let seq = u16::from_be_bytes([dg[0], dg[1]]);
                hub.encaminhar(group, &dg_room, id, seq, &dg[2..]);
            }
        }));

        // Vídeo que chega deste membro.
        let hub = self.hub.clone();
        let v_conn = conn.clone();
        let v_room = room.clone();
        let video = AbortOnDrop(tokio::spawn(async move {
            while let Ok(mut s) = v_conn.accept_uni().await {
                let hub = hub.clone();
                let v_room = v_room.clone();
                tokio::spawn(async move {
                    if let Ok(data) = s.read_to_end(MAX_IMAGEM).await {
                        let chave = e_chave(&data);
                        hub.encaminhar_video(group, &v_room, id, chave, Bytes::from(data));
                    }
                });
            }
        }));

        // Lista da sala para este membro, sempre que muda.
        let lista = AbortOnDrop(tokio::spawn(async move {
            loop {
                let l = sala.borrow_and_update().clone();
                if write_frame(&mut send, &VozAnfitriao::Sala(l)).await.is_err() {
                    break;
                }
                if sala.changed().await.is_err() {
                    break;
                }
            }
        }));

        loop {
            match read_frame::<VozCliente>(&mut recv).await {
                Ok(Some(VozCliente::Estado { muted, deaf })) => self.hub.set_estado(group, &room, id, muted, deaf),
                Ok(Some(VozCliente::Video { camera, screen })) => self.hub.set_video(group, &room, id, camera, screen),
                Ok(Some(VozCliente::Entrar { .. })) => bail!("Entrar repetido"),
                Ok(None) => break,
                Err(e) => {
                    debug!("voz: ligação de {remote} caiu: {e:#}");
                    break;
                }
            }
        }
        drop((audio, video, lista));
        conn.close(0u32.into(), b"adeus");
        Ok(())
    }
}

struct SaiAoAcabar {
    hub: Arc<Hub>,
    group: GroupId,
    room: String,
    id: u16,
}

impl Drop for SaiAoAcabar {
    fn drop(&mut self) {
        self.hub.sair(self.group, &self.room, self.id);
    }
}

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Uma chamada aberta numa sala de voz.
pub struct Chamada {
    pub group: GroupId,
    pub room: String,
    /// O nosso número na sala.
    pub id: u16,
    modo: Modo,
    sala: watch::Receiver<Vec<Participante>>,
    pacotes: Option<mpsc::Receiver<Pacote>>,
    imagens: Option<mpsc::Receiver<Imagem>>,
    seq: std::sync::atomic::AtomicU16,
    alive: watch::Receiver<bool>,
    _tarefas: Vec<AbortOnDrop>,
}

enum Modo {
    /// Somos o anfitrião: o áudio vai direto para o hub.
    Local { hub: Arc<Hub>, _sai: SaiAoAcabar, _alive: watch::Sender<bool> },
    Rede { conn: Connection, controlo: Arc<tokio::sync::Mutex<SendStream>> },
}

impl Chamada {
    /// Entra como anfitrião (o grupo é hospedado neste PC).
    pub(crate) fn local(hub: Arc<Hub>, group: GroupId, room: &str, key: [u8; 32]) -> Self {
        let (tx, rx) = mpsc::channel(256);
        let (vtx, vrx) = mpsc::channel(64);
        let (id, sala) = hub.entrar(group, room, key, Saida::Local(tx, vtx));
        let (alive_tx, alive) = watch::channel(true);
        Chamada {
            group,
            room: room.to_string(),
            id,
            modo: Modo::Local {
                _sai: SaiAoAcabar { hub: hub.clone(), group, room: room.to_string(), id },
                hub,
                _alive: alive_tx,
            },
            sala,
            pacotes: Some(rx),
            imagens: Some(vrx),
            seq: Default::default(),
            alive,
            _tarefas: Vec::new(),
        }
    }

    /// Entra como membro, ligando ao anfitrião.
    pub(crate) async fn remota(conn: Connection, group: GroupId, room: &str) -> Result<Self> {
        ensure!(room.len() <= MAX_SALA, "nome de sala demasiado comprido");
        let (mut send, mut recv) = conn.open_bi().await?;
        write_frame(&mut send, &VozCliente::Entrar { group, room: room.to_string() }).await?;
        let id = match read_frame::<VozAnfitriao>(&mut recv).await? {
            Some(VozAnfitriao::Aceite { id }) => id,
            Some(VozAnfitriao::Recusado(why)) => bail!("o anfitrião recusou: {why}"),
            other => bail!("resposta inesperada do anfitrião: {other:?}"),
        };
        let (sala_tx, sala) = watch::channel(Vec::<Participante>::new());
        let (alive_tx, alive) = watch::channel(true);
        let alive_tx = Arc::new(alive_tx);
        let (tx, rx) = mpsc::channel(256);

        let a = alive_tx.clone();
        let controlo = tokio::spawn(async move {
            loop {
                match read_frame::<VozAnfitriao>(&mut recv).await {
                    Ok(Some(VozAnfitriao::Sala(l))) => {
                        sala_tx.send_replace(l);
                    }
                    Ok(Some(other)) => debug!("voz: mensagem ignorada: {other:?}"),
                    Ok(None) => break,
                    Err(e) => {
                        warn!("voz: ligação ao anfitrião caiu: {e:#}");
                        break;
                    }
                }
            }
            let _ = a.send(false);
        });

        let sala_rx = sala.clone();
        let dg_conn = conn.clone();
        let a = alive_tx.clone();
        let audio = tokio::spawn(async move {
            while let Ok(dg) = dg_conn.read_datagram().await {
                if dg.len() < 5 {
                    continue;
                }
                let from = u16::from_be_bytes([dg[0], dg[1]]);
                let seq = u16::from_be_bytes([dg[2], dg[3]]);
                let key = sala_rx.borrow().iter().find(|p| p.id == from).map(|p| p.key);
                if let Some(from) = key {
                    let _ = tx.try_send(Pacote { from, seq, data: dg.slice(4..) });
                }
            }
            let _ = a.send(false);
        });

        let (vtx, vrx) = mpsc::channel(64);
        let sala_rx = sala.clone();
        let v_conn = conn.clone();
        let video = tokio::spawn(async move {
            while let Ok(mut s) = v_conn.accept_uni().await {
                let (vtx, sala_rx) = (vtx.clone(), sala_rx.clone());
                tokio::spawn(async move {
                    let Ok(data) = s.read_to_end(MAX_IMAGEM + 2).await else { return };
                    if data.len() < 3 {
                        return;
                    }
                    let from = u16::from_be_bytes([data[0], data[1]]);
                    let key = sala_rx.borrow().iter().find(|p| p.id == from).map(|p| p.key);
                    if let Some(from) = key {
                        let data = Bytes::from(data).slice(2..);
                        let _ = vtx.try_send(Imagem { from, data });
                    }
                });
            }
        });

        Ok(Chamada {
            group,
            room: room.to_string(),
            id,
            modo: Modo::Rede { conn, controlo: Arc::new(tokio::sync::Mutex::new(send)) },
            sala,
            pacotes: Some(rx),
            imagens: Some(vrx),
            seq: Default::default(),
            alive,
            _tarefas: vec![AbortOnDrop(controlo), AbortOnDrop(audio), AbortOnDrop(video)],
        })
    }

    /// Envia um pacote de áudio (Opus, 20 ms) aos outros da sala. Não espera: se a rede
    /// estiver congestionada, o pacote perde-se, como deve ser na voz.
    pub fn enviar(&self, data: &[u8]) {
        let seq = self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        match &self.modo {
            Modo::Local { hub, .. } => hub.encaminhar(self.group, &self.room, self.id, seq, data),
            Modo::Rede { conn, .. } => {
                let mut dg = BytesMut::with_capacity(2 + data.len());
                dg.put_u16(seq);
                dg.put_slice(data);
                if let Err(e) = conn.send_datagram(dg.freeze()) {
                    debug!("datagrama de voz não seguiu: {e}");
                }
            }
        }
    }

    /// Envia uma imagem de vídeo aos outros da sala. O primeiro byte é de quem envia, com o
    /// bit 0 ligado nas imagens chave (as outras dependem delas); o resto é livre. Se a
    /// ligação estiver lenta, as imagens que não são chave podem perder-se.
    pub fn enviar_video(&self, data: Bytes) {
        let chave = e_chave(&data);
        match &self.modo {
            Modo::Local { hub, .. } => hub.encaminhar_video(self.group, &self.room, self.id, chave, data),
            Modo::Rede { conn, .. } => {
                use std::sync::atomic::Ordering;
                static FILA: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                let fila = FILA.load(Ordering::Relaxed);
                if fila >= FILA_VIDEO_MAX || (fila >= FILA_VIDEO && !chave) {
                    return;
                }
                FILA.fetch_add(1, Ordering::Relaxed);
                let c = conn.clone();
                tokio::spawn(async move {
                    let r: Result<()> = async {
                        let mut s = c.open_uni().await?;
                        s.write_all(&data).await?;
                        s.finish()?;
                        s.stopped().await?;
                        Ok(())
                    }
                    .await;
                    if let Err(e) = r {
                        debug!("imagem de vídeo não seguiu: {e:#}");
                    }
                    FILA.fetch_sub(1, Ordering::Relaxed);
                });
            }
        }
    }

    /// As imagens de vídeo dos outros. Só se pode tirar uma vez.
    pub fn imagens(&mut self) -> Option<mpsc::Receiver<Imagem>> {
        self.imagens.take()
    }

    /// Diz aos outros se tens a câmara ligada ou estás a partilhar o ecrã.
    pub async fn set_video(&self, camera: bool, screen: bool) -> Result<()> {
        match &self.modo {
            Modo::Local { hub, .. } => {
                hub.set_video(self.group, &self.room, self.id, camera, screen);
                Ok(())
            }
            Modo::Rede { controlo, .. } => {
                write_frame(&mut *controlo.lock().await, &VozCliente::Video { camera, screen }).await
            }
        }
    }

    /// Os pacotes de áudio dos outros. Só se pode tirar uma vez.
    pub fn pacotes(&mut self) -> Option<mpsc::Receiver<Pacote>> {
        self.pacotes.take()
    }

    /// Quem está na sala (inclui-nos a nós).
    pub fn sala(&self) -> watch::Receiver<Vec<Participante>> {
        self.sala.clone()
    }

    /// Diz aos outros se estamos sem microfone ou sem som.
    pub async fn set_estado(&self, muted: bool, deaf: bool) -> Result<()> {
        match &self.modo {
            Modo::Local { hub, .. } => {
                hub.set_estado(self.group, &self.room, self.id, muted, deaf);
                Ok(())
            }
            Modo::Rede { controlo, .. } => {
                write_frame(&mut *controlo.lock().await, &VozCliente::Estado { muted, deaf }).await
            }
        }
    }

    /// A ligação ao anfitrião continua viva (como anfitrião, sempre).
    pub fn ligada(&self) -> bool {
        *self.alive.borrow()
    }

    /// Espera até a ligação ao anfitrião cair.
    pub async fn caiu(&self) {
        let mut rx = self.alive.clone();
        let _ = rx.wait_for(|v| !*v).await;
    }

    /// Tempo de ida e volta até ao anfitrião e se passa por um relay. `None` como anfitrião.
    pub fn ligacao(&self) -> Option<(Duration, bool)> {
        match &self.modo {
            Modo::Local { .. } => None,
            Modo::Rede { conn, .. } => {
                let paths = conn.paths();
                let p = paths.iter().find(|p| p.is_selected())?;
                Some((p.rtt(), p.is_relay()))
            }
        }
    }
}

impl Drop for Chamada {
    fn drop(&mut self) {
        if let Modo::Rede { conn, .. } = &self.modo {
            conn.close(0u32.into(), b"adeus");
        }
    }
}
