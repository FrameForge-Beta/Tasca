//! Cargos, permissões, moderação e definições do grupo (ver `tasca_core::cargos`).
//!
//! Tudo o que aqui se faz é um evento assinado por quem o faz. Cada dispositivo do grupo
//! só o aceita se a pessoa tiver a permissão (ver `tasca_core::store`), por isso funciona
//! sem o PC do dono ligado.

use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tasca_core::{
    EventKind, GroupId,
    cargos::{self, AcaoVoz, Alvo, Cargo, Definicoes, Excecao},
    event::now_ms,
};
use tauri::{AppHandle, State};

use crate::contas::Contas;
use crate::grupos::{Ativa, Rede, ativa, canal, hex, txt, unhex32};

type Res<T> = Result<T, String>;

fn grupo(id: &str) -> Res<GroupId> {
    unhex32(id).map(GroupId).ok_or_else(|| "Grupo desconhecido.".into())
}

/// Id de pessoa da interface (`me` ou hex) → chave.
fn pessoa(a: &Ativa, id: &str) -> Res<[u8; 32]> {
    if id == "me" {
        return Ok(a.conta);
    }
    unhex32(id).ok_or_else(|| "Pessoa desconhecida.".into())
}

/// Erros do núcleo em frases para a interface.
fn erro(e: anyhow::Error) -> String {
    txt(e)
}

