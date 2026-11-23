//! Grupos a sério: liga o `tasca_core::Node` aos comandos da interface (`ui/api.js`).
//!
//! Ao desbloquear a conta, o primeiro comando de grupos arranca um nó iroh com a identidade
//! da conta e a base de dados `contas/<chave>/grupos.db`. Para cada grupo:
//!
//! - com servidor de contas (o normal): o grupo vive na caixa cifrada do servidor. Uma
//!   tarefa lê a caixa (com espera longa) e outra diz quem está online. Não é preciso o PC
//!   do dono ligado para nada (só as chamadas e os ficheiros vão de PC para PC);
//! - sem servidor (testes, rede local): o grupo vive no PC do dono e os membros ligam-se a ele.
//!
//! Os eventos novos chegam à interface como `message`, `message_ack`, `group_changed` e
//! `host_status`.
//!
//! Ids na interface: grupo = 64 carateres hex; canal = `<grupo>/<id do canal>`; pessoa =
//! chave pública em hex, ou `me` para a própria conta.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex as StdMutex},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tasca_core::{
    ChannelDef, ChannelKind, EventKind, GroupHandle, GroupId, Identity, Invite, Node, NodeOptions, PublicKey, SignedEvent,
    Store, event::now_ms, servidor::AccountServer,
};
use tauri::{AppHandle, Emitter, State, async_runtime::JoinHandle};
use tokio::sync::Mutex;

use crate::contas::{Contas, pasta_contas};

type Res<T> = Result<T, String>;

/// Estado da rede guardado na app.
#[derive(Default)]
pub struct Rede {
    ativa: Mutex<Option<Arc<Ativa>>>,
}

/// O nó da conta desbloqueada e as ligações aos grupos.
pub(crate) struct Ativa {
    pub(crate) conta: [u8; 32],
    pub(crate) node: Node,
    servidor: bool,
    ligacoes: StdMutex<HashMap<GroupId, Ligacao>>,
    tarefas: StdMutex<Vec<JoinHandle<()>>>,
    /// Identidade da conta (para a caixa da conta).
    id: Identity,
    /// Servidor de contas (caixa da conta: grupos e perfil entre dispositivos).
    srv: Option<AccountServer>,
    sinc: StdMutex<Sinc>,
    caminho_sinc: PathBuf,
}

/// O que este dispositivo já leu e publicou na caixa da conta (ver `sincronia.rs`).
#[derive(Default, Serialize, Deserialize)]
struct Sinc {
    seq: u64,
    grupos_publicados: bool,
    perfil_publicado: bool,
    /// Impressões digitais do último perfil publicado ou recebido.
    perfil: u64,
    foto: u64,
    banner: u64,
    /// Quando mudou cada parte (para não aplicar novidades mais velhas do que a minha).
    #[serde(default)]
    em_perfil: u64,
    #[serde(default)]
    em_foto: u64,
    #[serde(default)]
    em_banner: u64,
    /// Grupos de que saíste neste dispositivo (não voltam pela sincronia da conta).
    #[serde(default)]
    esquecidos: Vec<String>,
    /// Os grupos já foram publicados com a chave da caixa (os outros dispositivos da conta
    /// abrem-nos sem precisar do PC do dono).
    #[serde(default)]
    grupos_com_chave: bool,
}

struct Ligacao {
    handle: GroupHandle,
    online: bool,
}

pub(crate) fn txt(e: anyhow::Error) -> String {
    eprintln!("erro de grupos: {e:#}");
    let s = e.to_string();
    let mut c = s.chars();
    match c.next() {
        Some(p) => p.to_uppercase().chain(c).collect(),
        None => "Erro desconhecido".into(),
    }
}

pub(crate) fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub(crate) fn unhex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

fn grupo(id: &str) -> Res<GroupId> {
    unhex32(id).map(GroupId).ok_or_else(|| "Grupo desconhecido.".into())
}

/// `<grupo>/<canal>` → (grupo, canal).
pub(crate) fn canal(id: &str) -> Res<(GroupId, String)> {
    let (g, c) = id.split_once('/').ok_or("Canal desconhecido.")?;
    Ok((grupo(g)?, c.to_string()))
}

const CORES: [&str; 8] = ["#9B2C3F", "#2A5CAA", "#5E8C4A", "#B8791A", "#6B4C9A", "#2E7D7A", "#7A3B2E", "#4A6FA5"];

pub(crate) fn cor(b: &[u8]) -> &'static str {
    CORES[(b[0] as usize + b[31] as usize) % CORES.len()]
}

fn iniciais(nome: &str) -> String {
    let s: String = nome.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect();
    s.to_uppercase()
}

impl Ativa {
    fn eu(&self, chave: &[u8; 32]) -> bool {
        *chave == self.conta
    }

    pub(crate) fn pessoa_id(&self, chave: &[u8; 32]) -> String {
        if self.eu(chave) { "me".into() } else { hex(chave) }
    }

    pub(crate) fn handle(&self, g: GroupId) -> Res<GroupHandle> {
        self.ligacoes
            .lock()
            .unwrap()
            .get(&g)
            .map(|l| l.handle.clone())
            .ok_or_else(|| "Grupo desconhecido.".into())
    }

    /// Notificação do sistema para uma mensagem nova de outra pessoa.
    fn avisar(&self, app: &AppHandle, g: GroupId, canal: &str, channel_id: &str, ev: &SignedEvent) {
        // só mensagens recentes (não o histórico que chega ao sincronizar)
        if now_ms().saturating_sub(ev.body.timestamp_ms) > 10 * 60 * 1000 {
            return;
        }
        let (texto, anexos) = match &ev.body.kind {
            EventKind::Message { text, .. } => (text.clone(), 0),
            EventKind::Post { text, files, .. } => (text.clone(), files.len()),
            _ => return,
        };
        let texto = if texto.trim().is_empty() && anexos > 0 { format!("📎 {anexos} anexo(s)") } else { texto };
        let (grupo, autor, nome_canal) = self.node.with_store(|s| {
            let grupo = s.group(g).ok().flatten().map(|i| i.name).unwrap_or_default();
            let autor = s
                .members(g)
                .ok()
                .and_then(|l| l.into_iter().find(|(k, _)| *k.as_bytes() == ev.body.author).map(|(_, n)| n))
                .unwrap_or_else(|| "Alguém".into());
            let nome_canal = s
                .layout(g)
                .ok()
                .and_then(|(cs, _)| cs.into_iter().find(|c| c.id == canal).map(|c| c.name))
                .unwrap_or_else(|| canal.to_string());
            (grupo, autor, nome_canal)
        });
        crate::avisos::mensagem(app, channel_id, Some(&hex(&g.0)), &format!("{autor} · #{nome_canal} ({grupo})"), &texto);
    }

    pub(crate) fn tem_servidor(&self) -> bool {
        self.servidor
    }

    fn online(&self, g: GroupId) -> bool {
        self.ligacoes.lock().unwrap().get(&g).is_some_and(|l| l.online)
    }

    fn definir(&self, app: &AppHandle, g: GroupId, handle: GroupHandle, online: bool) {
        let antes = self.ligacoes.lock().unwrap().insert(g, Ligacao { handle, online });
        if antes.map(|l| l.online) != Some(online) {
            let _ = app.emit("host_status", json!({ "groupId": hex(&g.0), "online": online }));
        }
    }

    fn tarefa(&self, f: impl std::future::Future<Output = ()> + Send + 'static) {
        self.tarefas.lock().unwrap().push(tauri::async_runtime::spawn(f));
    }

