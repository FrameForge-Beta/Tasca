//! Servidor de contas: login com utilizador e palavra-passe em qualquer PC.
//!
//! O servidor guarda apenas:
//! - o nome de utilizador e a chave pública;
//! - o cofre da conta, **já encriptado no PC** com a palavra-passe (ver [`crate::account`]);
//! - um verificador da "chave de login".
//!
//! A palavra-passe nunca sai do PC. A partir dela o PC deriva duas coisas diferentes:
//! a chave que encripta o cofre (com sal aleatório, dentro do cofre) e a
//! [`login_key`] (com sal derivado do nome de utilizador), que é a única coisa enviada
//! ao servidor para provar que se sabe a palavra-passe. Saber uma não ajuda a obter a outra.
//!
//! Este módulo tem os tipos do protocolo (usados também pelo `tasca-servidor`) e,
//! com a feature `servidor`, o cliente HTTP.

use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Tamanho mínimo e máximo de um nome de utilizador.
pub const USERNAME_MIN: usize = 3;
pub const USERNAME_MAX: usize = 24;

/// Normaliza e valida um nome de utilizador: minúsculas, letras, números, `_` e `.`.
pub fn normalize_username(name: &str) -> Result<String> {
    let n = name.trim().trim_start_matches('@').to_lowercase();
    ensure!(
        (USERNAME_MIN..=USERNAME_MAX).contains(&n.chars().count()),
        "o nome de utilizador tem de ter entre {USERNAME_MIN} e {USERNAME_MAX} caracteres"
    );
    ensure!(
        n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.'),
        "o nome de utilizador só pode ter letras, números, _ e ."
    );
    ensure!(!n.starts_with('.') && !n.ends_with('.'), "o nome de utilizador não pode começar nem acabar em .");
    Ok(n)
}

/// Deriva a chave de login a partir do utilizador e da palavra-passe (Argon2id, 64 MiB).
pub fn login_key(username: &str, password: &str) -> Result<Zeroizing<[u8; 32]>> {
    let user = normalize_username(username)?;
    let salt = blake3::derive_key("tasca 2026 login v1", user.as_bytes());
    let params = argon2::Params::new(64 * 1024, 3, 1, Some(32)).map_err(|e| anyhow!("Argon2: {e}"))?;
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(password.as_bytes(), &salt[..16], out.as_mut())
        .map_err(|e| anyhow!("Argon2: {e}"))?;
    Ok(out)
}

// ---------------- protocolo (JSON) ----------------

/// `POST /v1/registar`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    /// Chave de login em hex (32 bytes).
    pub login_key: String,
    /// Cofre encriptado em base64.
    pub vault: String,
}

/// `POST /v1/entrar`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub login_key: String,
}

/// Resposta de `POST /v1/entrar`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub vault: String,
}

/// `POST /v1/cofre`: substitui o cofre (por exemplo, depois de mudar a palavra-passe).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub username: String,
    pub login_key: String,
    /// Nova chave de login, se a palavra-passe mudou.
    #[serde(default)]
    pub new_login_key: Option<String>,
    pub vault: String,
}

/// Resposta de `GET /v1/utilizador/{nome}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    /// Chave pública (formato do iroh).
    pub public_key: String,
}

/// Corpo de erro: `{"erro": "..."}` em português, pronto a mostrar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub erro: String,
}

pub fn b64(bytes: &[u8]) -> String {
    data_encoding::BASE64.encode(bytes)
}

pub fn unb64(s: &str) -> Result<Vec<u8>> {
    data_encoding::BASE64.decode(s.as_bytes()).map_err(|_| anyhow!("base64 inválido"))
}

pub fn parse_key_hex(s: &str) -> Result<[u8; 32]> {
    let v = hex::decode(s).map_err(|_| anyhow!("chave inválida"))?;
    v.try_into().map_err(|_| anyhow!("chave com tamanho errado"))
}

// ---------------- cliente ----------------

#[cfg(feature = "servidor")]
pub use client::AccountServer;

#[cfg(feature = "servidor")]
mod client {
    use std::time::Duration;

    use anyhow::{Context, Result, anyhow, bail, ensure};

    use super::*;
    use crate::account::{Account, Vault};

    /// Cliente do servidor de contas.
    #[derive(Clone, Debug)]
    pub struct AccountServer {
        base: url::Url,
        http: reqwest::Client,
    }

    impl AccountServer {
        /// `url` é a raiz do servidor, por exemplo `https://contas.tasca.pt`.
        /// Só aceita `http://` para `localhost`/`127.0.0.1` (testes).
        pub fn new(url: &str) -> Result<Self> {
            let base = url::Url::parse(url).context("endereço do servidor inválido")?;
            let local = matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
            ensure!(base.scheme() == "https" || local, "o servidor de contas tem de usar https://");
            let http = reqwest::Client::builder().timeout(Duration::from_secs(20)).build()?;
            Ok(Self { base, http })
        }

