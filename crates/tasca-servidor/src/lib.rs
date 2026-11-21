//! Servidor de contas da Tasca.
//!
//! Guarda, por utilizador: a chave pública, o cofre **encriptado no PC do utilizador** e
//! um verificador da chave de login (Argon2id com sal aleatório). Não guarda palavras-passe
//! nem consegue abrir os cofres. Mensagens, grupos e chamadas não passam por aqui.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{ConnectInfo, DefaultBodyLimit, Path as UrlPath, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rusqlite::{Connection, OptionalExtension, params};
use tasca_core::{
    Vault,
    servidor::{
        ErrorBody, LoginRequest, LoginResponse, RegisterRequest, RenameRequest, UpdateRequest, UserInfo, b64,
        normalize_username, parse_key_hex, unb64,
    },
};
use tracing::info;

mod caixas;

/// Tentativas de login falhadas permitidas por utilizador antes de bloquear.
const MAX_FAILS: usize = 10;
const FAIL_WINDOW: Duration = Duration::from_secs(15 * 60);
/// Contas novas permitidas por IP por hora.
const MAX_REGISTER_PER_IP: usize = 5;
const REGISTER_WINDOW: Duration = Duration::from_secs(60 * 60);
const MAX_VAULT_BYTES: usize = 16 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub(crate) db: Arc<Mutex<Connection>>,
    limits: Arc<Mutex<Limits>>,
    pub(crate) caixas: Arc<caixas::Estado>,
    /// Atrás de um proxy (Caddy, Cloudflare): o IP real vem no `X-Forwarded-For`.
    trust_proxy: bool,
}

#[derive(Default)]
struct Limits {
    fails: HashMap<String, Vec<Instant>>,
    registers: HashMap<IpAddr, Vec<Instant>>,
}

impl Limits {
    fn recent(list: &mut Vec<Instant>, window: Duration) -> usize {
        list.retain(|t| t.elapsed() < window);
        list.len()
    }
}

impl AppState {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS contas (
                 utilizador  TEXT PRIMARY KEY,
                 chave_publica BLOB NOT NULL UNIQUE,
                 sal         BLOB NOT NULL,
                 verificador BLOB NOT NULL,
                 cofre       BLOB NOT NULL,
                 criada      INTEGER NOT NULL,
                 atualizada  INTEGER NOT NULL
             );",
        )?;
        caixas::init(&conn)?;
        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            limits: Default::default(),
            caixas: Default::default(),
            trust_proxy: false,
        })
    }
}

impl AppState {
    /// Usa o último IP do `X-Forwarded-For` (o que o proxy acrescentou) nos limites.
    /// Só ligar quando o servidor não está exposto diretamente à Internet.
    pub fn trust_proxy(mut self, on: bool) -> Self {
        self.trust_proxy = on;
        self
    }

    pub(crate) fn client_ip(&self, addr: SocketAddr, headers: &HeaderMap) -> IpAddr {
        if self.trust_proxy {
            let fwd = headers
                .get_all("x-forwarded-for")
                .iter()
                .filter_map(|v| v.to_str().ok())
                .flat_map(|v| v.split(','))
                .last()
                .and_then(|ip| ip.trim().parse().ok());
            if let Some(ip) = fwd {
                return ip;
            }
        }
        addr.ip()
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/saude", get(|| async { "ok" }))
        .route("/v1/registar", post(register))
        .route("/v1/entrar", post(login))
        .route("/v1/cofre", post(update))
        .route("/v1/renomear", post(rename))
        .route("/v1/utilizador/{nome}", get(lookup))
        .route("/v1/caixas/{id}", get(caixas::ler).post(caixas::escrever))
        .layer(DefaultBodyLimit::max(tasca_core::caixa::MAX_BLOCO))
        .with_state(state)
}

/// Erro com o código HTTP e a mensagem a mostrar ao utilizador.
pub struct ApiError(pub(crate) StatusCode, pub(crate) String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(ErrorBody { erro: self.1 })).into_response()
    }
}

