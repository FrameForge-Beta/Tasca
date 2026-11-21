//! Base de dados SQLite local. No anfitrião é a fonte da verdade do grupo;
//! nos membros é uma cópia (réplica) que serve de backup.

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail, ensure};
use iroh::PublicKey;
use rusqlite::{Connection, OptionalExtension, params};

use crate::caixa::ChaveCaixa;
use crate::event::{ChannelDef, ChannelKind, EventKind, GroupId, SignedEvent};

/// Tamanho máximo de uma mensagem de texto (em caracteres).
pub const MAX_MESSAGE_CHARS: usize = 4000;
const MAX_NAME_CHARS: usize = 64;
const MAX_TOPIC_CHARS: usize = 1024;
const MAX_CHANNELS: usize = 200;
const MAX_CATEGORIES: usize = 50;
/// Tamanho máximo da foto de perfil (`data:` URL), em carateres.
pub const MAX_PHOTO_CHARS: usize = 256 * 1024;

/// Perfil de um membro: cor e foto (o nome fica em `members`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub color: String,
    pub photo: Option<String>,
}

/// Cartão de perfil de um membro: @nome, biografia e banner.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileCard {
    pub username: Option<String>,
    pub bio: String,
    pub banner: Option<String>,
}

/// Canais e categorias de um grupo, por ordem.
pub type Layout = (Vec<ChannelDef>, Vec<String>);

/// Disposição de um grupo que ainda não publicou nenhuma: `geral` (texto) e `Sala` (voz).
/// O id `geral` é o mesmo que a CLI usa, por isso as mensagens antigas aparecem lá.
pub fn default_layout() -> Layout {
    (
        vec![
            ChannelDef {
                id: "geral".into(),
                kind: ChannelKind::Text,
                name: "geral".into(),
                topic: String::new(),
                category: "Conversa".into(),
            },
            ChannelDef {
                id: "sala".into(),
                kind: ChannelKind::Voice,
                name: "Sala".into(),
                topic: String::new(),
                category: "Voz".into(),
            },
        ],
        vec!["Conversa".into(), "Voz".into()],
    )
}

/// Regras de uma disposição de canais (também usadas pela app antes de publicar).
pub fn check_layout(channels: &[ChannelDef], categories: &[String]) -> Result<()> {
    ensure!(channels.len() <= MAX_CHANNELS, "demasiados canais");
    ensure!(categories.len() <= MAX_CATEGORIES, "demasiadas categorias");
    ensure!(
        channels.iter().any(|c| c.kind == ChannelKind::Text),
        "o grupo precisa de pelo menos um canal de texto"
    );
    let mut ids = std::collections::HashSet::new();
    for c in channels {
        check_name(&c.id)?;
        ensure!(ids.insert(c.id.as_str()), "canal repetido");
        check_name(&c.name)?;
        check_name(&c.category)?;
        ensure!(c.topic.chars().count() <= MAX_TOPIC_CHARS, "descrição do canal demasiado longa");
    }
    for c in categories {
        check_name(c)?;
    }
    Ok(())
}

