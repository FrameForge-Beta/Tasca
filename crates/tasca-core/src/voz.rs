//! Chamadas de voz: quem entra primeiro numa sala aloja a chamada e reencaminha o áudio
//! entre quem lá está. O papel pode passar para outra pessoa da chamada (ver
//! [`Chamada::passar_host`]) e passa sozinho quando quem aloja sai.
//!
//! Para os outros saberem quem aloja cada sala, quem aloja publica um [`Anuncio`]
//! assinado (na caixa do grupo e a quem perguntar com [`VozCliente::Quem`]).
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
pub const VOZ_ALPN: &[u8] = b"tasca/voz/1";

/// Um anúncio deixa de valer se não for renovado durante este tempo (ms).
pub const ANUNCIO_VALIDADE_MS: u64 = 5 * 60 * 1000;

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
    /// Chave iroh do dispositivo (no telemóvel é diferente da conta).
    pub endpoint: [u8; 32],
    /// É quem aloja a chamada.
    pub anfitriao: bool,
    /// Por onde o anfitrião o vê (para os outros lhe ligarem se ficar com a chamada).
    pub enderecos: Vec<iroh::TransportAddr>,
}

/// Para onde a chamada mudou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NovoHost {
    /// Dispositivo que passa a alojar a chamada.
    pub endpoint: [u8; 32],
    /// A conta dessa pessoa.
    pub conta: [u8; 32],
    /// Endereços conhecidos desse dispositivo.
    pub enderecos: Vec<iroh::TransportAddr>,
}

impl NovoHost {
    pub fn addr(&self) -> Result<iroh::EndpointAddr> {
        addr_de(&self.endpoint, &self.enderecos)
    }
}

fn addr_de(endpoint: &[u8; 32], enderecos: &[iroh::TransportAddr]) -> Result<iroh::EndpointAddr> {
    let id = iroh::PublicKey::from_bytes(endpoint).map_err(|e| anyhow!("chave inválida: {e}"))?;
    Ok(iroh::EndpointAddr::from_parts(id, enderecos.iter().cloned()))
}

fn enderecos_de(c: &Connection) -> Vec<iroh::TransportAddr> {
    c.paths().iter().map(|p| p.remote_addr().clone()).collect()
}

/// "Estou a alojar esta sala de voz": quem aloja assina e publica, para os outros
/// saberem onde ligar quando o anfitrião do grupo está desligado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anuncio {
    pub group: GroupId,
    pub room: String,
    pub endpoint: [u8; 32],
    pub enderecos: Vec<iroh::TransportAddr>,
    pub conta: [u8; 32],
    /// Contas que estão na sala (vazio = a chamada acabou).
    pub pessoas: Vec<[u8; 32]>,
    /// Quando esta pessoa começou a alojar (ms desde 1970).
    pub desde: u64,
    /// Quando o anúncio foi feito (ms desde 1970).
    pub ts: u64,
    pub sig: Vec<u8>,
}

pub(crate) fn agora_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Anuncio {
    fn msg(&self) -> Vec<u8> {
        let corpo = (&self.group, &self.room, &self.endpoint, &self.enderecos, &self.conta, &self.pessoas, self.desde, self.ts);
        let mut h = blake3::Hasher::new_derive_key("tasca 2026 anuncio voz v1");
        h.update(&postcard::to_stdvec(&corpo).unwrap_or_default());
        h.finalize().as_bytes().to_vec()
    }

    pub(crate) fn assinar(
        eu: &crate::identity::Identity,
        group: GroupId,
        room: &str,
        endpoint: [u8; 32],
        enderecos: Vec<iroh::TransportAddr>,
        pessoas: Vec<[u8; 32]>,
        desde: u64,
    ) -> Self {
        let mut a = Anuncio {
            group,
            room: room.to_string(),
            endpoint,
            enderecos,
            conta: *eu.public().as_bytes(),
            pessoas,
            desde,
            ts: agora_ms(),
            sig: Vec::new(),
        };
        a.sig = eu.secret().sign(&a.msg()).to_bytes().to_vec();
        a
    }

    /// Confirma a assinatura da conta.
    pub fn verificar(&self) -> Result<()> {
        let sig: [u8; 64] = self.sig.as_slice().try_into().map_err(|_| anyhow!("assinatura inválida"))?;
        let conta = iroh::PublicKey::from_bytes(&self.conta).map_err(|e| anyhow!("conta inválida: {e}"))?;
        conta
            .verify(&self.msg(), &iroh::Signature::from_bytes(&sig))
            .map_err(|_| anyhow!("anúncio de voz com assinatura inválida"))
    }

    pub fn addr(&self) -> Result<iroh::EndpointAddr> {
        addr_de(&self.endpoint, &self.enderecos)
    }

    /// Ainda vale e tem gente.
    pub fn ativo(&self) -> bool {
        !self.pessoas.is_empty() && agora_ms().saturating_sub(self.ts) < ANUNCIO_VALIDADE_MS
    }
}