/// Publica e espera um pouco pela resposta do anfitrião (para a interface ver logo o
/// resultado quando ele está ligado).
async fn publicar<F, Fut>(a: &Ativa, g: GroupId, f: F) -> Res<bool>
where
    F: FnOnce(tasca_core::GroupHandle) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<[u8; 32]>>,
{
    let h = a.handle(g)?;
    let ligado = h.is_connected() || h.is_hosted() || a.tem_servidor();
    if !ligado {
        return Err("O anfitrião do grupo está desligado. Tenta quando ele voltar.".into());
    }
    let id = tokio::time::timeout(Duration::from_secs(20), f(h)).await.map_err(|_| "Demorou demasiado.".to_string())?.map_err(erro)?;
    if ligado {
        // espera que o evento volte do grupo (aceite) até 5 s
        for _ in 0..50 {
            if a.node.with_store(|s| s.event_by_id(g, &id)).ok().flatten().is_some() {
                return Ok(true);
            }
            if let Some(motivo) = a.node.motivo_recusa(&id) {
                let mut c = motivo.chars();
                return Err(match c.next() {
                    Some(p) => p.to_uppercase().chain(c).collect::<String>() + ".",
                    None => "Não foi possível.".into(),
                });
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    Ok(ligado)
}

fn cargo_json(c: &Cargo, membros: usize) -> Value {
    json!({
        "id": c.id, "name": c.name, "color": c.color, "icon": c.icon, "hoist": c.hoist,
        "mentionable": c.mentionable, "perms": c.perms, "members": membros,
    })
}

#[derive(Deserialize)]
pub struct CargoUi {
    id: Option<String>,
    name: String,
    #[serde(default)]
    color: String,
    icon: Option<String>,
    #[serde(default)]
    hoist: bool,
    #[serde(default)]
    mentionable: bool,
    #[serde(default)]
    perms: u64,
}

fn novo_id() -> String {
    let mut b = [0u8; 6];
    let _ = getrandom::fill(&mut b);
    hex(&b)
}

/// As minhas permissões num grupo e em cada canal.
#[tauri::command]
pub async fn group_perms(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let eu = a.conta;
    let (geral, canais, rank, castigo, dono) = a.node.with_store(|s| -> anyhow::Result<_> {
        let geral = s.perms(g, &eu, None)?;
        let mut canais = serde_json::Map::new();
        for ch in s.layout(g)?.0 {
            canais.insert(format!("{}/{}", hex(&g.0), ch.id), json!(s.perms(g, &eu, Some(&ch.id))?));
        }
        let dono = s.group(g)?.is_some_and(|i| *i.owner.as_bytes() == eu);
        Ok((geral, canais, s.rank(g, &eu)?, s.castigo(g, &eu)?, dono))
    }).map_err(txt)?;
    Ok(json!({ "perms": geral, "channels": canais, "rank": rank, "timeoutUntil": if castigo > now_ms() { json!(castigo) } else { Value::Null }, "owner": dono }))
}

/// Cargos do grupo e a lista de permissões (para a página de cargos).
#[tauri::command]
pub async fn list_roles(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let (roles, dos) = a.node.with_store(|s| -> anyhow::Result<_> { Ok((s.roles(g)?, s.all_member_roles(g)?)) }).map_err(txt)?;
    let membros = a.node.with_store(|s| s.members(g)).map_err(txt)?;
    let lista: Vec<Value> = roles
        .iter()
        .map(|r| {
            let n = if r.id == cargos::TODOS {
                membros.len()
            } else {
                membros.iter().filter(|(k, _)| dos.get(k.as_bytes()).is_some_and(|l| l.contains(&r.id))).count()
            };
            cargo_json(r, n)
        })
        .collect();
    let catalogo: Vec<Value> = cargos::LISTA
        .iter()
        .map(|(cat, bit, nome, desc)| json!({ "cat": cat, "bit": bit, "name": nome, "desc": desc }))
        .collect();
    Ok(json!({ "roles": lista, "catalog": catalogo, "all": cargos::TODAS, "base": cargos::BASE }))
}

/// Guarda a lista completa de cargos (por ordem; o `@todos` fica sempre no fim).
#[tauri::command]
pub async fn save_roles(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String, roles: Vec<CargoUi>) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let mut lista: Vec<Cargo> = Vec::new();
    let mut todos = None;
    for r in roles {
        let id = r.id.filter(|i| !i.is_empty()).unwrap_or_else(novo_id);
        let nome: String = r.name.trim().chars().take(64).collect();
        let cargo = Cargo {
            name: if id == cargos::TODOS { "@todos".into() } else if nome.is_empty() { "cargo novo".into() } else { nome },
            id,
            color: r.color.chars().take(16).collect(),
            icon: r.icon.filter(|i| !i.is_empty()),
            hoist: r.hoist,
            mentionable: r.mentionable,
            perms: r.perms & cargos::TODAS,
        };
        if cargo.id == cargos::TODOS {
            todos = Some(cargo);
        } else {
            lista.push(cargo);
        }
    }
    lista.push(todos.unwrap_or_else(Cargo::todos));
    publicar(&a, g, |h| async move { h.set_roles(lista).await }).await
}

#[tauri::command]
pub async fn set_member_roles(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    user_id: String,
    roles: Vec<String>,
) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let k = pessoa(&a, &user_id)?;
    publicar(&a, g, |h| async move { h.set_member_roles(k, roles).await }).await
}

fn alvo_json(a: &Ativa, x: &Alvo) -> String {
    match x {
        Alvo::Cargo(id) => format!("role:{id}"),
        Alvo::Membro(k) => format!("user:{}", a.pessoa_id(k)),
    }
}

/// Exceções de um canal (`<grupo>/<id>`) ou categoria (`cat:<nome>`).
#[tauri::command]
pub async fn get_channel_perms(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String, channel: String) -> Res<Vec<Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let ch = chave_canal(&channel);
    let l = a.node.with_store(|s| s.channel_perms(g, &ch)).map_err(txt)?;
    Ok(l.iter().map(|e| json!({ "target": alvo_json(&a, &e.alvo), "allow": e.allow, "deny": e.deny })).collect())
}

fn chave_canal(ch: &str) -> String {
    if ch.starts_with("cat:") {
        ch.to_string()
    } else {
        ch.rsplit('/').next().unwrap_or(ch).to_string()
    }
}

#[derive(Deserialize)]
pub struct ExcecaoUi {
    target: String,
    #[serde(default)]
    allow: u64,
    #[serde(default)]
    deny: u64,
}

#[tauri::command]
pub async fn set_channel_perms(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    channel: String,
    overrides: Vec<ExcecaoUi>,
) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let ch = chave_canal(&channel);
    let mut lista = Vec::new();
    for o in overrides {
        let alvo = if let Some(r) = o.target.strip_prefix("role:") {
            Alvo::Cargo(r.to_string())
        } else if let Some(u) = o.target.strip_prefix("user:") {
            Alvo::Membro(pessoa(&a, u)?)
        } else {
            return Err("Exceção inválida.".into());
        };
        let (allow, deny) = (o.allow & cargos::TODAS, o.deny & cargos::TODAS & !o.allow);
        if allow | deny != 0 {
            lista.push(Excecao { alvo, allow, deny });
        }
    }
    publicar(&a, g, |h| async move { h.set_channel_perms(ch, lista).await }).await
}

