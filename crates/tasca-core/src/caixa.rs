//! Caixas de mensagens no servidor, cifradas de ponta a ponta.
//!
//! Cada conversa (um grupo ou uma conversa privada entre duas pessoas) tem uma chave
//! simétrica de 32 bytes que só os participantes conhecem. Dessa chave saem:
//!
//! - o **id da caixa** no servidor (não revela de que grupo ou de quem é);
//! - o **token** que dá acesso de leitura e escrita à caixa (o servidor só guarda o hash);
//! - a **chave de cifra** das mensagens (XChaCha20-Poly1305).
//!
//! O servidor guarda blocos cifrados e numera-os por ordem de chegada. Não consegue ler
//! nada nem forjar mensagens: cada evento continua assinado pelo autor, e quem o lê
//! verifica a assinatura depois de decifrar.
//!
//! Conversas privadas: a chave sai de uma troca Diffie-Hellman (X25519) entre as chaves
//! de identidade dos dois, por isso não é preciso combinar nada antes.
//!
//! Grupos: o dono gera a chave ao criar o grupo e o convite leva-a a quem entra. A caixa
//! do grupo é o registo do grupo: o servidor dá a ordem e cada dispositivo aplica as
//! mesmas regras pela mesma ordem, por isso o grupo funciona sem o PC do dono ligado.
//! Quando alguém é expulso, quem o expulsou roda a chave ([`NovaChave`]) e o grupo passa
//! para uma caixa nova.

use anyhow::{Result, anyhow, ensure};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use curve25519_dalek::edwards::CompressedEdwardsY;
use iroh::PublicKey;
use sha2::{Digest, Sha512};
use zeroize::{Zeroize, Zeroizing};

use crate::{
    event::{GroupId, SignedEvent},
    identity::Identity,
};

const CTX_ID: &str = "tasca 2026 caixa id v1";
const CTX_TOKEN: &str = "tasca 2026 caixa token v1";
const CTX_CIFRA: &str = "tasca 2026 caixa cifra v1";
const CTX_PRIVADA: &str = "tasca 2026 conversa privada v1";
const CTX_CONTA: &str = "tasca 2026 caixa da conta entre dispositivos v1";
const VERSAO: u8 = 1;

/// Tamanho máximo de um bloco cifrado aceite pelo servidor.
pub const MAX_BLOCO: usize = 64 * 1024;

/// A chave de uma conversa.
#[derive(Clone)]
pub struct ChaveCaixa(Zeroizing<[u8; 32]>);

impl std::fmt::Debug for ChaveCaixa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChaveCaixa({})", self.id_hex())
    }
}

