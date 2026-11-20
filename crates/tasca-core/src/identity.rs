//! A identidade de um utilizador é um par de chaves ed25519.
//!
//! A mesma chave identifica o utilizador no iroh, por isso a ligação QUIC já
//! prova quem está do outro lado.

use std::path::Path;

use anyhow::{Context, Result, bail};
use iroh::{PublicKey, SecretKey};

#[derive(Clone)]
pub struct Identity {
    secret: SecretKey,
}

impl Identity {
    pub fn generate() -> Self {
        Self {
            secret: SecretKey::generate(),
        }
    }

    /// Carrega a chave do ficheiro, ou cria uma nova se ainda não existir.
    pub fn load_or_create(path: &Path) -> Result<Self> {
        if path.exists() {
            let bytes = std::fs::read(path).with_context(|| format!("ler {}", path.display()))?;
            let Ok(arr) = <[u8; 32]>::try_from(bytes.as_slice()) else {
                bail!("ficheiro de identidade inválido: {}", path.display());
            };
            return Ok(Self {
                secret: SecretKey::from_bytes(&arr),
            });
        }
        let id = Self::generate();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, id.secret.to_bytes())?;
        Ok(id)
    }

    pub fn public(&self) -> PublicKey {
        self.secret.public()
    }

    pub fn secret(&self) -> &SecretKey {
        &self.secret
    }
}