pub struct Store {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub id: GroupId,
    pub name: String,
    pub owner: PublicKey,
    pub hosted: bool,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path).with_context(|| format!("abrir {}", path.display()))?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS groups (
                 id      BLOB PRIMARY KEY,
                 name    TEXT NOT NULL,
                 owner   BLOB NOT NULL,
                 hosted  INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS events (
                 group_id BLOB NOT NULL REFERENCES groups(id),
                 seq      INTEGER NOT NULL,
                 id       BLOB NOT NULL UNIQUE,
                 author   BLOB NOT NULL,
                 ts       INTEGER NOT NULL,
                 data     BLOB NOT NULL,
                 PRIMARY KEY (group_id, seq)
             );
             CREATE TABLE IF NOT EXISTS members (
                 group_id     BLOB NOT NULL REFERENCES groups(id),
                 pubkey       BLOB NOT NULL,
                 display_name TEXT NOT NULL,
                 PRIMARY KEY (group_id, pubkey)
             );
             CREATE TABLE IF NOT EXISTS invites (
                 group_id BLOB NOT NULL REFERENCES groups(id),
                 token    BLOB NOT NULL,
                 PRIMARY KEY (group_id, token)
             );
             -- Chaves das caixas do grupo no servidor (uma por época; a mais alta é a atual).
             -- Sem chave estrangeira: um membro recebe a chave antes do histórico.
             CREATE TABLE IF NOT EXISTS group_keys (
                 group_id  BLOB NOT NULL,
                 epoch     INTEGER NOT NULL,
                 key       BLOB NOT NULL,
                 -- até onde já foi copiado para a caixa (anfitrião) / lido da caixa
                 mirrored  INTEGER NOT NULL DEFAULT 0,
                 read      INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (group_id, epoch)
             );
             -- Última disposição de canais e categorias de cada grupo (postcard).
             CREATE TABLE IF NOT EXISTS layouts (
                 group_id BLOB PRIMARY KEY,
                 data     BLOB NOT NULL
             );
             CREATE TABLE IF NOT EXISTS profiles (
                 group_id BLOB NOT NULL,
                 pubkey   BLOB NOT NULL,
                 color    TEXT NOT NULL,
                 photo    TEXT,
                 PRIMARY KEY (group_id, pubkey)
             );
             CREATE TABLE IF NOT EXISTS profile_cards (
                 group_id BLOB NOT NULL,
                 pubkey   BLOB NOT NULL,
                 username TEXT,
                 bio      TEXT NOT NULL,
                 banner   TEXT,
                 PRIMARY KEY (group_id, pubkey)
             );",
        )?;
        Ok(Self { conn })
    }

    /// Cria um grupo novo hospedado neste PC, a partir de um `GroupCreated` assinado.
    pub fn create_hosted_group(&mut self, created: &SignedEvent) -> Result<GroupId> {
        created.verify()?;
        let EventKind::GroupCreated { name, .. } = &created.body.kind else {
            bail!("o primeiro evento tem de ser GroupCreated");
        };
        ensure!(
            created.body.group.is_none(),
            "GroupCreated não pode indicar grupo"
        );
        check_name(name)?;
        let group = created.group_id();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO groups (id, name, owner, hosted) VALUES (?1, ?2, ?3, 1)",
            params![group.0, name, created.body.author],
        )?;
        insert_event(&tx, group, 1, created)?;
        apply(&tx, group, created)?;
        tx.commit()?;
        Ok(group)
    }

    /// Anfitrião: valida um evento novo, dá-lhe o próximo número e guarda-o.
    ///
    /// `admitted_by_invite` indica se a ligação de onde veio apresentou um convite válido.
    /// Devolve `None` se o evento não muda nada (por exemplo, alguém que já é membro volta a entrar).
    pub fn append_as_host(
        &mut self,
        ev: &SignedEvent,
        admitted_by_invite: bool,
    ) -> Result<Option<u64>> {
        ev.verify()?;
        let group = ev.body.group.ok_or_else(|| anyhow!("evento sem grupo"))?;
        let info = self
            .group(group)?
            .ok_or_else(|| anyhow!("grupo desconhecido"))?;
        ensure!(info.hosted, "este grupo não é hospedado aqui");
        let author = ev.author()?;
        let member = self.is_member(group, &author)?;
        match &ev.body.kind {
            EventKind::GroupCreated { .. } => bail!("o grupo já existe"),
            EventKind::MemberJoined { display_name } => {
                check_name(display_name)?;
                if member {
                    return Ok(None);
                }
                ensure!(
                    admitted_by_invite,
                    "é preciso um convite válido para entrar"
                );
            }
            EventKind::Message { channel, text } => {
                ensure!(member, "só membros podem enviar mensagens");
                check_name(channel)?;
                ensure!(!text.trim().is_empty(), "mensagem vazia");
                ensure!(
                    text.chars().count() <= MAX_MESSAGE_CHARS,
                    "mensagem demasiado longa"
                );
            }
            EventKind::Layout { channels, categories } => {
                ensure!(author == info.owner, "só o dono pode mudar os canais");
                check_layout(channels, categories)?;
            }
            EventKind::Profile { name, color, photo } => {
                ensure!(member, "só membros podem ter perfil no grupo");
                check_name(name)?;
                ensure!(color.len() <= 32, "cor inválida");
                if let Some(p) = photo {
                    ensure!(p.starts_with("data:image/"), "a foto tem de ser uma imagem");
                    ensure!(p.len() <= MAX_PHOTO_CHARS, "foto demasiado grande");
                }
            }
            EventKind::ProfileCard { username, bio, banner } => {
                ensure!(member, "só membros podem ter perfil no grupo");
                if let Some(u) = username {
                    ensure!(crate::servidor::normalize_username(u).ok().as_deref() == Some(u.as_str()), "nome de utilizador inválido");
                }
                ensure!(bio.chars().count() <= crate::amigos::MAX_BIO, "biografia demasiado longa");
                if let Some(b) = banner {
                    ensure!(b.starts_with("data:image/") || b.starts_with('#'), "o banner tem de ser uma cor ou imagem");
                    ensure!(b.len() <= crate::amigos::MAX_BANNER, "banner demasiado grande");
                }
            }
        }
        let tx = self.conn.transaction()?;
        let seq = last_seq(&tx, group)? + 1;
        insert_event(&tx, group, seq, ev)?;
        apply(&tx, group, ev)?;
        tx.commit()?;
        Ok(Some(seq))
    }

    /// Membro: guarda um evento recebido do anfitrião na réplica local.
    /// Devolve `false` se já o tínhamos.
    pub fn insert_replica(&mut self, group: GroupId, seq: u64, ev: &SignedEvent) -> Result<bool> {
        ev.verify()?;
        ensure!(ev.group_id() == group, "evento de outro grupo");
        let tx = self.conn.transaction()?;
        let have = last_seq(&tx, group)?;
        if seq <= have {
            return Ok(false);
        }
        ensure!(
            seq == have + 1,
            "falta histórico: esperava {} e recebi {seq}",
            have + 1
        );
        if seq == 1 {
            let EventKind::GroupCreated { name, .. } = &ev.body.kind else {
                bail!("o primeiro evento tem de ser GroupCreated");
            };
            tx.execute(
                "INSERT OR IGNORE INTO groups (id, name, owner, hosted) VALUES (?1, ?2, ?3, 0)",
                params![group.0, name, ev.body.author],
            )?;
        }
        insert_event(&tx, group, seq, ev)?;
        apply(&tx, group, ev)?;
        tx.commit()?;
        Ok(true)
    }

    pub fn events_since(&self, group: GroupId, since: u64) -> Result<Vec<(u64, SignedEvent)>> {
        let mut stmt = self.conn.prepare(
            "SELECT seq, data FROM events WHERE group_id = ?1 AND seq > ?2 ORDER BY seq",
        )?;
        let rows = stmt.query_map(params![group.0, since as i64], |r| {
            Ok((r.get::<_, i64>(0)? as u64, r.get::<_, Vec<u8>>(1)?))
        })?;
        rows.map(|r| {
            let (seq, data) = r?;
            Ok((seq, postcard::from_bytes(&data)?))
        })
        .collect()
    }

    /// Guarda a chave de uma época. `mirrored` é o último evento que já não vai para esta
    /// caixa (ao rodar a chave, a caixa nova começa a partir dali).
    pub fn set_group_key(&self, group: GroupId, epoch: u32, key: &ChaveCaixa, mirrored: u64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO group_keys (group_id, epoch, key, mirrored) VALUES (?1, ?2, ?3, ?4)",
            params![group.0, epoch, key.as_bytes().as_slice(), mirrored as i64],
        )?;
        Ok(())
    }

    /// A chave atual do grupo: `(época, chave, copiado, lido)`.
    pub fn group_key(&self, group: GroupId) -> Result<Option<(u32, ChaveCaixa, u64, u64)>> {
        self.conn
            .query_row(
                "SELECT epoch, key, mirrored, read FROM group_keys WHERE group_id = ?1 ORDER BY epoch DESC LIMIT 1",
                params![group.0],
                |r| {
                    Ok((
                        r.get::<_, u32>(0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, i64>(2)? as u64,
                        r.get::<_, i64>(3)? as u64,
                    ))
                },
            )
            .optional()?
            .map(|(e, k, m, rd)| {
                let k: [u8; 32] = k.try_into().map_err(|_| anyhow!("chave do grupo inválida"))?;
                Ok((e, ChaveCaixa::from_bytes(k), m, rd))
            })
            .transpose()
    }

    pub fn set_key_cursors(&self, group: GroupId, epoch: u32, mirrored: u64, read: u64) -> Result<()> {
        self.conn.execute(
            "UPDATE group_keys SET mirrored = ?3, read = ?4 WHERE group_id = ?1 AND epoch = ?2",
            params![group.0, epoch, mirrored as i64, read as i64],
        )?;
        Ok(())
    }

    pub fn last_seq(&self, group: GroupId) -> Result<u64> {
        last_seq(&self.conn, group)
    }

    pub fn group(&self, group: GroupId) -> Result<Option<GroupInfo>> {
        self.conn
            .query_row(
                "SELECT name, owner, hosted FROM groups WHERE id = ?1",
                params![group.0],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, bool>(2)?,
                    ))
                },
            )
            .optional()?
            .map(|(name, owner, hosted)| {
                Ok(GroupInfo {
                    id: group,
                    name,
                    owner: pubkey(&owner)?,
                    hosted,
                })
            })
            .transpose()
    }

    /// Grupo de que a conta já é membro mas que este dispositivo ainda não tem (soube dele
    /// por outro dispositivo da conta). O histórico chega quando ligar ao anfitrião.
    /// Devolve `false` se já o tinha.
    pub fn add_known_group(&self, group: GroupId, name: &str, owner: &PublicKey) -> Result<bool> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO groups (id, name, owner, hosted) VALUES (?1, ?2, ?3, 0)",
            params![group.0, name, owner.as_bytes()],
        )?;
        Ok(n > 0)
    }

    pub fn groups(&self) -> Result<Vec<GroupInfo>> {
        let mut stmt = self.conn.prepare("SELECT id FROM groups ORDER BY rowid")?;
        let ids: Vec<Vec<u8>> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        ids.into_iter()
            .filter_map(|id| <[u8; 32]>::try_from(id.as_slice()).ok())
            .filter_map(|id| self.group(GroupId(id)).transpose())
            .collect()
    }

    pub fn is_member(&self, group: GroupId, who: &PublicKey) -> Result<bool> {
        Ok(self.display_name(group, who)?.is_some())
    }

    pub fn display_name(&self, group: GroupId, who: &PublicKey) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT display_name FROM members WHERE group_id = ?1 AND pubkey = ?2",
                params![group.0, who.as_bytes()],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Membros do grupo, pela ordem em que entraram: `(chave, nome)`.
    pub fn members(&self, group: GroupId) -> Result<Vec<(PublicKey, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT pubkey, display_name FROM members WHERE group_id = ?1 ORDER BY rowid",
        )?;
        let rows = stmt.query_map(params![group.0], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?))
        })?;
        rows.map(|r| {
            let (k, n) = r?;
            Ok((pubkey(&k)?, n))
        })
        .collect()
    }

    /// Perfis publicados no grupo, por chave.
    pub fn profiles(&self, group: GroupId) -> Result<std::collections::HashMap<[u8; 32], Profile>> {
        let mut stmt = self
            .conn
            .prepare("SELECT pubkey, color, photo FROM profiles WHERE group_id = ?1")?;
        let rows = stmt.query_map(params![group.0], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
        })?;
        let mut out = std::collections::HashMap::new();
        for r in rows {
            let (k, color, photo) = r?;
            if let Ok(k) = <[u8; 32]>::try_from(k.as_slice()) {
                out.insert(k, Profile { color, photo });
            }
        }
        Ok(out)
    }

    /// Cartões de perfil publicados no grupo, por chave.
    pub fn profile_cards(&self, group: GroupId) -> Result<std::collections::HashMap<[u8; 32], ProfileCard>> {
        let mut stmt = self
            .conn
            .prepare("SELECT pubkey, username, bio, banner FROM profile_cards WHERE group_id = ?1")?;
        let rows = stmt.query_map(params![group.0], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                ProfileCard { username: r.get(1)?, bio: r.get(2)?, banner: r.get(3)? },
            ))
        })?;
        let mut out = std::collections::HashMap::new();
        for r in rows {
            let (k, c) = r?;
            if let Ok(k) = <[u8; 32]>::try_from(k.as_slice()) {
                out.insert(k, c);
            }
        }
        Ok(out)
    }

    /// Quando `who` entrou no grupo (ms), pelo evento de entrada (ou de criação, no dono).
    pub fn member_since(&self, group: GroupId, who: &[u8; 32]) -> Result<Option<u64>> {
        for (_, ev) in self.events_since(group, 0)? {
            if ev.body.author == *who
                && matches!(ev.body.kind, EventKind::MemberJoined { .. } | EventKind::GroupCreated { .. })
            {
                return Ok(Some(ev.body.timestamp_ms));
            }
        }
        Ok(None)
    }

    /// Canais e categorias do grupo. Sem nenhum `Layout` publicado, a disposição de origem.
    pub fn layout(&self, group: GroupId) -> Result<Layout> {
        let data: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT data FROM layouts WHERE group_id = ?1",
                params![group.0],
                |r| r.get(0),
            )
            .optional()?;
        match data {
            Some(d) => Ok(postcard::from_bytes(&d)?),
            None => Ok(default_layout()),
        }
    }

    pub fn add_invite(&self, group: GroupId, token: [u8; 16]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO invites (group_id, token) VALUES (?1, ?2)",
            params![group.0, token],
        )?;
        Ok(())
    }

    pub fn invite_valid(&self, group: GroupId, token: &[u8; 16]) -> Result<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM invites WHERE group_id = ?1 AND token = ?2",
                params![group.0, token],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }
}

