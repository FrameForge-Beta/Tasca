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
}

#[derive(Debug, Serialize, Deserialize)]
enum VozAnfitriao {
    Aceite { id: u16 },
    Recusado(String),
    Sala(Vec<Participante>),
}

/// As salas de voz que este PC hospeda.
#[derive(Default)]
pub(crate) struct Hub {
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
    saida: Saida,
    sala: watch::Sender<Vec<Participante>>,
}

enum Saida {
    Rede(Connection),
    Local(mpsc::Sender<Pacote>),
}

impl Hub {
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
            Membro { key, muted: false, deaf: false, saida, sala: tx },
        );
        for r in antigas {
            Self::anunciar(&mut e, group, &r);
        }
        Self::anunciar(&mut e, group, room);
        drop(e);
        self.mudou.send_modify(|v| *v += 1);
        (id, rx)
    }

    fn sair(&self, group: GroupId, room: &str, id: u16) {
        let mut e = self.estado.lock().unwrap();
        let chave = (group, room.to_string());
        let Some(membros) = e.salas.get_mut(&chave) else { return };
        if membros.remove(&id).is_none() {
            return;
        }
        if membros.is_empty() {
            e.salas.remove(&chave);
        } else {
            Self::anunciar(&mut e, group, room);
        }
        drop(e);
        self.mudou.send_modify(|v| *v += 1);
    }

    fn set_estado(&self, group: GroupId, room: &str, id: u16, muted: bool, deaf: bool) {
        let mut e = self.estado.lock().unwrap();
        let Some(m) = e.salas.get_mut(&(group, room.to_string())).and_then(|s| s.get_mut(&id)) else { return };
        if (m.muted, m.deaf) == (muted, deaf) {
            return;
        }
        m.muted = muted;
        m.deaf = deaf;
        Self::anunciar(&mut e, group, room);
        drop(e);
        self.mudou.send_modify(|v| *v += 1);
    }

    /// Manda a lista nova da sala a todos os que lá estão.
    fn anunciar(e: &mut HubEstado, group: GroupId, room: &str) {
        let Some(membros) = e.salas.get(&(group, room.to_string())) else { return };
        let mut lista: Vec<Participante> = membros
            .iter()
            .map(|(id, m)| Participante { id: *id, key: m.key, muted: m.muted, deaf: m.deaf })
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
                Saida::Local(tx) => {
                    let _ = tx.try_send(Pacote { from: autor.key, seq, data: dg.slice(4..) });
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
                    .map(|(id, m)| Participante { id: *id, key: m.key, muted: m.muted, deaf: m.deaf })
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
        let remote = conn.remote_id();
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
                Ok(Some(VozCliente::Entrar { .. })) => bail!("Entrar repetido"),
                Ok(None) => break,
                Err(e) => {
                    debug!("voz: ligação de {remote} caiu: {e:#}");
                    break;
                }
            }
        }
        drop((audio, lista));
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
        let (id, sala) = hub.entrar(group, room, key, Saida::Local(tx));
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

        Ok(Chamada {
            group,
            room: room.to_string(),
            id,
            modo: Modo::Rede { conn, controlo: Arc::new(tokio::sync::Mutex::new(send)) },
            sala,
            pacotes: Some(rx),
            seq: Default::default(),
            alive,
            _tarefas: vec![AbortOnDrop(controlo), AbortOnDrop(audio)],
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
