//! Núcleo da Tasca: identidade, registo de eventos assinado, armazenamento e rede P2P.
//!
//! Cada grupo vive no PC do anfitrião (o dono). Os membros ligam-se diretamente
//! ao anfitrião através do iroh e guardam uma cópia local do histórico.

pub mod event;
pub mod identity;
pub mod invite;
pub mod node;
pub mod protocol;
pub mod store;

pub use event::{EventKind, GroupId, SignedEvent};
pub use identity::Identity;
pub use invite::Invite;
pub use node::{GroupHandle, Node, NodeOptions};
pub use store::Store;

/// Identificador do protocolo da Tasca na negociação QUIC (ALPN).
pub const ALPN: &[u8] = b"tasca/0";