#[tauri::command]
pub async fn kick_member(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String, user_id: String, reason: Option<String>) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let k = pessoa(&a, &user_id)?;
    let r = reason.unwrap_or_default().trim().chars().take(500).collect();
    publicar(&a, g, |h| async move { h.kick(k, r).await }).await
}

#[tauri::command]
pub async fn ban_member(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    user_id: String,
    reason: Option<String>,
    delete_secs: Option<u64>,
) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let k = pessoa(&a, &user_id)?;
    let r = reason.unwrap_or_default().trim().chars().take(500).collect();
    let d = delete_secs.unwrap_or(0).min(7 * 24 * 3600);
    publicar(&a, g, |h| async move { h.ban(k, r, d).await }).await
}

#[tauri::command]
pub async fn unban_member(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String, user_id: String) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let k = pessoa(&a, &user_id)?;
    publicar(&a, g, |h| async move { h.unban(k).await }).await
}

/// Castigo de `secs` segundos (0 tira o castigo).
#[tauri::command]
pub async fn timeout_member(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    user_id: String,
    secs: u64,
    reason: Option<String>,
) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let k = pessoa(&a, &user_id)?;
    let ate = if secs == 0 { 0 } else { now_ms() + secs.min(28 * 24 * 3600) * 1000 };
    let r = reason.unwrap_or_default().trim().chars().take(500).collect();
    publicar(&a, g, |h| async move { h.timeout(k, ate, r).await }).await
}

#[tauri::command]
pub async fn set_nick(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String, user_id: String, nick: Option<String>) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let k = pessoa(&a, &user_id)?;
    let n = nick.map(|n| n.trim().chars().take(32).collect::<String>()).filter(|n| !n.is_empty());
    publicar(&a, g, |h| async move { h.set_nick(k, n).await }).await
}

#[tauri::command]
pub async fn pin_message(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, channel_id: String, id: String, on: bool) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let (g, _) = canal(&channel_id)?;
    let alvo = unhex32(&id).ok_or("Mensagem desconhecida.")?;
    publicar(&a, g, |h| async move { h.pin(alvo, on).await }).await
}

/// Mensagens fixadas de um canal (a mais recente primeiro).
#[tauri::command]
pub async fn list_pins(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, channel_id: String) -> Res<Vec<Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let (g, ch) = canal(&channel_id)?;
    let pins = a.node.with_store(|s| s.pins(g)).map_err(txt)?;
    let mut out = Vec::new();
    for p in pins {
        if let Some((c2, Some(m))) = crate::mensagens::uma(&a, g, &p) {
            if c2 == ch {
                out.push(m);
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn report_message(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, channel_id: String, id: String, reason: String) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let (g, _) = canal(&channel_id)?;
    let alvo = unhex32(&id).ok_or("Mensagem desconhecida.")?;
    let r: String = reason.trim().chars().take(500).collect();
    publicar(&a, g, |h| async move { h.report(alvo, r).await }).await
}

#[tauri::command]
pub async fn get_group_settings(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let d = a.node.with_store(|s| s.settings(g)).map_err(txt)?;
    serde_json::to_value(d).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_group_settings(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String, settings: Value) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let d: Definicoes = serde_json::from_value(settings).map_err(|e| format!("Definições inválidas: {e}"))?;
    if d.name.trim().is_empty() {
        return Err("Dá um nome ao grupo.".into());
    }
    publicar(&a, g, |h| async move { h.set_settings(d).await }).await
}

/// Moderação numa chamada: `action` = mute, unmute, deafen, undeafen, disconnect, move, stopvideo.
#[tauri::command]
pub async fn voice_mod(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    user_id: String,
    action: String,
    room: Option<String>,
) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let k = pessoa(&a, &user_id)?;
    let acao = match action.as_str() {
        "mute" => AcaoVoz::Silenciar(true),
        "unmute" => AcaoVoz::Silenciar(false),
        "deafen" => AcaoVoz::Ensurdecer(true),
        "undeafen" => AcaoVoz::Ensurdecer(false),
        "disconnect" => AcaoVoz::Desligar,
        "stopvideo" => AcaoVoz::PararVideo,
        "move" => AcaoVoz::Mover(chave_canal(&room.ok_or("Escolhe a sala.")?)),
        _ => return Err("Ação desconhecida.".into()),
    };
    publicar(&a, g, |h| async move { h.voice_mod(k, acao).await }).await
}

#[tauri::command]
pub async fn list_invites(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<Vec<Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let l = a.node.with_store(|s| s.group_invites(g)).map_err(txt)?;
    Ok(l.iter()
        .map(|i| json!({
            "hash": hex(&i.hash), "by": a.pessoa_id(&i.by), "maxUses": i.max_uses, "uses": i.uses,
            "expires": if i.expires_ms == 0 { Value::Null } else { json!(i.expires_ms) }, "ts": i.ts,
        }))
        .collect())
}

#[tauri::command]
pub async fn revoke_invite(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String, hash: String) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let h32 = unhex32(&hash).ok_or("Convite desconhecido.")?;
    publicar(&a, g, |h| async move { h.revoke_invite(h32).await }).await
}

