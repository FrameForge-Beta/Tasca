//! Eventos do grupo. Tudo o que acontece num grupo é um evento assinado pelo autor.

use anyhow::{Result, anyhow};
use iroh::{PublicKey, Signature};
use serde::{Deserialize, Serialize};

use crate::identity::Identity;

/// Identificador de um grupo: o hash do evento que o criou.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct GroupId(pub [u8; 32]);

impl std::fmt::Debug for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GroupId({})", self)
    }
}

impl std::fmt::Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &hex::encode(self.0)[..12])
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    /// Primeiro evento de qualquer grupo. Quem o assina é o dono.
    GroupCreated {
        name: String,
        owner_name: String,
        nonce: [u8; 16],
    },
    /// Um utilizador entrou com um convite válido.
    MemberJoined { display_name: String },
    /// Mensagem de texto num canal.
    Message { channel: String, text: String },
    /// Disposição dos canais e categorias do grupo (lista completa; a última vale).
    /// Só o dono a pode publicar. Fica no fim do enum para não mudar os números do postcard.
    Layout {
        channels: Vec<ChannelDef>,
        categories: Vec<String>,
    },
    /// Perfil do autor neste grupo: nome, cor e foto (`data:image/...`, pequena).
    Profile {
        name: String,
        color: String,
        photo: Option<String>,
    },
    /// Cartão de perfil do autor: @nome de utilizador, biografia e banner (cor `#RRGGBB`
    /// ou imagem `data:image/...`). O @nome confirma-se no servidor de contas.
    ProfileCard {
        username: Option<String>,
        bio: String,
        banner: Option<String>,
    },
    /// Mensagem com resposta e/ou anexos (o `Message` simples continua a valer).
    Post {
        channel: String,
        text: String,
        reply_to: Option<[u8; 32]>,
        files: Vec<Anexo>,
    },
    /// O autor mudou o texto de uma mensagem sua.
    Edit { target: [u8; 32], text: String },
    /// Mensagem apagada (pelo autor ou pelo dono do grupo).
    Delete { target: [u8; 32] },
    /// Reação a uma mensagem (`on: false` tira-a).
    React { target: [u8; 32], emoji: String, on: bool },
}

/// Um ficheiro anexado a uma mensagem. O conteúdo não vai no evento: pede-se a quem o
/// tem pelo hash (ver [`crate::ficheiros`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anexo {
    /// BLAKE3 do conteúdo.
    pub hash: [u8; 32],
    pub name: String,
    pub size: u64,
    pub mime: String,
    /// Miniatura pequena (`data:image/...`) para imagens.
    pub thumb: Option<String>,
    /// Dispositivo de quem o enviou (primeiro sítio onde pedir).
    pub de: [u8; 32],
}

/// Tipo de canal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelKind {
    Text,
    Voice,
}

/// Um canal do grupo, tal como aparece na lista. `id` é o que vai em `Message::channel`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelDef {
    pub id: String,
    pub kind: ChannelKind,
    pub name: String,
    pub topic: String,
    pub category: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBody {
    /// `None` apenas no `GroupCreated`, porque o id do grupo é o hash desse evento.
    pub group: Option<GroupId>,
    pub author: [u8; 32],
    pub timestamp_ms: u64,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedEvent {
    pub body: EventBody,
    pub signature: Vec<u8>,
}

impl SignedEvent {
    pub fn sign(identity: &Identity, group: Option<GroupId>, kind: EventKind) -> Self {
        let body = EventBody {
            group,
            author: *identity.public().as_bytes(),
            timestamp_ms: now_ms(),
            kind,
        };
        let bytes = postcard::to_stdvec(&body).expect("serializar evento");
        let signature = identity.secret().sign(&bytes).to_bytes().to_vec();
        Self { body, signature }
    }

    /// Hash único do evento.
    pub fn id(&self) -> [u8; 32] {
        let bytes = postcard::to_stdvec(&self.body).expect("serializar evento");
        *blake3::hash(&bytes).as_bytes()
    }

    pub fn author(&self) -> Result<PublicKey> {
        PublicKey::from_bytes(&self.body.author)
            .map_err(|e| anyhow!("chave de autor inválida: {e}"))
    }

    /// Confirma que a assinatura é do autor indicado e que o conteúdo não foi alterado.
    pub fn verify(&self) -> Result<()> {
        let sig: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("assinatura com tamanho errado"))?;
        let bytes = postcard::to_stdvec(&self.body)?;
        self.author()?
            .verify(&bytes, &Signature::from_bytes(&sig))
            .map_err(|_| anyhow!("assinatura inválida"))
    }

    /// O grupo a que este evento pertence.
    pub fn group_id(&self) -> GroupId {
        match self.body.group {
            Some(g) => g,
            None => GroupId(self.id()),
        }
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assinatura_valida_e_adulteracao_detetada() {
        let id = Identity::generate();
        let ev = SignedEvent::sign(
            &id,
            None,
            EventKind::GroupCreated {
                name: "malta".into(),
                owner_name: "tiago".into(),
                nonce: [1; 16],
            },
        );
        ev.verify().unwrap();

        let mut mexido = ev.clone();
        mexido.body.kind = EventKind::GroupCreated {
            name: "outro".into(),
            owner_name: "tiago".into(),
            nonce: [1; 16],
        };
        assert!(mexido.verify().is_err());
    }
}
