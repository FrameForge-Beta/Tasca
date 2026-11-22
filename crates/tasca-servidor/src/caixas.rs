//! Caixas de mensagens cifradas (ver `tasca_core::caixa`).
//!
//! Cada caixa é identificada por um id aleatório e protegida por um token que só os
//! participantes conhecem. O servidor guarda o hash do token e os blocos cifrados, por
//! ordem de chegada. Não sabe a que grupo ou pessoas pertence a caixa nem o que diz.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json,
    body::Bytes,
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use tasca_core::{
    caixa::{Entrada, Entradas, Escrita},
    servidor::b64,
};
use tokio::sync::Notify;

use crate::{ApiError, AppState, internal};

/// Espaço máximo por caixa. Só texto: ficheiros e fotos ficam nos PCs.
const MAX_CAIXA_BYTES: i64 = 512 * 1024 * 1024;
/// Escritas por IP por minuto.
const MAX_ESCRITAS_MIN: usize = 240;
/// Entradas devolvidas por leitura.
const MAX_LEITURA: i64 = 500;
const MAX_ESPERA: u64 = 25;

/// Quanto tempo vale um bloco de presença (os dispositivos renovam-no a cada 30 s).
const PRESENCA_VALIDADE: Duration = Duration::from_secs(90);
const MAX_PRESENCA_BLOCO: usize = 2048;
const MAX_PRESENCAS_CAIXA: usize = 2000;

/// Presença numa caixa: lugar → (bloco, hora, quando chegou).
type Presencas = HashMap<[u8; 16], (Vec<u8>, u64, Instant)>;

#[derive(Default)]
pub struct Estado {
    avisos: Mutex<HashMap<[u8; 32], Arc<Notify>>>,
    escritas: Mutex<HashMap<std::net::IpAddr, Vec<Instant>>>,
    /// Quem está online em cada caixa. Só em memória: some ao fim de 90 s sem renovar.
    presencas: Mutex<HashMap<[u8; 32], Presencas>>,
}

impl Estado {
    fn aviso(&self, id: [u8; 32]) -> Arc<Notify> {
        let mut m = self.avisos.lock().unwrap();
        // Limpa avisos sem ninguém à espera, para o mapa não crescer para sempre.
        if m.len() > 10_000 {
            m.retain(|_, n| Arc::strong_count(n) > 1);
        }
        m.entry(id).or_default().clone()
    }
}

pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS caixas (
             id          BLOB PRIMARY KEY,
             verificador BLOB NOT NULL,
             bytes       INTEGER NOT NULL DEFAULT 0,
             ultima_seq  INTEGER NOT NULL DEFAULT 0,
             criada      INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS entradas (
             caixa BLOB NOT NULL,
             seq   INTEGER NOT NULL,
             dados BLOB NOT NULL,
             em    INTEGER NOT NULL,
             PRIMARY KEY (caixa, seq)
         ) WITHOUT ROWID;",
    )
}

fn nao_autorizado() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "Sem acesso a esta conversa.".into())
}

fn parse_id(s: &str) -> Result<[u8; 32], ApiError> {
    hex::decode(s)
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Conversa não encontrada.".into()))
}

/// Hash do token que vem em `Authorization: Bearer <hex>`.
fn verificador(headers: &HeaderMap) -> Result<[u8; 32], ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|t| hex::decode(t.trim()).ok())
        .filter(|t| t.len() == 32)
        .ok_or_else(nao_autorizado)?;
    Ok(*blake3::hash(&token).as_bytes())
}

fn iguais(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Confirma o token. Devolve `false` se a caixa ainda não existir.
fn confere(conn: &Connection, id: &[u8; 32], ver: &[u8; 32]) -> Result<bool, ApiError> {
    let guardado: Option<Vec<u8>> = conn
        .query_row("SELECT verificador FROM caixas WHERE id = ?1", [id.as_slice()], |r| r.get(0))
        .optional()
        .map_err(internal)?;
    match guardado {
        None => Ok(false),
        Some(g) if iguais(&g, ver) => Ok(true),
        Some(_) => Err(nao_autorizado()),
    }
}

pub async fn escrever(
    State(st): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    headers: HeaderMap,
    dados: Bytes,
) -> Result<Json<Escrita>, ApiError> {
    let id = parse_id(&id)?;
    let ver = verificador(&headers)?;
    if dados.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "Mensagem vazia.".into()));
    }
    {
        let ip = st.client_ip(addr, &headers);
        let mut m = st.caixas.escritas.lock().unwrap();
        let l = m.entry(ip).or_default();
        l.retain(|t| t.elapsed() < Duration::from_secs(60));
        if l.len() >= MAX_ESCRITAS_MIN {
            return Err(ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                "Estás a enviar demasiado depressa. Espera um minuto.".into(),
            ));
        }
        l.push(Instant::now());
    }
    let seq = {
        let mut conn = st.db.lock().unwrap();
        let tx = conn.transaction().map_err(internal)?;
        if !confere(&tx, &id, &ver)? {
            // Primeira escrita: quem tem a chave cria a caixa.
            tx.execute(
                "INSERT INTO caixas (id, verificador, criada) VALUES (?1, ?2, ?3)",
                params![id.as_slice(), ver.as_slice(), crate::now()],
            )
            .map_err(internal)?;
        }
        let (bytes, ultima): (i64, i64) = tx
            .query_row("SELECT bytes, ultima_seq FROM caixas WHERE id = ?1", [id.as_slice()], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(internal)?;
        if bytes + dados.len() as i64 > MAX_CAIXA_BYTES {
            return Err(ApiError(
                StatusCode::INSUFFICIENT_STORAGE,
                "Esta conversa encheu o espaço no servidor.".into(),
            ));
        }
        let seq = ultima + 1;
        let em = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        tx.execute(
            "INSERT INTO entradas (caixa, seq, dados, em) VALUES (?1, ?2, ?3, ?4)",
            params![id.as_slice(), seq, dados.as_ref(), em],
        )
        .map_err(internal)?;
        tx.execute(
            "UPDATE caixas SET bytes = bytes + ?2, ultima_seq = ?3 WHERE id = ?1",
            params![id.as_slice(), dados.len() as i64, seq],
        )
        .map_err(internal)?;
        tx.commit().map_err(internal)?;
        seq
    };
    st.caixas.aviso(id).notify_waiters();
    Ok(Json(Escrita { seq: seq as u64 }))
}

