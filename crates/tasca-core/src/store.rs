//! Base de dados SQLite local. No anfitrião é a fonte da verdade do grupo;
//! nos membros é uma cópia (réplica) que serve de backup.

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail, ensure};
use iroh::PublicKey;
use rusqlite::{Connection, OptionalExtension, params};

use crate::caixa::ChaveCaixa;
use crate::cargos::{self, AcaoVoz, Alvo, Cargo, Definicoes, Excecao};
use crate::event::{ChannelDef, ChannelKind, EventKind, GroupId, SignedEvent};

/// Tamanho máximo de uma mensagem de texto (em caracteres).
pub const MAX_MESSAGE_CHARS: usize = 4000;
const MAX_NAME_CHARS: usize = 64;
const MAX_TOPIC_CHARS: usize = 1024;
const MAX_CHANNELS: usize = 200;
const MAX_CATEGORIES: usize = 50;
/// Tamanho máximo da foto de perfil (`data:` URL), em carateres.
pub const MAX_PHOTO_CHARS: usize = 256 * 1024;
/// Anexos por mensagem.
pub const MAX_ANEXOS: usize = 10;

/// Mensagem de texto (simples ou com anexos/resposta).
pub fn e_mensagem(k: &EventKind) -> bool {
    matches!(k, EventKind::Message { .. } | EventKind::Post { .. })
}

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
    /// O que o AutoMod apanhou no último evento (o anfitrião trata do castigo e do alerta).
    automod: Option<AutoModCaso>,
    /// Hora a usar nas regras enquanto se aplica um evento da caixa (a hora do servidor),
    /// para todos os dispositivos chegarem ao mesmo resultado.
    relogio: Option<u64>,
}

/// Uma mensagem apanhada pelo AutoMod.
#[derive(Debug, Clone)]
pub struct AutoModCaso {
    pub bloqueio: cargos::Bloqueio,
    pub autor: [u8; 32],
    pub canal: String,
    pub texto: String,
}

/// Uma pessoa banida.
#[derive(Debug, Clone)]
pub struct Banido {
    pub pubkey: [u8; 32],
    pub name: String,
    pub reason: String,
    pub by: [u8; 32],
    pub ts: u64,
}

/// Um convite publicado no grupo.
#[derive(Debug, Clone)]
pub struct ConviteInfo {
    pub hash: [u8; 32],
    pub by: [u8; 32],
    pub max_uses: u32,
    pub expires_ms: u64,
    pub uses: u32,
    pub ts: u64,
}