impl ChaveCaixa {
    /// Chave nova e aleatória (para um grupo novo ou depois de rodar a chave).
    pub fn generate() -> Result<Self> {
        let mut k = Zeroizing::new([0u8; 32]);
        getrandom::fill(k.as_mut()).map_err(|e| anyhow!("gerador aleatório: {e}"))?;
        Ok(Self(k))
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Chave da conversa privada entre `eu` e `outro`. Os dois lados obtêm a mesma.
    pub fn privada(eu: &Identity, outro: &PublicKey) -> Result<Self> {
        // Escalar X25519 a partir da chave ed25519 (como no RFC 8032: SHA-512 da semente).
        let mut h: [u8; 64] = Sha512::digest(eu.secret().to_bytes()).into();
        let mut escalar = [0u8; 32];
        escalar.copy_from_slice(&h[..32]);
        h.zeroize();
        let ponto = CompressedEdwardsY(*outro.as_bytes())
            .decompress()
            .ok_or_else(|| anyhow!("chave pública inválida"))?
            .to_montgomery();
        let mut partilhado = ponto.mul_clamped(escalar).to_bytes();
        escalar.zeroize();
        ensure!(partilhado != [0u8; 32], "chave pública inválida");

        // Ordem fixa das duas chaves públicas, para os dois lados darem o mesmo resultado.
        let (a, b) = {
            let (x, y) = (eu.public(), *outro);
            if x.as_bytes() <= y.as_bytes() { (x, y) } else { (y, x) }
        };
        let mut hasher = blake3::Hasher::new_derive_key(CTX_PRIVADA);
        hasher.update(&partilhado);
        hasher.update(a.as_bytes());
        hasher.update(b.as_bytes());
        partilhado.zeroize();
        Ok(Self(Zeroizing::new(*hasher.finalize().as_bytes())))
    }

    /// Caixa só dos dispositivos da própria conta (PC, telemóvel…): a chave sai da
    /// identidade, por isso só quem tem a conta a consegue abrir. Serve para sincronizar a
    /// lista de amigos, os grupos e o perfil entre dispositivos.
    pub fn da_conta(eu: &Identity) -> Self {
        Self(Zeroizing::new(blake3::derive_key(CTX_CONTA, &eu.secret().to_bytes())))
    }

    /// Cifra dados quaisquer para esta caixa.
    pub fn selar(&self, dados: &[u8]) -> Result<Vec<u8>> {
        self.seal_bytes(dados)
    }

    /// Decifra dados selados com [`Self::selar`].
    pub fn abrir(&self, bloco: &[u8]) -> Result<Vec<u8>> {
        Ok(self.open_bytes(bloco)?.to_vec())
    }

    /// Id da caixa no servidor.
    pub fn id(&self) -> [u8; 32] {
        blake3::derive_key(CTX_ID, self.0.as_ref())
    }

    pub fn id_hex(&self) -> String {
        hex::encode(self.id())
    }

    /// Token de acesso à caixa. Só quem tem a chave o consegue calcular.
    pub fn token(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(blake3::derive_key(CTX_TOKEN, self.0.as_ref()))
    }

    fn cifra(&self) -> XChaCha20Poly1305 {
        let k = Zeroizing::new(blake3::derive_key(CTX_CIFRA, self.0.as_ref()));
        XChaCha20Poly1305::new_from_slice(k.as_ref()).expect("chave de 32 bytes")
    }

    /// Cifra um evento para guardar na caixa (conversas privadas).
    pub fn seal(&self, ev: &SignedEvent) -> Result<Vec<u8>> {
        self.seal_bytes(&postcard::to_stdvec(ev)?)
    }

    /// Decifra um bloco da caixa e verifica a assinatura do autor.
    pub fn open(&self, bloco: &[u8]) -> Result<SignedEvent> {
        let ev: SignedEvent = postcard::from_bytes(&self.open_bytes(bloco)?)?;
        ev.verify()?;
        Ok(ev)
    }

    /// Cifra uma entrada da caixa de um grupo.
    pub fn seal_item(&self, item: &ItemGrupo) -> Result<Vec<u8>> {
        self.seal_bytes(&postcard::to_stdvec(item)?)
    }

    /// Decifra uma entrada da caixa de um grupo e verifica a assinatura do autor do evento.
    /// A assinatura do anfitrião num `Registo` é verificada por quem conhece o dono.
    pub fn open_item(&self, bloco: &[u8]) -> Result<ItemGrupo> {
        let item: ItemGrupo = postcard::from_bytes(&self.open_bytes(bloco)?)?;
        match &item {
            ItemGrupo::Voz(a) => a.verificar()?,
            ItemGrupo::Chave(c) => c.verificar()?,
            ItemGrupo::Anterior { .. } => {}
            _ => item.event().expect("registo ou pedido").verify()?,
        }
        Ok(item)
    }

    /// Formato: versão (1) | nonce (24) | cifra.
    pub(crate) fn seal_bytes(&self, plain: &[u8]) -> Result<Vec<u8>> {
        let plain = Zeroizing::new(plain.to_vec());
        let mut nonce = [0u8; 24];
        getrandom::fill(&mut nonce).map_err(|e| anyhow!("gerador aleatório: {e}"))?;
        let id = self.id();
        let ct = self
            .cifra()
            .encrypt(&XNonce::from(nonce), Payload { msg: &plain, aad: &id })
            .map_err(|_| anyhow!("não consegui cifrar a mensagem"))?;
        let mut out = Vec::with_capacity(1 + 24 + ct.len());
        out.push(VERSAO);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        ensure!(out.len() <= MAX_BLOCO, "mensagem demasiado grande");
        Ok(out)
    }

    pub(crate) fn open_bytes(&self, bloco: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        ensure!(bloco.len() > 25 && bloco[0] == VERSAO, "bloco desconhecido");
        let nonce: [u8; 24] = bloco[1..25].try_into().expect("24 bytes");
        let id = self.id();
        let plain = Zeroizing::new(
            self.cifra()
                .decrypt(&XNonce::from(nonce), Payload { msg: &bloco[25..], aad: &id })
                .map_err(|_| anyhow!("bloco adulterado ou de outra conversa"))?,
        );
        Ok(plain)
    }
}

/// Uma entrada na caixa de um grupo.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ItemGrupo {
    /// Evento já aceite pelo anfitrião, com o número de ordem que ele lhe deu. A
    /// assinatura do anfitrião impede um membro de inventar números de ordem.
    Registo { seq: u64, event: SignedEvent, anfitriao: Vec<u8> },
    /// Evento enviado por um membro enquanto o anfitrião estava desligado. O anfitrião
    /// valida-o e dá-lhe número quando voltar.
    Pedido { event: SignedEvent },
    /// Quem está a alojar uma sala de voz (assinado por essa pessoa).
    Voz(crate::voz::Anuncio),
    /// Entrada com convite: o `MemberJoined` mais a prova de que quem entra tem o código
    /// (assinatura com a chave do convite, ver [`crate::invite::Convite`]).
    Entrada { event: SignedEvent, convite: [u8; 32], prova: Vec<u8> },
    /// Chave nova do grupo (depois de expulsar ou banir alguém). Fecha esta caixa.
    Chave(NovaChave),
    /// A chave da época anterior, guardada na caixa nova, para quem entra depois conseguir
    /// ler o histórico todo.
    Anterior { epoch: u32, chave: [u8; 32] },
}