#[derive(Deserialize)]
pub struct Leitura {
    #[serde(default)]
    desde: u64,
    #[serde(default)]
    espera: u64,
}

fn entradas(st: &AppState, id: &[u8; 32], ver: &[u8; 32], desde: u64) -> Result<Vec<Entrada>, ApiError> {
    let conn = st.db.lock().unwrap();
    if !confere(&conn, id, ver)? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare_cached(
            "SELECT seq, dados, em FROM entradas WHERE caixa = ?1 AND seq > ?2 ORDER BY seq LIMIT ?3",
        )
        .map_err(internal)?;
    let rows = stmt
        .query_map(params![id.as_slice(), desde as i64, MAX_LEITURA], |r| {
            Ok(Entrada {
                seq: r.get::<_, i64>(0)? as u64,
                dados: b64(&r.get::<_, Vec<u8>>(1)?),
                em: r.get::<_, i64>(2)? as u64,
            })
        })
        .map_err(internal)?;
    rows.collect::<Result<_, _>>().map_err(internal)
}

pub async fn ler(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<Leitura>,
    headers: HeaderMap,
) -> Result<Json<Entradas>, ApiError> {
    let id = parse_id(&id)?;
    let ver = verificador(&headers)?;
    let espera = q.espera.min(MAX_ESPERA);
    // Regista o interesse antes de ler, para não perder uma escrita que chegue entretanto.
    let aviso = st.caixas.aviso(id);
    let notificado = aviso.notified();
    tokio::pin!(notificado);
    notificado.as_mut().enable();
    let lista = entradas(&st, &id, &ver, q.desde)?;
    if !lista.is_empty() || espera == 0 {
        return Ok(Json(Entradas { entradas: lista }));
    }
    let _ = tokio::time::timeout(Duration::from_secs(espera), notificado).await;
    Ok(Json(Entradas { entradas: entradas(&st, &id, &ver, q.desde)? }))
}

#[derive(Deserialize)]
pub struct Lugar {
    lugar: String,
}

/// Deixa (ou renova) um bloco de presença cifrado. O servidor não sabe de quem é: só
/// guarda o bloco no lugar indicado durante 90 segundos.
pub async fn presenca_escrever(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<Lugar>,
    headers: HeaderMap,
    dados: Bytes,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&id)?;
    let ver = verificador(&headers)?;
    let lugar: [u8; 16] = hex::decode(&q.lugar)
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Pedido inválido.".into()))?;
    if dados.is_empty() || dados.len() > MAX_PRESENCA_BLOCO {
        return Err(ApiError(StatusCode::BAD_REQUEST, "Pedido inválido.".into()));
    }
    {
        let conn = st.db.lock().unwrap();
        if !confere(&conn, &id, &ver)? {
            return Err(ApiError(StatusCode::NOT_FOUND, "Conversa não encontrada.".into()));
        }
    }
    let mut m = st.caixas.presencas.lock().unwrap();
    if m.len() > 50_000 {
        m.retain(|_, p| {
            p.retain(|_, v| v.2.elapsed() < PRESENCA_VALIDADE);
            !p.is_empty()
        });
    }
    let p = m.entry(id).or_default();
    p.retain(|_, v| v.2.elapsed() < PRESENCA_VALIDADE);
    if p.len() >= MAX_PRESENCAS_CAIXA && !p.contains_key(&lugar) {
        return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "Demasiadas pessoas online.".into()));
    }
    p.insert(lugar, (dados.to_vec(), crate::now() as u64 * 1000, Instant::now()));
    Ok(StatusCode::NO_CONTENT)
}

/// Os blocos de presença que ainda valem.
pub async fn presenca_ler(
    State(st): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Entradas>, ApiError> {
    let id = parse_id(&id)?;
    let ver = verificador(&headers)?;
    {
        let conn = st.db.lock().unwrap();
        if !confere(&conn, &id, &ver)? {
            return Ok(Json(Entradas { entradas: Vec::new() }));
        }
    }
    let mut m = st.caixas.presencas.lock().unwrap();
    let entradas = match m.get_mut(&id) {
        Some(p) => {
            p.retain(|_, v| v.2.elapsed() < PRESENCA_VALIDADE);
            p.values().map(|(b, em, _)| Entrada { seq: 0, dados: b64(b), em: *em }).collect()
        }
        None => Vec::new(),
    };
    Ok(Json(Entradas { entradas }))
}
