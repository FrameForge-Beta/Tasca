//! Login com Google e cópia da conta no Google Drive.
//!
//! Funciona sem servidor nenhum da Tasca:
//! 1. A app abre o browser na página de login da Google (OAuth 2.0 para apps de
//!    desktop, com PKCE e redirecionamento para `127.0.0.1`).
//! 2. A Google devolve um código à app, que o troca por um token.
//! 3. Com esse token a app guarda o cofre da conta (já encriptado com a palavra-passe)
//!    na pasta escondida da app no Drive (`appDataFolder`). Só a Tasca vê essa pasta,
//!    e a Google não consegue abrir o cofre.
//!
//! Para funcionar é preciso um "OAuth client ID" do tipo *Desktop app* criado na
//! Google Cloud Console, com a API do Google Drive ativa.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use crate::account::Vault;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const DRIVE_UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const SCOPES: &str = "openid email https://www.googleapis.com/auth/drive.appdata";
const VAULT_NAME: &str = "tasca-conta.cofre";

/// Credenciais do cliente OAuth (tipo "Desktop app") da Google Cloud Console.
///
/// Em apps de desktop a Google não considera o `client_secret` secreto, por isso
/// pode ir dentro da app.
#[derive(Clone, Debug)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
}

impl GoogleConfig {
    /// Lê `TASCA_GOOGLE_CLIENT_ID` e `TASCA_GOOGLE_CLIENT_SECRET`, primeiro em tempo de
    /// compilação (para a versão distribuída) e depois em tempo de execução.
    pub fn from_env() -> Option<Self> {
        let id = option_env!("TASCA_GOOGLE_CLIENT_ID")
            .map(String::from)
            .or_else(|| std::env::var("TASCA_GOOGLE_CLIENT_ID").ok())?;
        let secret = option_env!("TASCA_GOOGLE_CLIENT_SECRET")
            .map(String::from)
            .or_else(|| std::env::var("TASCA_GOOGLE_CLIENT_SECRET").ok())?;
        if id.is_empty() {
            return None;
        }
        Some(Self {
            client_id: id,
            client_secret: secret,
        })
    }

    /// Começa o login: abre uma porta local para receber a resposta e devolve o URL
    /// que a app deve abrir no browser.
    pub async fn begin_sign_in(&self) -> Result<PendingSignIn> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("abrir porta local")?;
        let port = listener.local_addr()?.port();
        let redirect_uri = format!("http://127.0.0.1:{port}");
        let verifier = random_urlsafe(48);
        let state = random_urlsafe(16);
        let url = auth_url(
            &self.client_id,
            &redirect_uri,
            &pkce_challenge(&verifier),
            &state,
        );
        Ok(PendingSignIn {
            url,
            listener,
            verifier,
            state,
            redirect_uri,
            config: self.clone(),
        })
    }
}

/// Login a meio: à espera que o utilizador termine no browser.
pub struct PendingSignIn {
    /// Abrir este URL no browser do utilizador.
    pub url: String,
    listener: TcpListener,
    verifier: String,
    state: String,
    redirect_uri: String,
    config: GoogleConfig,
}

impl PendingSignIn {
    /// Espera pela resposta da Google (até `timeout`) e termina o login.
    pub async fn finish(self, timeout: Duration) -> Result<GoogleSession> {
        let code = tokio::time::timeout(timeout, wait_for_code(&self.listener, &self.state))
            .await
            .map_err(|_| anyhow!("o login com Google demorou demasiado. Tenta outra vez."))??;
        let http = http_client()?;
        let token: TokenResponse = http
            .post(TOKEN_URL)
            .form(&[
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("code", code.as_str()),
                ("code_verifier", self.verifier.as_str()),
                ("grant_type", "authorization_code"),
                ("redirect_uri", self.redirect_uri.as_str()),
            ])
            .send()
            .await?
            .error_for_status()
            .context("a Google recusou o código de login")?
            .json()
            .await?;
        let mut session = GoogleSession {
            http,
            config: self.config,
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_at: Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(60)),
            email: String::new(),
        };
        session.email = session.fetch_email().await?;
        Ok(session)
    }
}

/// Sessão Google ativa (fica só em memória).
pub struct GoogleSession {
    http: reqwest::Client,
    config: GoogleConfig,
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Instant,
    pub email: String,
}

impl std::fmt::Debug for GoogleSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoogleSession")
            .field("email", &self.email)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default = "default_expiry")]
    expires_in: u64,
}

fn default_expiry() -> u64 {
    3600
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<DriveFile>,
}

#[derive(Deserialize)]
struct DriveFile {
    id: String,
}