    async fn parar(&self) {
        for t in self.tarefas.lock().unwrap().drain(..) {
            t.abort();
        }
        let _ = self.node.clone().shutdown().await;
    }

    /// Põe um grupo a funcionar: eventos para a interface, ligação ao anfitrião e caixa.
    fn arrancar(self: &Arc<Self>, app: &AppHandle, g: GroupId, ligado: Option<GroupHandle>) -> Res<()> {
        let info = self.node.with_store(|s| s.group(g)).map_err(txt)?.ok_or("Grupo desconhecido.")?;

        // eventos novos → interface
        let (a, ap) = (self.clone(), app.clone());
        let mut rx = self.node.subscribe(g);
        self.tarefa(async move {
            use tokio::sync::broadcast::error::RecvError;
            loop {
                match rx.recv().await {
                    Ok((_, ev)) => a.emitir(&ap, g, &ev),
                    Err(RecvError::Lagged(_)) => {
                        let _ = ap.emit("group_changed", json!({ "groupId": hex(&g.0) }));
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        });

        // quem está online (e em que sala) mudou → interface volta a ler o grupo
        let ap = app.clone();
        let mut pres = self.node.presence(g);
        self.tarefa(async move {
            while pres.changed().await.is_ok() {
                let _ = ap.emit("group_changed", json!({ "groupId": hex(&g.0) }));
            }
        });

        // salas de voz alojadas neste dispositivo ou anunciadas por outros mudaram
        let ap = app.clone();
        let mut voz = self.node.watch_voz();
        self.tarefa(async move {
            while voz.changed().await.is_ok() {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                voz.borrow_and_update();
                let _ = ap.emit("group_changed", json!({ "groupId": hex(&g.0) }));
            }
        });

        if self.servidor {
            // o grupo vive na caixa do servidor: está sempre disponível
            let h = self.node.open_offline(g).map_err(txt)?;
            self.definir(app, g, h, true);
            // quem está online (de 30 em 30 segundos)
            let a = self.clone();
            self.tarefa(async move {
                loop {
                    if let Err(e) = a.node.presenca(g).await {
                        eprintln!("presença no grupo {g}: {e:#}");
                    }
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            });
        } else if info.hosted {
            let h = self.node.open_hosted(g).map_err(txt)?;
            self.definir(app, g, h, true);
        } else {
            match &ligado {
                Some(h) => self.definir(app, g, h.clone(), true),
                None => self.definir(app, g, self.node.open_offline(g).map_err(txt)?, false),
            }
            let (a, ap) = (self.clone(), app.clone());
            self.tarefa(async move { a.manter_ligacao(&ap, g, ligado).await });
        }

        if self.servidor {
            let a = self.clone();
            self.tarefa(async move {
                let mut erros = 0u32;
                loop {
                    let t = Instant::now();
                    let r = a.node.sync_group(g, 25).await;
                    if r.is_ok() {
                        erros = 0;
                    }
                    let pausa = match r {
                        // espera longa: só volta logo se houve novidades (ou ainda sem chave)
                        Ok(_) if t.elapsed() < Duration::from_secs(1) => Duration::from_secs(3),
                        Ok(_) => Duration::from_millis(200),
                        Err(e) => {
                            eprintln!("caixa do grupo {g}: {e:#}");
                            // falhou: tenta depressa da primeira vez, depois mais devagar
                            erros += 1;
                            Duration::from_secs((2u64 << erros.min(4)).min(20))
                        }
                    };
                    tokio::time::sleep(pausa).await;
                }
            });
        }
        Ok(())
    }

    /// Membro: mantém a ligação ao PC do anfitrião, e volta a ligar quando cai.
    async fn manter_ligacao(&self, app: &AppHandle, g: GroupId, mut ligado: Option<GroupHandle>) {
        let mut espera = 3;
        loop {
            let h = match ligado.take() {
                Some(h) => h,
                None => match tokio::time::timeout(Duration::from_secs(30), self.node.reconnect(g)).await {
                    Ok(Ok(h)) => h,
                    _ => {
                        tokio::time::sleep(Duration::from_secs(espera)).await;
                        espera = (espera * 2).min(60);
                        continue;
                    }
                },
            };
            espera = 3;
            self.definir(app, g, h.clone(), true);
            h.disconnected().await;
            if let Ok(off) = self.node.open_offline(g) {
                self.definir(app, g, off, false);
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    fn emitir(&self, app: &AppHandle, g: GroupId, ev: &SignedEvent) {
        match &ev.body.kind {
            EventKind::Message { channel, .. } | EventKind::Post { channel, .. } => {
                if self.node.with_store(|s| s.group(g)).ok().flatten().is_some_and(|i| i.hosted) {
                    crate::mensagens::guardar_copias(self, g, ev);
                }
                let channel_id = format!("{}/{channel}", hex(&g.0));
                if !self.eu(&ev.body.author) {
                    self.avisar(app, g, channel, &channel_id, ev);
                }
                if self.eu(&ev.body.author) {
                    // a minha mensagem voltou do anfitrião: está guardada no grupo
                    let _ = app.emit("message_ack", json!({ "channelId": channel_id, "id": hex(&ev.id()) }));
                }
                // as minhas também vão como mensagem: podem ter sido escritas noutro
                // dispositivo da conta (a interface ignora as que já tem)
                let _ = app.emit(
                    "message",
                    json!({ "channelId": channel_id, "groupId": hex(&g.0), "message": crate::mensagens::nova_json(self, g, ev) }),
                );
            }
            EventKind::Edit { target, .. } | EventKind::Delete { target } | EventKind::React { target, .. } => {
                if let Some((ch, m)) = crate::mensagens::uma(self, g, target) {
                    let channel_id = format!("{}/{ch}", hex(&g.0));
                    let _ = app.emit(
                        "message_update",
                        json!({ "channelId": channel_id, "groupId": hex(&g.0), "id": hex(target), "message": m }),
                    );
                }
            }
            EventKind::VoiceMod { member, action } if self.eu(member) => {
                use tasca_core::cargos::AcaoVoz;
                let (acao, sala) = match action {
                    AcaoVoz::Silenciar(true) => ("mute", None),
                    AcaoVoz::Silenciar(false) => ("unmute", None),
                    AcaoVoz::Ensurdecer(true) => ("deafen", None),
                    AcaoVoz::Ensurdecer(false) => ("undeafen", None),
                    AcaoVoz::Desligar => ("disconnect", None),
                    AcaoVoz::Mover(s) => ("move", Some(format!("{}/{s}", hex(&g.0)))),
                    AcaoVoz::PararVideo => ("stopvideo", None),
                };
                let por = self.nome_em(g, &ev.body.author);
                let _ = app.emit("voice_mod", json!({ "groupId": hex(&g.0), "action": acao, "room": sala, "by": por }));
                let _ = app.emit("group_changed", json!({ "groupId": hex(&g.0) }));
            }
            EventKind::Kick { member, reason } | EventKind::Ban { member, reason, .. } if self.eu(member) => {
                let banido = matches!(ev.body.kind, EventKind::Ban { .. });
                let nome = self.node.with_store(|s| s.group(g)).ok().flatten().map(|i| i.name).unwrap_or_default();
                let _ = app.emit("group_removed", json!({ "groupId": hex(&g.0), "name": nome, "reason": reason, "banned": banido }));
            }
            EventKind::Timeout { member, until_ms, reason } if self.eu(member) => {
                let _ = app.emit("timed_out", json!({ "groupId": hex(&g.0), "until": until_ms, "reason": reason }));
                let _ = app.emit("group_changed", json!({ "groupId": hex(&g.0) }));
            }
            EventKind::Pin { target, .. } => {
                if let Some((ch, _)) = crate::mensagens::uma(self, g, target) {
                    let _ = app.emit("pins_changed", json!({ "channelId": format!("{}/{ch}", hex(&g.0)) }));
                }
            }
            _ => {
                let _ = app.emit("group_changed", json!({ "groupId": hex(&g.0) }));
            }
        }
    }

    /// Nome (ou apelido) de alguém num grupo.
    pub(crate) fn nome_em(&self, g: GroupId, k: &[u8; 32]) -> String {
        self.node.with_store(|s| {
            if let Some(n) = s.nicks(g).ok().and_then(|mut m| m.remove(k)) {
                return n;
            }
            PublicKey::from_bytes(k)
                .ok()
                .and_then(|pk| s.display_name(g, &pk).ok().flatten())
                .unwrap_or_else(|| "Alguém".into())
        })
    }


    /// Quem está online no grupo agora, e em que sala de voz.
    fn presenca(&self, g: GroupId) -> tasca_core::protocol::Presence {
        self.node.presence(g).borrow().clone()
    }

    /// Está só no telemóvel (para mostrar o ícone do telemóvel em vez da flor).
    fn no_telemovel(&self, pres: &tasca_core::protocol::Presence, k: &[u8; 32]) -> bool {
        if self.eu(k) {
            return self.node.e_dispositivo();
        }
        pres.contains_key(&tasca_core::protocol::marca_telemovel(k))
    }

    /// Uma pessoa para a interface, com o perfil que publicou no grupo.
    fn pessoa_json(
        &self,
        k: &[u8; 32],
        nome: &str,
        perfil: Option<&tasca_core::store::Profile>,
        cartao: Option<&tasca_core::store::ProfileCard>,
        online: bool,
    ) -> Value {
        json!({
            "username": cartao.and_then(|c| c.username.clone()),
            "bio": cartao.map(|c| c.bio.as_str()).unwrap_or(""),
            "banner": cartao.and_then(|c| c.banner.clone()),
            "id": self.pessoa_id(k),
            "name": nome,
            "color": perfil.map(|p| p.color.as_str()).filter(|c| !c.is_empty()).unwrap_or(cor(k)),
            "photo": perfil.and_then(|p| p.photo.clone()),
            "key": format!("ed25519:{}…{}", &hex(k)[..4], &hex(k)[60..]),
            "status": if online || self.eu(k) { "online" } else { "offline" },
        })
    }

    fn grupo_json(&self, g: GroupId) -> Res<Value> {
        let info = self.node.with_store(|s| s.group(g)).map_err(txt)?.ok_or("Grupo desconhecido.")?;
        let online = info.hosted || self.servidor || self.online(g);
        let def = self.node.with_store(|s| s.settings(g)).map_err(txt)?;
        let perms = self.node.with_store(|s| s.perms(g, &self.conta, None)).unwrap_or(0);
        Ok(json!({
            "id": hex(&g.0),
            "name": info.name,
            "icon": def.icon,
            "banner": def.banner,
            "description": def.description,
            "tag": def.tag,
            "perms": perms,
            "server": self.servidor,
            "short": iniciais(&info.name),
            "owner": self.pessoa_id(info.owner.as_bytes()),
            "color": cor(&g.0),
            "host": if self.servidor {
                json!({ "online": true, "route": "Sempre disponível", "ping": null })
            } else if info.hosted {
                json!({ "online": true, "route": "este PC", "ping": 0 })
            } else {
                json!({ "online": online, "route": "P2P", "ping": null, "lastSeen": "há pouco" })
            },
            "unread": 0,
        }))
    }

    fn canal_json(&self, g: GroupId, c: &ChannelDef) -> Value {
        let voz = c.kind == ChannelKind::Voice;
        let mut v = json!({
            "id": format!("{}/{}", hex(&g.0), c.id),
            "kind": if voz { "voice" } else { "text" },
            "name": c.name,
            "topic": c.topic,
            "cat": c.category,
        });
        if voz {
            v["inVoice"] = json!([]);
        }
        v
    }

    /// Canais e categorias, com as categorias dos canais sempre presentes na lista.
    fn layout(&self, g: GroupId) -> Res<(Vec<ChannelDef>, Vec<String>)> {
        let (canais, mut cats) = self.node.with_store(|s| s.layout(g)).map_err(txt)?;
        for c in &canais {
            if !cats.contains(&c.category) {
                cats.push(c.category.clone());
            }
        }
        Ok((canais, cats))
    }

    /// Dono: muda a disposição de canais e publica-a para todos.
    async fn mudar<T>(
        &self,
        g: GroupId,
        f: impl FnOnce(&mut Vec<ChannelDef>, &mut Vec<String>) -> Res<T>,
    ) -> Res<T> {
        let h = self.handle(g)?;
        let p = self.node.with_store(|s| s.perms(g, &self.conta, None)).map_err(txt)?;
        if p & tasca_core::cargos::GERIR_CANAIS == 0 {
            return Err("Não tens permissão para mudar os canais.".into());
        }
        if !h.is_hosted() && !h.is_connected() && !self.servidor {
            return Err("O anfitrião está desligado. Tenta quando ele voltar.".into());
        }
        let (mut canais, mut cats) = self.layout(g)?;
        let r = f(&mut canais, &mut cats)?;
        h.set_layout(canais, cats).await.map_err(txt)?;
        Ok(r)
    }
}

impl Ativa {
    fn guardar_sinc(&self) {
        let b = serde_json::to_vec(&*self.sinc.lock().unwrap());
        if let Ok(b) = b {
            if let Err(e) = std::fs::write(&self.caminho_sinc, b) {
                eprintln!("sincronia: não consegui guardar: {e}");
            }
        }
    }

    fn item_grupo(&self, g: GroupId) -> Option<crate::sincronia::Item> {
        let info = self.node.with_store(|s| s.group(g)).ok()??;
        let chave = self
            .node
            .with_store(|s| s.group_key(g))
            .ok()
            .flatten()
            .map(|(e, k, _, _)| format!("{e}:{}", hex(k.as_bytes())));
        Some(crate::sincronia::Item::Grupo {
            id: hex(&g.0),
            nome: info.name,
            dono: hex(info.owner.as_bytes()),
            em: now_ms(),
            chave,
        })
    }

    /// Conta aos outros dispositivos da conta que estou neste grupo.
    fn partilhar_grupo(&self, g: GroupId) {
        if let (Some(srv), Some(item)) = (self.srv.clone(), self.item_grupo(g)) {
            crate::sincronia::publicar_depois(srv, self.id.clone(), item);
        }
    }

    /// Lê a caixa da conta: grupos e perfil vindos dos outros dispositivos.
    async fn ler_conta(self: Arc<Self>, app: AppHandle) {
        let Some(srv) = self.srv.clone() else { return };
        let falta = {
            let s = self.sinc.lock().unwrap();
            !s.grupos_publicados || !s.grupos_com_chave
        };
        if falta {
            let grupos = self.node.with_store(|s| s.groups()).unwrap_or_default();
            let mut tudo = true;
            for info in grupos {
                let Some(item) = self.item_grupo(info.id) else { continue };
                if let Err(e) = crate::sincronia::publicar(&srv, &self.id, &item).await {
                    eprintln!("sincronia: grupos não publicados: {e:#}");
                    tudo = false;
                    break;
                }
            }
            if tudo {
                let mut s = self.sinc.lock().unwrap();
                s.grupos_publicados = true;
                s.grupos_com_chave = true;
                drop(s);
                self.guardar_sinc();
            }
        }
        loop {
            let desde = self.sinc.lock().unwrap().seq;
            match crate::sincronia::ler(&srv, &self.id, desde, 25).await {
                Ok((ultimo, itens)) => {
                    for it in itens {
                        self.aplicar_conta(&app, it);
                    }
                    if ultimo != desde {
                        self.sinc.lock().unwrap().seq = ultimo;
                        self.guardar_sinc();
                    }
                }
                Err(e) => {
                    eprintln!("sincronia: caixa da conta: {e:#}");
                    tokio::time::sleep(Duration::from_secs(15)).await;
                }
            }
        }
    }

    fn aplicar_conta(self: &Arc<Self>, app: &AppHandle, it: crate::sincronia::Item) {
        use crate::sincronia::{Item, marca};
        match it {
            Item::Grupo { id, nome, dono, chave, .. } => {
                let (Some(g), Some(dono)) = (unhex32(&id).map(GroupId), unhex32(&dono)) else { return };
                let Ok(dono_pk) = PublicKey::from_bytes(&dono) else { return };
                if self.sinc.lock().unwrap().esquecidos.contains(&id) {
                    return;
                }
                // a chave da caixa do grupo: com ela, este dispositivo lê o grupo sozinho
                let chave = chave.as_deref().and_then(|c| {
                    let (e, k) = c.split_once(':')?;
                    Some((e.parse::<u32>().ok()?, unhex32(k)?))
                });
                if let Some((e, k)) = chave {
                    let _ = self.node.with_store(|s| s.set_group_key(g, e, &tasca_core::ChaveCaixa::from_bytes(k), 0));
                }
                if self.node.with_store(|s| s.group(g)).ok().flatten().is_some() {
                    return;
                }
                // sem servidor, um grupo meu só vive no PC que o criou
                if dono == self.conta && !self.node.e_dispositivo() && !self.servidor {
                    return;
                }
                let nome: String = nome.chars().take(100).collect();
                match self.node.with_store(|s| s.add_known_group(g, &nome, &dono_pk)) {
                    Ok(true) => {
                        if let Err(e) = self.arrancar(app, g, None) {
                            eprintln!("grupo {g} vindo de outro dispositivo: {e}");
                        }
                        let _ = app.emit("group_changed", json!({ "groupId": hex(&g.0) }));
                    }
                    Ok(false) => {}
                    Err(e) => eprintln!("grupo {g} vindo de outro dispositivo: {e:#}"),
                }
            }
            Item::Perfil { cor, bio, em } => {
                let cor: String = cor.chars().take(32).collect();
                let bio: String = bio.chars().take(tasca_core::amigos::MAX_BIO).collect();
                {
                    let mut s = self.sinc.lock().unwrap();
                    if em <= s.em_perfil {
                        return; // é a minha, ou mais velha do que a minha
                    }
                    s.em_perfil = em;
                    s.perfil = marca(&(cor.as_str(), bio.as_str()));
                }
                self.guardar_sinc();
                let _ = app.emit("profile_synced", json!({ "color": cor, "bio": bio }));
            }
            Item::Imagem { campo, valor, em } => {
                let valor = valor.filter(|v| v.starts_with("data:image/") || (campo == "banner" && v.starts_with('#')));
                let m = marca(&valor);
                {
                    let mut guarda = self.sinc.lock().unwrap();
                    let s = &mut *guarda;
                    let (em_x, marca_x) = match campo.as_str() {
                        "foto" => (&mut s.em_foto, &mut s.foto),
                        "banner" => (&mut s.em_banner, &mut s.banner),
                        _ => return,
                    };
                    if em <= *em_x {
                        return;
                    }
                    *em_x = em;
                    *marca_x = m;
                }
                self.guardar_sinc();
                let chave = if campo == "foto" { "photo" } else { "banner" };
                let _ = app.emit("profile_synced", json!({ chave: valor }));
            }
            Item::Pessoa { .. } => {} // amigos.rs
        }
    }

    /// O perfil mudou aqui: publica na caixa da conta o que ainda lá não está. `forcar` =
    /// a pessoa acabou de o mudar (senão só publica uma vez, no PC, para arrancar).
    fn partilhar_perfil(&self, cor: &str, bio: &str, foto: &Option<String>, banner: &Option<String>, forcar: bool) {
        use crate::sincronia::{Item, marca};
        let Some(srv) = self.srv.clone() else { return };
        let mut itens = Vec::new();
        {
            let mut s = self.sinc.lock().unwrap();
            // o telemóvel acabado de entrar ainda tem o perfil por omissão: não o espalha
            if !forcar && (s.perfil_publicado || cfg!(mobile)) {
                return;
            }
            s.perfil_publicado = true;
            let em = now_ms();
            let m = marca(&(cor, bio));
            if s.perfil != m {
                s.perfil = m;
                s.em_perfil = em;
                itens.push(Item::Perfil { cor: cor.into(), bio: bio.into(), em });
            }
            let m = marca(foto);
            if s.foto != m {
                s.foto = m;
                s.em_foto = em;
                itens.push(Item::Imagem { campo: "foto".into(), valor: foto.clone(), em });
            }
            let m = marca(banner);
            if s.banner != m {
                s.banner = m;
                s.em_banner = em;
                itens.push(Item::Imagem { campo: "banner".into(), valor: banner.clone(), em });
            }
        }
        self.guardar_sinc();
        for it in itens {
            crate::sincronia::publicar_depois(srv.clone(), self.id.clone(), it);
        }
    }
}

/// Chave própria deste dispositivo (só no telemóvel; ver `tasca_core::dispositivo`).
fn chave_dispositivo(pasta: &std::path::Path) -> Res<tasca_core::SecretKey> {
    let f = pasta.join("dispositivo.key");
    if let Ok(b) = std::fs::read(&f) {
        if let Ok(arr) = <[u8; 32]>::try_from(b.as_slice()) {
            return Ok(tasca_core::SecretKey::from_bytes(&arr));
        }
    }
    let k = Identity::generate().secret().clone();
    std::fs::create_dir_all(pasta).map_err(|e| e.to_string())?;
    std::fs::write(&f, k.to_bytes()).map_err(|e| format!("não consegui guardar a chave do dispositivo: {e}"))?;
    Ok(k)
}

/// Fecha o nó (ao sair da app): avisa os outros PCs de que as ligações acabaram.
pub(crate) async fn desligar(rede: &Rede) {
    if let Some(a) = rede.ativa.lock().await.take() {
        a.parar().await;
    }
}

/// O nó da conta desbloqueada (arranca-o na primeira vez, ou se a conta mudou).
pub(crate) async fn ativa(app: &AppHandle, rede: &Rede, contas: &Contas) -> Res<Arc<Ativa>> {
    let id = contas.identidade().await.ok_or("Desbloqueia a conta primeiro.")?;
    let pk = *id.public().as_bytes();
    let mut g = rede.ativa.lock().await;
    if let Some(a) = g.as_ref() {
        if a.conta == pk {
            return Ok(a.clone());
        }
    }
    if let Some(velha) = g.take() {
        velha.parar().await;
    }
    let pasta = pasta_contas(app)?.join(hex(&pk));
    let caminho = pasta.join("grupos.db");
    let store = Store::open(&caminho).map_err(txt)?;
    let servidor = AccountServer::from_env();
    let tem_servidor = servidor.is_some();
    // o telemóvel tem chave própria e apresenta-se com certificado da conta: assim não
    // tira o lugar ao PC na rede quando os dois estão ligados
    let dispositivo = if cfg!(mobile) { Some(chave_dispositivo(&pasta)?) } else { None };
    let ficheiros = Some(pasta.join("ficheiros"));
    let node = Node::spawn(id.clone(), store, NodeOptions { servidor: servidor.clone(), dispositivo, ficheiros, ..Default::default() })
        .await
        .map_err(txt)?;
    let caminho_sinc = pasta.join("sincronia.json");
    let sinc: Sinc = std::fs::read(&caminho_sinc).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let a = Arc::new(Ativa {
        conta: pk,
        node,
        servidor: tem_servidor,
        ligacoes: Default::default(),
        tarefas: Default::default(),
        id,
        srv: servidor,
        sinc: StdMutex::new(sinc),
        caminho_sinc,
    });
    for info in a.node.with_store(|s| s.groups()).map_err(txt)? {
        if let Err(e) = a.arrancar(app, info.id, None) {
            eprintln!("grupo {}: {e}", info.id);
        }
    }
    if a.srv.is_some() {
        let (a2, ap) = (a.clone(), app.clone());
        a.tarefa(async move { a2.ler_conta(ap).await });
    }
    // o anfitrião recusou algo meu (sem permissão, castigo, AutoMod...)
    let (mut recusas, ap) = (a.node.recusas(), app.clone());
    a.tarefa(async move {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match recusas.recv().await {
                Ok((g, why)) => {
                    let mut c = why.chars();
                    let msg: String = match c.next() {
                        Some(p) => p.to_uppercase().chain(c).collect(),
                        None => "O anfitrião recusou.".into(),
                    };
                    let _ = ap.emit("group_error", json!({ "groupId": hex(&g.0), "message": msg }));
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });
    // chamadas privadas a chegar
    let (mut toques, ap) = (a.node.toques(), app.clone());
    a.tarefa(async move {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match toques.recv().await {
                Ok(t) => crate::voz::tocou(&ap, t),
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });
    *g = Some(a.clone());
    Ok(a)
}

// ---------------- comandos ----------------

/// Pessoas dos grupos (menos a própria conta), por id.
#[tauri::command]
pub async fn list_group_people(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    let mut out = serde_json::Map::new();
    let grupos = a.node.with_store(|s| s.groups()).map_err(txt)?;
    for info in grupos {
        let pres = a.presenca(info.id);
        let perfis = a.node.with_store(|s| s.profiles(info.id)).map_err(txt)?;
        let cartoes = a.node.with_store(|s| s.profile_cards(info.id)).map_err(txt)?;
        for (pk, nome) in a.node.with_store(|s| s.members(info.id)).map_err(txt)? {
            let k = pk.as_bytes();
            if a.eu(k) {
                continue;
            }
            let id = hex(k);
            let ja = out.get(&id).and_then(|p| p["status"].as_str()) == Some("online");
            let mut p = a.pessoa_json(k, &nome, perfis.get(k), cartoes.get(k), ja || pres.contains_key(k));
            // no telemóvel só se nenhum grupo o vê no PC
            let tel = a.no_telemovel(&pres, k) && out.get(&id).is_none_or(|o| o["mobile"] == true || o["status"] != "online");
            p["mobile"] = json!(tel && p["status"] == "online");
            // outro grupo já tinha foto (ou cartão) e este não: fica com o que há
            for campo in ["photo", "username", "banner"] {
                if p[campo].is_null() {
                    if let Some(f) = out.get(&id).map(|o| o[campo].clone()) {
                        p[campo] = f;
                    }
                }
            }
            if p["bio"] == "" {
                if let Some(f) = out.get(&id).map(|o| o["bio"].clone()) {
                    p["bio"] = f;
                }
            }
            out.insert(id, p);
        }
        // quem já saiu: só o nome, para as mensagens antigas
        for (k, nome) in a.node.with_store(|s| s.ex_members(info.id)).unwrap_or_default() {
            let id = hex(&k);
            if !a.eu(&k) && !out.contains_key(&id) {
                let mut p = a.pessoa_json(&k, &nome, perfis.get(&k), cartoes.get(&k), false);
                p["left"] = json!(true);
                out.insert(id, p);
            }
        }
    }
    Ok(Value::Object(out))
}

#[tauri::command]
pub async fn list_groups(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>) -> Res<Vec<Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let grupos = a.node.with_store(|s| s.groups()).map_err(txt)?;
    grupos.into_iter().map(|g| a.grupo_json(g.id)).collect()
}

#[tauri::command]
pub async fn list_members(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<Vec<Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let info = a.node.with_store(|s| s.group(g)).map_err(txt)?.ok_or("Grupo desconhecido.")?;
    let pres = a.presenca(g);
    let perfis = a.node.with_store(|s| s.profiles(g)).map_err(txt)?;
    let cartoes = a.node.with_store(|s| s.profile_cards(g)).map_err(txt)?;
    let membros = a.node.with_store(|s| s.members(g)).map_err(txt)?;
    let (cargos_, dos, apelidos, castigos, voz) = a
        .node
        .with_store(|s| -> anyhow::Result<_> {
            Ok((s.roles(g)?, s.all_member_roles(g)?, s.nicks(g)?, s.timeouts(g)?, s.voice_mods(g)?))
        })
        .map_err(txt)?;
    Ok(membros
        .into_iter()
        .map(|(pk, nome)| {
            let k = pk.as_bytes();
            let mut p = a.pessoa_json(k, &nome, perfis.get(k), cartoes.get(k), pres.contains_key(k));
            p["mobile"] = json!(a.no_telemovel(&pres, k));
            p["owner"] = json!(pk == info.owner);
            p["since"] = json!(a.node.with_store(|s| s.member_since(g, k)).ok().flatten());
            // cargos pela ordem da hierarquia
            let meus: Vec<&tasca_core::cargos::Cargo> = dos
                .get(k)
                .map(|l| cargos_.iter().filter(|c| l.contains(&c.id)).collect())
                .unwrap_or_default();
            p["roles"] = json!(meus.iter().map(|c| c.id.clone()).collect::<Vec<_>>());
            p["roleColor"] = json!(meus.iter().find(|c| !c.color.is_empty()).map(|c| c.color.clone()));
            p["roleIcon"] = json!(meus.iter().find(|c| c.icon.is_some()).and_then(|c| c.icon.clone()));
            p["hoistRole"] = json!(meus.iter().find(|c| c.hoist).map(|c| c.id.clone()));
            p["rank"] = json!(if pk == info.owner { -1 } else { tasca_core::cargos::posicao(false, &cargos_, &dos.get(k).cloned().unwrap_or_default()) });
            p["nick"] = json!(apelidos.get(k));
            p["timeoutUntil"] = json!(castigos.get(k));
            if let Some((m, d)) = voz.get(k) {
                p["serverMuted"] = json!(m);
                p["serverDeaf"] = json!(d);
            }
            p
        })
        .collect())
}

#[tauri::command]
pub async fn list_channels(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<Vec<Value>> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let (canais, _) = a.layout(g)?;
    let pres = a.presenca(g);
    let salas = a.node.salas_conhecidas(g);
    let eu = a.conta;
    let perms: Vec<u64> = canais.iter().map(|ch| a.node.with_store(|s| s.perms(g, &eu, Some(&ch.id))).unwrap_or(0)).collect();
    Ok(canais
        .iter()
        .zip(perms)
        .filter(|(_, p)| p & tasca_core::cargos::VER_CANAIS != 0)
        .map(|(ch, perm)| {
            let mut v = a.canal_json(g, ch);
            v["perms"] = json!(perm);
            let privado = a.node.with_store(|s| s.channel_perms(g, &ch.id)).unwrap_or_default().iter().any(|e| {
                e.alvo == tasca_core::cargos::Alvo::Cargo(tasca_core::cargos::TODOS.into()) && e.deny & tasca_core::cargos::VER_CANAIS != 0
            });
            if privado {
                v["private"] = json!(true);
            }
            if ch.kind == ChannelKind::Voice {
                let mut quem: Vec<String> = pres
                    .iter()
                    .filter(|(_, sala)| sala.as_deref() == Some(ch.id.as_str()))
                    .map(|(k, _)| a.pessoa_id(k))
                    .collect();
                // Chamadas alojadas por outros (o dono do grupo pode estar desligado).
                for s in salas.iter().filter(|s| s.room == ch.id) {
                    for k in &s.pessoas {
                        let id = a.pessoa_id(k);
                        if !quem.contains(&id) {
                            quem.push(id);
                        }
                    }
                    v["voiceHost"] = json!(a.pessoa_id(&s.conta));
                }
                v["inVoice"] = json!(quem);
            }
            v
        })
        .collect())
}

#[tauri::command]
pub async fn list_categories(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<Vec<String>> {
    let a = ativa(&app, &rede, &c).await?;
    Ok(a.layout(grupo(&group_id)?)?.1)
}

/// Últimas mensagens de um canal (as 500 mais recentes).
#[tauri::command]
pub async fn list_messages(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, channel_id: String) -> Res<Vec<Value>> {
    if channel_id.starts_with("dm-") {
        return Ok(vec![]); // conversas privadas: ver dms.rs
    }
    let a = ativa(&app, &rede, &c).await?;
    let (g, ch) = canal(&channel_id)?;
    crate::mensagens::listar(&a, g, &ch)
}

#[tauri::command]
pub async fn create_group(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, name: String) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    if a.node.e_dispositivo() && !a.servidor {
        return Err("No telemóvel ainda não dá para criar grupos. Cria-o no PC e ele aparece aqui.".into());
    }
    let nome = c.nome().await.unwrap_or_else(|| "Eu".into());
    let g = a.node.create_group(&name, &nome).map_err(txt)?;
    a.arrancar(&app, g, None)?;
    a.partilhar_grupo(g);
    a.grupo_json(g)
}

#[derive(Deserialize)]
pub struct CanalUi {
    id: Option<String>,
    kind: String,
    name: String,
    #[serde(default)]
    topic: String,
    cat: Option<String>,
}

fn canal_de_ui(c: CanalUi) -> ChannelDef {
    let voz = c.kind == "voice";
    let id = c
        .id
        .map(|i| i.rsplit('/').next().unwrap_or(&i).chars().take(64).collect::<String>())
        .filter(|i| !i.trim().is_empty())
        .unwrap_or_else(novo_id);
    ChannelDef {
        id,
        kind: if voz { ChannelKind::Voice } else { ChannelKind::Text },
        name: limpar_nome(&c.name, voz),
        topic: c.topic,
        category: c.cat.filter(|x| !x.trim().is_empty()).unwrap_or_else(|| if voz { "Voz" } else { "Conversa" }.into()),
    }
}

/// Traz para o núcleo um grupo que versões antigas só guardavam na interface.
#[tauri::command]
pub async fn import_local_group(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    name: String,
    channels: Vec<CanalUi>,
    categories: Option<Vec<String>>,
) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    if a.node.e_dispositivo() && !a.servidor {
        return Err("No telemóvel não dá para criar grupos.".into());
    }
    let nome = c.nome().await.unwrap_or_else(|| "Eu".into());
    let g = a.node.create_group(&name, &nome).map_err(txt)?;
    a.arrancar(&app, g, None)?;
    a.partilhar_grupo(g);
    let canais: Vec<ChannelDef> = channels.into_iter().map(canal_de_ui).collect();
    if canais.iter().any(|c| c.kind == ChannelKind::Text) {
        let cats = categories.unwrap_or_default();
        if let Err(e) = a.mudar(g, |cs, ks| {
            *cs = canais;
            *ks = cats;
            Ok(())
        })
        .await
        {
            eprintln!("canais do grupo importado: {e}");
        }
    }
    a.grupo_json(g)
}

#[tauri::command]
pub async fn join_group(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, invite: String) -> Res<Value> {
    let codigo = invite.trim();
    // o código pode vir dentro de um link
    let codigo = codigo.rsplit(['/', '=', '#', ' ']).next().unwrap_or(codigo);
    if let Ok(convite) = tasca_core::invite::Convite::decode(codigo) {
        let a = ativa(&app, &rede, &c).await?;
        if !a.servidor {
            return Err("Para entrar com este convite, a Tasca precisa de estar ligada à Internet.".into());
        }
        let g = convite.group;
        let ja = a.node.with_store(|s| -> anyhow::Result<bool> {
            Ok(s.group(g)?.is_some() && s.is_member(g, &PublicKey::from_bytes(&a.conta)?)?)
        });
        if ja.unwrap_or(false) && a.ligacoes.lock().unwrap().contains_key(&g) {
            return a.grupo_json(g);
        }
        let nome = c.nome().await.unwrap_or_else(|| "Eu".into());
        tokio::time::timeout(Duration::from_secs(60), a.node.entrar_por_convite(&convite, &nome))
            .await
            .map_err(|_| "Demorou demasiado. Tenta outra vez.".to_string())?
            .map_err(|e| {
                eprintln!("entrar no grupo: {e:#}");
                let s = e.to_string();
                if s.contains("banido") {
                    "Foste banido deste grupo.".to_string()
                } else if s.contains("convite") || s.contains("pausa") || s.contains("entradas") {
                    let mut c = s.chars();
                    match c.next() {
                        Some(p) => p.to_uppercase().chain(c).collect::<String>() + ".",
                        None => s,
                    }
                } else if s.contains("não encontrei") {
                    "Este convite não está a funcionar. Pede um convite novo.".to_string()
                } else {
                    "Não foi possível entrar agora. Verifica a ligação à Internet e tenta outra vez.".to_string()
                }
            })?;
        if !a.ligacoes.lock().unwrap().contains_key(&g) {
            a.arrancar(&app, g, None)?;
        }
        a.partilhar_grupo(g);
        let _ = app.emit("group_changed", json!({ "groupId": hex(&g.0) }));
        return a.grupo_json(g);
    }
    if Invite::decode(codigo).is_ok() {
        let a = ativa(&app, &rede, &c).await?;
        if a.servidor {
            return Err("Este convite é antigo e já não funciona. Pede um convite novo.".into());
        }
    }
    let convite = Invite::decode(codigo)
        .map_err(|_| "Este código não parece um convite da Tasca.".to_string())?;
    let a = ativa(&app, &rede, &c).await?;
    let g = convite.group;
    let conhecido = a.node.with_store(|s| s.group(g)).map_err(txt)?;
    if let Some(info) = &conhecido {
        if info.hosted || a.online(g) {
            return a.grupo_json(g);
        }
    }
    let nome = c.nome().await.unwrap_or_else(|| "Eu".into());
    let h = tokio::time::timeout(Duration::from_secs(40), a.node.join(&convite, &nome))
        .await
        .map_err(|_| "Não foi possível chegar ao anfitrião. Ele tem de ter a Tasca aberta; tenta mais tarde.".to_string())?
        .map_err(|e| {
            eprintln!("entrar no grupo: {e:#}");
            let s = e.to_string();
            if s.contains("recusou") {
                "O anfitrião recusou este convite.".to_string()
            } else {
                "Não foi possível chegar ao anfitrião. Ele tem de ter a Tasca aberta; tenta mais tarde.".to_string()
            }
        })?;
    // espera pelo histórico (pelo menos a criação do grupo)
    let ate = Instant::now() + Duration::from_secs(15);
    while a.node.with_store(|s| s.group(g)).map_err(txt)?.is_none() {
        if Instant::now() > ate {
            return Err("O anfitrião não mandou o grupo. Tenta outra vez.".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if conhecido.is_some() && a.ligacoes.lock().unwrap().contains_key(&g) {
        // já estava a tentar ligar: a tarefa dele volta a ligar sozinha; usa esta ligação já
        a.definir(&app, g, h, true);
    } else {
        a.arrancar(&app, g, Some(h))?;
    }
    a.partilhar_grupo(g);
    a.grupo_json(g)
}

#[tauri::command]
pub async fn create_invite(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    max_uses: Option<u32>,
    expires_secs: Option<u64>,
) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let h = a.handle(g)?;
    if !h.is_hosted() && !h.is_connected() && !a.servidor {
        return Err("O anfitrião está desligado: o convite só funciona quando ele voltar.".into());
    }
    let max = max_uses.unwrap_or(0).min(10_000);
    let exp = match expires_secs.unwrap_or(0) {
        0 => 0,
        s => now_ms() + s.min(365 * 24 * 3600) * 1000,
    };
    if a.servidor {
        let p = a.node.with_store(|s| s.perms(g, &a.conta, None)).unwrap_or(0);
        if p & tasca_core::cargos::CRIAR_CONVITES == 0 {
            return Err("Não tens permissão para criar convites.".into());
        }
        let inv = tokio::time::timeout(Duration::from_secs(20), h.criar_convite(max, exp))
            .await
            .map_err(|_| "Demorou demasiado.".to_string())?
            .map_err(txt)?;
        return Ok(json!({ "code": inv.encode(), "expires": if exp == 0 { Value::Null } else { json!(exp) }, "maxUses": max, "pending": false }));
    }
    let inv = tokio::time::timeout(Duration::from_secs(20), h.create_invite(max, exp))
        .await
        .map_err(|_| "Demorou demasiado.".to_string())?
        .map_err(txt)?;
    Ok(json!({ "code": inv.encode(), "expires": if exp == 0 { Value::Null } else { json!(exp) }, "maxUses": max, "pending": !h.is_hosted() && !h.is_connected() }))
}

// ---------------- canais e categorias (só o anfitrião) ----------------

fn novo_id() -> String {
    let mut b = [0u8; 6];
    let _ = getrandom::fill(&mut b);
    hex(&b)
}

/// Nome de canal: texto em minúsculas com hífens; voz fica como está.
fn limpar_nome(n: &str, voz: bool) -> String {
    let n: String = n.trim().chars().take(40).collect();
    if voz {
        return n;
    }
    let mut out = String::new();
    for ch in n.to_lowercase().chars() {
        if ch.is_whitespace() || ch == '-' {
            if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        } else if ch.is_alphanumeric() || ch == '_' {
            out.push(ch);
        }
    }
    out.trim_end_matches('-').to_string()
}

fn nome_categoria(n: &str) -> Res<String> {
    let n: String = n.trim().chars().take(24).collect();
    if n.is_empty() {
        return Err("Dá um nome à categoria.".into());
    }
    Ok(n)
}

#[tauri::command]
pub async fn create_channel(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    name: String,
    kind: String,
    topic: Option<String>,
    cat: Option<String>,
) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let novo = canal_de_ui(CanalUi { id: None, kind, name, topic: topic.unwrap_or_default(), cat });
    if novo.name.is_empty() {
        return Err("Dá um nome ao canal.".into());
    }
    let c2 = novo.clone();
    a.mudar(g, move |cs, ks| {
        if !ks.contains(&c2.category) {
            ks.push(c2.category.clone());
        }
        // fica no fim da sua categoria
        let at = cs.iter().rposition(|x| x.category == c2.category).map(|i| i + 1).unwrap_or(cs.len());
        cs.insert(at, c2);
        Ok(())
    })
    .await?;
    Ok(a.canal_json(g, &novo))
}

#[tauri::command]
pub async fn update_channel(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    channel_id: String,
    name: Option<String>,
    topic: Option<String>,
    cat: Option<String>,
) -> Res<Value> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let (_, id) = canal(&channel_id)?;
    let novo = a
        .mudar(g, move |cs, ks| {
            let ch = cs.iter_mut().find(|x| x.id == id).ok_or("Esse canal já não existe.")?;
            if let Some(n) = name {
                let n = limpar_nome(&n, ch.kind == ChannelKind::Voice);
                if n.is_empty() {
                    return Err("Dá um nome ao canal.".into());
                }
                ch.name = n;
            }
            if let Some(t) = topic {
                ch.topic = t;
            }
            if let Some(k) = cat.filter(|k| !k.trim().is_empty()) {
                ch.category = k.trim().to_string();
                if !ks.contains(&ch.category) {
                    ks.push(ch.category.clone());
                }
            }
            Ok(ch.clone())
        })
        .await?;
    Ok(a.canal_json(g, &novo))
}

#[tauri::command]
pub async fn delete_channel(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    channel_id: String,
) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    let (_, id) = canal(&channel_id)?;
    a.mudar(g, move |cs, _| {
        let Some(i) = cs.iter().position(|x| x.id == id) else { return Ok(()) };
        if cs[i].kind == ChannelKind::Text && cs.iter().filter(|x| x.kind == ChannelKind::Text).count() <= 1 {
            return Err("O grupo precisa de pelo menos um canal de texto.".into());
        }
        cs.remove(i);
        Ok(())
    })
    .await?;
    Ok(true)
}

#[tauri::command]
pub async fn create_category(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    name: String,
) -> Res<String> {
    let a = ativa(&app, &rede, &c).await?;
    let n = nome_categoria(&name)?;
    let r = n.clone();
    a.mudar(grupo(&group_id)?, move |_, ks| {
        if ks.iter().any(|x| x.to_lowercase() == n.to_lowercase()) {
            return Err("Já há uma categoria com esse nome.".into());
        }
        ks.push(n);
        Ok(())
    })
    .await?;
    Ok(r)
}

#[tauri::command]
pub async fn rename_category(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    from: String,
    to: String,
) -> Res<String> {
    let a = ativa(&app, &rede, &c).await?;
    let n = nome_categoria(&to)?;
    let r = n.clone();
    a.mudar(grupo(&group_id)?, move |cs, ks| {
        if n != from && ks.iter().any(|x| x.to_lowercase() == n.to_lowercase()) {
            return Err("Já há uma categoria com esse nome.".into());
        }
        for k in ks.iter_mut() {
            if *k == from {
                *k = n.clone();
            }
        }
        for ch in cs.iter_mut() {
            if ch.category == from {
                ch.category = n.clone();
            }
        }
        Ok(())
    })
    .await?;
    Ok(r)
}

/// Apaga a categoria; os canais dela passam para a categoria de cima (ou a de baixo).
#[tauri::command]
pub async fn delete_category(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    name: String,
) -> Res<String> {
    let a = ativa(&app, &rede, &c).await?;
    a.mudar(grupo(&group_id)?, move |cs, ks| {
        if ks.len() <= 1 {
            return Err("O grupo precisa de pelo menos uma categoria.".into());
        }
        let i = ks.iter().position(|x| *x == name).ok_or("Essa categoria já não existe.")?;
        let para = if i > 0 { ks[i - 1].clone() } else { ks[i + 1].clone() };
        for ch in cs.iter_mut() {
            if ch.category == name {
                ch.category = para.clone();
            }
        }
        ks.remove(i);
        Ok(para)
    })
    .await
}

#[tauri::command]
pub async fn reorder_categories(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    order: Vec<String>,
) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    a.mudar(grupo(&group_id)?, move |_, ks| {
        let mut novo: Vec<String> = order.iter().filter(|x| ks.contains(x)).cloned().collect();
        novo.dedup();
        for k in ks.iter() {
            if !novo.contains(k) {
                novo.push(k.clone());
            }
        }
        *ks = novo;
        Ok(())
    })
    .await?;
    Ok(true)
}

#[derive(Deserialize)]
pub struct Posicao {
    id: String,
    cat: Option<String>,
}

/// Nova ordem dos canais, cada um com a sua categoria (texto e voz podem misturar-se).
#[tauri::command]
pub async fn reorder_channels(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    group_id: String,
    order: Vec<Posicao>,
) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    a.mudar(grupo(&group_id)?, move |cs, ks| {
        let mut resto = std::mem::take(cs);
        for p in order {
            let id = p.id.rsplit('/').next().unwrap_or(&p.id).to_string();
            if let Some(i) = resto.iter().position(|x| x.id == id) {
                let mut ch = resto.remove(i);
                if let Some(k) = p.cat.filter(|k| !k.trim().is_empty()) {
                    ch.category = k;
                    if !ks.contains(&ch.category) {
                        ks.push(ch.category.clone());
                    }
                }
                cs.push(ch);
            }
        }
        cs.extend(resto);
        Ok(())
    })
    .await?;
    Ok(true)
}

// ---------------- presença e perfil ----------------

/// Publica o teu perfil (nome da conta, cor e foto) nos grupos onde ainda está diferente.
#[tauri::command]
pub async fn publish_profile(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    color: String,
    photo: Option<String>,
    bio: Option<String>,
    banner: Option<String>,
    sync: Option<bool>,
) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let nome = c.nome().await.unwrap_or_else(|| "Eu".into());
    let photo = photo.filter(|p| p.starts_with("data:image/") && p.len() <= tasca_core::store::MAX_PHOTO_CHARS);
    let novo = tasca_core::store::Profile { color: color.chars().take(32).collect(), photo };
    let cartao = tasca_core::store::ProfileCard {
        username: crate::contas::ler_utilizador(&app),
        bio: bio.unwrap_or_default().trim().chars().take(tasca_core::amigos::MAX_BIO).collect(),
        banner: banner.filter(|b| {
            b.len() <= tasca_core::amigos::MAX_BANNER && (b.starts_with("data:image/") || b.starts_with('#'))
        }),
    };
    a.partilhar_perfil(&novo.color, &cartao.bio, &novo.photo, &cartao.banner, sync == Some(true));
    crate::amigos::definir_meu_perfil(
        &a.node,
        tasca_core::amigos::Perfil {
            nome: nome.clone(),
            utilizador: cartao.username.clone(),
            cor: novo.color.clone(),
            foto: novo.photo.clone(),
            banner: cartao.banner.clone(),
            bio: cartao.bio.clone(),
        },
    );
    for info in a.node.with_store(|s| s.groups()).map_err(txt)? {
        let g = info.id;
        let (perfil, nome_grupo) = a.node.with_store(|s| {
            let p = s.profiles(g).ok().and_then(|mut m| m.remove(&a.conta));
            let n = s.members(g).ok().and_then(|m| m.into_iter().find(|(k, _)| *k.as_bytes() == a.conta).map(|(_, n)| n));
            (p, n)
        });
        let cartao_grupo = a.node.with_store(|s| s.profile_cards(g).ok().and_then(|mut m| m.remove(&a.conta)));
        let perfil_igual = perfil.as_ref() == Some(&novo) && nome_grupo.as_deref() == Some(nome.as_str());
        let vazio = cartao == tasca_core::store::ProfileCard::default();
        let cartao_igual = cartao_grupo.as_ref() == Some(&cartao) || (cartao_grupo.is_none() && vazio);
        if perfil_igual && cartao_igual {
            continue;
        }
        let Ok(h) = a.handle(g) else { continue };
        if !h.is_connected() && !a.servidor {
            continue; // volta a tentar na próxima vez que a interface pedir
        }
        if !perfil_igual {
            if let Err(e) = h.set_profile(&nome, &novo.color, novo.photo.clone()).await {
                eprintln!("perfil no grupo {g}: {e:#}");
            }
        }
        if !cartao_igual {
            if let Err(e) = h.set_profile_card(cartao.clone()).await {
                eprintln!("cartão de perfil no grupo {g}: {e:#}");
            }
        }
    }
    Ok(true)
}

/// Sai de um grupo (ou apaga-o, se fores o dono) neste dispositivo. Os outros membros ficam
/// com a cópia deles. A rede reinicia para parar tudo o que estava ligado a este grupo.
#[tauri::command]
pub async fn leave_group(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>, group_id: String) -> Res<bool> {
    let a = ativa(&app, &rede, &c).await?;
    let g = grupo(&group_id)?;
    // avisa o grupo de que saíste (se não fores o dono)
    let dono = a.node.with_store(|s| s.group(g)).ok().flatten().is_some_and(|i| i.hosted || *i.owner.as_bytes() == a.conta);
    if !dono {
        if let Ok(h) = a.handle(g) {
            if h.is_connected() || a.servidor {
                let _ = tokio::time::timeout(Duration::from_secs(5), h.leave()).await;
            }
        }
    }
    a.node.forget_group(g).map_err(txt)?;
    a.sinc.lock().unwrap().esquecidos.push(group_id);
    a.guardar_sinc();
    if let Some(velha) = rede.ativa.lock().await.take() {
        velha.parar().await;
    }
    ativa(&app, &rede, &c).await?;
    Ok(true)
}

/// Guarda uma cópia de todos os grupos deste dispositivo na pasta Transferências.
#[tauri::command]
pub async fn backup_groups(app: AppHandle, rede: State<'_, Rede>, c: State<'_, Contas>) -> Res<String> {
    use tauri::Manager;
    let a = ativa(&app, &rede, &c).await?;
    let pasta = app.path().download_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&pasta).map_err(|e| e.to_string())?;
    let dia = {
        let s = now_ms() / 1000;
        let (d, m, y) = civil(s / 86400);
        format!("{y:04}-{m:02}-{d:02}")
    };
    let mut destino = pasta.join(format!("Tasca-grupos-{dia}.db"));
    let mut n = 1;
    while destino.exists() {
        destino = pasta.join(format!("Tasca-grupos-{dia} ({n}).db"));
        n += 1;
    }
    let d2 = destino.clone();
    a.node.with_store(|s| s.copy_to(&d2)).map_err(txt)?;
    #[cfg(desktop)]
    {
        use tauri_plugin_opener::OpenerExt;
        let _ = app.opener().reveal_item_in_dir(&destino);
    }
    Ok(destino.to_string_lossy().into_owned())
}

/// Dias desde 1970 → (dia, mês, ano).
fn civil(z: u64) -> (u64, u64, i64) {
    let z = z as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (d as u64, m as u64, y)
}