/// Tamanho máximo de um ícone de grupo ou de cargo (`data:` URL).
pub const MAX_ICON_CHARS: usize = 128 * 1024;
const MAX_ROLES: usize = 250;
const MAX_REASON: usize = 512;
/// Castigo mais longo: 28 dias.
pub const MAX_TIMEOUT_MS: u64 = 28 * 24 * 3600 * 1000;

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
             );
             -- cargos (k = 'roles') e definições (k = 'settings'), em postcard
             CREATE TABLE IF NOT EXISTS group_state (
                 group_id BLOB NOT NULL,
                 k        TEXT NOT NULL,
                 data     BLOB NOT NULL,
                 PRIMARY KEY (group_id, k)
             );
             CREATE TABLE IF NOT EXISTS member_roles (
                 group_id BLOB NOT NULL,
                 pubkey   BLOB NOT NULL,
                 data     BLOB NOT NULL,
                 PRIMARY KEY (group_id, pubkey)
             );
             CREATE TABLE IF NOT EXISTS channel_perms (
                 group_id BLOB NOT NULL,
                 channel  TEXT NOT NULL,
                 data     BLOB NOT NULL,
                 PRIMARY KEY (group_id, channel)
             );
             CREATE TABLE IF NOT EXISTS bans (
                 group_id BLOB NOT NULL,
                 pubkey   BLOB NOT NULL,
                 name     TEXT NOT NULL,
                 reason   TEXT NOT NULL,
                 by       BLOB NOT NULL,
                 ts       INTEGER NOT NULL,
                 PRIMARY KEY (group_id, pubkey)
             );
             CREATE TABLE IF NOT EXISTS timeouts (
                 group_id BLOB NOT NULL,
                 pubkey   BLOB NOT NULL,
                 until    INTEGER NOT NULL,
                 PRIMARY KEY (group_id, pubkey)
             );
             CREATE TABLE IF NOT EXISTS nicks (
                 group_id BLOB NOT NULL,
                 pubkey   BLOB NOT NULL,
                 nick     TEXT NOT NULL,
                 PRIMARY KEY (group_id, pubkey)
             );
             CREATE TABLE IF NOT EXISTS pins (
                 group_id BLOB NOT NULL,
                 target   BLOB NOT NULL,
                 by       BLOB NOT NULL,
                 ts       INTEGER NOT NULL,
                 PRIMARY KEY (group_id, target)
             );
             CREATE TABLE IF NOT EXISTS voice_mod (
                 group_id BLOB NOT NULL,
                 pubkey   BLOB NOT NULL,
                 muted    INTEGER NOT NULL,
                 deaf     INTEGER NOT NULL,
                 PRIMARY KEY (group_id, pubkey)
             );
             CREATE TABLE IF NOT EXISTS group_invites (
                 group_id BLOB NOT NULL,
                 hash     BLOB NOT NULL,
                 by       BLOB NOT NULL,
                 max_uses INTEGER NOT NULL,
                 expires  INTEGER NOT NULL,
                 uses     INTEGER NOT NULL DEFAULT 0,
                 revoked  INTEGER NOT NULL DEFAULT 0,
                 ts       INTEGER NOT NULL,
                 PRIMARY KEY (group_id, hash)
             );
             -- mensagens que os membros deixaram na caixa com o anfitrião desligado; saem
             -- daqui quando o anfitrião as aceita (e passam a ter número)
             CREATE TABLE IF NOT EXISTS pending_events (
                 group_id BLOB NOT NULL,
                 id       BLOB NOT NULL,
                 ts       INTEGER NOT NULL,
                 data     BLOB NOT NULL,
                 PRIMARY KEY (group_id, id)
             );
             -- quem já saiu (para os nomes das mensagens antigas)
             CREATE TABLE IF NOT EXISTS ex_members (
                 group_id     BLOB NOT NULL,
                 pubkey       BLOB NOT NULL,
                 display_name TEXT NOT NULL,
                 PRIMARY KEY (group_id, pubkey)
             );",
        )?;
        conn.execute_batch(
            "-- épocas da caixa do grupo já fechadas (a chave mudou e a leitura passou à seguinte)
             CREATE TABLE IF NOT EXISTS caixa_epocas (
                 group_id BLOB NOT NULL,
                 epoch    INTEGER NOT NULL,
                 PRIMARY KEY (group_id, epoch)
             );",
        )?;
        Ok(Self { conn, automod: None, relogio: None })
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
        let group = ev.body.group.ok_or_else(|| anyhow!("evento sem grupo"))?;
        let info = self.group(group)?.ok_or_else(|| anyhow!("grupo desconhecido"))?;
        ensure!(info.hosted, "este grupo não é hospedado aqui");
        self.validar_e_guardar(ev, admitted_by_invite)
    }

    /// A hora das regras: a do servidor ao aplicar a caixa, senão a deste dispositivo.
    fn agora(&self) -> u64 {
        self.relogio.unwrap_or_else(crate::event::now_ms)
    }

    /// Valida um evento novo com as regras do grupo, dá-lhe o próximo número e guarda-o.
    fn validar_e_guardar(&mut self, ev: &SignedEvent, admitted_by_invite: bool) -> Result<Option<u64>> {
        ev.verify()?;
        let group = ev.body.group.ok_or_else(|| anyhow!("evento sem grupo"))?;
        let info = self
            .group(group)?
            .ok_or_else(|| anyhow!("grupo desconhecido"))?;
        let author = ev.author()?;
        let a = ev.body.author;
        let member = self.is_member(group, &author)?;
        let dono = author == info.owner;
        let agora = self.agora();
        self.automod = None;
        // permissões do autor no grupo (ou num canal)
        let pode = |s: &Self, ch: Option<&str>, p: u64| -> Result<()> {
            ensure!(member, "só membros podem fazer isto");
            ensure!(s.perms(group, &a, ch)? & p != 0, "não tens permissão para isto");
            Ok(())
        };
        match &ev.body.kind {
            EventKind::GroupCreated { .. } => bail!("o grupo já existe"),
            EventKind::MemberJoined { display_name } => {
                check_name(display_name)?;
                if member {
                    return Ok(None);
                }
                ensure!(!self.is_banned(group, &a)?, "foste banido deste grupo");
                ensure!(
                    admitted_by_invite,
                    "é preciso um convite válido para entrar"
                );
                let def = self.settings(group)?;
                ensure!(!def.invites_paused, "os convites deste grupo estão em pausa");
                if def.raid_limit > 0 {
                    let n: i64 = self.conn.query_row(
                        "SELECT COUNT(*) FROM members m JOIN events e ON e.group_id = m.group_id AND e.author = m.pubkey
                         WHERE m.group_id = ?1 AND e.ts > ?2 AND e.seq = (SELECT MIN(seq) FROM events WHERE group_id = ?1 AND author = m.pubkey)",
                        params![group.0, (agora.saturating_sub(60_000)) as i64],
                        |r| r.get(0),
                    )?;
                    ensure!(
                        (n as u32) < def.raid_limit,
                        "o grupo recebeu muitas entradas seguidas; tenta daqui a um minuto"
                    );
                }
            }
            EventKind::Message { channel, text } => {
                ensure!(member, "só membros podem enviar mensagens");
                check_name(channel)?;
                ensure!(!text.trim().is_empty(), "mensagem vazia");
                ensure!(
                    text.chars().count() <= MAX_MESSAGE_CHARS,
                    "mensagem demasiado longa"
                );
                self.pode_escrever(group, &a, channel, false, dono, agora)?;
                self.passar_automod(group, &a, channel, text, dono)?;
            }
            EventKind::Post { channel, text, files, reply_to } => {
                ensure!(member, "só membros podem enviar mensagens");
                check_name(channel)?;
                ensure!(!text.trim().is_empty() || !files.is_empty(), "mensagem vazia");
                ensure!(text.chars().count() <= MAX_MESSAGE_CHARS, "mensagem demasiado longa");
                ensure!(files.len() <= MAX_ANEXOS, "demasiados anexos");
                for f in files {
                    ensure!(!f.name.is_empty() && f.name.chars().count() <= 200, "nome de ficheiro inválido");
                    ensure!(f.mime.len() <= 100, "tipo de ficheiro inválido");
                    if let Some(t) = &f.thumb {
                        ensure!(t.starts_with("data:image/") && t.len() <= MAX_PHOTO_CHARS, "miniatura inválida");
                    }
                }
                let _ = reply_to;
                self.pode_escrever(group, &a, channel, !files.is_empty(), dono, agora)?;
                if !text.trim().is_empty() {
                    self.passar_automod(group, &a, channel, text, dono)?;
                }
            }
            EventKind::Edit { target, text } => {
                let alvo = self.event_by_id(group, target)?.ok_or_else(|| anyhow!("mensagem desconhecida"))?;
                ensure!(alvo.body.author == ev.body.author, "só podes editar as tuas mensagens");
                ensure!(e_mensagem(&alvo.body.kind), "só se editam mensagens");
                ensure!(!text.trim().is_empty(), "mensagem vazia");
                ensure!(text.chars().count() <= MAX_MESSAGE_CHARS, "mensagem demasiado longa");
                let ch = canal_de(&alvo.body.kind).unwrap_or_default();
                ensure!(self.castigo(group, &a)? <= agora, "estás de castigo");
                self.passar_automod(group, &a, &ch, text, dono)?;
            }
            EventKind::Delete { target } => {
                let alvo = self.event_by_id(group, target)?.ok_or_else(|| anyhow!("mensagem desconhecida"))?;
                ensure!(e_mensagem(&alvo.body.kind), "só se apagam mensagens");
                if alvo.body.author != ev.body.author {
                    let ch = canal_de(&alvo.body.kind);
                    pode(self, ch.as_deref(), cargos::GERIR_MENSAGENS)
                        .map_err(|_| anyhow!("só o autor ou quem gere mensagens as apaga"))?;
                }
            }
            EventKind::React { target, emoji, .. } => {
                ensure!(member, "só membros reagem");
                ensure!(!emoji.is_empty() && emoji.len() <= 64, "reação inválida");
                let alvo = self.event_by_id(group, target)?.ok_or_else(|| anyhow!("mensagem desconhecida"))?;
                ensure!(e_mensagem(&alvo.body.kind), "só se reage a mensagens");
                let ch = canal_de(&alvo.body.kind);
                pode(self, ch.as_deref(), cargos::REAGIR)?;
            }
            EventKind::Layout { channels, categories } => {
                pode(self, None, cargos::GERIR_CANAIS).map_err(|_| anyhow!("não tens permissão para mudar os canais"))?;
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
            EventKind::Roles { roles } => {
                pode(self, None, cargos::GERIR_CARGOS)?;
                check_roles(roles)?;
                if !dono {
                    let antes = self.roles(group)?;
                    let minha = self.rank(group, &a)?;
                    let fixos = ((minha + 1).max(0) as usize).min(antes.len());
                    ensure!(
                        roles.len() >= fixos && roles[..fixos] == antes[..fixos],
                        "só mexes em cargos abaixo do teu"
                    );
                    let meus = self.perms(group, &a, None)?;
                    for r in roles {
                        if antes.iter().any(|x| x == r) {
                            continue;
                        }
                        ensure!(r.perms & !meus == 0, "não podes dar permissões que não tens");
                    }
                }
            }
            EventKind::MemberRoles { member: alvo, roles } => {
                pode(self, None, cargos::GERIR_CARGOS)?;
                ensure!(self.is_member(group, &pubkey(alvo)?)?, "essa pessoa não é membro");
                ensure!(roles.len() <= 50, "demasiados cargos");
                let todos = self.roles(group)?;
                for r in roles {
                    ensure!(r != cargos::TODOS && todos.iter().any(|c| &c.id == r), "cargo desconhecido");
                }
                if !dono {
                    let minha = self.rank(group, &a)?;
                    let antes = self.member_roles(group, alvo)?;
                    for r in roles.iter().filter(|r| !antes.contains(r)).chain(antes.iter().filter(|r| !roles.contains(r))) {
                        let i = todos.iter().position(|c| &c.id == r).map(|i| i as i64).unwrap_or(i64::MAX);
                        ensure!(i > minha, "só dás ou tiras cargos abaixo do teu");
                    }
                }
            }
            EventKind::ChannelPerms { channel, overrides } => {
                pode(self, None, cargos::GERIR_CANAIS)?;
                ensure!(overrides.len() <= 100, "demasiadas exceções");
                let (canais, cats) = self.layout(group)?;
                let existe = match channel.strip_prefix("cat:") {
                    Some(c) => cats.iter().any(|x| x == c) || canais.iter().any(|x| x.category == c),
                    None => canais.iter().any(|x| &x.id == channel),
                };
                ensure!(existe, "canal desconhecido");
                let todos = self.roles(group)?;
                let meus = self.perms(group, &a, None)?;
                for o in overrides {
                    ensure!((o.allow | o.deny) & !cargos::TODAS == 0, "permissão desconhecida");
                    if let Alvo::Cargo(id) = &o.alvo {
                        ensure!(todos.iter().any(|c| &c.id == id) || id == cargos::TODOS, "cargo desconhecido");
                    }
                    ensure!(dono || (o.allow | o.deny) & !meus == 0, "não podes mexer em permissões que não tens");
                }
            }
            EventKind::Kick { member: alvo, reason } => {
                pode(self, None, cargos::EXPULSAR)?;
                ensure!(reason.chars().count() <= MAX_REASON, "motivo demasiado longo");
                ensure!(self.is_member(group, &pubkey(alvo)?)?, "essa pessoa não é membro");
                self.acima(group, &info, &a, alvo)?;
            }
            EventKind::Ban { member: alvo, reason, .. } => {
                pode(self, None, cargos::BANIR)?;
                ensure!(reason.chars().count() <= MAX_REASON, "motivo demasiado longo");
                ensure!(!self.is_banned(group, alvo)?, "essa pessoa já está banida");
                pubkey(alvo)?;
                self.acima(group, &info, &a, alvo)?;
            }
            EventKind::Unban { member: alvo } => {
                pode(self, None, cargos::BANIR)?;
                ensure!(self.is_banned(group, alvo)?, "essa pessoa não está banida");
            }
            EventKind::Timeout { member: alvo, until_ms, reason } => {
                pode(self, None, cargos::CASTIGAR)?;
                ensure!(reason.chars().count() <= MAX_REASON, "motivo demasiado longo");
                ensure!(*until_ms <= agora + MAX_TIMEOUT_MS + 60_000, "castigo demasiado longo (máximo 28 dias)");
                ensure!(self.is_member(group, &pubkey(alvo)?)?, "essa pessoa não é membro");
                self.acima(group, &info, &a, alvo)?;
            }
            EventKind::Nick { member: alvo, nick } => {
                if let Some(n) = nick {
                    ensure!(!n.trim().is_empty() && n.chars().count() <= 32, "o apelido tem de ter entre 1 e 32 carateres");
                }
                if *alvo == a {
                    pode(self, None, cargos::MUDAR_APELIDO)?;
                } else {
                    pode(self, None, cargos::GERIR_APELIDOS)?;
                    ensure!(self.is_member(group, &pubkey(alvo)?)?, "essa pessoa não é membro");
                    self.acima(group, &info, &a, alvo)?;
                }
            }
            EventKind::Pin { target, .. } => {
                let alvo = self.event_by_id(group, target)?.ok_or_else(|| anyhow!("mensagem desconhecida"))?;
                ensure!(e_mensagem(&alvo.body.kind), "só se fixam mensagens");
                let ch = canal_de(&alvo.body.kind);
                pode(self, ch.as_deref(), cargos::GERIR_MENSAGENS)?;
            }
            EventKind::Settings(d) => {
                pode(self, None, cargos::GERIR_GRUPO)?;
                check_settings(d)?;
            }
            EventKind::VoiceMod { member: alvo, action } => {
                pode(self, None, action.permissao())?;
                ensure!(self.is_member(group, &pubkey(alvo)?)?, "essa pessoa não é membro");
                if let AcaoVoz::Mover(sala) = action {
                    let (canais, _) = self.layout(group)?;
                    ensure!(
                        canais.iter().any(|c| &c.id == sala && c.kind == ChannelKind::Voice),
                        "sala de voz desconhecida"
                    );
                }
                if *alvo != a {
                    self.acima(group, &info, &a, alvo)?;
                }
            }
            EventKind::InviteCreated { max_uses, expires_ms, .. } => {
                pode(self, None, cargos::CRIAR_CONVITES)?;
                ensure!(*max_uses <= 10_000, "demasiados usos");
                ensure!(*expires_ms == 0 || *expires_ms > agora, "o convite já expirou");
                ensure!(!self.settings(group)?.invites_paused, "os convites deste grupo estão em pausa");
            }
            EventKind::InviteRevoked { hash } => {
                let by: Option<Vec<u8>> = self
                    .conn
                    .query_row(
                        "SELECT by FROM group_invites WHERE group_id = ?1 AND hash = ?2",
                        params![group.0, hash],
                        |r| r.get(0),
                    )
                    .optional()?;
                let by = by.ok_or_else(|| anyhow!("convite desconhecido"))?;
                if by != a {
                    pode(self, None, cargos::GERIR_GRUPO)?;
                }
            }
            EventKind::Report { target, reason } => {
                ensure!(member, "só membros denunciam");
                ensure!(reason.chars().count() <= MAX_REASON, "motivo demasiado longo");
                ensure!(self.event_by_id(group, target)?.is_some(), "mensagem desconhecida");
            }
            EventKind::Leave => {
                ensure!(member, "não és membro");
                ensure!(!dono, "o dono não pode sair do próprio grupo");
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
        tx.execute("DELETE FROM pending_events WHERE group_id = ?1 AND id = ?2", params![group.0, ev.id()])?;
        tx.commit()?;
        Ok(true)
    }

    /// Esquece um grupo neste dispositivo: apaga o histórico, os membros e as chaves.
    pub fn forget_group(&mut self, group: GroupId) -> Result<()> {
        let tx = self.conn.transaction()?;
        for t in [
            "events", "members", "invites", "group_keys", "layouts", "profiles", "profile_cards", "group_state",
            "member_roles", "channel_perms", "bans", "timeouts", "nicks", "pins", "voice_mod", "group_invites",
            "ex_members", "pending_events", "caixa_epocas",
        ] {
            tx.execute(&format!("DELETE FROM {t} WHERE group_id = ?1"), params![group.0])?;
        }
        tx.execute("DELETE FROM groups WHERE id = ?1", params![group.0])?;
        tx.commit()?;
        Ok(())
    }

    /// Copia a base de dados inteira para um ficheiro novo (cópia de segurança).
    pub fn copy_to(&self, path: &Path) -> Result<()> {
        ensure!(!path.exists(), "esse ficheiro já existe");
        self.conn.execute("VACUUM INTO ?1", params![path.to_string_lossy()])?;
        Ok(())
    }

    /// Um evento do grupo pelo id.
    pub fn event_by_id(&self, group: GroupId, id: &[u8; 32]) -> Result<Option<SignedEvent>> {
        let data: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT data FROM events WHERE group_id = ?1 AND id = ?2",
                params![group.0, id],
                |r| r.get(0),
            )
            .optional()?;
        data.map(|d| Ok(postcard::from_bytes(&d)?)).transpose()
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
        let hash = *blake3::hash(token).as_bytes();
        if self.group_invites(group)?.iter().any(|c| c.hash == hash) {
            return Ok(true);
        }
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

// ---------------- cargos, permissões e moderação ----------------

impl Store {
    fn estado<T: serde::de::DeserializeOwned>(&self, group: GroupId, k: &str) -> Result<Option<T>> {
        let data: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT data FROM group_state WHERE group_id = ?1 AND k = ?2",
                params![group.0, k],
                |r| r.get(0),
            )
            .optional()?;
        data.map(|d| Ok(postcard::from_bytes(&d)?)).transpose()
    }

    /// Cargos do grupo, do mais alto para o mais baixo; o último é sempre o `@todos`.
    pub fn roles(&self, group: GroupId) -> Result<Vec<Cargo>> {
        Ok(self.estado(group, "roles")?.unwrap_or_else(|| vec![Cargo::todos()]))
    }

    /// Definições do grupo (sem nenhuma publicada, as de origem com o nome do grupo).
    pub fn settings(&self, group: GroupId) -> Result<Definicoes> {
        match self.estado(group, "settings")? {
            Some(d) => Ok(d),
            None => Ok(Definicoes {
                name: self.group(group)?.map(|g| g.name).unwrap_or_default(),
                welcome: true,
                leave_notices: true,
                afk_secs: 300,
                ..Default::default()
            }),
        }
    }

    pub fn member_roles(&self, group: GroupId, who: &[u8; 32]) -> Result<Vec<String>> {
        let data: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT data FROM member_roles WHERE group_id = ?1 AND pubkey = ?2",
                params![group.0, who],
                |r| r.get(0),
            )
            .optional()?;
        Ok(data.map(|d| postcard::from_bytes(&d)).transpose()?.unwrap_or_default())
    }

    /// Cargos de todos os membros, por chave.
    pub fn all_member_roles(&self, group: GroupId) -> Result<std::collections::HashMap<[u8; 32], Vec<String>>> {
        let mut stmt = self.conn.prepare("SELECT pubkey, data FROM member_roles WHERE group_id = ?1")?;
        let rows = stmt.query_map(params![group.0], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?)))?;
        let mut out = std::collections::HashMap::new();
        for r in rows {
            let (k, d) = r?;
            if let Ok(k) = <[u8; 32]>::try_from(k.as_slice()) {
                out.insert(k, postcard::from_bytes(&d)?);
            }
        }
        Ok(out)
    }

    /// Exceções de um canal (id) ou categoria (`cat:<nome>`).
    pub fn channel_perms(&self, group: GroupId, channel: &str) -> Result<Vec<Excecao>> {
        let data: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT data FROM channel_perms WHERE group_id = ?1 AND channel = ?2",
                params![group.0, channel],
                |r| r.get(0),
            )
            .optional()?;
        Ok(data.map(|d| postcard::from_bytes(&d)).transpose()?.unwrap_or_default())
    }

    /// Até quando a pessoa está de castigo (ms; 0 = não está).
    pub fn castigo(&self, group: GroupId, who: &[u8; 32]) -> Result<u64> {
        let v: Option<i64> = self
            .conn
            .query_row(
                "SELECT until FROM timeouts WHERE group_id = ?1 AND pubkey = ?2",
                params![group.0, who],
                |r| r.get(0),
            )
            .optional()?;
        Ok(v.unwrap_or(0) as u64)
    }

    /// Permissões de uma pessoa no grupo, ou num canal.
    pub fn perms(&self, group: GroupId, who: &[u8; 32], channel: Option<&str>) -> Result<u64> {
        let Some(info) = self.group(group)? else { return Ok(0) };
        let dono = info.owner.as_bytes() == who;
        let cargos_ = self.roles(group)?;
        let meus = self.member_roles(group, who)?;
        let castigo = self.castigo(group, who)? > self.agora();
        let mut listas: Vec<Vec<Excecao>> = Vec::new();
        if let Some(ch) = channel {
            let (canais, _) = self.layout(group)?;
            if let Some(c) = canais.iter().find(|c| c.id == ch) {
                listas.push(self.channel_perms(group, &cargos::chave_categoria(&c.category))?);
            }
            listas.push(self.channel_perms(group, ch)?);
        }
        let refs: Vec<&[Excecao]> = listas.iter().map(|l| l.as_slice()).collect();
        Ok(cargos::calcular(dono, &cargos_, &meus, who, &refs, castigo))
    }

    /// Posição na hierarquia (ver [`cargos::posicao`]).
    pub fn rank(&self, group: GroupId, who: &[u8; 32]) -> Result<i64> {
        let dono = self.group(group)?.is_some_and(|g| g.owner.as_bytes() == who);
        Ok(cargos::posicao(dono, &self.roles(group)?, &self.member_roles(group, who)?))
    }

    /// O autor manda mais do que o alvo (o dono manda em todos; ninguém mexe no dono).
    fn acima(&self, group: GroupId, info: &GroupInfo, autor: &[u8; 32], alvo: &[u8; 32]) -> Result<()> {
        ensure!(info.owner.as_bytes() != alvo, "ninguém mexe no dono do grupo");
        ensure!(autor != alvo, "não podes fazer isto a ti próprio");
        if info.owner.as_bytes() == autor {
            return Ok(());
        }
        ensure!(
            self.rank(group, autor)? < self.rank(group, alvo)?,
            "essa pessoa tem um cargo igual ou acima do teu"
        );
        Ok(())
    }

    fn pode_escrever(&self, group: GroupId, who: &[u8; 32], ch: &str, anexos: bool, dono: bool, agora: u64) -> Result<()> {
        if dono {
            return Ok(());
        }
        let ate = self.castigo(group, who)?;
        ensure!(ate <= agora, "estás de castigo e não podes escrever");
        let p = self.perms(group, who, Some(ch))?;
        ensure!(p & cargos::ENVIAR_MENSAGENS != 0, "não podes escrever neste canal");
        ensure!(!anexos || p & cargos::ANEXAR != 0, "não podes enviar ficheiros neste canal");
        let def = self.settings(group)?;
        if def.min_member_secs > 0 {
            let desde: Option<i64> = self
                .conn
                .query_row(
                    "SELECT ts FROM events WHERE group_id = ?1 AND author = ?2 ORDER BY seq LIMIT 1",
                    params![group.0, who],
                    |r| r.get(0),
                )
                .optional()?;
            let falta = (desde.unwrap_or(0) as u64 + def.min_member_secs as u64 * 1000).saturating_sub(agora);
            ensure!(falta == 0, "és membro há pouco tempo; podes escrever daqui a {} min", falta.div_ceil(60_000));
        }
        if def.need_username {
            let tem: Option<Option<String>> = self
                .conn
                .query_row(
                    "SELECT username FROM profile_cards WHERE group_id = ?1 AND pubkey = ?2",
                    params![group.0, who],
                    |r| r.get(0),
                )
                .optional()?;
            ensure!(tem.flatten().is_some(), "este grupo só deixa escrever a contas com @nome de utilizador");
        }
        Ok(())
    }

    /// Corre o AutoMod; guarda o caso para o anfitrião tratar do castigo e do alerta.
    fn passar_automod(&mut self, group: GroupId, who: &[u8; 32], ch: &str, texto: &str, dono: bool) -> Result<()> {
        if dono {
            return Ok(());
        }
        let def = self.settings(group)?;
        if def.automod.iter().all(|r| !r.on) {
            return Ok(());
        }
        let meus = self.member_roles(group, who)?;
        // administradores e quem gere o grupo não são apanhados
        if self.perms(group, who, None)? & (cargos::ADMINISTRADOR | cargos::GERIR_GRUPO) != 0 {
            return Ok(());
        }
        let anteriores = self.textos_recentes(group, who, ch, 6)?;
        if let Some(b) = cargos::automod(&def.automod, ch, texto, &meus, &anteriores) {
            let bloqueia = b.block;
            self.automod = Some(AutoModCaso { bloqueio: b.clone(), autor: *who, canal: ch.into(), texto: texto.into() });
            if bloqueia {
                return Err(anyhow::Error::new(b));
            }
        }
        Ok(())
    }

    /// O caso do AutoMod do último evento (só o anfitrião usa).
    pub fn take_automod(&mut self) -> Option<AutoModCaso> {
        self.automod.take()
    }

    /// Os últimos textos de alguém num canal (o mais recente no fim), nos últimos 2 minutos.
    fn textos_recentes(&self, group: GroupId, who: &[u8; 32], ch: &str, n: usize) -> Result<Vec<String>> {
        let desde = self.agora().saturating_sub(120_000) as i64;
        let mut stmt = self.conn.prepare(
            "SELECT data FROM events WHERE group_id = ?1 AND author = ?2 AND ts > ?3 ORDER BY seq DESC LIMIT 30",
        )?;
        let rows = stmt.query_map(params![group.0, who, desde], |r| r.get::<_, Vec<u8>>(0))?;
        let mut out = Vec::new();
        for r in rows {
            let ev: SignedEvent = postcard::from_bytes(&r?)?;
            match &ev.body.kind {
                EventKind::Message { channel, text } | EventKind::Post { channel, text, .. } if channel == ch => {
                    out.push(text.clone())
                }
                _ => {}
            }
            if out.len() >= n {
                break;
            }
        }
        out.reverse();
        Ok(out)
    }

    pub fn is_banned(&self, group: GroupId, who: &[u8; 32]) -> Result<bool> {
        Ok(self
            .conn
            .query_row("SELECT 1 FROM bans WHERE group_id = ?1 AND pubkey = ?2", params![group.0, who], |_| Ok(()))
            .optional()?
            .is_some())
    }

    pub fn bans(&self, group: GroupId) -> Result<Vec<Banido>> {
        let mut stmt = self
            .conn
            .prepare("SELECT pubkey, name, reason, by, ts FROM bans WHERE group_id = ?1 ORDER BY ts DESC")?;
        let rows = stmt.query_map(params![group.0], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Vec<u8>>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (k, name, reason, by, ts) = r?;
            if let (Ok(k), Ok(by)) = (<[u8; 32]>::try_from(k.as_slice()), <[u8; 32]>::try_from(by.as_slice())) {
                out.push(Banido { pubkey: k, name, reason, by, ts: ts as u64 });
            }
        }
        Ok(out)
    }

    /// Castigos ainda a decorrer, por chave (até quando, em ms).
    pub fn timeouts(&self, group: GroupId) -> Result<std::collections::HashMap<[u8; 32], u64>> {
        let agora = crate::event::now_ms() as i64;
        let mut stmt = self.conn.prepare("SELECT pubkey, until FROM timeouts WHERE group_id = ?1 AND until > ?2")?;
        let rows = stmt.query_map(params![group.0, agora], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?)))?;
        let mut out = std::collections::HashMap::new();
        for r in rows {
            let (k, u) = r?;
            if let Ok(k) = <[u8; 32]>::try_from(k.as_slice()) {
                out.insert(k, u as u64);
            }
        }
        Ok(out)
    }

    /// Apelidos neste grupo, por chave.
    pub fn nicks(&self, group: GroupId) -> Result<std::collections::HashMap<[u8; 32], String>> {
        let mut stmt = self.conn.prepare("SELECT pubkey, nick FROM nicks WHERE group_id = ?1")?;
        let rows = stmt.query_map(params![group.0], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = std::collections::HashMap::new();
        for r in rows {
            let (k, n) = r?;
            if let Ok(k) = <[u8; 32]>::try_from(k.as_slice()) {
                out.insert(k, n);
            }
        }
        Ok(out)
    }

    /// Mensagens fixadas, da mais recente para a mais antiga.
    pub fn pins(&self, group: GroupId) -> Result<Vec<[u8; 32]>> {
        let mut stmt = self.conn.prepare("SELECT target FROM pins WHERE group_id = ?1 ORDER BY ts DESC")?;
        let rows = stmt.query_map(params![group.0], |r| r.get::<_, Vec<u8>>(0))?;
        let mut out = Vec::new();
        for r in rows {
            if let Ok(k) = <[u8; 32]>::try_from(r?.as_slice()) {
                out.push(k);
            }
        }
        Ok(out)
    }

    /// Quem está silenciado ou ensurdecido por um moderador: chave → (silenciado, surdo).
    pub fn voice_mods(&self, group: GroupId) -> Result<std::collections::HashMap<[u8; 32], (bool, bool)>> {
        let mut stmt = self.conn.prepare("SELECT pubkey, muted, deaf FROM voice_mod WHERE group_id = ?1")?;
        let rows = stmt.query_map(params![group.0], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, bool>(1)?, r.get::<_, bool>(2)?))
        })?;
        let mut out = std::collections::HashMap::new();
        for r in rows {
            let (k, m, d) = r?;
            if let Ok(k) = <[u8; 32]>::try_from(k.as_slice()) {
                if m || d {
                    out.insert(k, (m, d));
                }
            }
        }
        Ok(out)
    }

    /// Convites ativos (não revogados nem expirados).
    pub fn group_invites(&self, group: GroupId) -> Result<Vec<ConviteInfo>> {
        let agora = crate::event::now_ms() as i64;
        let mut stmt = self.conn.prepare(
            "SELECT hash, by, max_uses, expires, uses, ts FROM group_invites
             WHERE group_id = ?1 AND revoked = 0 AND (expires = 0 OR expires > ?2) ORDER BY ts DESC",
        )?;
        let rows = stmt.query_map(params![group.0, agora], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (h, by, max, exp, uses, ts) = r?;
            if let (Ok(hash), Ok(by)) = (<[u8; 32]>::try_from(h.as_slice()), <[u8; 32]>::try_from(by.as_slice())) {
                if max == 0 || uses < max {
                    out.push(ConviteInfo {
                        hash,
                        by,
                        max_uses: max as u32,
                        expires_ms: exp as u64,
                        uses: uses as u32,
                        ts: ts as u64,
                    });
                }
            }
        }
        Ok(out)
    }

    /// Anfitrião: conta mais um uso de um convite publicado.
    pub fn use_invite(&self, group: GroupId, token: &[u8; 16]) -> Result<()> {
        let hash = *blake3::hash(token).as_bytes();
        self.conn.execute(
            "UPDATE group_invites SET uses = uses + 1 WHERE group_id = ?1 AND hash = ?2",
            params![group.0, hash],
        )?;
        Ok(())
    }

    /// Nomes de quem já saiu do grupo, por chave.
    pub fn ex_members(&self, group: GroupId) -> Result<Vec<([u8; 32], String)>> {
        let mut stmt = self.conn.prepare("SELECT pubkey, display_name FROM ex_members WHERE group_id = ?1")?;
        let rows = stmt.query_map(params![group.0], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = Vec::new();
        for r in rows {
            let (k, n) = r?;
            if let Ok(k) = <[u8; 32]>::try_from(k.as_slice()) {
                out.push((k, n));
            }
        }
        Ok(out)
    }

    /// Registo de auditoria: os eventos de gestão e moderação, do mais recente para o mais
    /// antigo (no máximo `limite`).
    pub fn audit_log(&self, group: GroupId, limite: usize) -> Result<Vec<(u64, SignedEvent)>> {
        let mut out = Vec::new();
        for (seq, ev) in self.events_since(group, 0)?.into_iter().rev() {
            if e_gestao(&ev.body.kind) {
                out.push((seq, ev));
                if out.len() >= limite {
                    break;
                }
            }
        }
        Ok(out)
    }
}