fn last_seq(conn: &Connection, group: GroupId) -> Result<u64> {
    let seq: Option<i64> = conn.query_row(
        "SELECT MAX(seq) FROM events WHERE group_id = ?1",
        params![group.0],
        |r| r.get(0),
    )?;
    Ok(seq.unwrap_or(0) as u64)
}

fn insert_event(conn: &Connection, group: GroupId, seq: u64, ev: &SignedEvent) -> Result<()> {
    conn.execute(
        "INSERT INTO events (group_id, seq, id, author, ts, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            group.0,
            seq as i64,
            ev.id(),
            ev.body.author,
            ev.body.timestamp_ms as i64,
            postcard::to_stdvec(ev)?
        ],
    )?;
    Ok(())
}

/// Atualiza as tabelas derivadas (membros) a partir de um evento.
fn apply(conn: &Connection, group: GroupId, ev: &SignedEvent) -> Result<()> {
    match &ev.body.kind {
        EventKind::GroupCreated { owner_name, .. } => {
            conn.execute(
                "INSERT OR REPLACE INTO members (group_id, pubkey, display_name) VALUES (?1, ?2, ?3)",
                params![group.0, ev.body.author, owner_name],
            )?;
        }
        EventKind::MemberJoined { display_name } => {
            conn.execute(
                "INSERT OR REPLACE INTO members (group_id, pubkey, display_name) VALUES (?1, ?2, ?3)",
                params![group.0, ev.body.author, display_name],
            )?;
        }
        EventKind::Message { .. } => {}
        EventKind::Layout { channels, categories } => {
            conn.execute(
                "INSERT OR REPLACE INTO layouts (group_id, data) VALUES (?1, ?2)",
                params![group.0, postcard::to_stdvec(&(channels, categories))?],
            )?;
        }
        EventKind::Profile { name, color, photo } => {
            conn.execute(
                "INSERT OR REPLACE INTO profiles (group_id, pubkey, color, photo) VALUES (?1, ?2, ?3, ?4)",
                params![group.0, ev.body.author, color, photo],
            )?;
            conn.execute(
                "UPDATE members SET display_name = ?3 WHERE group_id = ?1 AND pubkey = ?2",
                params![group.0, ev.body.author, name.trim()],
            )?;
        }
        EventKind::ProfileCard { username, bio, banner } => {
            conn.execute(
                "INSERT OR REPLACE INTO profile_cards (group_id, pubkey, username, bio, banner) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![group.0, ev.body.author, username, bio, banner],
            )?;
        }
    }
    Ok(())
}