fn bad(msg: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, capitalize(&msg.to_string()))
}

pub(crate) fn internal(e: impl std::fmt::Display) -> ApiError {
    tracing::error!("erro interno: {e}");
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Erro no servidor. Tenta mais tarde.".into())
}

fn wrong_login() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "Utilizador ou palavra-passe errados.".into())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Verificador = Argon2id(chave_login, sal). A chave de login já vem de um Argon2 forte no
/// PC, por isso aqui bastam parâmetros leves.
fn verifier(login_key: &[u8; 32], salt: &[u8]) -> Result<[u8; 32], ApiError> {
    let params = argon2::Params::new(8 * 1024, 1, 1, Some(32)).map_err(internal)?;
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut out = [0u8; 32];
    argon.hash_password_into(login_key, salt, &mut out).map_err(internal)?;
    Ok(out)
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn parse_vault(b64_vault: &str) -> Result<(Vec<u8>, Vault), ApiError> {
    let bytes = unb64(b64_vault).map_err(bad)?;
    if bytes.len() > MAX_VAULT_BYTES {
        return Err(bad("cofre demasiado grande"));
    }
    let vault = Vault::from_bytes(&bytes).map_err(bad)?;
    vault.public_key().map_err(bad)?;
    Ok((bytes, vault))
}

async fn register(
    State(st): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Result<StatusCode, ApiError> {
    let username = normalize_username(&req.username).map_err(bad)?;
    let key = parse_key_hex(&req.login_key).map_err(bad)?;
    let (bytes, vault) = parse_vault(&req.vault)?;
    {
        let mut l = st.limits.lock().unwrap();
        let list = l.registers.entry(st.client_ip(addr, &headers)).or_default();
        if Limits::recent(list, REGISTER_WINDOW) >= MAX_REGISTER_PER_IP {
            return Err(ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                "Criaste demasiadas contas. Tenta daqui a uma hora.".into(),
            ));
        }
        list.push(Instant::now());
    }
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(internal)?;
    let ver = verifier(&key, &salt)?;
    let t = now();
    let db = st.db.lock().unwrap();
    let res = db.execute(
        "INSERT INTO contas (utilizador, chave_publica, sal, verificador, cofre, criada, atualizada)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![username, vault.public_key, salt, ver, bytes, t],
    );
    match res {
        Ok(_) => {
            info!("conta criada: {username}");
            Ok(StatusCode::CREATED)
        }
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
            Err(ApiError(StatusCode::CONFLICT, "Esse nome de utilizador já está ocupado.".into()))
        }
        Err(e) => Err(internal(e)),
    }
}

/// Confirma a chave de login, contando falhas por utilizador.
fn authenticate(st: &AppState, username: &str, key_hex: &str) -> Result<(), ApiError> {
    {
        let mut l = st.limits.lock().unwrap();
        let list = l.fails.entry(username.to_string()).or_default();
        if Limits::recent(list, FAIL_WINDOW) >= MAX_FAILS {
            return Err(ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                "Demasiadas tentativas erradas. Espera 15 minutos.".into(),
            ));
        }
    }
    let key = parse_key_hex(key_hex).map_err(bad)?;
    let row: Option<(Vec<u8>, Vec<u8>)> = st
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT sal, verificador FROM contas WHERE utilizador = ?1",
            params![username],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(internal)?;
    let ok = match &row {
        Some((salt, stored)) => ct_eq(&verifier(&key, salt)?, stored),
        None => {
            // Faz o mesmo trabalho para não revelar se o utilizador existe pelo tempo de resposta.
            let _ = verifier(&key, &[0u8; 16]);
            false
        }
    };
    if !ok {
        st.limits.lock().unwrap().fails.entry(username.to_string()).or_default().push(Instant::now());
        return Err(wrong_login());
    }
    st.limits.lock().unwrap().fails.remove(username);
    Ok(())
}

