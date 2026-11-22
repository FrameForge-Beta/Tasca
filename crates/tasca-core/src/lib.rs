//! Núcleo da Tasca: identidade, registo de eventos assinado, armazenamento e rede P2P.
//!
//! Cada grupo vive no PC do anfitrião (o dono). Os membros ligam-se diretamente
//! ao anfitrião através do iroh e guardam uma cópia local do histórico.

pub mod account;
pub mod amigos;
pub mod caixa;
#[cfg(feature = "servidor")]
mod dns;
pub mod dispositivo;
pub mod event;
pub mod ficheiros;
#[cfg(feature = "google")]
pub mod google;
pub mod identity;
pub mod invite;
pub mod node;
pub mod protocol;
pub mod servidor;
pub mod store;
pub mod voz;

/// Cliente HTTPS para o servidor de contas e o Google.
///
/// No Android não se usa o verificador de certificados do sistema (precisa de uma parte
/// Kotlin ligada à JVM, e sem ela o primeiro pedido falhava e o login ficava pendurado):
/// usa-se a lista de autoridades de certificação da Mozilla, embutida na app.
#[cfg(any(feature = "google", feature = "servidor"))]
pub(crate) fn http_builder() -> reqwest::ClientBuilder {
    let b = reqwest::Client::builder();
    #[cfg(target_os = "android")]
    let b = b.tls_certs_only(
        webpki_root_certs::TLS_SERVER_ROOT_CERTS
            .iter()
            .filter_map(|c| reqwest::Certificate::from_der(c).ok()),
    );
    b
}

pub use caixa::ChaveCaixa;
pub use account::{Account, AccountStore, RecoveryPhrase, Vault};
pub use event::{ChannelDef, ChannelKind, EventKind, GroupId, SignedEvent};
pub use identity::Identity;
pub use invite::Invite;
pub use node::{GroupHandle, Node, NodeOptions};
pub use store::Store;
pub use iroh::{PublicKey, SecretKey};
pub use voz::{Anuncio, Chamada, Imagem, NovoHost, Participante, VOZ_ALPN};

/// Identificador do protocolo da Tasca na negociação QUIC (ALPN).
pub const ALPN: &[u8] = b"tasca/0";

/// Converte 32 bytes numa chave pública do iroh, em texto.
pub fn iroh_public_key(bytes: &[u8; 32]) -> anyhow::Result<String> {
    Ok(iroh::PublicKey::from_bytes(bytes).map_err(|e| anyhow::anyhow!("{e}"))?.to_string())
}
