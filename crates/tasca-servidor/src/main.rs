//! `tasca-servidor`: servidor de contas da Tasca.
//!
//! Variáveis de ambiente:
//! - `TASCA_ENDERECO`: onde ouvir (por omissão `0.0.0.0:8080`).
//! - `TASCA_DADOS`: ficheiro SQLite (por omissão `./dados/contas.db`).
//! - `TASCA_PROXY=1`: está atrás de um proxy; conta o IP do `X-Forwarded-For` nos limites.
//!
//! Deve ficar atrás de HTTPS (por exemplo Caddy ou um túnel da Cloudflare).

use std::net::SocketAddr;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let addr: SocketAddr = std::env::var("TASCA_ENDERECO")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()
        .context("TASCA_ENDERECO inválido")?;
    let path = std::env::var("TASCA_DADOS").unwrap_or_else(|_| "dados/contas.db".into());
    let state = tasca_servidor::AppState::open(std::path::Path::new(&path))?
        .trust_proxy(std::env::var("TASCA_PROXY").is_ok_and(|v| v == "1"));
    tasca_servidor::serve(state, addr).await
}
