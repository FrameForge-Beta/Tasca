//! Códigos de convite: dizem onde está o anfitrião, qual o grupo, e trazem um segredo.

use anyhow::{Context, Result, bail};
use iroh::EndpointAddr;
use serde::{Deserialize, Serialize};

use crate::event::GroupId;

const PREFIX: &str = "tasca1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invite {
    pub host: EndpointAddr,
    pub group: GroupId,
    pub token: [u8; 16],
}

impl Invite {
    pub fn encode(&self) -> String {
        let bytes = postcard::to_stdvec(self).expect("serializar convite");
        format!(
            "{PREFIX}{}",
            data_encoding::BASE32_NOPAD.encode(&bytes).to_lowercase()
        )
    }

    pub fn decode(code: &str) -> Result<Self> {
        let code = code.trim();
        let Some(rest) = code.strip_prefix(PREFIX) else {
            bail!("isto não é um convite da Tasca");
        };
        let bytes = data_encoding::BASE32_NOPAD
            .decode(rest.to_uppercase().as_bytes())
            .context("convite mal formado")?;
        postcard::from_bytes(&bytes).context("convite mal formado")
    }
}

const PREFIX_CONVITE: &str = "tasca2";

/// Convite de um grupo que vive na caixa do servidor: traz o grupo, a chave da caixa (para
/// ler o histórico) e um segredo. Do segredo sai um par de chaves; o grupo guarda só a
/// pública (no `InviteCreated`), e quem entra assina a entrada com a privada. Assim
/// qualquer dispositivo confirma o convite sem o dono estar ligado.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Convite {
    pub group: GroupId,
    pub token: [u8; 16],
    pub epoch: u32,
    pub chave: [u8; 32],
}

impl Convite {
    pub fn encode(&self) -> String {
        let bytes = postcard::to_stdvec(self).expect("serializar convite");
        format!("{PREFIX_CONVITE}{}", data_encoding::BASE32_NOPAD.encode(&bytes).to_lowercase())
    }

    pub fn decode(code: &str) -> Result<Self> {
        let code = code.trim();
        let Some(rest) = code.strip_prefix(PREFIX_CONVITE) else {
            bail!("isto não é um convite da Tasca");
        };
        let bytes = data_encoding::BASE32_NOPAD
            .decode(rest.to_uppercase().as_bytes())
            .context("convite mal formado")?;
        postcard::from_bytes(&bytes).context("convite mal formado")
    }

    /// Identidade do convite (sai do segredo).
    pub fn identidade(&self) -> crate::identity::Identity {
        identidade_convite(&self.token)
    }

    /// O que fica publicado no grupo (`InviteCreated::hash`).
    pub fn publica(&self) -> [u8; 32] {
        *self.identidade().public().as_bytes()
    }

    /// Prova de que quem entra tem o código: assinatura do evento de entrada.
    pub fn prova(&self, entrada: &crate::event::SignedEvent) -> Vec<u8> {
        self.identidade()
            .secret()
            .sign(&msg_prova(&self.group, &entrada.id()))
            .to_bytes()
            .to_vec()
    }
}

pub fn identidade_convite(token: &[u8; 16]) -> crate::identity::Identity {
    let seed = blake3::derive_key("tasca 2026 convite v2", token);
    crate::identity::Identity::from_secret(iroh::SecretKey::from_bytes(&seed))
}

fn msg_prova(group: &GroupId, entrada: &[u8; 32]) -> Vec<u8> {
    let mut h = blake3::Hasher::new_derive_key("tasca 2026 entrada com convite v1");
    h.update(&group.0);
    h.update(entrada);
    h.finalize().as_bytes().to_vec()
}

/// Confirma que `prova` foi feita com o convite cuja chave pública é `convite`.
pub fn verificar_prova(convite: &[u8; 32], group: &GroupId, entrada: &[u8; 32], prova: &[u8]) -> bool {
    let Ok(pk) = iroh::PublicKey::from_bytes(convite) else { return false };
    let Ok(sig) = <[u8; 64]>::try_from(prova) else { return false };
    pk.verify(&msg_prova(group, entrada), &iroh::Signature::from_bytes(&sig)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convite_ida_e_volta_e_prova() {
        let c = Convite { group: GroupId([7; 32]), token: [3; 16], epoch: 2, chave: [9; 32] };
        let code = c.encode();
        assert!(code.starts_with("tasca2"));
        assert_eq!(Convite::decode(&code).unwrap(), c);
        let ana = crate::identity::Identity::generate();
        let ev = crate::event::SignedEvent::sign(&ana, Some(c.group), crate::event::EventKind::MemberJoined { display_name: "Ana".into() });
        let p = c.prova(&ev);
        assert!(verificar_prova(&c.publica(), &c.group, &ev.id(), &p));
        assert!(!verificar_prova(&c.publica(), &GroupId([8; 32]), &ev.id(), &p));
    }
}