/// "Grupo" de uma chamada privada entre duas contas: o mesmo para os dois lados e
/// impossível de adivinhar por quem não é nenhuma delas.
pub fn grupo_privado(a: &iroh::PublicKey, b: &iroh::PublicKey) -> GroupId {
    let (x, y) = if a.as_bytes() <= b.as_bytes() { (a, b) } else { (b, a) };
    let mut h = blake3::Hasher::new_derive_key("tasca 2026 chamada privada v1");
    h.update(x.as_bytes());
    h.update(y.as_bytes());
    GroupId(*h.finalize().as_bytes())
}

/// Alguém está a ligar-te (chamada privada).
#[derive(Debug, Clone)]
pub struct Toque {
    /// Conta de quem liga.
    pub de: [u8; 32],
    /// Dispositivo de onde liga (é lá que a chamada está).
    pub endpoint: iroh::EndpointAddr,
}

/// O anfitrião mandou ligar a outra pessoa (ver [`NovoHost`]).
#[derive(Debug)]
pub struct Redirecionar(pub NovoHost);

impl std::fmt::Display for Redirecionar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a chamada mudou de anfitrião")
    }
}
impl std::error::Error for Redirecionar {}

/// Esse dispositivo não está a alojar a sala (ainda, ou já não).
#[derive(Debug)]
pub struct NaoAloja;

impl std::fmt::Display for NaoAloja {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("esse dispositivo não está a alojar esta chamada")
    }
}
impl std::error::Error for NaoAloja {}

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
pub(crate) enum VozCliente {
    Entrar { group: GroupId, room: String },
    Estado { muted: bool, deaf: bool },
    Video { camera: bool, screen: bool },
    /// Quem está a alojar salas deste grupo? (só para membros)
    Quem { group: GroupId },
    /// Passa a chamada a outra pessoa da sala (só o anfitrião do grupo, por rede).
    PassarHost { para: u16 },
    /// Para o dono do grupo saber quem aloja (e dizer a quem perguntar).
    Anunciar(Anuncio),
    /// Chamada privada: "estou a ligar-te" (o grupo é [`grupo_privado`] dos dois).
    Tocar { group: GroupId },
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum VozAnfitriao {
    Aceite { id: u16 },
    Recusado(String),
    Sala(Vec<Participante>),
    /// Liga-te a outra pessoa: é ela que aloja agora.
    NovoHost(NovoHost),
    /// Não estou a alojar essa sala.
    NaoAloja,
    /// Resposta a `Quem`.
    Conhecidos(Vec<Anuncio>),
}

/// As salas de voz que este PC hospeda.
pub(crate) struct Hub {
    inner: Weak<Inner>,
    /// Chave iroh deste dispositivo.
    eu: [u8; 32],
    estado: Mutex<HubEstado>,
    /// Muda sempre que alguém entra, sai ou muda o estado numa sala.
    mudou: watch::Sender<u64>,
}

#[derive(Default)]
struct HubEstado {
    salas: HashMap<(GroupId, String), HashMap<u16, Membro>>,
    /// Desde quando alojamos cada sala (ms).
    desde: HashMap<(GroupId, String), u64>,
    /// Salas que passámos a outra pessoa há pouco: quem chegar vai para lá.
    passadas: HashMap<(GroupId, String), (NovoHost, std::time::Instant)>,
    proximo: u16,
}

struct Membro {
    key: [u8; 32],
    endpoint: [u8; 32],
    novo: watch::Sender<Option<NovoHost>>,
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
            endpoint: self.endpoint,
            anfitriao: matches!(self.saida, Saida::Local(..)),
            enderecos: match &self.saida {
                Saida::Rede(c) => enderecos_de(c),
                Saida::Local(..) => Vec::new(),
            },
        }
    }
}

/// O primeiro byte de uma imagem diz se é chave (bit 0), por convenção de quem a envia.
fn e_chave(data: &[u8]) -> bool {
    data.first().is_some_and(|b| b & 1 == 1)
}