#[tauri::command]
pub async fn list_bans(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<Vec<Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let l = a.node.with_store(|s| s.bans(g)).map_err(txt)?;
    Ok(l.iter()
        .map(|b| json!({ "id": a.pessoa_id(&b.pubkey), "name": b.name, "reason": b.reason, "by": a.pessoa_id(&b.by), "ts": b.ts }))
        .collect())
}

/// Registo de auditoria: as últimas 300 ações de gestão e moderação.
#[tauri::command]
pub async fn audit_log(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<Vec<Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let l = a.node.with_store(|s| s.audit_log(g, 300)).map_err(txt)?;
    let p = |k: &[u8; 32]| a.pessoa_id(k);
    Ok(l.iter()
        .map(|(seq, ev)| {
            let (acao, alvo, det): (&str, Option<String>, Value) = match &ev.body.kind {
                EventKind::Layout { channels, .. } => ("channels", None, json!({ "n": channels.len() })),
                EventKind::Roles { roles } => ("roles", None, json!({ "names": roles.iter().map(|r| r.name.clone()).collect::<Vec<_>>() })),
                EventKind::MemberRoles { member, roles } => ("member_roles", Some(p(member)), json!({ "roles": roles })),
                EventKind::ChannelPerms { channel, overrides } => ("channel_perms", None, json!({ "channel": channel, "n": overrides.len() })),
                EventKind::Kick { member, reason } => ("kick", Some(p(member)), json!({ "reason": reason })),
                EventKind::Ban { member, reason, delete_secs } => ("ban", Some(p(member)), json!({ "reason": reason, "deleteSecs": delete_secs })),
                EventKind::Unban { member } => ("unban", Some(p(member)), Value::Null),
                EventKind::Timeout { member, until_ms, reason } => ("timeout", Some(p(member)), json!({ "until": until_ms, "reason": reason })),
                EventKind::Nick { member, nick } => ("nick", Some(p(member)), json!({ "nick": nick })),
                EventKind::Pin { target, on } => ("pin", None, json!({ "target": hex(target), "on": on })),
                EventKind::Settings(d) => ("settings", None, json!({ "name": d.name })),
                EventKind::VoiceMod { member, action } => ("voice", Some(p(member)), json!({ "action": format!("{action:?}") })),
                EventKind::InviteCreated { max_uses, expires_ms, .. } => ("invite", None, json!({ "maxUses": max_uses, "expires": expires_ms })),
                EventKind::InviteRevoked { .. } => ("invite_revoked", None, Value::Null),
                EventKind::Report { target, reason } => {
                    let texto = a.node.with_store(|s| s.event_by_id(g, target)).ok().flatten().and_then(|e| match e.body.kind {
                        EventKind::Message { text, .. } | EventKind::Post { text, .. } => Some((text, e.body.author)),
                        _ => None,
                    });
                    (
                        "report",
                        texto.as_ref().map(|(_, k)| p(k)),
                        json!({ "reason": reason, "target": hex(target), "text": texto.map(|t| t.0.chars().take(300).collect::<String>()) }),
                    )
                }
                EventKind::MemberJoined { .. } => ("join", None, Value::Null),
                EventKind::Leave => ("leave", None, Value::Null),
                _ => ("other", None, Value::Null),
            };
            json!({ "seq": seq, "ts": ev.body.timestamp_ms, "by": p(&ev.body.author), "action": acao, "target": alvo, "detail": det })
        })
        .collect())
}