        /// Lê `TASCA_SERVIDOR_URL` (em tempo de compilação e depois de execução).
        pub fn from_env() -> Option<Self> {
            let url = option_env!("TASCA_SERVIDOR_URL")
                .map(String::from)
                .or_else(|| std::env::var("TASCA_SERVIDOR_URL").ok())
                .filter(|u| !u.is_empty())?;
            Self::new(&url).ok()
        }

        fn url(&self, path: &str) -> url::Url {
            self.base.join(path).expect("caminho fixo")
        }

        async fn check(resp: reqwest::Response) -> Result<reqwest::Response> {
            if resp.status().is_success() {
                return Ok(resp);
            }
            let status = resp.status();
            let msg = resp
                .json::<ErrorBody>()
                .await
                .map(|e| e.erro)
                .unwrap_or_else(|_| format!("o servidor respondeu {status}"));
            bail!("{msg}")
        }

        /// Cria a conta no servidor com o cofre já fechado com a palavra-passe.
        pub async fn register(&self, username: &str, password: &str, vault: &Vault) -> Result<()> {
            let username = normalize_username(username)?;
            let key = login_key(&username, password)?;
            let body = RegisterRequest {
                username,
                login_key: hex::encode(key.as_ref()),
                vault: b64(&vault.to_bytes()),
            };
            Self::check(self.http.post(self.url("v1/registar")).json(&body).send().await.map_err(net)?).await?;
            Ok(())
        }

        /// Entra com utilizador e palavra-passe: descarrega o cofre e abre-o no PC.
        pub async fn login(&self, username: &str, password: &str) -> Result<(Vault, Account)> {
            let username = normalize_username(username)?;
            let key = login_key(&username, password)?;
            let body = LoginRequest { username, login_key: hex::encode(key.as_ref()) };
            let resp: LoginResponse = Self::check(
                self.http.post(self.url("v1/entrar")).json(&body).send().await.map_err(net)?,
            )
            .await?
            .json()
            .await?;
            let vault = Vault::from_bytes(&unb64(&resp.vault)?)?;
            let account = vault.open(password)?;
            Ok((vault, account))
        }

        /// Substitui o cofre no servidor. `new_password` quando a palavra-passe mudou.
        pub async fn update(
            &self,
            username: &str,
            password: &str,
            new_password: Option<&str>,
            vault: &Vault,
        ) -> Result<()> {
            let username = normalize_username(username)?;
            let key = login_key(&username, password)?;
            let new_key = new_password.map(|p| login_key(&username, p)).transpose()?;
            let body = UpdateRequest {
                username,
                login_key: hex::encode(key.as_ref()),
                new_login_key: new_key.map(|k| hex::encode(k.as_ref())),
                vault: b64(&vault.to_bytes()),
            };
            Self::check(self.http.post(self.url("v1/cofre")).json(&body).send().await.map_err(net)?).await?;
            Ok(())
        }

        /// Procura a chave pública de um utilizador (para convites por nome).
        pub async fn lookup(&self, username: &str) -> Result<Option<UserInfo>> {
            let username = normalize_username(username)?;
            let resp = self
                .http
                .get(self.url(&format!("v1/utilizador/{username}")))
                .send()
                .await
                .map_err(net)?;
            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok(None);
            }
            Ok(Some(Self::check(resp).await?.json().await?))
        }
    }

    fn net(e: reqwest::Error) -> anyhow::Error {
        if e.is_timeout() || e.is_connect() {
            anyhow!("não consegui ligar ao servidor de contas. Verifica a internet e tenta outra vez.")
        } else {
            anyhow!("erro de rede: {e}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomes_de_utilizador() {
        assert_eq!(normalize_username("  @Tiago_F ").unwrap(), "tiago_f");
        assert!(normalize_username("ab").is_err());
        assert!(normalize_username("tiago fernandes").is_err());
        assert!(normalize_username("çé").is_err());
        assert!(normalize_username(".tiago").is_err());
        assert!(normalize_username(&"a".repeat(25)).is_err());
    }

    #[test]
    fn chave_de_login_depende_do_utilizador() {
        let a = login_key("tiago", "palavra-passe").unwrap();
        let b = login_key("TIAGO", "palavra-passe").unwrap();
        let c = login_key("outro", "palavra-passe").unwrap();
        assert_eq!(*a, *b);
        assert_ne!(*a, *c);
    }
}
