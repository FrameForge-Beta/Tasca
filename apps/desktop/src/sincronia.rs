//! Sincronização entre os dispositivos da mesma conta (PC, telemóvel…).
//!
//! A conta tem uma caixa própria no servidor ([`ChaveCaixa::da_conta`]): a chave sai da
//! identidade, por isso só os dispositivos da conta a abrem. Cada dispositivo deixa lá o
//! que mudou e lê o que os outros deixaram, com espera longa (chega logo):
//!
//! - **amigos**: quem é amigo, pedidos enviados, bloqueados (ver `amigos.rs`);
//! - **grupos**: os grupos onde a conta está, com a chave da caixa de cada um;
//! - **perfil**: cor, bio, foto e banner.
//!
//! As mensagens não passam por aqui: as privadas já estão nas caixas das conversas e as
//! dos grupos vêm do anfitrião (ou da caixa do grupo).
//!
//! Cada aspeto (amigos e grupos/perfil) lê a caixa com o seu próprio cursor, guardado no
//! ficheiro dele.

use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tasca_core::{ChaveCaixa, Identity, caixa::MAX_BLOCO, servidor::AccountServer};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub(crate) enum Item {
    /// Uma pessoa da lista de amigos mudou (`dados = None`: saiu da lista).
    Pessoa { chave: String, dados: Option<Value>, em: u64 },
    /// A conta está neste grupo. `chave` = "época:chave em hex" da caixa do grupo.
    Grupo {
        id: String,
        nome: String,
        dono: String,
        em: u64,
        #[serde(default)]
        chave: Option<String>,
    },
    /// Cor e bio do perfil.
    Perfil { cor: String, bio: String, em: u64 },
    /// Foto ou banner do perfil (`campo` = "foto" ou "banner"; `valor = None`: tirou).
    Imagem { campo: String, valor: Option<String>, em: u64 },
}

/// Deixa uma novidade para os outros dispositivos da conta.
pub(crate) async fn publicar(srv: &AccountServer, eu: &Identity, item: &Item) -> anyhow::Result<()> {
    let chave = ChaveCaixa::da_conta(eu);
    let bloco = chave.selar(&serde_json::to_vec(item)?)?;
    anyhow::ensure!(bloco.len() <= MAX_BLOCO, "novidade demasiado grande para a caixa da conta");
    srv.caixa_enviar_bloco(&chave, bloco).await?;
    Ok(())
}

/// Igual a [`publicar`], em segundo plano (com algumas tentativas).
pub(crate) fn publicar_depois(srv: AccountServer, eu: Identity, item: Item) {
    tauri::async_runtime::spawn(async move {
        for i in 0..4u64 {
            match publicar(&srv, &eu, &item).await {
                Ok(()) => return,
                Err(e) => {
                    eprintln!("sincronia: {e:#}");
                    tokio::time::sleep(std::time::Duration::from_secs(5 * (i + 1))).await;
                }
            }
        }
    });
}

/// Lê as novidades depois de `desde` (espera até `espera` segundos por alguma). Devolve o
/// último número lido e as novidades que se perceberam (as de versões novas são saltadas).
pub(crate) async fn ler(srv: &AccountServer, eu: &Identity, desde: u64, espera: u32) -> anyhow::Result<(u64, Vec<Item>)> {
    let chave = ChaveCaixa::da_conta(eu);
    let blocos = srv.caixa_ler_blocos(&chave, desde, espera).await?;
    let ultimo = blocos.last().map(|(s, _)| *s).unwrap_or(desde);
    let itens = blocos
        .into_iter()
        .filter_map(|(_, b)| chave.abrir(&b).ok())
        .filter_map(|b| serde_json::from_slice::<Item>(&b).ok())
        .collect();
    Ok((ultimo, itens))
}

/// Impressão digital de um valor (para não voltar a publicar o que já está lá).
pub(crate) fn marca<T: Hash + ?Sized>(v: &T) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}