impl GoogleSession {
    async fn token(&mut self) -> Result<String> {
        if Instant::now() >= self.expires_at {
            let Some(refresh) = self.refresh_token.clone() else {
                bail!("a sessão Google expirou. Entra outra vez com Google.");
            };
            let token: TokenResponse = self
                .http
                .post(TOKEN_URL)
                .form(&[
                    ("client_id", self.config.client_id.as_str()),
                    ("client_secret", self.config.client_secret.as_str()),
                    ("refresh_token", refresh.as_str()),
                    ("grant_type", "refresh_token"),
                ])
                .send()
                .await?
                .error_for_status()
                .context("não consegui renovar a sessão Google")?
                .json()
                .await?;
            self.access_token = token.access_token;
            self.expires_at =
                Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(60));
        }
        Ok(self.access_token.clone())
    }

    async fn fetch_email(&mut self) -> Result<String> {
        #[derive(Deserialize)]
        struct UserInfo {
            email: String,
        }
        let token = self.token().await?;
        let info: UserInfo = self
            .http
            .get(USERINFO_URL)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()
            .context("não consegui ler o email da conta Google")?
            .json()
            .await?;
        Ok(info.email)
    }

    async fn find_vault_id(&mut self) -> Result<Option<String>> {
        let token = self.token().await?;
        let list: FileList = self
            .http
            .get(DRIVE_FILES_URL)
            .bearer_auth(token)
            .query(&[
                ("spaces", "appDataFolder"),
                ("q", &format!("name = '{VAULT_NAME}' and trashed = false")),
                ("fields", "files(id)"),
                ("orderBy", "modifiedTime desc"),
            ])
            .send()
            .await?
            .error_for_status()
            .context("não consegui ler o Google Drive")?
            .json()
            .await?;
        Ok(list.files.into_iter().next().map(|f| f.id))
    }

    /// Há uma cópia da conta neste Google Drive?
    pub async fn has_vault(&mut self) -> Result<bool> {
        Ok(self.find_vault_id().await?.is_some())
    }

    /// Descarrega a cópia da conta (ainda encriptada).
    pub async fn download_vault(&mut self) -> Result<Option<Vault>> {
        let Some(id) = self.find_vault_id().await? else {
            return Ok(None);
        };
        let token = self.token().await?;
        let bytes = self
            .http
            .get(format!("{DRIVE_FILES_URL}/{id}"))
            .bearer_auth(token)
            .query(&[("alt", "media")])
            .send()
            .await?
            .error_for_status()
            .context("não consegui descarregar a conta do Google Drive")?
            .bytes()
            .await?;
        ensure!(
            bytes.len() < 64 * 1024,
            "ficheiro da conta demasiado grande"
        );
        Ok(Some(Vault::from_bytes(&bytes)?))
    }

    /// Guarda (ou substitui) a cópia da conta no Google Drive.
    pub async fn upload_vault(&mut self, vault: &Vault) -> Result<()> {
        let data = vault.to_bytes();
        let existing = self.find_vault_id().await?;
        let token = self.token().await?;
        let request = match existing {
            Some(id) => self
                .http
                .patch(format!("{DRIVE_UPLOAD_URL}/{id}"))
                .query(&[("uploadType", "media")])
                .header("Content-Type", "application/octet-stream")
                .body(data),
            None => {
                let boundary = random_urlsafe(24);
                self.http
                    .post(DRIVE_UPLOAD_URL)
                    .query(&[("uploadType", "multipart")])
                    .header(
                        "Content-Type",
                        format!("multipart/related; boundary={boundary}"),
                    )
                    .body(multipart_body(&boundary, &data))
            }
        };
        request
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()
            .context("não consegui guardar a conta no Google Drive")?;
        Ok(())
    }
}

fn http_client() -> Result<reqwest::Client> {
    Ok(crate::http_builder()
        .timeout(Duration::from_secs(30))
        .build()?)
}

fn auth_url(client_id: &str, redirect_uri: &str, challenge: &str, state: &str) -> String {
    let mut url = url::Url::parse(AUTH_URL).expect("URL fixo");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPES)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "select_account consent");
    url.into()
}

fn pkce_challenge(verifier: &str) -> String {
    data_encoding::BASE64URL_NOPAD.encode(&Sha256::digest(verifier.as_bytes()))
}

fn random_urlsafe(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("gerador aleatório do sistema");
    data_encoding::BASE64URL_NOPAD.encode(&buf)
}