impl ItemGrupo {
    pub fn event(&self) -> Option<&SignedEvent> {
        match self {
            Self::Registo { event, .. } | Self::Pedido { event } | Self::Entrada { event, .. } => Some(event),
            Self::Voz(_) | Self::Chave(_) | Self::Anterior { .. } => None,
        }
    }

    /// O que o anfitrião assina num `Registo`.
    pub fn registo_msg(group: &GroupId, seq: u64, event: &SignedEvent) -> Vec<u8> {
        let mut h = blake3::Hasher::new_derive_key("tasca 2026 registo v1");
        h.update(&group.0);
        h.update(&seq.to_be_bytes());
        h.update(&event.id());
        h.finalize().as_bytes().to_vec()
    }

    pub fn registo(anfitriao: &Identity, group: &GroupId, seq: u64, event: SignedEvent) -> Self {
        let sig = anfitriao.secret().sign(&Self::registo_msg(group, seq, &event)).to_bytes().to_vec();
        Self::Registo { seq, event, anfitriao: sig }
    }

    /// Confirma que um `Registo` foi assinado por `dono`.
    pub fn verify_registo(&self, group: &GroupId, dono: &PublicKey) -> Result<()> {
        let Self::Registo { seq, event, anfitriao } = self else {
            anyhow::bail!("não é um registo");
        };
        let sig: [u8; 64] = anfitriao.as_slice().try_into().map_err(|_| anyhow!("assinatura inválida"))?;
        dono.verify(&Self::registo_msg(group, *seq, event), &iroh::Signature::from_bytes(&sig))
            .map_err(|_| anyhow!("registo não assinado pelo anfitrião"))
    }
}

/// Chave nova de um grupo, cifrada para cada membro (e para cada convite ativo, para quem
/// entrar com um convite antigo). Assinada por quem a rodou.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NovaChave {
    pub group: GroupId,
    /// Época nova.
    pub epoch: u32,
    pub autor: [u8; 32],
    /// (chave pública do destinatário, chave nova cifrada para ele)
    pub para: Vec<([u8; 32], Vec<u8>)>,
    pub assinatura: Vec<u8>,
}

impl NovaChave {
    fn msg(group: &GroupId, epoch: u32, autor: &[u8; 32], para: &[([u8; 32], Vec<u8>)]) -> Vec<u8> {
        let mut h = blake3::Hasher::new_derive_key("tasca 2026 chave nova v1");
        h.update(&group.0);
        h.update(&epoch.to_be_bytes());
        h.update(autor);
        for (k, c) in para {
            h.update(k);
            h.update(&(c.len() as u64).to_be_bytes());
            h.update(c);
        }
        h.finalize().as_bytes().to_vec()
    }