impl Store {
    /// Membro: guarda um evento que outro membro (ou eu) deixou na caixa enquanto o
    /// anfitrião está desligado, para se ver já. Só mensagens, edições, apagar e reações
    /// de membros, e que eu saiba sem castigo. Devolve `false` se já o tinha (ou se não serve).
    pub fn add_pending(&self, group: GroupId, ev: &SignedEvent) -> Result<bool> {
        ev.verify()?;
        ensure!(ev.body.group == Some(group), "evento de outro grupo");
        let conversa = matches!(
            ev.body.kind,
            EventKind::Message { .. } | EventKind::Post { .. } | EventKind::Edit { .. } | EventKind::Delete { .. } | EventKind::React { .. }
        );
        if !conversa || !self.is_member(group, &ev.author()?)? || self.castigo(group, &ev.body.author)? > ev.body.timestamp_ms {
            return Ok(false);
        }
        let id = ev.id();
        if self.event_by_id(group, &id)?.is_some() {
            return Ok(false);
        }
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO pending_events (group_id, id, ts, data) VALUES (?1, ?2, ?3, ?4)",
            params![group.0, id, ev.body.timestamp_ms as i64, postcard::to_stdvec(ev)?],
        )?;
        Ok(n > 0)
    }

    /// Eventos à espera do anfitrião, por ordem de escrita.
    pub fn pending(&self, group: GroupId) -> Result<Vec<SignedEvent>> {
        // os muito antigos (mais de 30 dias) já não vão ser aceites
        let limite = crate::event::now_ms().saturating_sub(30 * 24 * 3600 * 1000) as i64;
        self.conn.execute("DELETE FROM pending_events WHERE group_id = ?1 AND ts < ?2", params![group.0, limite])?;
        let mut stmt = self.conn.prepare("SELECT data FROM pending_events WHERE group_id = ?1 ORDER BY ts")?;
        let rows = stmt.query_map(params![group.0], |r| r.get::<_, Vec<u8>>(0))?;
        rows.map(|r| Ok(postcard::from_bytes(&r?)?)).collect()
    }
}