impl Hub {
    pub(crate) fn new(inner: Weak<Inner>, eu: [u8; 32]) -> Self {
        Hub { inner, eu, estado: Default::default(), mudou: watch::channel(0).0 }
    }

    /// Este dispositivo está a alojar a sala (está nela localmente).
    pub(crate) fn hospeda(&self, group: GroupId, room: &str) -> bool {
        let e = self.estado.lock().unwrap();
        e.salas
            .get(&(group, room.to_string()))
            .is_some_and(|m| m.values().any(|m| matches!(m.saida, Saida::Local(..))))
    }

    /// Para onde passámos a sala há pouco (até um minuto).
    fn passada(&self, group: GroupId, room: &str) -> Option<NovoHost> {
        let e = self.estado.lock().unwrap();
        e.passadas
            .get(&(group, room.to_string()))
            .filter(|(_, t)| t.elapsed() < Duration::from_secs(60))
            .map(|(n, _)| n.clone())
    }

    /// Passa a sala a outra pessoa que lá está (por número).
    pub(crate) fn passar(&self, group: GroupId, room: &str, para: u16) -> Result<()> {
        let mut e = self.estado.lock().unwrap();
        let chave = (group, room.to_string());
        let membros = e.salas.get(&chave).ok_or_else(|| anyhow!("a sala está vazia"))?;
        let alvo = membros.get(&para).ok_or_else(|| anyhow!("essa pessoa não está na chamada"))?;
        let Saida::Rede(c) = &alvo.saida else { bail!("essa pessoa já aloja a chamada") };
        let novo = NovoHost { endpoint: *c.remote_id().as_bytes(), conta: alvo.key, enderecos: enderecos_de(c) };
        Self::redirecionar(&mut e, chave, novo);
        Ok(())
    }

    /// Manda toda a gente da sala (incluindo quem está neste dispositivo) para outro
    /// anfitrião. Ligações que não saiam sozinhas fecham-se passado um bocado.
    pub(crate) fn mudar_para(&self, group: GroupId, room: &str, novo: NovoHost) {
        let mut e = self.estado.lock().unwrap();
        Self::redirecionar(&mut e, (group, room.to_string()), novo);
    }

