//! Amigos.
//!
//! **Pedidos de amizade** (pedir, aceitar, recusar, cancelar, remover) vão pelo servidor de
//! contas, numa caixa por pessoa cujo endereço sai da chave pública dela. Qualquer pessoa
//! pode lá deixar um aviso, mas cada aviso vai selado para o destinatário com uma chave
//! descartável (X25519): o servidor e quem espreitar a caixa só veem blocos cifrados, sem
//! saber quem pediu amizade a quem. Dentro, o aviso vem assinado pela identidade do autor.
//!
//! **Estado online e perfil** vão diretamente entre os PCs dos amigos, pelo iroh (ALPN
//! [`ALPN`]). Cada PC só responde a quem está na sua lista de amigos. Uma ligação que não
//! consegue chegar ao outro PC quer dizer que ele está offline (ou invisível).

use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
    time::Duration,
};

use anyhow::{Result, anyhow, bail, ensure};
use iroh::{
    PublicKey, Signature,
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::{
    caixa::ChaveCaixa,
    event::now_ms,
    identity::Identity,
    protocol::{read_frame, write_frame},
};

/// Protocolo de estado entre amigos.
pub const ALPN: &[u8] = b"tasca/amigos/0";

const CTX_PEDIDOS: &str = "tasca 2026 caixa de pedidos de amizade v1";
const CTX_ASSINATURA: &str = "tasca 2026 aviso de amizade v1";
/// Tamanho máximo da biografia, em carateres.
pub const MAX_BIO: usize = 190;
/// Tamanho máximo do banner (`data:` URL ou cor), em carateres.
pub const MAX_BANNER: usize = 60 * 1024;

/// O que um aviso diz.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Aviso {
    /// Quero ser teu amigo.
    Pedido,
    /// Aceitei o teu pedido.
    Aceite,
    /// Recusei o teu pedido.
    Recusado,
    /// Já não quero o pedido que te fiz.
    Cancelado,
    /// Deixámos de ser amigos.
    Removido,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpoAviso {
    pub de: [u8; 32],
    pub para: [u8; 32],
    /// Quando foi escrito (ms). Serve para ignorar avisos repetidos ou antigos.
    pub em: u64,
    /// Nome a mostrar de quem escreve.
    pub nome: String,
    /// @nome de utilizador de quem escreve. Quem recebe confirma-o no servidor.
    pub utilizador: Option<String>,
    pub aviso: Aviso,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvisoAssinado {
    pub corpo: CorpoAviso,
    pub assinatura: Vec<u8>,
}

fn msg_assinada(corpo: &CorpoAviso) -> Result<Vec<u8>> {
    let mut h = blake3::Hasher::new_derive_key(CTX_ASSINATURA);
    h.update(&postcard::to_stdvec(corpo)?);
    Ok(h.finalize().as_bytes().to_vec())
}

impl AvisoAssinado {
    pub fn assinar(eu: &Identity, para: &PublicKey, nome: &str, utilizador: Option<String>, aviso: Aviso) -> Result<Self> {
        let corpo = CorpoAviso {
            de: *eu.public().as_bytes(),
            para: *para.as_bytes(),
            em: now_ms(),
            nome: nome.chars().take(64).collect(),
            utilizador,
            aviso,
        };
        let assinatura = eu.secret().sign(&msg_assinada(&corpo)?).to_bytes().to_vec();
        Ok(Self { corpo, assinatura })
    }

    pub fn autor(&self) -> Result<PublicKey> {
        PublicKey::from_bytes(&self.corpo.de).map_err(|e| anyhow!("chave inválida: {e}"))
    }

    /// Confirma a assinatura e que o aviso é mesmo para `eu`.
    pub fn verificar(&self, eu: &PublicKey) -> Result<()> {
        ensure!(self.corpo.para == *eu.as_bytes(), "aviso para outra pessoa");
        ensure!(self.corpo.de != *eu.as_bytes(), "aviso de mim para mim");
        let sig: [u8; 64] = self.assinatura.as_slice().try_into().map_err(|_| anyhow!("assinatura inválida"))?;
        self.autor()?
            .verify(&msg_assinada(&self.corpo)?, &Signature::from_bytes(&sig))
            .map_err(|_| anyhow!("assinatura inválida"))
    }

    /// Sela o aviso para `para`: chave pública descartável (32) | bloco cifrado.
    pub fn selar(&self, para: &PublicKey) -> Result<Vec<u8>> {
        let descartavel = Identity::generate();
        let chave = ChaveCaixa::privada(&descartavel, para)?;
        let mut out = descartavel.public().as_bytes().to_vec();
        out.extend(chave.seal_bytes(&postcard::to_stdvec(self)?)?);
        Ok(out)
    }

    /// Abre um bloco selado para `eu` e verifica-o.
    pub fn abrir(eu: &Identity, bloco: &[u8]) -> Result<Self> {
        ensure!(bloco.len() > 32, "bloco curto");
        let outro = PublicKey::from_bytes(bloco[..32].try_into().expect("32 bytes"))
            .map_err(|_| anyhow!("bloco inválido"))?;
        let chave = ChaveCaixa::privada(eu, &outro)?;
        let aviso: Self = postcard::from_bytes(&chave.open_bytes(&bloco[32..])?)?;
        aviso.verificar(&eu.public())?;
        Ok(aviso)
    }
}

/// Caixa de pedidos de amizade de uma pessoa. Toda a gente a consegue calcular.
pub fn caixa_pedidos(pk: &PublicKey) -> ChaveCaixa {
    let mut h = blake3::Hasher::new_derive_key(CTX_PEDIDOS);
    h.update(pk.as_bytes());
    ChaveCaixa::from_bytes(*h.finalize().as_bytes())
}

// ---------------- estado e perfil entre amigos (P2P) ----------------

/// Estado que mostro aos amigos.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Estado {
    /// `online`, `away`, `dnd` ou `offline` (invisível).
    pub estado: String,
    /// Mensagem de estado ("a jogar Mario Kart").
    pub mensagem: String,
}

/// Perfil que mostro aos amigos (no cartão de perfil).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Perfil {
    pub nome: String,
    pub utilizador: Option<String>,
    pub cor: String,
    pub foto: Option<String>,
    /// Cor (`#RRGGBB`) ou imagem (`data:image/...`).
    pub banner: Option<String>,
    pub bio: String,
}

