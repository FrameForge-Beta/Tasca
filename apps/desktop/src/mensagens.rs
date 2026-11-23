//! Mensagens dos grupos: responder, editar, apagar, reações, anexos, pesquisa e por ler.
//!
//! O histórico é uma lista de eventos assinados. Editar, apagar e reagir são eventos
//! novos que apontam para a mensagem; aqui junta-se tudo no estado que a interface mostra.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tasca_core::{EventKind, GroupId, SignedEvent, event::Anexo};
use tauri::{AppHandle, State};

use crate::contas::Contas;
use crate::grupos::{Ativa, Rede, ativa, canal, hex, txt, unhex32};

type Res<T> = Result<T, String>;

/// Uma mensagem já com edições, reações e resposta aplicadas.
struct Msg {
    ev: SignedEvent,
    channel: String,
    text: String,
    reply_to: Option<[u8; 32]>,
    files: Vec<Anexo>,
    edited: bool,
    /// emoji → quem reagiu (por ordem de chegada do emoji)
    reacts: Vec<(String, Vec<[u8; 32]>)>,
}

/// Junta os eventos de um grupo em mensagens. `ch`: só as desse canal.
fn juntar(eventos: &[(u64, SignedEvent)], ch: Option<&str>) -> (Vec<Msg>, HashMap<[u8; 32], usize>) {
    let mut msgs: Vec<Option<Msg>> = Vec::new();
    let mut idx: HashMap<[u8; 32], usize> = HashMap::new();
    for (_, ev) in eventos {
        match &ev.body.kind {
            // a mesma mensagem duas vezes (por exemplo, histórico e caixa): conta uma
            EventKind::Message { .. } | EventKind::Post { .. } if idx.contains_key(&ev.id()) => {}
            EventKind::Message { channel, text } if ch.is_none_or(|c| c == channel) => {
                idx.insert(ev.id(), msgs.len());
                msgs.push(Some(Msg { ev: ev.clone(), channel: channel.clone(), text: text.clone(), reply_to: None, files: vec![], edited: false, reacts: vec![] }));
            }
            EventKind::Post { channel, text, reply_to, files } if ch.is_none_or(|c| c == channel) => {
                idx.insert(ev.id(), msgs.len());
                msgs.push(Some(Msg { ev: ev.clone(), channel: channel.clone(), text: text.clone(), reply_to: *reply_to, files: files.clone(), edited: false, reacts: vec![] }));
            }
            EventKind::Edit { target, text } => {
                if let Some(Some(m)) = idx.get(target).and_then(|i| msgs.get_mut(*i)) {
                    if m.ev.body.author == ev.body.author {
                        m.text = text.clone();
                        m.edited = true;
                    }
                }
            }
            EventKind::Delete { target } => {
                if let Some(i) = idx.get(target) {
                    msgs[*i] = None;
                }
            }
            EventKind::Ban { member, delete_secs, .. } if *delete_secs > 0 => {
                // banido com "apagar mensagens": some o que escreveu nesse tempo
                let desde = ev.body.timestamp_ms.saturating_sub(delete_secs * 1000);
                for m in msgs.iter_mut() {
                    if m.as_ref().is_some_and(|m| m.ev.body.author == *member && m.ev.body.timestamp_ms >= desde) {
                        *m = None;
                    }
                }
            }
            EventKind::React { target, emoji, on } => {
                if let Some(Some(m)) = idx.get(target).and_then(|i| msgs.get_mut(*i)) {
                    let quem = ev.body.author;
                    match m.reacts.iter_mut().find(|(e, _)| e == emoji) {
                        Some((_, l)) => {
                            l.retain(|k| *k != quem);
                            if *on {
                                l.push(quem);
                            }
                        }
                        None if *on => m.reacts.push((emoji.clone(), vec![quem])),
                        None => {}
                    }
                    m.reacts.retain(|(_, l)| !l.is_empty());
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    let mut novo_idx = HashMap::new();
    for m in msgs.into_iter().flatten() {
        novo_idx.insert(m.ev.id(), out.len());
        out.push(m);
    }
    (out, novo_idx)
}

fn anexo_json(f: &Anexo) -> Value {
    json!({ "hash": hex(&f.hash), "name": f.name, "size": f.size, "mime": f.mime, "thumb": f.thumb, "de": hex(&f.de) })
}

fn msg_json(a: &Ativa, m: &Msg, todas: &[Msg], idx: &HashMap<[u8; 32], usize>) -> Value {
    let mut v = json!({
        "id": hex(&m.ev.id()),
        "ts": m.ev.body.timestamp_ms,
        "from": a.pessoa_id(&m.ev.body.author),
        "text": m.text,
    });
    if m.edited {
        v["edited"] = json!(true);
    }
    if !m.files.is_empty() {
        v["files"] = m.files.iter().map(anexo_json).collect();
    }
    if !m.reacts.is_empty() {
        v["reactions"] = m
            .reacts
            .iter()
            .map(|(e, l)| json!({ "e": e, "n": l.len(), "mine": l.contains(&a.conta) }))
            .collect();
    }
    if let Some(r) = m.reply_to {
        v["replyTo"] = match idx.get(&r).map(|i| &todas[*i]) {
            Some(o) => json!({ "id": hex(&r), "from": a.pessoa_id(&o.ev.body.author), "text": resumo(o) }),
            None => json!({ "id": hex(&r), "deleted": true }),
        };
    }
    v
}

fn resumo(m: &Msg) -> String {
    let t: String = m.text.chars().take(120).collect();
    if t.trim().is_empty() {
        m.files.first().map(|f| format!("📎 {}", f.name)).unwrap_or_default()
    } else {
        t
    }
}

/// O histórico do grupo, mais o que ainda está à espera do anfitrião (no fim, com número 0).
fn eventos(a: &Ativa, g: GroupId) -> Res<Vec<(u64, SignedEvent)>> {
    let mut v = a.node.with_store(|s| s.events_since(g, 0)).map_err(txt)?;
    // um pedido que o anfitrião já aceitou está no histórico: não aparece outra vez
    let ja: std::collections::HashSet<[u8; 32]> = v.iter().map(|(_, e)| e.id()).collect();
    v.extend(
        a.node
            .with_store(|s| s.pending(g))
            .unwrap_or_default()
            .into_iter()
            .filter(|e| !ja.contains(&e.id()))
            .map(|e| (0, e)),
    );
    Ok(v)
}

/// As últimas 500 mensagens de um canal.
pub(crate) fn listar(a: &Ativa, g: GroupId, ch: &str) -> Res<Vec<Value>> {
    let ev = eventos(a, g)?;
    let (msgs, idx) = juntar(&ev, Some(ch));
    let de = msgs.len().saturating_sub(500);
    let fixadas = a.node.with_store(|s| s.pins(g)).unwrap_or_default();
    let espera: std::collections::HashSet<[u8; 32]> = ev.iter().filter(|(n, _)| *n == 0).map(|(_, e)| e.id()).collect();
    Ok(msgs[de..]
        .iter()
        .map(|m| {
            let mut v = msg_json(a, m, &msgs, &idx);
            if fixadas.contains(&m.ev.id()) {
                v["pinned"] = json!(true);
            }
            if espera.contains(&m.ev.id()) {
                v["queued"] = json!(true);
            }
            v
        })
        .collect())
}

/// A mensagem `id` como está agora (para a interface atualizar depois de uma edição ou
/// reação). `None` se foi apagada. Devolve também o canal.
pub(crate) fn uma(a: &Ativa, g: GroupId, id: &[u8; 32]) -> Option<(String, Option<Value>)> {
    let ev = eventos(a, g).ok()?;
    let alvo = ev.iter().find(|(_, e)| e.id() == *id)?;
    let ch = match &alvo.1.body.kind {
        EventKind::Message { channel, .. } | EventKind::Post { channel, .. } => channel.clone(),
        _ => return None,
    };
    let (msgs, idx) = juntar(&ev, Some(&ch));
    Some((ch, idx.get(id).map(|i| msg_json(a, &msgs[*i], &msgs, &idx))))
}

/// Uma mensagem acabada de chegar (sem precisar de ler o histórico todo).
pub(crate) fn nova_json(a: &Ativa, g: GroupId, ev: &SignedEvent) -> Value {
    let ch = match &ev.body.kind {
        EventKind::Post { channel, .. } | EventKind::Message { channel, .. } => channel.clone(),
        _ => String::new(),
    };
    let reply = match &ev.body.kind {
        EventKind::Post { reply_to: Some(_), .. } => true,
        _ => false,
    };
    if reply {
        if let Some((_, Some(v))) = uma(a, g, &ev.id()) {
            return v;
        }
    }
    let (msgs, idx) = juntar(std::slice::from_ref(&(0, ev.clone())), Some(&ch));
    msgs.first().map(|m| msg_json(a, m, &msgs, &idx)).unwrap_or(json!({}))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnexoUi {
    hash: String,
    name: String,
    size: u64,
    mime: String,
    thumb: Option<String>,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn send_message(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    channel_id: String,
    text: String,
    reply_to: Option<String>,
    files: Option<Vec<AnexoUi>>,
) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    let (g, ch) = canal(&channel_id)?;
    let h = a.handle(g)?;
    let ligado = h.is_connected();
    let hosted = h.is_hosted();
    if !ligado && !a.tem_servidor() {
        return Err("O anfitrião está offline. Tenta quando ele voltar.".into());
    }
    let de = a.node.endpoint_id();
    let files: Vec<Anexo> = files
        .unwrap_or_default()
        .into_iter()
        .map(|f| {
            Ok(Anexo {
                hash: unhex32(&f.hash).ok_or("Anexo inválido.")?,
                name: f.name,
                size: f.size,
                mime: f.mime,
                thumb: f.thumb.filter(|t| t.len() < 200_000),
                de,
            })
        })
        .collect::<Res<_>>()?;
    let reply = reply_to.as_deref().and_then(unhex32);
    let id = tokio::time::timeout(Duration::from_secs(20), h.post(&ch, &text, reply, files.clone()))
        .await
        .map_err(|_| "O envio demorou demasiado.".to_string())?
        .map_err(txt)?;
    if a.tem_servidor() {
        // se a mensagem não voltar logo do grupo (leitura pendurada, rede a mudar), lê já
        let a2 = a.clone();
        tauri::async_runtime::spawn(async move {
            for espera in [2u64, 4, 8] {
                tokio::time::sleep(Duration::from_secs(espera)).await;
                if a2.node.with_store(|s| s.event_by_id(g, &id)).ok().flatten().is_some() {
                    return;
                }
                let _ = tokio::time::timeout(Duration::from_secs(15), a2.node.sync_group(g, 0)).await;
            }
        });
    }
    let mut v = json!({
        "id": hex(&id),
        "ts": tasca_core::event::now_ms(),
        "from": "me",
        "text": text,
        // fica pendente até voltar do grupo (message_ack)
        "pending": a.tem_servidor() || (ligado && !hosted),
    });
    if !files.is_empty() {
        v["files"] = files.iter().map(anexo_json).collect();
    }
    if let Some(r) = reply {
        if let Some((_, Some(o))) = uma(&a, g, &r) {
            v["replyTo"] = json!({ "id": hex(&r), "from": o["from"], "text": o["text"] });
        }
    }
    Ok(v)
}

async fn publicar(
    app: &AppHandle,
    rede: &State<'_, Rede>,
    c: &State<'_, Contas>,
    channel_id: &str,
    f: impl AsyncFnOnce(tasca_core::GroupHandle) -> anyhow::Result<[u8; 32]>,
) -> Res<bool> {
    let a = ativa(app, rede, c).await?;
    let (g, _) = canal(channel_id)?;
    let h = a.handle(g)?;
    if !h.is_connected() && !a.tem_servidor() {
        return Err("O anfitrião está offline. Tenta quando ele voltar.".into());
    }
    tokio::time::timeout(Duration::from_secs(20), f(h))
        .await
        .map_err(|_| "Demorou demasiado.".to_string())?
        .map_err(txt)?;
    Ok(true)
}

#[tauri::command]
pub async fn edit_message(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, channel_id: String, id: String, text: String) -> Res<bool> {
    let alvo = unhex32(&id).ok_or("Mensagem inválida.")?;
    publicar(&app, &rede, &c, &channel_id, async move |h| h.edit_message(alvo, &text).await).await
}

#[tauri::command]
pub async fn delete_message(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, channel_id: String, id: String) -> Res<bool> {
    let alvo = unhex32(&id).ok_or("Mensagem inválida.")?;
    publicar(&app, &rede, &c, &channel_id, async move |h| h.delete_message(alvo).await).await
}

#[tauri::command]
pub async fn react_message(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    channel_id: String,
    id: String,
    emoji: String,
    on: bool,
) -> Res<bool> {
    let alvo = unhex32(&id).ok_or("Mensagem inválida.")?;
    publicar(&app, &rede, &c, &channel_id, async move |h| h.react(alvo, &emoji, on).await).await
}

/// Procura texto nas mensagens de um grupo (todas as salas de texto).
#[tauri::command]
pub async fn search_messages(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String, query: String) -> Res<Vec<Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let g = GroupId(unhex32(&group_id).ok_or("Grupo inválido.")?);
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let ev = eventos(&a, g)?;
    let (msgs, idx) = juntar(&ev, None);
    Ok(msgs
        .iter()
        .rev()
        .filter(|m| m.text.to_lowercase().contains(&q) || m.files.iter().any(|f| f.name.to_lowercase().contains(&q)))
        .take(60)
        .map(|m| {
            let mut v = msg_json(&a, m, &msgs, &idx);
            v["channelId"] = json!(format!("{}/{}", hex(&g.0), m.channel));
            v
        })
        .collect())
}

/// Quantas mensagens de outros há em cada canal depois de `since` (ms por canal).
/// A interface guarda até onde leu.
#[tauri::command]
pub async fn unread_counts(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    since: BTreeMap<String, u64>,
) -> Res<BTreeMap<String, Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let g = GroupId(unhex32(&group_id).ok_or("Grupo inválido.")?);
    let ev = eventos(&a, g)?;
    let (msgs, _) = juntar(&ev, None);
    let mut out: BTreeMap<String, (u32, bool)> = BTreeMap::new();
    let eu_nome = a.node.amigos().perfil_eu().utilizador.map(|u| format!("@{u}"));
    for m in &msgs {
        if m.ev.body.author == a.conta {
            continue;
        }
        let id = format!("{}/{}", hex(&g.0), m.channel);
        // canais nunca abertos contam a partir de agora (não aparecem 500 por ler)
        let Some(desde) = since.get(&id) else { continue };
        if m.ev.body.timestamp_ms > *desde {
            let e = out.entry(id).or_default();
            e.0 += 1;
            if eu_nome.as_ref().is_some_and(|n| m.text.contains(n.as_str())) {
                e.1 = true;
            }
        }
    }
    Ok(out.into_iter().map(|(k, (n, men))| (k, json!({ "n": n, "mention": men }))).collect())
}

/// Guarda um ficheiro escolhido pela interface (corpo cru) e devolve o hash. O nome e o
/// tipo vêm nos cabeçalhos `x-name` e `x-mime`; quem o envia publica depois a mensagem.
#[tauri::command]
pub async fn upload_file(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, request: tauri::ipc::Request<'_>) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    let Some(dados) = crate::corpo::bytes(request.body()) else {
        return Err("Ficheiro em falta.".into());
    };
    let pasta = a.node.pasta().ok_or("Sem pasta de ficheiros.")?.clone();
    let tam = dados.len() as u64;
    let dados = dados.into_owned();
    let hash = tauri::async_runtime::spawn_blocking(move || pasta.guardar(&dados))
        .await
        .map_err(|e| e.to_string())?
        .map_err(txt)?;
    Ok(json!({ "hash": hex(&hash), "size": tam }))
}

fn anexo_de(a: &Ativa, g: GroupId, hash: &[u8; 32]) -> Res<Anexo> {
    let ev = eventos(a, g)?;
    ev.iter()
        .find_map(|(_, e)| match &e.body.kind {
            EventKind::Post { files, .. } => files.iter().find(|f| f.hash == *hash).cloned(),
            _ => None,
        })
        .ok_or_else(|| "Esse ficheiro não está neste grupo.".into())
}

/// O conteúdo de um anexo (pede-o a quem o tiver, se for preciso).
#[tauri::command]
pub async fn get_file(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, channel_id: String, hash: String) -> Res<tauri::ipc::Response> {
    let a = ativa(&app, &rede, &c).await?;
    let (g, _) = canal(&channel_id)?;
    let h = unhex32(&hash).ok_or("Ficheiro inválido.")?;
    let caminho = match a.node.pasta() {
        Some(p) if p.tem(&h) => p.caminho(&h),
        _ => {
            let anexo = anexo_de(&a, g, &h)?;
            a.node.obter_ficheiro(g, &anexo).await.map_err(|e| {
                eprintln!("anexo: {e:#}");
                "Ninguém com este ficheiro está online agora.".to_string()
            })?
        }
    };
    let dados = tokio::fs::read(&caminho).await.map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(dados))
}

/// Guarda um anexo na pasta Transferências e mostra-o (no PC). Devolve o caminho.
#[tauri::command]
pub async fn save_file(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, channel_id: String, hash: String) -> Res<String> {
    use tauri::Manager;
    let a = ativa(&app, &rede, &c).await?;
    let (g, _) = canal(&channel_id)?;
    let h = unhex32(&hash).ok_or("Ficheiro inválido.")?;
    let anexo = anexo_de(&a, g, &h)?;
    let origem = a.node.obter_ficheiro(g, &anexo).await.map_err(|e| {
        eprintln!("anexo: {e:#}");
        "Ninguém com este ficheiro está online agora.".to_string()
    })?;
    let pasta = app.path().download_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&pasta).map_err(|e| e.to_string())?;
    // nome limpo e sem pisar ficheiros que já lá estão
    let limpo: String = anexo.name.chars().map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '_' } else { c }).collect();
    let p = std::path::Path::new(&limpo);
    let (base, ext) = (
        p.file_stem().and_then(|s| s.to_str()).unwrap_or("ficheiro").to_string(),
        p.extension().and_then(|s| s.to_str()).map(|e| format!(".{e}")).unwrap_or_default(),
    );
    let mut destino = pasta.join(format!("{base}{ext}"));
    let mut n = 1;
    while destino.exists() {
        destino = pasta.join(format!("{base} ({n}){ext}"));
        n += 1;
    }
    std::fs::copy(&origem, &destino).map_err(|e| e.to_string())?;
    #[cfg(desktop)]
    {
        use tauri_plugin_opener::OpenerExt;
        let _ = app.opener().reveal_item_in_dir(&destino);
    }
    Ok(destino.to_string_lossy().into_owned())
}

/// Anfitrião: guarda uma cópia dos anexos novos, para não desaparecerem quando quem os
/// enviou desligar.
pub(crate) fn guardar_copias(a: &Ativa, g: GroupId, ev: &SignedEvent) {
    let EventKind::Post { files, .. } = &ev.body.kind else { return };
    if ev.body.author == a.conta {
        return;
    }
    for f in files.iter().filter(|f| f.size <= 25 * 1024 * 1024).cloned() {
        let node = a.node.clone();
        tauri::async_runtime::spawn(async move {
            for t in 0..3u64 {
                match node.obter_ficheiro(g, &f).await {
                    Ok(_) => break,
                    Err(e) => eprintln!("cópia do anexo {}: {e:#}", f.name),
                }
                tokio::time::sleep(Duration::from_secs(5 * (t + 1))).await;
            }
        });
    }
}