    /// Cifra `nova` para cada destinatário e assina.
    pub fn criar(autor: &Identity, group: GroupId, epoch: u32, nova: &ChaveCaixa, destinos: &[[u8; 32]]) -> Self {
        let mut para = Vec::new();
        for d in destinos {
            let Ok(pk) = PublicKey::from_bytes(d) else { continue };
            let Ok(k) = ChaveCaixa::privada(autor, &pk) else { continue };
            if let Ok(c) = k.seal_bytes(nova.as_bytes()) {
                if !para.iter().any(|(x, _)| x == d) {
                    para.push((*d, c));
                }
            }
        }
        let a = *autor.public().as_bytes();
        let assinatura = autor.secret().sign(&Self::msg(&group, epoch, &a, &para)).to_bytes().to_vec();
        Self { group, epoch, autor: a, para, assinatura }
    }

    pub fn verificar(&self) -> Result<()> {
        let sig: [u8; 64] = self.assinatura.as_slice().try_into().map_err(|_| anyhow!("assinatura inválida"))?;
        PublicKey::from_bytes(&self.autor)
            .map_err(|_| anyhow!("autor inválido"))?
            .verify(&Self::msg(&self.group, self.epoch, &self.autor, &self.para), &iroh::Signature::from_bytes(&sig))
            .map_err(|_| anyhow!("chave nova com assinatura inválida"))
    }

    /// A chave nova, se vier cifrada para `eu`.
    pub fn abrir(&self, eu: &Identity) -> Option<ChaveCaixa> {
        let me = eu.public();
        let (_, c) = self.para.iter().find(|(k, _)| k == me.as_bytes())?;
        let autor = PublicKey::from_bytes(&self.autor).ok()?;
        let k = ChaveCaixa::privada(eu, &autor).ok()?;
        let b: [u8; 32] = k.open_bytes(c).ok()?.as_slice().try_into().ok()?;
        Some(ChaveCaixa::from_bytes(b))
    }
}