// ---------------- grupo na caixa do servidor ----------------

impl Store {
    /// Aplica um evento lido da caixa do grupo, com as mesmas regras que o anfitrião usava.
    /// Todos os dispositivos leem a caixa pela mesma ordem e com a mesma hora (`em`, a do
    /// servidor), por isso chegam todos ao mesmo estado sem precisar do PC do dono.
    ///
    /// `convite`: a chave pública do convite com que alguém entra (prova já confirmada).
    /// Devolve o número local do evento, ou `None` se já o tinha ou não muda nada.
    pub fn aplicar_caixa(&mut self, ev: &SignedEvent, em: u64, convite: Option<[u8; 32]>) -> Result<Option<u64>> {
        ev.verify()?;
        let group = ev.group_id();
        let id = ev.id();
        if self.event_by_id(group, &id)?.is_some() {
            self.conn.execute("DELETE FROM pending_events WHERE group_id = ?1 AND id = ?2", params![group.0, id])?;
            return Ok(None);
        }
        if let EventKind::GroupCreated { .. } = &ev.body.kind {
            return self.criar_da_caixa(ev);
        }
        self.automod = None;
        self.relogio = Some(em);
        let r = (|| {
            let by_invite = match (&ev.body.kind, convite) {
                (EventKind::MemberJoined { .. }, Some(h)) => self.convite_aberto(group, &h, em)?,
                _ => false,
            };
            let r = self.validar_e_guardar(ev, by_invite)?;
            if let (Some(_), true, Some(h)) = (r, by_invite, convite) {
                self.conn.execute(
                    "UPDATE group_invites SET uses = uses + 1 WHERE group_id = ?1 AND hash = ?2",
                    params![group.0, h],
                )?;
            }
            Ok(r)
        })();
        self.relogio = None;
        // AutoMod: o castigo aplica-se igual em todos os dispositivos.
        if let Some(c) = &self.automod {
            if c.bloqueio.timeout_secs > 0 {
                let ate = em + c.bloqueio.timeout_secs as u64 * 1000;
                self.conn.execute(
                    "INSERT OR REPLACE INTO timeouts (group_id, pubkey, until) VALUES (?1, ?2, ?3)",
                    params![group.0, c.autor, ate as i64],
                )?;
            }
        }
        self.conn.execute("DELETE FROM pending_events WHERE group_id = ?1 AND id = ?2", params![group.0, id])?;
        if let Ok(Some(_)) = &r {
            if matches!(ev.body.kind, EventKind::Kick { .. } | EventKind::Ban { .. }) {
                // quem saiu ainda tem a chave: falta rodá-la. Depois de banir, os convites
                // abertos não recebem a chave nova (quem foi banido pode ter um).
                let banido = matches!(ev.body.kind, EventKind::Ban { .. });
                let antes = self.rodar_pendente(group)?.is_some_and(|r| r.2);
                self.set_estado(group, "rodar", &(em, ev.body.author, banido || antes))?;
            }
        }
        r
    }

