//! Mensagens trocadas entre membro e anfitrião, em frames `u32 + postcard`.

use anyhow::{Result, bail};
use iroh::endpoint::{RecvStream, SendStream};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::event::{GroupId, SignedEvent};

/// Tamanho máximo de uma frame (1 MiB).
const MAX_FRAME: u32 = 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMsg {
    /// Primeira mensagem. `since` é o último número de sequência que o membro já tem.
    Hello {
        group: GroupId,
        invite_token: Option<[u8; 16]>,
        since: u64,
    },
    Publish(SignedEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HostMsg {
    Welcome,
    Rejected(String),
    Event { seq: u64, event: SignedEvent },
    PublishFailed(String),
}

pub async fn write_frame<T: Serialize>(send: &mut SendStream, msg: &T) -> Result<()> {
    let bytes = postcard::to_stdvec(msg)?;
    send.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
    send.write_all(&bytes).await?;
    Ok(())
}

/// Lê a próxima frame. Devolve `None` quando o outro lado fecha o stream.
pub async fn read_frame<T: DeserializeOwned>(recv: &mut RecvStream) -> Result<Option<T>> {
    let mut len = [0u8; 4];
    match recv.read_exact(&mut len).await {
        Ok(()) => {}
        Err(iroh::endpoint::ReadExactError::FinishedEarly(0)) => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_be_bytes(len);
    if len > MAX_FRAME {
        bail!("frame demasiado grande ({len} bytes)");
    }
    let mut buf = vec![0u8; len as usize];
    recv.read_exact(&mut buf).await?;
    Ok(Some(postcard::from_bytes(&buf)?))
}
