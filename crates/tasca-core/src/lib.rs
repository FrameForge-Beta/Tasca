//! Núcleo da Tasca: identidade, registo de eventos assinado, armazenamento e rede P2P.
//!
//! Cada grupo vive no PC do anfitrião (o dono). Os membros ligam-se diretamente
//! ao anfitrião através do iroh e guardam uma cópia local do histórico.

pub mod account;
pub mod amigos;
pub mod caixa;
pub mod event;
#[cfg(feature = "google")]
pub mod google;
pub mod identity;
pub mod invite;
pub mod node;
pub mod protocol;
pub mod servidor;
pub mod store;

pub use caixa::ChaveCaixa;
pub use account::{Account, AccountStore, RecoveryPhrase, Vault};
pub use event::{ChannelDef, ChannelKind, EventKind, GroupId, SignedEvent};
pub use identity::Identity;
pub use invite::Invite;
pub use node::{GroupHandle, Node, NodeOptions};
pub use store::Store;
pub use iroh::PublicKey;

/// Identificador do protocolo da Tasca na negociação QUIC (ALPN).
pub const ALPN: &[u8] = b"tasca/0";

/// Converte 32 bytes numa chave pública do iroh, em texto.
pub fn iroh_public_key(bytes: &[u8; 32]) -> anyhow::Result<String> {
    Ok(iroh::PublicKey::from_bytes(bytes).map_err(|e| anyhow::anyhow!("{e}"))?.to_string())
}