    /// Registo antigo (assinado pelo anfitrião, de antes de o grupo viver na caixa): entra
    /// sem voltar a validar, se ainda não o tivermos.
    pub fn inserir_registo(&mut self, ev: &SignedEvent) -> Result<Option<u64>> {
        ev.verify()?;
        let group = ev.group_id();
        let id = ev.id();
        if self.event_by_id(group, &id)?.is_some() {
            return Ok(None);
        }
        if let EventKind::GroupCreated { .. } = &ev.body.kind {
            return self.criar_da_caixa(ev);
        }
        ensure!(self.last_seq(group)? > 0, "grupo desconhecido");
        let tx = self.conn.transaction()?;
        let seq = last_seq(&tx, group)? + 1;
        insert_event(&tx, group, seq, ev)?;
        apply(&tx, group, ev)?;
        tx.execute("DELETE FROM pending_events WHERE group_id = ?1 AND id = ?2", params![group.0, id])?;
        tx.commit()?;
        Ok(Some(seq))
    }

    /// O `GroupCreated` lido da caixa: o grupo passa a existir aqui.
    fn criar_da_caixa(&mut self, ev: &SignedEvent) -> Result<Option<u64>> {
        let EventKind::GroupCreated { name, .. } = &ev.body.kind else { bail!("não é GroupCreated") };
        ensure!(ev.body.group.is_none(), "GroupCreated não pode indicar grupo");
        check_name(name)?;
        let group = ev.group_id();
        let tx = self.conn.transaction()?;
        if last_seq(&tx, group)? > 0 {
            return Ok(None);
        }
        tx.execute(
            "INSERT OR IGNORE INTO groups (id, name, owner, hosted) VALUES (?1, ?2, ?3, 0)",
            params![group.0, name, ev.body.author],
        )?;
        let dono: Vec<u8> = tx.query_row("SELECT owner FROM groups WHERE id = ?1", params![group.0], |r| r.get(0))?;
        ensure!(dono == ev.body.author, "o grupo tem outro dono");
        insert_event(&tx, group, 1, ev)?;
        apply(&tx, group, ev)?;
        tx.commit()?;
        Ok(Some(1))
    }