    fn redirecionar(e: &mut HubEstado, chave: (GroupId, String), novo: NovoHost) {
        e.passadas.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(60));
        let mut conns = Vec::new();
        if let Some(membros) = e.salas.get(&chave) {
            for m in membros.values() {
                m.novo.send_replace(Some(novo.clone()));
                if let Saida::Rede(c) = &m.saida {
                    conns.push(c.clone());
                }
            }
        }
        e.passadas.insert(chave, (novo, std::time::Instant::now()));
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            rt.spawn(async move {
                tokio::time::sleep(Duration::from_secs(15)).await;
                for c in conns {
                    c.close(2u32.into(), b"mudou de anfitriao");
                }
            });
        }
    }

    /// Anúncios das salas que este dispositivo aloja.
    pub(crate) fn anuncios(&self, eu: &crate::identity::Identity, enderecos: &[iroh::TransportAddr]) -> Vec<Anuncio> {
        let e = self.estado.lock().unwrap();
        let mut out = Vec::new();
        for ((g, r), membros) in &e.salas {
            if !membros.values().any(|m| matches!(m.saida, Saida::Local(..))) {
                continue;
            }
            let mut ids: Vec<_> = membros.iter().collect();
            ids.sort_by_key(|(id, _)| **id);
            let pessoas = ids.into_iter().map(|(_, m)| m.key).collect();
            let desde = e.desde.get(&(*g, r.clone())).copied().unwrap_or_else(agora_ms);
            out.push(Anuncio::assinar(eu, *g, r, self.eu, enderecos.to_vec(), pessoas, desde));
        }
        out
    }

    /// Põe a sala de voz na presença do grupo, que o anfitrião manda a todos os membros.
    fn presenca(&self, group: GroupId, key: [u8; 32], room: Option<String>) {
        if let Some(inner) = self.inner.upgrade() {
            inner.set_voice_presence(group, key, room);
        }
    }

    fn entrar(
        &self,
        group: GroupId,
        room: &str,
        key: [u8; 32],
        endpoint: [u8; 32],
        saida: Saida,
    ) -> (u16, watch::Receiver<Vec<Participante>>, watch::Receiver<Option<NovoHost>>) {
        let mut e = self.estado.lock().unwrap();
        if matches!(saida, Saida::Local(..)) {
            // Voltámos a alojar: esquece para onde a tínhamos passado.
            e.passadas.remove(&(group, room.to_string()));
        }
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
        let vivas: Vec<_> = e.salas.keys().cloned().collect();
        e.desde.retain(|k, _| vivas.contains(k));
        e.desde.entry(sala_chave.clone()).or_insert_with(agora_ms);
        let id = loop {
            e.proximo = e.proximo.wrapping_add(1);
            let id = e.proximo;
            if !e.salas.values().any(|m| m.contains_key(&id)) {
                break id;
            }
        };
        let (tx, rx) = watch::channel(Vec::new());
        let (novo_tx, novo_rx) = watch::channel(None);
        e.salas.entry(sala_chave).or_default().insert(
            id,
            Membro {
                key,
                endpoint,
                novo: novo_tx,
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
        (id, rx, novo_rx)
    }

    fn sair(&self, group: GroupId, room: &str, id: u16) {
        let mut e = self.estado.lock().unwrap();
        let chave = (group, room.to_string());
        let Some(membros) = e.salas.get_mut(&chave) else { return };
        let Some(saiu) = membros.remove(&id) else { return };
        // Quem aloja saiu e ficou gente: a chamada passa para quem está há mais tempo.
        let sucessor = if matches!(saiu.saida, Saida::Local(..)) {
            membros
                .iter()
                .filter_map(|(id, m)| match &m.saida {
                    Saida::Rede(c) => Some((*id, NovoHost { endpoint: *c.remote_id().as_bytes(), conta: m.key, enderecos: enderecos_de(c) })),
                    Saida::Local(..) => None,
                })
                .min_by_key(|(id, _)| *id)
                .map(|(_, n)| n)
        } else {
            None
        };
        if membros.is_empty() {
            e.salas.remove(&chave);
            e.desde.remove(&chave);
        } else {
            Self::anunciar(&mut e, group, room);
        }
        let ja_passada = e
            .passadas
            .get(&chave)
            .is_some_and(|(_, t)| t.elapsed() < Duration::from_secs(60));
        if let (Some(n), false) = (sucessor, ja_passada) {
            Self::redirecionar(&mut e, chave, n);
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

    /// Avisa quem vigia as salas (por exemplo, chegou um anúncio de outro dispositivo).
    pub(crate) fn tocar(&self) {
        self.mudou.send_modify(|v| *v += 1);
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
        let (group, room) = match read_frame(&mut recv).await? {
            Some(VozCliente::Entrar { group, room }) => (group, room),
            Some(VozCliente::Tocar { group }) => {
                if let Some(inner) = self.inner.upgrade() {
                    if group == grupo_privado(&inner.identity().public(), &remote) {
                        let endpoint = iroh::EndpointAddr::from_parts(conn.remote_id(), enderecos_de(&conn));
                        let _ = inner.toques.send(Toque { de: key, endpoint });
                    }
                }
                send.finish()?;
                return Ok(());
            }
            Some(VozCliente::Anunciar(a)) => {
                if let Some(inner) = self.inner.upgrade() {
                    if inner.guardar_anuncio(a) {
                        self.hub.tocar();
                    }
                }
                send.finish()?;
                return Ok(());
            }
            Some(VozCliente::Quem { group }) => {
                let lista = {
                    let inner = self.inner.upgrade().ok_or_else(|| anyhow!("nó desligado"))?;
                    if inner.store().is_member(group, &remote)? {
                        inner.anuncios_de(group, &self.hub)
                    } else {
                        Vec::new()
                    }
                };
                write_frame(&mut send, &VozAnfitriao::Conhecidos(lista)).await?;
                send.finish()?;
                let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
                return Ok(());
            }
            _ => bail!("esperava Entrar"),
        };
        let (admitido, dono) = (|| -> Result<(bool, bool)> {
            let inner = self.inner.upgrade().ok_or_else(|| anyhow!("nó desligado"))?;
            let store = inner.store();
            let info = store.group(group)?;
            let privada = group == grupo_privado(&inner.identity().public(), &remote);
            Ok((
                room.len() <= MAX_SALA && (privada || (info.is_some() && store.is_member(group, &remote)?)),
                info.is_some_and(|g| g.owner == remote),
            ))
        })()?;
        let resposta = if !admitido {
            Some(VozAnfitriao::Recusado("não és membro deste grupo".into()))
        } else if let Some(n) = self.hub.passada(group, &room) {
            Some(VozAnfitriao::NovoHost(n))
        } else if !self.hub.hospeda(group, &room) {
            Some(VozAnfitriao::NaoAloja)
        } else {
            None
        };
        if let Some(r) = resposta {
            write_frame(&mut send, &r).await?;
            send.finish()?;
            let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
            return Ok(());
        }

        let endpoint = *conn.remote_id().as_bytes();
        let (id, mut sala, mut novo) = self.hub.entrar(group, &room, key, endpoint, Saida::Rede(conn.clone()));
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
        // e para onde ir quando a chamada mudar de anfitrião.
        let lista = AbortOnDrop(tokio::spawn(async move {
            let (mut manda_sala, mut manda_novo) = (true, false);
            loop {
                if manda_sala {
                    let l = sala.borrow_and_update().clone();
                    if write_frame(&mut send, &VozAnfitriao::Sala(l)).await.is_err() {
                        break;
                    }
                }
                if manda_novo {
                    let n = novo.borrow_and_update().clone();
                    if let Some(n) = n {
                        if write_frame(&mut send, &VozAnfitriao::NovoHost(n)).await.is_err() {
                            break;
                        }
                    }
                }
                (manda_sala, manda_novo) = tokio::select! {
                    r = sala.changed() => { if r.is_err() { break } (true, false) }
                    r = novo.changed() => { if r.is_err() { break } (false, true) }
                };
            }
        }));

        loop {
            match read_frame::<VozCliente>(&mut recv).await {
                Ok(Some(VozCliente::Estado { muted, deaf })) => self.hub.set_estado(group, &room, id, muted, deaf),
                Ok(Some(VozCliente::Video { camera, screen })) => self.hub.set_video(group, &room, id, camera, screen),
                Ok(Some(VozCliente::PassarHost { para })) => {
                    if dono {
                        if let Err(e) = self.hub.passar(group, &room, para) {
                            debug!("voz: não passei a chamada: {e:#}");
                        }
                    }
                }
                Ok(Some(VozCliente::Entrar { .. } | VozCliente::Quem { .. } | VozCliente::Anunciar(_) | VozCliente::Tocar { .. })) => {
                    bail!("Entrar repetido")
                }
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
    novo: watch::Receiver<Option<NovoHost>>,
    /// Dispositivo que aloja a chamada.
    pub host: [u8; 32],
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
        let eu = hub.eu;
        let (id, sala, novo) = hub.entrar(group, room, key, eu, Saida::Local(tx, vtx));
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
            novo,
            host: eu,
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
            Some(VozAnfitriao::NovoHost(n)) => return Err(Redirecionar(n).into()),
            Some(VozAnfitriao::NaoAloja) => return Err(NaoAloja.into()),
            Some(VozAnfitriao::Recusado(why)) => bail!("o anfitrião recusou: {why}"),
            other => bail!("resposta inesperada do anfitrião: {other:?}"),
        };
        let (sala_tx, sala) = watch::channel(Vec::<Participante>::new());
        let (novo_tx, novo) = watch::channel(None);
        let host = *conn.remote_id().as_bytes();
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
                    Ok(Some(VozAnfitriao::NovoHost(n))) => {
                        novo_tx.send_replace(Some(n));
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
            novo,
            host,
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

    /// Muda para `Some` quando a chamada passa para outro dispositivo: é preciso sair e
    /// ligar a esse (ver [`crate::node::Node::entrar_voz_em`]).
    pub fn novo_host(&self) -> watch::Receiver<Option<NovoHost>> {
        self.novo.clone()
    }

    /// Este dispositivo aloja a chamada.
    pub fn e_anfitriao(&self) -> bool {
        matches!(self.modo, Modo::Local { .. })
    }

    /// Passa a chamada a outra pessoa da sala. Quem aloja pode sempre; por rede, só o
    /// dono do grupo (o anfitrião ignora os outros).
    pub async fn passar_host(&self, para: u16) -> Result<()> {
        match &self.modo {
            Modo::Local { hub, .. } => hub.passar(self.group, &self.room, para),
            Modo::Rede { controlo, .. } => {
                write_frame(&mut *controlo.lock().await, &VozCliente::PassarHost { para }).await
            }
        }
    }

    /// Para quem a chamada passaria se quem a aloja desaparecesse sem avisar: quem tem o
    /// número mais baixo, tirando o anfitrião. Todos chegam à mesma resposta.
    pub fn sucessor(&self) -> Option<NovoHost> {
        self.sala
            .borrow()
            .iter()
            .filter(|p| !p.anfitriao)
            .min_by_key(|p| p.id)
            .map(|p| NovoHost { endpoint: p.endpoint, conta: p.key, enderecos: p.enderecos.clone() })
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