fn multipart_body(boundary: &str, data: &[u8]) -> Vec<u8> {
    let meta = serde_json::json!({ "name": VAULT_NAME, "parents": ["appDataFolder"] });
    let mut body = Vec::new();
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

const PAGE_OK: &str = "<!doctype html><meta charset=utf-8><title>Tasca</title>\
<body style=\"font-family:sans-serif;background:#1C1714;color:#F6EEE1;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center\"><h1>Sessão iniciada</h1><p>Já podes fechar esta janela e voltar à Tasca.</p></div>";
const PAGE_ERR: &str = "<!doctype html><meta charset=utf-8><title>Tasca</title>\
<body style=\"font-family:sans-serif;background:#1C1714;color:#F6EEE1;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center\"><h1>O login não foi concluído</h1><p>Volta à Tasca e tenta outra vez.</p></div>";

/// Espera pelo pedido do browser em `127.0.0.1` e extrai o código.
async fn wait_for_code(listener: &TcpListener, expected_state: &str) -> Result<String> {
    loop {
        let (mut sock, _) = listener.accept().await?;
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await?;
        let request = String::from_utf8_lossy(&buf[..n]);
        let Some(target) = request
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
        else {
            continue;
        };
        // O browser às vezes pede o favicon: ignora tudo o que não traga parâmetros.
        if !target.contains('?') {
            respond(&mut sock, "404 Not Found", "").await;
            continue;
        }
        let result = parse_callback(target, expected_state);
        let page = if result.is_ok() { PAGE_OK } else { PAGE_ERR };
        respond(&mut sock, "200 OK", page).await;
        return result;
    }
}

async fn respond(sock: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = sock.write_all(resp.as_bytes()).await;
    let _ = sock.shutdown().await;
}

fn parse_callback(target: &str, expected_state: &str) -> Result<String> {
    let url = url::Url::parse(&format!("http://127.0.0.1{target}")).context("resposta inválida")?;
    let get = |k: &str| {
        url.query_pairs()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.into_owned())
    };
    if let Some(err) = get("error") {
        if err == "access_denied" {
            bail!("cancelaste o login com Google");
        }
        bail!("a Google devolveu um erro: {err}");
    }
    ensure!(
        get("state").as_deref() == Some(expected_state),
        "resposta de login inválida (state)"
    );
    get("code").ok_or_else(|| anyhow!("a Google não devolveu código"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_como_no_rfc_7636() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn url_de_login_tem_o_necessario() {
        let u = url::Url::parse(&auth_url("cid", "http://127.0.0.1:5000", "chal", "st")).unwrap();
        let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
        assert_eq!(q["client_id"], "cid");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:5000");
        assert_eq!(q["code_challenge_method"], "S256");
        assert!(q["scope"].contains("drive.appdata"));
        assert_eq!(q["access_type"], "offline");
    }

    #[test]
    fn resposta_do_browser() {
        assert_eq!(
            parse_callback("/?state=abc&code=xyz", "abc").unwrap(),
            "xyz"
        );
        assert!(parse_callback("/?state=outro&code=xyz", "abc").is_err());
        let e = parse_callback("/?error=access_denied&state=abc", "abc").unwrap_err();
        assert!(e.to_string().contains("cancelaste"));
    }

    #[test]
    fn corpo_multipart() {
        let body = String::from_utf8(multipart_body("B", b"dados")).unwrap();
        assert!(body.starts_with("--B\r\nContent-Type: application/json"));
        assert!(body.contains("\"parents\":[\"appDataFolder\"]"));
        assert!(body.contains("\r\n\r\ndados\r\n--B--\r\n"));
    }

    #[tokio::test]
    async fn recebe_o_codigo_no_127_0_0_1() {
        let cfg = GoogleConfig {
            client_id: "cid".into(),
            client_secret: "s".into(),
        };
        let pending = cfg.begin_sign_in().await.unwrap();
        let u = url::Url::parse(&pending.url).unwrap();
        let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
        let redirect = q["redirect_uri"].clone();
        let state = q["state"].clone();
        let port: u16 = redirect.rsplit(':').next().unwrap().parse().unwrap();

        let browser = tokio::spawn(async move {
            // Primeiro um pedido ao favicon, que deve ser ignorado.
            for path in [
                "/favicon.ico".to_string(),
                format!("/?state={state}&code=CODIGO"),
            ] {
                let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
                    .await
                    .unwrap();
                s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
                    .await
                    .unwrap();
                let mut out = String::new();
                s.read_to_string(&mut out).await.unwrap();
                if path.contains("code") {
                    assert!(out.contains("Sessão iniciada"));
                }
            }
        });
        let code = wait_for_code(&pending.listener, &pending.state)
            .await
            .unwrap();
        assert_eq!(code, "CODIGO");
        browser.await.unwrap();
    }
}