    /// Convite publicado, não revogado, não expirado e com usos à hora `em`.
    fn convite_aberto(&self, group: GroupId, hash: &[u8; 32], em: u64) -> Result<bool> {
        let c: Option<(i64, i64, i64, i64)> = self
            .conn
            .query_row(
                "SELECT max_uses, expires, uses, revoked FROM group_invites WHERE group_id = ?1 AND hash = ?2",
                params![group.0, hash],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        Ok(c.is_some_and(|(max, exp, uses, rev)| rev == 0 && (exp == 0 || exp as u64 > em) && (max == 0 || uses < max)))
    }

    /// Chaves públicas dos convites ainda abertos (para lhes dar a chave nova do grupo).
    pub fn convites_abertos(&self, group: GroupId) -> Result<Vec<[u8; 32]>> {
        Ok(self.group_invites(group)?.into_iter().map(|c| c.hash).collect())
    }

    fn set_estado<T: serde::Serialize>(&self, group: GroupId, k: &str, v: &T) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO group_state (group_id, k, data) VALUES (?1, ?2, ?3)",
            params![group.0, k, postcard::to_stdvec(v)?],
        )?;
        Ok(())
    }

    /// Falta rodar a chave do grupo (alguém foi expulso ou banido):
    /// `(quando, quem expulsou, houve banimento)`.
    pub fn rodar_pendente(&self, group: GroupId) -> Result<Option<(u64, [u8; 32], bool)>> {
        Ok(self.estado::<(u64, [u8; 32], bool)>(group, "rodar").ok().flatten())
    }

    /// Uma chave nova lida da caixa vale? Só depois de expulsar alguém, e de quem pode expulsar.
    pub fn chave_nova_valida(&self, group: GroupId, autor: &[u8; 32], em: u64) -> Result<bool> {
        if self.rodar_pendente(group)?.is_none() {
            return Ok(false);
        }
        let Some(info) = self.group(group)? else { return Ok(false) };
        if info.owner.as_bytes() == autor {
            return Ok(true);
        }
        if !self.is_member(group, &pubkey(autor)?)? {
            return Ok(false);
        }
        let _ = em;
        Ok(self.perms(group, autor, None)? & (cargos::EXPULSAR | cargos::BANIR) != 0)
    }

    /// A chave mudou: fecha a época `epoch` (a leitura passa à seguinte).
    pub fn fechar_epoca(&self, group: GroupId, epoch: u32) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO caixa_epocas (group_id, epoch) VALUES (?1, ?2)",
            params![group.0, epoch],
        )?;
        self.conn.execute("DELETE FROM group_state WHERE group_id = ?1 AND k = 'rodar'", params![group.0])?;
        Ok(())
    }

    /// Todas as chaves do grupo, da mais antiga para a mais nova: `(época, chave, lido, fechada)`.
    pub fn epocas(&self, group: GroupId) -> Result<Vec<(u32, ChaveCaixa, u64, bool)>> {
        let mut stmt = self.conn.prepare(
            "SELECT k.epoch, k.key, k.read, EXISTS(SELECT 1 FROM caixa_epocas e WHERE e.group_id = k.group_id AND e.epoch = k.epoch)
             FROM group_keys k WHERE k.group_id = ?1 ORDER BY k.epoch",
        )?;
        let rows = stmt.query_map(params![group.0], |r| {
            Ok((r.get::<_, u32>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, i64>(2)? as u64, r.get::<_, bool>(3)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (e, k, rd, f) = r?;
            let k: [u8; 32] = k.try_into().map_err(|_| anyhow!("chave do grupo inválida"))?;
            out.push((e, ChaveCaixa::from_bytes(k), rd, f));
        }
        Ok(out)
    }

    /// Até onde já se leu a caixa de uma época.
    pub fn set_lido(&self, group: GroupId, epoch: u32, read: u64) -> Result<()> {
        self.conn.execute(
            "UPDATE group_keys SET read = ?3 WHERE group_id = ?1 AND epoch = ?2",
            params![group.0, epoch, read as i64],
        )?;
        Ok(())
    }

    /// Passagem para o grupo na caixa (uma vez por grupo): volta a ler as caixas desde o
    /// início, para aplicar o que ficou à espera do anfitrião. Devolve `true` se ainda não
    /// tinha sido feita.
    pub fn migrar_para_caixa(&self, group: GroupId) -> Result<bool> {
        if self.estado::<bool>(group, "na_caixa")?.is_some() {
            return Ok(false);
        }
        self.conn.execute("UPDATE group_keys SET read = 0 WHERE group_id = ?1", params![group.0])?;
        Ok(true)
    }

    /// O grupo já passou para a caixa.
    pub fn migracao_feita_em(&self, group: GroupId) -> Result<bool> {
        Ok(self.estado::<bool>(group, "na_caixa")?.is_some())
    }

    pub fn migracao_feita(&self, group: GroupId) -> Result<()> {
        self.set_estado(group, "na_caixa", &true)
    }

    /// Os meus eventos que ainda não voltaram da caixa (para os voltar a enviar).
    pub fn meus_pendentes(&self, group: GroupId, eu: &[u8; 32]) -> Result<Vec<SignedEvent>> {
        Ok(self.pending(group)?.into_iter().filter(|e| &e.body.author == eu).collect())
    }
}

/// Eventos que entram no registo de auditoria.
pub fn e_gestao(k: &EventKind) -> bool {
    matches!(
        k,
        EventKind::Layout { .. }
            | EventKind::Roles { .. }
            | EventKind::MemberRoles { .. }
            | EventKind::ChannelPerms { .. }
            | EventKind::Kick { .. }
            | EventKind::Ban { .. }
            | EventKind::Unban { .. }
            | EventKind::Timeout { .. }
            | EventKind::Nick { .. }
            | EventKind::Pin { .. }
            | EventKind::Settings(_)
            | EventKind::VoiceMod { .. }
            | EventKind::InviteCreated { .. }
            | EventKind::InviteRevoked { .. }
            | EventKind::Report { .. }
            | EventKind::MemberJoined { .. }
            | EventKind::Leave
    )
}

/// Canal de uma mensagem.
pub fn canal_de(k: &EventKind) -> Option<String> {
    match k {
        EventKind::Message { channel, .. } | EventKind::Post { channel, .. } => Some(channel.clone()),
        _ => None,
    }
}

fn check_roles(roles: &[Cargo]) -> Result<()> {
    ensure!(!roles.is_empty() && roles.len() <= MAX_ROLES, "número de cargos inválido");
    ensure!(roles.last().is_some_and(|r| r.id == cargos::TODOS), "o @todos tem de ser o último cargo");
    let mut ids = std::collections::HashSet::new();
    for r in roles {
        ensure!(!r.id.is_empty() && r.id.len() <= 64, "id de cargo inválido");
        ensure!(ids.insert(r.id.as_str()), "cargo repetido");
        check_name(&r.name)?;
        ensure!(r.color.len() <= 16, "cor inválida");
        ensure!(r.perms & !cargos::TODAS == 0, "permissão desconhecida");
        if let Some(i) = &r.icon {
            ensure!(
                (i.starts_with("data:image/") && i.len() <= MAX_ICON_CHARS) || i.chars().count() <= 16,
                "ícone do cargo inválido"
            );
        }
    }
    Ok(())
}

fn check_settings(d: &Definicoes) -> Result<()> {
    check_name(&d.name)?;
    ensure!(d.description.chars().count() <= 1024, "descrição demasiado longa");
    ensure!(d.rules.chars().count() <= 4000, "regras demasiado longas");
    ensure!(d.tag.chars().count() <= 6, "a etiqueta tem no máximo 6 carateres");
    ensure!(d.color.len() <= 16, "cor inválida");
    for img in [&d.icon, &d.banner].into_iter().flatten() {
        ensure!(
            (img.starts_with("data:image/") && img.len() <= MAX_PHOTO_CHARS) || (img.starts_with('#') && img.len() <= 16),
            "imagem inválida"
        );
    }
    ensure!(d.automod.len() <= 50, "demasiadas regras no AutoMod");
    for r in &d.automod {
        check_name(&r.name)?;
        ensure!(r.timeout_secs as u64 * 1000 <= MAX_TIMEOUT_MS, "castigo demasiado longo");
        if let cargos::TipoRegra::Palavras(l) = &r.tipo {
            ensure!(l.len() <= 1000, "demasiadas palavras");
        }
    }
    ensure!(d.extra.len() <= 100, "demasiadas definições");
    Ok(())
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
        EventKind::Message { .. }
        | EventKind::Post { .. }
        | EventKind::Edit { .. }
        | EventKind::Delete { .. }
        | EventKind::React { .. }
        | EventKind::Report { .. } => {}
        EventKind::Roles { roles } => {
            conn.execute(
                "INSERT OR REPLACE INTO group_state (group_id, k, data) VALUES (?1, 'roles', ?2)",
                params![group.0, postcard::to_stdvec(roles)?],
            )?;
        }
        EventKind::Settings(d) => {
            conn.execute(
                "INSERT OR REPLACE INTO group_state (group_id, k, data) VALUES (?1, 'settings', ?2)",
                params![group.0, postcard::to_stdvec(d)?],
            )?;
            conn.execute("UPDATE groups SET name = ?2 WHERE id = ?1", params![group.0, d.name.trim()])?;
        }
        EventKind::MemberRoles { member, roles } => {
            conn.execute(
                "INSERT OR REPLACE INTO member_roles (group_id, pubkey, data) VALUES (?1, ?2, ?3)",
                params![group.0, member, postcard::to_stdvec(roles)?],
            )?;
        }
        EventKind::ChannelPerms { channel, overrides } => {
            conn.execute(
                "INSERT OR REPLACE INTO channel_perms (group_id, channel, data) VALUES (?1, ?2, ?3)",
                params![group.0, channel, postcard::to_stdvec(overrides)?],
            )?;
        }
        EventKind::Kick { member, .. } => tirar_membro(conn, group, member)?,
        EventKind::Leave => tirar_membro(conn, group, &ev.body.author)?,
        EventKind::Ban { member, reason, .. } => {
            let nome: Option<String> = conn
                .query_row(
                    "SELECT display_name FROM members WHERE group_id = ?1 AND pubkey = ?2
                     UNION ALL SELECT display_name FROM ex_members WHERE group_id = ?1 AND pubkey = ?2 LIMIT 1",
                    params![group.0, member],
                    |r| r.get(0),
                )
                .optional()?;
            tirar_membro(conn, group, member)?;
            conn.execute(
                "INSERT OR REPLACE INTO bans (group_id, pubkey, name, reason, by, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![group.0, member, nome.unwrap_or_default(), reason, ev.body.author, ev.body.timestamp_ms as i64],
            )?;
        }
        EventKind::Unban { member } => {
            conn.execute("DELETE FROM bans WHERE group_id = ?1 AND pubkey = ?2", params![group.0, member])?;
        }
        EventKind::Timeout { member, until_ms, .. } => {
            conn.execute(
                "INSERT OR REPLACE INTO timeouts (group_id, pubkey, until) VALUES (?1, ?2, ?3)",
                params![group.0, member, *until_ms as i64],
            )?;
        }
        EventKind::Nick { member, nick } => match nick {
            Some(n) => {
                conn.execute(
                    "INSERT OR REPLACE INTO nicks (group_id, pubkey, nick) VALUES (?1, ?2, ?3)",
                    params![group.0, member, n.trim()],
                )?;
            }
            None => {
                conn.execute("DELETE FROM nicks WHERE group_id = ?1 AND pubkey = ?2", params![group.0, member])?;
            }
        },
        EventKind::Pin { target, on } => {
            if *on {
                conn.execute(
                    "INSERT OR REPLACE INTO pins (group_id, target, by, ts) VALUES (?1, ?2, ?3, ?4)",
                    params![group.0, target, ev.body.author, ev.body.timestamp_ms as i64],
                )?;
            } else {
                conn.execute("DELETE FROM pins WHERE group_id = ?1 AND target = ?2", params![group.0, target])?;
            }
        }
        EventKind::VoiceMod { member, action } => {
            let (m, d): (bool, bool) = conn
                .query_row(
                    "SELECT muted, deaf FROM voice_mod WHERE group_id = ?1 AND pubkey = ?2",
                    params![group.0, member],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?
                .unwrap_or((false, false));
            let (m, d) = match action {
                AcaoVoz::Silenciar(on) => (*on, d),
                AcaoVoz::Ensurdecer(on) => (m, *on),
                _ => (m, d),
            };
            conn.execute(
                "INSERT OR REPLACE INTO voice_mod (group_id, pubkey, muted, deaf) VALUES (?1, ?2, ?3, ?4)",
                params![group.0, member, m, d],
            )?;
        }
        EventKind::InviteCreated { hash, max_uses, expires_ms } => {
            conn.execute(
                "INSERT OR IGNORE INTO group_invites (group_id, hash, by, max_uses, expires, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![group.0, hash, ev.body.author, *max_uses as i64, *expires_ms as i64, ev.body.timestamp_ms as i64],
            )?;
        }
        EventKind::InviteRevoked { hash } => {
            conn.execute(
                "UPDATE group_invites SET revoked = 1 WHERE group_id = ?1 AND hash = ?2",
                params![group.0, hash],
            )?;
        }
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

/// Tira alguém da lista de membros (fica o nome, para as mensagens antigas).
fn tirar_membro(conn: &Connection, group: GroupId, who: &[u8; 32]) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO ex_members (group_id, pubkey, display_name)
         SELECT group_id, pubkey, display_name FROM members WHERE group_id = ?1 AND pubkey = ?2",
        params![group.0, who],
    )?;
    for t in ["members", "member_roles", "nicks", "voice_mod"] {
        conn.execute(&format!("DELETE FROM {t} WHERE group_id = ?1 AND pubkey = ?2"), params![group.0, who])?;
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

    #[test]
    fn cargos_e_moderacao() {
        use crate::cargos::{self, Cargo, Definicoes, Regra, TipoRegra};
        let mut s = Store::open_in_memory().unwrap();
        let dono = Identity::generate();
        let g = new_group(&mut s, &dono);
        let (mo, ze, rui) = (Identity::generate(), Identity::generate(), Identity::generate());
        let ev = |id: &Identity, k: EventKind| SignedEvent::sign(id, Some(g), k);
        for (id, n) in [(&mo, "mo"), (&ze, "ze"), (&rui, "rui")] {
            s.append_as_host(&ev(id, EventKind::MemberJoined { display_name: n.into() }), true).unwrap();
        }
        let k = |id: &Identity| *id.public().as_bytes();
        let cargo = |id: &str, perms| Cargo { id: id.into(), name: id.into(), color: "#ff0000".into(), icon: None, hoist: true, mentionable: false, perms };

        // só o dono (ou quem gere cargos) mexe nos cargos
        let roles = vec![cargo("mod", cargos::EXPULSAR | cargos::CASTIGAR | cargos::GERIR_CARGOS), cargo("vip", 0), Cargo::todos()];
        assert!(s.append_as_host(&ev(&ze, EventKind::Roles { roles: roles.clone() }), false).is_err());
        s.append_as_host(&ev(&dono, EventKind::Roles { roles: roles.clone() }), false).unwrap();
        s.append_as_host(&ev(&dono, EventKind::MemberRoles { member: k(&mo), roles: vec!["mod".into()] }), false).unwrap();
        assert!(s.perms(g, &k(&mo), None).unwrap() & cargos::EXPULSAR != 0);

        // o mod dá "vip" (abaixo dele), mas não "mod" (o dele)
        s.append_as_host(&ev(&mo, EventKind::MemberRoles { member: k(&ze), roles: vec!["vip".into()] }), false).unwrap();
        assert!(s.append_as_host(&ev(&mo, EventKind::MemberRoles { member: k(&ze), roles: vec!["mod".into()] }), false).is_err());
        // não dá permissões que não tem
        let mut r2 = roles.clone();
        r2[1].perms = cargos::BANIR;
        assert!(s.append_as_host(&ev(&mo, EventKind::Roles { roles: r2 }), false).is_err());

        // castigo: o zé deixa de escrever
        let msg = |id: &Identity, t: &str| ev(id, EventKind::Message { channel: "geral".into(), text: t.into() });
        s.append_as_host(&msg(&ze, "olá"), false).unwrap();
        s.append_as_host(&ev(&mo, EventKind::Timeout { member: k(&ze), until_ms: crate::event::now_ms() + 60_000, reason: String::new() }), false).unwrap();
        assert!(s.append_as_host(&msg(&ze, "outra"), false).is_err());
        // ninguém castiga o dono, e o zé não expulsa o mod
        assert!(s.append_as_host(&ev(&mo, EventKind::Kick { member: k(&dono), reason: String::new() }), false).is_err());
        assert!(s.append_as_host(&ev(&ze, EventKind::Kick { member: k(&mo), reason: String::new() }), false).is_err());

        // expulsar e banir
        s.append_as_host(&ev(&mo, EventKind::Kick { member: k(&rui), reason: "chato".into() }), false).unwrap();
        assert!(!s.is_member(g, &rui.public()).unwrap());
        assert!(s.ex_members(g).unwrap().iter().any(|(x, n)| *x == k(&rui) && n == "rui"));
        assert!(s.append_as_host(&ev(&mo, EventKind::Ban { member: k(&ze), reason: String::new(), delete_secs: 0 }), false).is_err());
        s.append_as_host(&ev(&dono, EventKind::Ban { member: k(&ze), reason: "spam".into(), delete_secs: 0 }), false).unwrap();
        // banido não volta, mesmo com convite
        assert!(s.append_as_host(&ev(&ze, EventKind::MemberJoined { display_name: "ze".into() }), true).is_err());
        s.append_as_host(&ev(&dono, EventKind::Unban { member: k(&ze) }), false).unwrap();
        s.append_as_host(&ev(&ze, EventKind::MemberJoined { display_name: "ze".into() }), true).unwrap();

        // canal só de leitura
        s.append_as_host(
            &ev(&dono, EventKind::ChannelPerms {
                channel: "geral".into(),
                overrides: vec![cargos::Excecao { alvo: cargos::Alvo::Cargo(cargos::TODOS.into()), allow: 0, deny: cargos::ENVIAR_MENSAGENS }],
            }),
            false,
        )
        .unwrap();
        s.append_as_host(&ev(&mo, EventKind::Timeout { member: k(&ze), until_ms: 0, reason: String::new() }), false).unwrap();
        assert!(s.append_as_host(&msg(&ze, "posso?"), false).is_err());
        s.append_as_host(&msg(&dono, "eu posso"), false).unwrap();

        // AutoMod
        s.append_as_host(&ev(&dono, EventKind::ChannelPerms { channel: "geral".into(), overrides: vec![] }), false).unwrap();
        let mut d = s.settings(g).unwrap();
        d.automod = vec![Regra {
            id: "p".into(), name: "palavrões".into(), on: true, tipo: TipoRegra::Palavras(vec!["burro".into()]),
            block: true, alert_channel: Some("geral".into()), timeout_secs: 60, exempt_roles: vec![], exempt_channels: vec![],
        }];
        d.name = "Malta nova".into();
        s.append_as_host(&ev(&dono, EventKind::Settings(d)), false).unwrap();
        assert_eq!(s.group(g).unwrap().unwrap().name, "Malta nova");
        assert!(s.append_as_host(&msg(&ze, "és burro"), false).is_err());
        assert!(s.take_automod().is_some_and(|c| c.bloqueio.timeout_secs == 60));
        s.append_as_host(&msg(&ze, "és fixe"), false).unwrap();
        let _ = Definicoes::default();
        // registo de auditoria
        assert!(s.audit_log(g, 100).unwrap().iter().any(|(_, e)| matches!(e.body.kind, EventKind::Ban { .. })));
    }
}
