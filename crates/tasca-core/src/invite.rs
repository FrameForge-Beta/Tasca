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
