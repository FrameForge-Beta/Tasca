//! Vários dispositivos na mesma conta.
//!
//! O PC usa a chave da conta como chave do nó iroh: é por ela que os outros o encontram
//! (anfitrião dos grupos, amigos). Um segundo dispositivo (o telemóvel) não pode usar a
//! mesma chave ao mesmo tempo, porque dois nós com a mesma chave lutam pelo mesmo lugar
//! no relay e as ligações caem. Por isso tem uma chave própria e, antes de cada ligação,
//! apresenta-se ao outro lado com um [`Certificado`] assinado pela conta: "este dispositivo
//! é meu". Daí em diante o outro lado trata as ligações desse dispositivo como sendo da
//! conta.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Result, anyhow, ensure};
use iroh::{
    Endpoint, EndpointAddr, PublicKey, Signature,
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
};
use serde::{Deserialize, Serialize};

use crate::{
    identity::Identity,
    protocol::{read_frame, write_frame},
};

/// Protocolo onde um dispositivo se apresenta.
pub const ALPN: &[u8] = b"tasca/dispositivo/0";

const CTX: &str = "tasca 2026 certificado de dispositivo v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Certificado {
    /// Chave da conta.
    pub conta: [u8; 32],
    /// Chave do nó iroh do dispositivo.
    pub dispositivo: [u8; 32],
    pub assinatura: Vec<u8>,
}

fn mensagem(conta: &[u8; 32], dispositivo: &[u8; 32]) -> [u8; 32] {
    let mut h = blake3::Hasher::new_derive_key(CTX);
    h.update(conta);
    h.update(dispositivo);
    *h.finalize().as_bytes()
}

impl Certificado {
    pub fn emitir(conta: &Identity, dispositivo: &PublicKey) -> Self {
        let c = *conta.public().as_bytes();
        let d = *dispositivo.as_bytes();
        let assinatura = conta.secret().sign(&mensagem(&c, &d)).to_bytes().to_vec();
        Self { conta: c, dispositivo: d, assinatura }
    }

    /// Confirma a assinatura e que o certificado é do dispositivo `de` (quem ligou).
    /// Devolve a chave da conta.
    pub fn verificar(&self, de: &PublicKey) -> Result<PublicKey> {
        ensure!(self.dispositivo == *de.as_bytes(), "certificado de outro dispositivo");
        let sig: [u8; 64] = self.assinatura.as_slice().try_into().map_err(|_| anyhow!("assinatura inválida"))?;
        let conta = PublicKey::from_bytes(&self.conta).map_err(|e| anyhow!("chave inválida: {e}"))?;
        conta
            .verify(&mensagem(&self.conta, &self.dispositivo), &Signature::from_bytes(&sig))
            .map_err(|_| anyhow!("assinatura inválida"))?;
        Ok(conta)
    }
}

/// Dispositivos que se apresentaram a este nó: chave do dispositivo → chave da conta.
#[derive(Default)]
pub struct Conhecidos(Mutex<HashMap<[u8; 32], [u8; 32]>>);

impl Conhecidos {
    /// A conta de quem está do outro lado de uma ligação. Um dispositivo que não se
    /// apresentou é a própria conta (o PC usa a chave da conta como chave do nó).
    pub fn conta_de(&self, remoto: &PublicKey) -> PublicKey {
        self.0
            .lock()
            .unwrap()
            .get(remoto.as_bytes())
            .and_then(|c| PublicKey::from_bytes(c).ok())
            .unwrap_or(*remoto)
    }

    fn juntar(&self, dispositivo: [u8; 32], conta: [u8; 32]) {
        let mut m = self.0.lock().unwrap();
        if m.len() > 10_000 {
            m.clear();
        }
        m.insert(dispositivo, conta);
    }
}

/// Lado de quem recebe: guarda o certificado de quem se apresenta.
#[derive(Clone)]
pub(crate) struct Apresentacao(pub(crate) Arc<Conhecidos>);

impl std::fmt::Debug for Apresentacao {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Apresentacao")
    }
}

impl ProtocolHandler for Apresentacao {
    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        self.atender(conn).await.map_err(|e| AcceptError::from_boxed(e.into()))
    }
}

impl Apresentacao {
    async fn atender(&self, conn: Connection) -> Result<()> {
        let remoto = conn.remote_id();
        let (mut send, mut recv) = conn.accept_bi().await?;
        let Some(cert) = read_frame::<Certificado>(&mut recv).await? else {
            return Ok(());
        };
        let ok = match cert.verificar(&remoto) {
            Ok(conta) => {
                self.0.juntar(*remoto.as_bytes(), *conta.as_bytes());
                true
            }
            Err(_) => false,
        };
        write_frame(&mut send, &ok).await?;
        send.finish()?;
        let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
        Ok(())
    }
}

/// Lado do dispositivo: apresenta-se a `a` antes de abrir outra ligação.
pub(crate) async fn apresentar(endpoint: &Endpoint, a: EndpointAddr, cert: &Certificado) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(20), async {
        let conn = endpoint.connect(a, ALPN).await?;
        let (mut send, mut recv) = conn.open_bi().await?;
        write_frame(&mut send, cert).await?;
        let ok = read_frame::<bool>(&mut recv).await?;
        conn.close(0u32.into(), b"ok");
        ensure!(ok == Some(true), "o outro lado não aceitou o certificado");
        anyhow::Ok(())
    })
    .await
    .map_err(|_| anyhow!("o outro lado não respondeu"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificado_so_vale_para_o_seu_dispositivo() {
        let conta = Identity::generate();
        let tel = iroh::SecretKey::generate().public();
        let outro = iroh::SecretKey::generate().public();
        let c = Certificado::emitir(&conta, &tel);
        assert_eq!(c.verificar(&tel).unwrap(), conta.public());
        assert!(c.verificar(&outro).is_err());
        let mut falso = c.clone();
        falso.conta = *Identity::generate().public().as_bytes();
        assert!(falso.verificar(&tel).is_err());
    }
}