fn check_name(name: &str) -> Result<()> {
    let n = name.trim().chars().count();
    ensure!(
        n > 0 && n <= MAX_NAME_CHARS,
        "nome tem de ter entre 1 e {MAX_NAME_CHARS} caracteres"
    );
    Ok(())
}

fn pubkey(bytes: &[u8]) -> Result<PublicKey> {
    let arr: [u8; 32] = bytes.try_into().map_err(|_| anyhow!("chave inválida"))?;
    PublicKey::from_bytes(&arr).map_err(|e| anyhow!("chave inválida: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    fn new_group(store: &mut Store, owner: &Identity) -> GroupId {
        let ev = SignedEvent::sign(
            owner,
            None,
            EventKind::GroupCreated {
                name: "malta".into(),
                owner_name: "tiago".into(),
                nonce: [7; 16],
            },
        );
        store.create_hosted_group(&ev).unwrap()
    }

    #[test]
    fn regras_do_anfitriao() {
        let mut s = Store::open_in_memory().unwrap();
        let dono = Identity::generate();
        let g = new_group(&mut s, &dono);
        let estranho = Identity::generate();

        let msg = |id: &Identity| {
            SignedEvent::sign(
                id,
                Some(g),
                EventKind::Message {
                    channel: "geral".into(),
                    text: "olá".into(),
                },
            )
        };
        // Estranho não pode falar nem entrar sem convite.
        assert!(s.append_as_host(&msg(&estranho), false).is_err());
        let entrar = SignedEvent::sign(
            &estranho,
            Some(g),
            EventKind::MemberJoined {
                display_name: "zé".into(),
            },
        );
        assert!(s.append_as_host(&entrar, false).is_err());
        // Com convite entra, e a seguir pode falar.
        assert_eq!(s.append_as_host(&entrar, true).unwrap(), Some(2));
        assert_eq!(s.append_as_host(&msg(&estranho), false).unwrap(), Some(3));
        // Entrar outra vez não cria evento novo.
        let outra = SignedEvent::sign(
            &estranho,
            Some(g),
            EventKind::MemberJoined {
                display_name: "zé".into(),
            },
        );
        assert_eq!(s.append_as_host(&outra, true).unwrap(), None);
        assert_eq!(
            s.display_name(g, &estranho.public()).unwrap().as_deref(),
            Some("zé")
        );
    }

    #[test]
    fn replica_exige_ordem_e_assinaturas() {
        let mut host = Store::open_in_memory().unwrap();
        let dono = Identity::generate();
        let g = new_group(&mut host, &dono);
        host.append_as_host(
            &SignedEvent::sign(
                &dono,
                Some(g),
                EventKind::Message {
                    channel: "geral".into(),
                    text: "a".into(),
                },
            ),
            false,
        )
        .unwrap();
        let evs = host.events_since(g, 0).unwrap();

        let mut rep = Store::open_in_memory().unwrap();
        // Não aceita saltos no histórico.
        assert!(rep.insert_replica(g, evs[1].0, &evs[1].1).is_err());
        for (seq, ev) in &evs {
            assert!(rep.insert_replica(g, *seq, ev).unwrap());
        }
        // Repetidos são ignorados.
        assert!(!rep.insert_replica(g, evs[1].0, &evs[1].1).unwrap());
        assert_eq!(rep.last_seq(g).unwrap(), 2);
        assert!(!rep.group(g).unwrap().unwrap().hosted);
    }
}
