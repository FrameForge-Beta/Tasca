//! Ficheiros e imagens anexados a mensagens, partilhados P2P.
//!
//! O evento da mensagem só leva o [`Anexo`] (nome, tamanho, BLAKE3). O conteúdo fica na
//! pasta de quem o enviou e de quem já o descarregou, e pede-se pelo hash com o ALPN
//! [`ALPN`]. Só membros do grupo recebem, e só ficheiros que aparecem nesse grupo.
//! O anfitrião do grupo guarda uma cópia, para o ficheiro não desaparecer quando quem o
//! enviou desligar o PC.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail, ensure};
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::{
    event::{EventKind, GroupId},
    node::Inner,
    protocol::{read_frame, write_frame},
};

pub const ALPN: &[u8] = b"tasca/ficheiros/0";

/// Tamanho máximo de um anexo (100 MiB).
pub const MAX_FICHEIRO: u64 = 100 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct Pedido {
    group: GroupId,
    hash: [u8; 32],
}

#[derive(Debug, Serialize, Deserialize)]
enum Resposta {
    /// Segue-se o conteúdo, com este tamanho.
    Vai(u64),
    NaoTenho,
    Recusado,
}

/// Onde ficam os ficheiros deste dispositivo.
#[derive(Clone, Debug)]
pub struct Pasta(pub PathBuf);

impl Pasta {
    pub fn caminho(&self, hash: &[u8; 32]) -> PathBuf {
        self.0.join(hex::encode(hash))
    }

    pub fn tem(&self, hash: &[u8; 32]) -> bool {
        self.caminho(hash).is_file()
    }

    /// Guarda bytes e devolve o hash.
    pub fn guardar(&self, dados: &[u8]) -> Result<[u8; 32]> {
        ensure!(dados.len() as u64 <= MAX_FICHEIRO, "ficheiro maior que 100 MB");
        std::fs::create_dir_all(&self.0)?;
        let hash = *blake3::hash(dados).as_bytes();
        let fim = self.caminho(&hash);
        if !fim.exists() {
            let tmp = fim.with_extension("tmp");
            std::fs::write(&tmp, dados)?;
            std::fs::rename(&tmp, &fim)?;
        }
        Ok(hash)
    }

    /// Copia um ficheiro do disco para a pasta (sem o ler todo para a memória).
    pub fn guardar_de(&self, origem: &Path) -> Result<([u8; 32], u64)> {
        let tam = std::fs::metadata(origem)?.len();
        ensure!(tam <= MAX_FICHEIRO, "ficheiro maior que 100 MB");
        std::fs::create_dir_all(&self.0)?;
        let mut h = blake3::Hasher::new();
        h.update_reader(std::fs::File::open(origem)?)?;
        let hash = *h.finalize().as_bytes();
        let fim = self.caminho(&hash);
        if !fim.exists() {
            let tmp = fim.with_extension("tmp");
            std::fs::copy(origem, &tmp)?;
            std::fs::rename(&tmp, &fim)?;
        }
        Ok((hash, tam))
    }
}

/// O ficheiro aparece numa mensagem deste grupo.
fn no_grupo(inner: &Inner, group: GroupId, hash: &[u8; 32]) -> Result<bool> {
    let eventos = inner.store().events_since(group, 0)?;
    Ok(eventos.iter().any(|(_, ev)| match &ev.body.kind {
        EventKind::Post { files, .. } => files.iter().any(|f| f.hash == *hash),
        _ => false,
    }))
}

/// Lado de quem tem o ficheiro.
#[derive(Clone)]
pub(crate) struct Servir {
    pub(crate) inner: std::sync::Weak<Inner>,
    pub(crate) pasta: Option<Pasta>,
}

impl std::fmt::Debug for Servir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Servir")
    }
}

impl ProtocolHandler for Servir {
    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        self.serve(conn).await.map_err(|e| AcceptError::from_boxed(e.into()))
    }
}

impl Servir {
    async fn serve(&self, conn: Connection) -> Result<()> {
        let (mut send, mut recv) = conn.accept_bi().await?;
        let Some(Pedido { group, hash }) = read_frame(&mut recv).await? else { return Ok(()) };
        let caminho = {
            let inner = self.inner.upgrade().ok_or_else(|| anyhow!("nó desligado"))?;
            let quem = inner.conta_de(&conn.remote_id());
            let pode = inner.store().is_member(group, &quem)? && no_grupo(&inner, group, &hash)?;
            match (&self.pasta, pode) {
                (_, false) => Err(Resposta::Recusado),
                (Some(p), true) if p.tem(&hash) => Ok(p.caminho(&hash)),
                _ => Err(Resposta::NaoTenho),
            }
        };
        match caminho {
            Err(r) => {
                write_frame(&mut send, &r).await?;
                send.finish()?;
            }
            Ok(c) => {
                let mut f = tokio::fs::File::open(&c).await?;
                let tam = f.metadata().await?.len();
                write_frame(&mut send, &Resposta::Vai(tam)).await?;
                tokio::io::copy(&mut f, &mut send).await?;
                send.finish()?;
            }
        }
        let _ = tokio::time::timeout(std::time::Duration::from_secs(30), conn.closed()).await;
        Ok(())
    }
}

/// Pede um ficheiro a um dispositivo e guarda-o na pasta (confirmando o hash).
pub(crate) async fn pedir(
    ep: &iroh::Endpoint,
    alvo: iroh::EndpointAddr,
    pasta: &Pasta,
    group: GroupId,
    hash: [u8; 32],
) -> Result<PathBuf> {
    let conn = tokio::time::timeout(std::time::Duration::from_secs(8), ep.connect(alvo, ALPN))
        .await
        .map_err(|_| anyhow!("não respondeu"))??;
    let (mut send, mut recv) = conn.open_bi().await?;
    write_frame(&mut send, &Pedido { group, hash }).await?;
    send.finish()?;
    let tam = match read_frame::<Resposta>(&mut recv).await? {
        Some(Resposta::Vai(t)) => t,
        Some(Resposta::NaoTenho) => bail!("não tem o ficheiro"),
        Some(Resposta::Recusado) => bail!("recusou"),
        None => bail!("ligação fechada"),
    };
    ensure!(tam <= MAX_FICHEIRO, "ficheiro demasiado grande");
    std::fs::create_dir_all(&pasta.0)?;
    let fim = pasta.caminho(&hash);
    let tmp = fim.with_extension(format!("dl{}", rand_u32()));
    let mut f = tokio::fs::File::create(&tmp).await?;
    let mut h = blake3::Hasher::new();
    let mut falta = tam;
    let mut buf = vec![0u8; 64 * 1024];
    while falta > 0 {
        let n = recv.read(&mut buf).await?.ok_or_else(|| anyhow!("ficheiro cortado"))?;
        let n = n.min(falta as usize);
        h.update(&buf[..n]);
        f.write_all(&buf[..n]).await?;
        falta -= n as u64;
    }
    f.flush().await?;
    drop(f);
    conn.close(0u32.into(), b"obrigado");
    if *h.finalize().as_bytes() != hash {
        let _ = std::fs::remove_file(&tmp);
        bail!("o ficheiro chegou estragado");
    }
    std::fs::rename(&tmp, &fim)?;
    Ok(fim)
}

fn rand_u32() -> u32 {
    let mut b = [0u8; 4];
    let _ = getrandom::fill(&mut b);
    u32::from_le_bytes(b)
}