/// Uma entrada da caixa como vem do servidor.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Entrada {
    pub seq: u64,
    /// Bloco cifrado em base64.
    pub dados: String,
    /// Quando o servidor o recebeu (ms desde 1970).
    pub em: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Entradas {
    pub entradas: Vec<Entrada>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Escrita {
    pub seq: u64,
}

#[cfg(feature = "servidor")]
mod cliente {
    use super::*;
    use crate::servidor::{AccountServer, unb64};

    impl AccountServer {
        /// Guarda um evento na caixa da conversa. Devolve o número de ordem.
        pub async fn caixa_enviar(&self, chave: &ChaveCaixa, ev: &SignedEvent) -> Result<u64> {
            self.caixa_enviar_bloco(chave, chave.seal(ev)?).await
        }

        /// Guarda um bloco já cifrado.
        pub async fn caixa_enviar_bloco(&self, chave: &ChaveCaixa, bloco: Vec<u8>) -> Result<u64> {
            // repetir é seguro: o mesmo bloco duas vezes dá o mesmo evento, e quem lê ignora-o
            let r: Escrita = self
                .send_repetivel(
                    self.http()
                        .post(self.url(&format!("v1/caixas/{}", chave.id_hex())))
                        .bearer_auth(hex::encode(chave.token().as_ref()))
                        .body(bloco),
                )
                .await?
                .json()
                .await?;
            Ok(r.seq)
        }

        /// Lê os eventos depois de `desde`. Com `espera` (segundos, até 25), o servidor
        /// segura o pedido até chegar algo novo. Blocos que não abrem são ignorados.
        pub async fn caixa_ler(
            &self,
            chave: &ChaveCaixa,
            desde: u64,
            espera: u32,
        ) -> Result<Vec<(u64, SignedEvent)>> {
            let mut out = Vec::new();
            for (seq, b) in self.caixa_ler_blocos(chave, desde, espera).await? {
                match chave.open(&b) {
                    Ok(ev) => out.push((seq, ev)),
                    Err(err) => tracing::warn!("bloco {seq} ignorado: {err}"),
                }
            }
            Ok(out)
        }

        /// Lê as entradas depois de `desde` (bloco e hora do servidor), sem as abrir.
        pub async fn caixa_ler_entradas(
            &self,
            chave: &ChaveCaixa,
            desde: u64,
            espera: u32,
        ) -> Result<Vec<(u64, Vec<u8>, u64)>> {
            let r: Entradas = self
                .send(
                    self.http()
                        .get(self.url(&format!("v1/caixas/{}", chave.id_hex())))
                        .query(&[("desde", desde), ("espera", u64::from(espera.min(25)))])
                        .timeout(std::time::Duration::from_secs(u64::from(espera.min(25)) + 20))
                        .bearer_auth(hex::encode(chave.token().as_ref())),
                )
                .await?
                .json()
                .await?;
            r.entradas.into_iter().map(|e| Ok((e.seq, unb64(&e.dados)?, e.em))).collect()
        }

        /// Deixa um bloco de presença (quem está online) no lugar `lugar` da caixa. O
        /// servidor guarda-o só uns minutos.
        pub async fn presenca_enviar(&self, chave: &ChaveCaixa, lugar: &[u8; 16], bloco: Vec<u8>) -> Result<()> {
            self.send(
                self.http()
                    .post(self.url(&format!("v1/caixas/{}/presenca", chave.id_hex())))
                    .query(&[("lugar", hex::encode(lugar))])
                    .timeout(std::time::Duration::from_secs(15))
                    .bearer_auth(hex::encode(chave.token().as_ref()))
                    .body(bloco),
            )
            .await?;
            Ok(())
        }

        /// Os blocos de presença que ainda valem (bloco, hora do servidor).
        pub async fn presenca_ler(&self, chave: &ChaveCaixa) -> Result<Vec<(Vec<u8>, u64)>> {
            let r: Entradas = self
                .send(
                    self.http()
                        .get(self.url(&format!("v1/caixas/{}/presenca", chave.id_hex())))
                        .timeout(std::time::Duration::from_secs(15))
                        .bearer_auth(hex::encode(chave.token().as_ref())),
                )
                .await?
                .json()
                .await?;
            r.entradas.into_iter().map(|e| Ok((unb64(&e.dados)?, e.em))).collect()
        }

        /// Lê os blocos cifrados depois de `desde`, sem os abrir.
        pub async fn caixa_ler_blocos(
            &self,
            chave: &ChaveCaixa,
            desde: u64,
            espera: u32,
        ) -> Result<Vec<(u64, Vec<u8>)>> {
            let r: Entradas = self
                .send(
                    self.http()
                        .get(self.url(&format!("v1/caixas/{}", chave.id_hex())))
                        .query(&[("desde", desde), ("espera", u64::from(espera.min(25)))])
                        .timeout(std::time::Duration::from_secs(u64::from(espera.min(25)) + 20))
                        .bearer_auth(hex::encode(chave.token().as_ref())),
                )
                .await?
                .json()
                .await?;
            r.entradas.into_iter().map(|e| Ok((e.seq, unb64(&e.dados)?))).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventKind;

    #[test]
    fn conversa_privada_da_a_mesma_chave_aos_dois() {
        let ana = Identity::generate();
        let rui = Identity::generate();
        let eva = Identity::generate();
        let a = ChaveCaixa::privada(&ana, &rui.public()).unwrap();
        let b = ChaveCaixa::privada(&rui, &ana.public()).unwrap();
        assert_eq!(a.as_bytes(), b.as_bytes());
        let c = ChaveCaixa::privada(&ana, &eva.public()).unwrap();
        assert_ne!(a.as_bytes(), c.as_bytes());
    }

    #[test]
    fn cifrar_e_decifrar() {
        let ana = Identity::generate();
        let k = ChaveCaixa::generate().unwrap();
        let ev = SignedEvent::sign(&ana, None, EventKind::Message { channel: "geral".into(), text: "olá".into() });
        let bloco = k.seal(&ev).unwrap();
        assert_eq!(k.open(&bloco).unwrap(), ev);

        // Outra chave não abre; um byte mudado também não.
        assert!(ChaveCaixa::generate().unwrap().open(&bloco).is_err());
        let mut mau = bloco.clone();
        *mau.last_mut().unwrap() ^= 1;
        assert!(k.open(&mau).is_err());
    }
}