impl Perfil {
    pub fn hash(&self) -> [u8; 32] {
        *blake3::hash(&postcard::to_stdvec(self).unwrap_or_default()).as_bytes()
    }

    /// Corta o que vier demasiado grande de outro PC.
    pub fn limpar(mut self) -> Self {
        self.nome = self.nome.chars().take(64).collect();
        self.cor = self.cor.chars().take(32).collect();
        self.bio = self.bio.chars().take(MAX_BIO).collect();
        self.foto = self.foto.filter(|f| f.starts_with("data:image/") && f.len() <= crate::store::MAX_PHOTO_CHARS);
        self.banner = self.banner.filter(|b| b.len() <= MAX_BANNER && (b.starts_with("data:image/") || b.starts_with('#')));
        self.utilizador = self.utilizador.filter(|u| crate::servidor::normalize_username(u).ok().as_deref() == Some(u));
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum Msg {
    /// Quem liga: o meu estado e o hash do teu perfil que já tenho.
    Ola { estado: Estado, tenho: [u8; 32] },
    /// Quem atende: o meu estado, o meu perfil (se o teu estiver velho) e o hash do teu.
    Resposta { estado: Estado, perfil: Option<Perfil>, tenho: [u8; 32] },
    /// Não és meu amigo (ou estou invisível).
    Recusado,
    /// Quem liga: o meu perfil, se o que tens estiver velho.
    Perfil(Option<Perfil>),
}

/// Algo novo sobre um amigo.
#[derive(Clone, Debug)]
pub struct Novidade {
    pub quem: [u8; 32],
    /// `None` quando deixou de responder (offline).
    pub estado: Option<Estado>,
    pub perfil: Option<Perfil>,
}

/// Estado partilhado entre o nó e o protocolo.
pub struct Amigos {
    lista: Mutex<HashSet<[u8; 32]>>,
    eu: Mutex<(Estado, Perfil)>,
    /// Perfis dos amigos que já recebi (hash → para não os pedir outra vez).
    perfis: Mutex<HashMap<[u8; 32], Perfil>>,
    novidades: broadcast::Sender<Novidade>,
}

impl Default for Amigos {
    fn default() -> Self {
        Self {
            lista: Default::default(),
            eu: Mutex::new((Estado { estado: "online".into(), mensagem: String::new() }, Perfil::default())),
            perfis: Default::default(),
            novidades: broadcast::channel(256).0,
        }
    }
}

impl Amigos {
    /// Quem pode ver o meu estado (e de quem pergunto).
    pub fn definir_lista(&self, lista: impl IntoIterator<Item = [u8; 32]>) {
        *self.lista.lock().unwrap() = lista.into_iter().collect();
    }

    pub fn definir_eu(&self, estado: Estado, perfil: Perfil) {
        *self.eu.lock().unwrap() = (estado, perfil);
    }

    pub fn definir_estado(&self, estado: Estado) {
        self.eu.lock().unwrap().0 = estado;
    }

    pub fn definir_perfil(&self, perfil: Perfil) {
        self.eu.lock().unwrap().1 = perfil;
    }

    pub fn perfil_eu(&self) -> Perfil {
        self.eu.lock().unwrap().1.clone()
    }

    /// Perfil de um amigo que já recebi antes (por exemplo, guardado em disco).
    pub fn lembrar_perfil(&self, quem: [u8; 32], perfil: Perfil) {
        self.perfis.lock().unwrap().insert(quem, perfil);
    }

    pub fn subscrever(&self) -> broadcast::Receiver<Novidade> {
        self.novidades.subscribe()
    }

    fn e_amigo(&self, k: &[u8; 32]) -> bool {
        self.lista.lock().unwrap().contains(k)
    }

    fn hash_de(&self, k: &[u8; 32]) -> [u8; 32] {
        self.perfis.lock().unwrap().get(k).map(Perfil::hash).unwrap_or_default()
    }

    fn recebi(&self, quem: [u8; 32], estado: Option<Estado>, perfil: Option<Perfil>) {
        let perfil = perfil.map(Perfil::limpar);
        if let Some(p) = &perfil {
            self.perfis.lock().unwrap().insert(quem, p.clone());
        }
        let _ = self.novidades.send(Novidade { quem, estado, perfil });
    }

    /// Liga ao PC de um amigo: troca estados e perfis. `Ok(None)` = ele não quis responder.
    pub(crate) async fn perguntar(&self, endpoint: &iroh::Endpoint, quem: iroh::EndpointAddr) -> Result<Option<Estado>> {
        let k = *quem.id.as_bytes();
        let r = tokio::time::timeout(Duration::from_secs(15), async {
            let conn = endpoint.connect(quem, ALPN).await?;
            let (mut send, mut recv) = conn.open_bi().await?;
            let (estado, perfil) = self.eu.lock().unwrap().clone();
            write_frame(&mut send, &Msg::Ola { estado, tenho: self.hash_de(&k) }).await?;
            let out = match read_frame::<Msg>(&mut recv).await? {
                Some(Msg::Resposta { estado, perfil: o_dele, tenho }) => {
                    let meu = (tenho != perfil.hash()).then_some(perfil);
                    write_frame(&mut send, &Msg::Perfil(meu)).await?;
                    Some((estado, o_dele))
                }
                Some(Msg::Recusado) | None => None,
                Some(other) => bail!("resposta inesperada: {other:?}"),
            };
            send.finish()?;
            let _ = tokio::time::timeout(Duration::from_secs(2), send.stopped()).await;
            conn.close(0u32.into(), b"ok");
            anyhow::Ok(out)
        })
        .await;
        match r {
            Ok(Ok(Some((estado, perfil)))) => {
                let online = (estado.estado != "offline").then_some(estado);
                self.recebi(k, online.clone(), perfil);
                Ok(online)
            }
            Ok(Ok(None)) => {
                self.recebi(k, None, None);
                Ok(None)
            }
            Ok(Err(e)) => {
                self.recebi(k, None, None);
                Err(e)
            }
            Err(_) => {
                self.recebi(k, None, None);
                Err(anyhow!("o PC do amigo não respondeu"))
            }
        }
    }

    async fn atender(&self, conn: Connection) -> Result<()> {
        let r = self.conversa(&conn).await;
        // Espera que quem ligou leia tudo e feche (fechar já perderia a resposta).
        let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
        r
    }

    async fn conversa(&self, conn: &Connection) -> Result<()> {
        let quem = *conn.remote_id().as_bytes();
        let (mut send, mut recv) = conn.accept_bi().await?;
        let Some(Msg::Ola { estado: o_dele, tenho }) = read_frame(&mut recv).await? else {
            bail!("esperava Ola");
        };
        let (meu_estado, meu_perfil) = self.eu.lock().unwrap().clone();
        if !self.e_amigo(&quem) || meu_estado.estado == "offline" {
            write_frame(&mut send, &Msg::Recusado).await?;
            send.finish()?;
            return Ok(());
        }
        let perfil = (tenho != meu_perfil.hash()).then_some(meu_perfil);
        write_frame(&mut send, &Msg::Resposta { estado: meu_estado, perfil, tenho: self.hash_de(&quem) }).await?;
        let o_perfil = match read_frame::<Msg>(&mut recv).await? {
            Some(Msg::Perfil(p)) => p,
            _ => None,
        };
        send.finish()?;
        let online = (o_dele.estado != "offline").then_some(o_dele);
        self.recebi(quem, online, o_perfil);
        Ok(())
    }
}

/// Atende as ligações de amigos.
#[derive(Clone)]
pub(crate) struct Protocolo(pub(crate) std::sync::Arc<Amigos>);

impl std::fmt::Debug for Protocolo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AmigosProtocolo")
    }
}