async fn login(State(st): State<AppState>, Json(req): Json<LoginRequest>) -> Result<Json<LoginResponse>, ApiError> {
    let username = normalize_username(&req.username).map_err(|_| wrong_login())?;
    authenticate(&st, &username, &req.login_key)?;
    let vault: Vec<u8> = st
        .db
        .lock()
        .unwrap()
        .query_row("SELECT cofre FROM contas WHERE utilizador = ?1", params![username], |r| r.get(0))
        .map_err(internal)?;
    Ok(Json(LoginResponse { vault: b64(&vault) }))
}

async fn update(State(st): State<AppState>, Json(req): Json<UpdateRequest>) -> Result<StatusCode, ApiError> {
    let username = normalize_username(&req.username).map_err(|_| wrong_login())?;
    authenticate(&st, &username, &req.login_key)?;
    let (bytes, vault) = parse_vault(&req.vault)?;
    let db = st.db.lock().unwrap();
    let current: Vec<u8> = db
        .query_row("SELECT chave_publica FROM contas WHERE utilizador = ?1", params![username], |r| r.get(0))
        .map_err(internal)?;
    if current != vault.public_key {
        return Err(bad("o cofre novo é de outra conta"));
    }
    match &req.new_login_key {
        Some(new_hex) => {
            let new_key = parse_key_hex(new_hex).map_err(bad)?;
            let mut salt = [0u8; 16];
            getrandom::fill(&mut salt).map_err(internal)?;
            let ver = verifier(&new_key, &salt)?;
            db.execute(
                "UPDATE contas SET cofre = ?2, sal = ?3, verificador = ?4, atualizada = ?5 WHERE utilizador = ?1",
                params![username, bytes, salt, ver, now()],
            )
            .map_err(internal)?;
        }
        None => {
            db.execute(
                "UPDATE contas SET cofre = ?2, atualizada = ?3 WHERE utilizador = ?1",
                params![username, bytes, now()],
            )
            .map_err(internal)?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn rename(State(st): State<AppState>, Json(req): Json<RenameRequest>) -> Result<StatusCode, ApiError> {
    let username = normalize_username(&req.username).map_err(|_| wrong_login())?;
    let new_username = normalize_username(&req.new_username).map_err(bad)?;
    authenticate(&st, &username, &req.login_key)?;
    if new_username == username {
        return Ok(StatusCode::NO_CONTENT);
    }
    let new_key = parse_key_hex(&req.new_login_key).map_err(bad)?;
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(internal)?;
    let ver = verifier(&new_key, &salt)?;
    let db = st.db.lock().unwrap();
    let res = db.execute(
        "UPDATE contas SET utilizador = ?2, sal = ?3, verificador = ?4, atualizada = ?5 WHERE utilizador = ?1",
        params![username, new_username, salt, ver, now()],
    );
    match res {
        Ok(_) => {
            info!("conta renomeada: {username} -> {new_username}");
            Ok(StatusCode::NO_CONTENT)
        }
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
            Err(ApiError(StatusCode::CONFLICT, "Esse nome de utilizador já está ocupado.".into()))
        }
        Err(e) => Err(internal(e)),
    }
}

async fn lookup(State(st): State<AppState>, UrlPath(nome): UrlPath<String>) -> Result<Json<UserInfo>, ApiError> {
    let username = normalize_username(&nome).map_err(bad)?;
    let key: Option<Vec<u8>> = st
        .db
        .lock()
        .unwrap()
        .query_row("SELECT chave_publica FROM contas WHERE utilizador = ?1", params![username], |r| r.get(0))
        .optional()
        .map_err(internal)?;
    let key = key.ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Utilizador não encontrado.".into()))?;
    let arr: [u8; 32] = key.try_into().map_err(|_| internal("chave guardada inválida"))?;
    let pk = tasca_core::iroh_public_key(&arr).map_err(internal)?;
    Ok(Json(UserInfo { username, public_key: pk }))
}

/// Arranca o servidor em `addr` (bloqueia até terminar).
pub async fn serve(state: AppState, addr: SocketAddr) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("servidor de contas a ouvir em {}", listener.local_addr()?);
    axum::serve(listener, router(state).into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}