impl ProtocolHandler for Protocolo {
    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        self.0.atender(conn).await.map_err(|e| AcceptError::from_boxed(e.into()))
    }
}

#[cfg(feature = "servidor")]
mod cliente {
    use super::*;
    use crate::servidor::AccountServer;

    impl AccountServer {
        /// Deixa um aviso de amizade na caixa de `para`.
        pub async fn amizade_enviar(&self, aviso: &AvisoAssinado, para: &PublicKey) -> Result<u64> {
            self.caixa_enviar_bloco(&caixa_pedidos(para), aviso.selar(para)?).await
        }

        /// Lê os avisos de amizade para `eu` depois de `desde`. Devolve o último número
        /// visto (para a próxima leitura) e os avisos válidos.
        pub async fn amizade_ler(&self, eu: &Identity, desde: u64, espera: u32) -> Result<(u64, Vec<AvisoAssinado>)> {
            let blocos = self.caixa_ler_blocos(&caixa_pedidos(&eu.public()), desde, espera).await?;
            let ultimo = blocos.last().map(|(s, _)| *s).unwrap_or(desde);
            let avisos = blocos
                .into_iter()
                .filter_map(|(seq, b)| match AvisoAssinado::abrir(eu, &b) {
                    Ok(a) => Some(a),
                    Err(e) => {
                        tracing::debug!("aviso {seq} ignorado: {e}");
                        None
                    }
                })
                .collect();
            Ok((ultimo, avisos))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aviso_selado_so_abre_no_destinatario() {
        let ana = Identity::generate();
        let rui = Identity::generate();
        let eva = Identity::generate();
        let a = AvisoAssinado::assinar(&ana, &rui.public(), "Ana", Some("ana".into()), Aviso::Pedido).unwrap();
        let bloco = a.selar(&rui.public()).unwrap();
        // não leva a chave da Ana às claras
        assert!(!bloco.windows(32).any(|w| w == ana.public().as_bytes()));
        let aberto = AvisoAssinado::abrir(&rui, &bloco).unwrap();
        assert_eq!(aberto, a);
        assert!(AvisoAssinado::abrir(&eva, &bloco).is_err());
    }

    #[test]
    fn aviso_adulterado_ou_reencaminhado_falha() {
        let ana = Identity::generate();
        let rui = Identity::generate();
        let eva = Identity::generate();
        let mut a = AvisoAssinado::assinar(&ana, &rui.public(), "Ana", None, Aviso::Pedido).unwrap();
        a.verificar(&rui.public()).unwrap();
        // a Eva não pode usar um aviso que não era para ela
        assert!(a.verificar(&eva.public()).is_err());
        a.corpo.aviso = Aviso::Aceite;
        assert!(a.verificar(&rui.public()).is_err());
    }

    #[test]
    fn caixa_de_pedidos_e_da_pessoa() {
        let ana = Identity::generate();
        let rui = Identity::generate();
        assert_eq!(caixa_pedidos(&ana.public()).id(), caixa_pedidos(&ana.public()).id());
        assert_ne!(caixa_pedidos(&ana.public()).id(), caixa_pedidos(&rui.public()).id());
    }
}
