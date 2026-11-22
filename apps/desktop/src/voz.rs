//! Chamadas de voz a sério.
//!
//! O microfone e a saída de som ficam na interface (Web Audio): assim a escolha de
//! dispositivos das definições é a mesma, e o cancelamento de eco do sistema funciona.
//! A cada 20 ms a interface manda o bloco do microfone (`voice_frame`) e recebe o bloco
//! já misturado dos outros para tocar. Aqui fica o resto:
//!
//! microfone → supressão de ruído (DeepFilterNet ou RNNoise) → sensibilidade → Opus →
//! anfitrião (iroh) → outros participantes → buffer de jitter → Opus → mistura → colunas.
//!
//! Eventos para a interface: `voice_activity` (quem está a falar), `voice_status` (ping,
//! rota, ligação perdida) e `group_changed` quando muda quem está na sala.

use std::sync::{
    Arc, Mutex as StdMutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::Duration;

use serde_json::json;
use tasca_core::{Chamada, GroupId};
use tasca_voz::{Captura, FRAME, Filtro, Mistura, Ruido};
use tauri::{
    AppHandle, Emitter, State,
    async_runtime::JoinHandle,
    ipc::{Channel, InvokeResponseBody},
};
use tokio::sync::Mutex;

use crate::contas::Contas;
use crate::grupos::{Ativa, Rede, ativa, canal, hex};

type Res<T> = Result<T, String>;

/// A chamada em curso (no máximo uma).
#[derive(Default)]
pub struct Voz {
    atual: Mutex<Option<Arc<EmChamada>>>,
    /// Quem nos ligou há pouco, e de onde (para atender ligando a esse dispositivo).
    toques: StdMutex<std::collections::HashMap<String, (tasca_core::EndpointAddr, std::time::Instant)>>,
}

/// Alguém está a ligar-te: a interface mostra o ecrã de atender e o sistema avisa.
pub fn tocou(app: &AppHandle, t: tasca_core::voz::Toque) {
    use tauri::Manager;
    let id = hex(&t.de);
    if let Some(v) = app.try_state::<Voz>() {
        let mut m = v.toques.lock().unwrap();
        m.retain(|_, (_, quando)| quando.elapsed() < Duration::from_secs(120));
        m.insert(id.clone(), (t.endpoint, std::time::Instant::now()));
    }
    let _ = app.emit("incoming_call", json!({ "userId": id }));
    crate::avisos::mensagem(app, &format!("dm-{id}"), None, "Chamada", "Estão a ligar-te na Tasca");
}

struct EmChamada {
    canal: String,
    group: GroupId,
    /// A ligação à sala. Troca quando volta a ligar depois de a ligação cair.
    chamada: StdMutex<Option<Arc<Chamada>>>,
    captura: StdMutex<Captura>,
    mistura: StdMutex<Mistura<[u8; 32]>>,
    muted: AtomicBool,
    camera: AtomicBool,
    screen: AtomicBool,
    /// Para onde vão as imagens de vídeo dos outros (canal aberto pela interface).
    video: StdMutex<Option<Channel<InvokeResponseBody>>>,
    deaf: AtomicBool,
    blocos: AtomicU32,
    tarefas: StdMutex<Vec<JoinHandle<()>>>,
    app: AppHandle,
    ativa: Arc<Ativa>,
}

impl Drop for EmChamada {
    fn drop(&mut self) {
        for t in self.tarefas.lock().unwrap().drain(..) {
            t.abort();
        }
    }
}

fn modo(nome: Option<&str>) -> Ruido {
    match nome {
        Some("off") => Ruido::Desligado,
        Some("light") => Ruido::Leve,
        _ => Ruido::Forte,
    }
}

async fn filtro(m: Ruido) -> Filtro {
    // Carregar o modelo do DeepFilterNet leva um bocado: fora da linha do áudio.
    let r = tauri::async_runtime::spawn_blocking(move || Filtro::novo(m)).await;
    match r {
        Ok(Ok(f)) => f,
        Ok(Err(e)) => {
            eprintln!("supressão de ruído indisponível ({e:#}); a usar o modo leve");
            Filtro::novo(Ruido::Leve).unwrap_or(Filtro::Desligado)
        }
        Err(_) => Filtro::Desligado,
    }
}

impl EmChamada {
    fn chamada(&self) -> Arc<Chamada> {
        self.chamada.lock().unwrap().clone().expect("a chamada liga-se antes de ser usada")
    }

    /// Liga as tarefas de uma ligação à sala: pacotes → mistura, lista da sala → interface.
    /// As tarefas da ligação antiga acabam sozinhas quando ela cai.
    fn ligar(self: &Arc<Self>, mut c: Chamada) {
        let pacotes = c.pacotes();
        let imagens = c.imagens();
        let c = Arc::new(c);
        *self.chamada.lock().unwrap() = Some(c.clone());
        let mut t = self.tarefas.lock().unwrap();

        // Vídeo dos outros → interface: `tamanho do id: u8` + id da pessoa + imagem.
        if let Some(mut rx) = imagens {
            let me = Arc::downgrade(self);
            t.push(tauri::async_runtime::spawn(async move {
                while let Some(img) = rx.recv().await {
                    let Some(me) = me.upgrade() else { break };
                    let Some(ch) = me.video.lock().unwrap().clone() else { continue };
                    let id = me.ativa.pessoa_id(&img.from);
                    let mut out = Vec::with_capacity(1 + id.len() + img.data.len());
                    out.push(id.len() as u8);
                    out.extend_from_slice(id.as_bytes());
                    out.extend_from_slice(&img.data);
                    let _ = ch.send(InvokeResponseBody::Raw(out));
                }
            }));
        }

        if let Some(mut rx) = pacotes {
            let me = Arc::downgrade(self);
            t.push(tauri::async_runtime::spawn(async move {
                while let Some(p) = rx.recv().await {
                    let Some(me) = me.upgrade() else { break };
                    if me.deaf.load(Ordering::Relaxed) {
                        continue;
                    }
                    let r = me.mistura.lock().unwrap().receber(p.from, p.seq, p.data.to_vec());
                    if let Err(e) = r {
                        eprintln!("voz: pacote ignorado: {e:#}");
                    }
                }
            }));
        }

        // Quem está na sala mudou: a interface volta a ler os canais.
        let mut sala = c.sala();
        let (me, gid) = (Arc::downgrade(self), hex(&self.group.0));
        t.push(tauri::async_runtime::spawn(async move {
            while sala.changed().await.is_ok() {
                let Some(me) = me.upgrade() else { break };
                let l = sala.borrow_and_update().clone();
                me.mistura.lock().unwrap().manter(|k| l.iter().any(|p| p.key == *k));
                let _ = me.app.emit("group_changed", json!({ "groupId": gid }));
                let estados: Vec<_> = l
                    .iter()
                    .map(|p| {
                        json!({
                            "id": me.ativa.pessoa_id(&p.key), "muted": p.muted, "deaf": p.deaf, "camera": p.camera,
                            "screen": p.screen, "host": p.anfitriao,
                            // o dispositivo de onde está (muda quando passa do PC para o telemóvel)
                            "device": hex(&p.endpoint[..8]),
                            // o PC usa a chave da conta; o telemóvel tem uma chave própria
                            "mobile": p.endpoint != p.key,
                        })
                    })
                    .collect();
                let _ = me.app.emit("voice_members", json!({ "channelId": me.canal, "members": estados }));
            }
        }));
    }

    fn estado_ligacao(&self) -> serde_json::Value {
        let c = self.chamada();
        match c.ligacao() {
            None if c.ligada() => json!({ "route": "estás a alojar a chamada", "ping": 0, "connected": true, "hosting": true }),
            None => json!({ "route": "sem ligação", "ping": 0, "connected": false }),
            Some((rtt, relay)) => json!({
                "route": if relay { "pelo relay do iroh" } else { "direto P2P" },
                "ping": rtt.as_millis() as u64,
                "connected": c.ligada(),
            }),
        }
    }

    /// Vigia a ligação: mostra o ping, segue a chamada quando muda de anfitrião e volta a
    /// entrar na sala se a ligação cair.
    fn vigiar(self: &Arc<Self>) {
        let me = Arc::downgrade(self);
        let h = tauri::async_runtime::spawn(async move {
            let mut espera = 1;
            loop {
                let (mut novo, mut fora) = match me.upgrade() {
                    Some(eu) => {
                        let c = eu.chamada();
                        (c.novo_host(), c.noutro_dispositivo())
                    }
                    None => break,
                };
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                    _ = novo.wait_for(|n| n.is_some()) => {}
                    _ = fora.wait_for(|f| *f) => {}
                }
                let Some(eu) = me.upgrade() else { break };
                // Entraste nesta chamada noutro dispositivo (por exemplo no telemóvel): aqui
                // acaba, sem voltar a entrar (senão os dois tiravam-se um ao outro).
                if *fora.borrow() {
                    use tauri::Manager;
                    if let Some(v) = eu.app.try_state::<Voz>() {
                        let mut atual = v.atual.lock().await;
                        if atual.as_ref().is_some_and(|x| Arc::ptr_eq(x, &eu)) {
                            atual.take();
                        }
                    }
                    let _ = eu.app.emit("voice_moved", json!({ "channelId": eu.canal }));
                    break;
                }
                let _ = eu.app.emit("voice_status", eu.estado_ligacao());
                let c = eu.chamada();
                let destino = c.novo_host().borrow().clone();
                if destino.is_none() && c.ligada() {
                    espera = 1;
                    continue;
                }
                // Para onde ir: quem o anfitrião indicou ou, se a ligação caiu sem aviso, quem
                // fica com a chamada (todos escolhem a mesma pessoa).
                let alvo = destino.or_else(|| c.sucessor()).and_then(|n| n.addr().ok());
                let (node, g, room) = (eu.ativa.node.clone(), eu.group, c.room.clone());
                drop((eu, c));
                let r = match alvo {
                    Some(alvo) => {
                        let _ = me.upgrade().map(|eu| eu.app.emit("voice_status", json!({ "route": "a mudar de anfitrião", "ping": 0, "connected": false })));
                        match node.entrar_voz_em(g, &room, alvo, Duration::from_secs(12)).await {
                            Ok(c) => Ok(c),
                            Err(e) => {
                                eprintln!("voz: não consegui seguir a chamada: {e:#}");
                                node.entrar_voz(g, &room).await
                            }
                        }
                    }
                    None => {
                        tokio::time::sleep(Duration::from_secs(espera)).await;
                        espera = (espera * 2).min(30);
                        node.entrar_voz(g, &room).await
                    }
                };
                if let Ok(c) = r {
                    let Some(eu) = me.upgrade() else { break };
                    let (m, d) = (eu.muted.load(Ordering::Relaxed), eu.deaf.load(Ordering::Relaxed));
                    let _ = c.set_estado(m, d).await;
                    let (cam, ecra) = (eu.camera.load(Ordering::Relaxed), eu.screen.load(Ordering::Relaxed));
                    if cam || ecra {
                        let _ = c.set_video(cam, ecra).await;
                    }
                    eu.ligar(c);
                    let _ = eu.app.emit("voice_status", eu.estado_ligacao());
                }
            }
        });
        self.tarefas.lock().unwrap().push(h);
    }
}

/// Entra numa sala de voz. Sai da anterior, se estavas noutra.
#[tauri::command]
pub async fn join_voice(
    app: AppHandle,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    voz: State<'_, Voz>,
    channel_id: String,
    noise: Option<String>,
    sensitivity: Option<u8>,
) -> Res<serde_json::Value> {
    let a = ativa(&app, &rede, &c).await?;
    // Chamada privada com um amigo: "dm-<chave>".
    let privada = channel_id.strip_prefix("dm-").map(|k| {
        let pk = crate::grupos::unhex32(k).and_then(|b| tasca_core::PublicKey::from_bytes(&b).ok());
        (k.to_string(), pk)
    });
    let (g, sala) = match &privada {
        Some((_, Some(pk))) => (tasca_core::voz::grupo_privado(&a.node.identity().public(), pk), "privada".to_string()),
        Some((_, None)) => return Err("Pessoa desconhecida.".into()),
        None => canal(&channel_id)?,
    };
    let mut atual = voz.atual.lock().await;
    if let Some(velha) = atual.take() {
        if velha.canal == channel_id {
            let info = velha.estado_ligacao();
            *atual = Some(velha);
            return Ok(info);
        }
    }
    // O modelo da supressão de ruído carrega ao mesmo tempo que se procura a sala.
    let onde = match &privada {
        Some((k, Some(_))) => voz.toques.lock().unwrap().remove(k).map(|(e, _)| e),
        _ => None,
    };
    let entrar = async {
        match &privada {
            Some((_, Some(pk))) => a.node.ligar_privado(*pk, onde).await,
            _ => a.node.entrar_voz(g, &sala).await,
        }
    };
    let (f, chamada) = tokio::join!(filtro(modo(noise.as_deref())), entrar);
    let chamada = chamada.map_err(|e| {
        eprintln!("voz: {e:#}");
        format!("Não consegui entrar na chamada: {e}")
    })?;
    let mut captura = Captura::new(f).map_err(|e| e.to_string())?;
    captura.sensibilidade = sensitivity.unwrap_or(0).min(100);
    // O dono do grupo (se estiver ligado) mostra aos outros em que sala estás.
    if let (None, Ok(h)) = (&privada, a.handle(g)) {
        let s = sala.clone();
        tauri::async_runtime::spawn(async move {
            let _ = h.set_voice(Some(s)).await;
        });
    }
    let em = Arc::new(EmChamada {
        canal: channel_id,
        group: g,
        chamada: StdMutex::new(None),
        captura: StdMutex::new(captura),
        mistura: StdMutex::new(Mistura::default()),
        muted: AtomicBool::new(false),
        camera: AtomicBool::new(false),
        screen: AtomicBool::new(false),
        video: StdMutex::new(None),
        deaf: AtomicBool::new(false),
        blocos: AtomicU32::new(0),
        tarefas: Default::default(),
        app: app.clone(),
        ativa: a,
    });
    em.ligar(chamada);
    em.vigiar();
    let mut info = em.estado_ligacao();
    info["codec"] = json!("Opus 48 kbps");
    *atual = Some(em);
    Ok(info)
}

#[tauri::command]
pub async fn leave_voice(voz: State<'_, Voz>) -> Res<bool> {
    if let Some(em) = voz.atual.lock().await.take() {
        if let Ok(h) = em.ativa.handle(em.group) {
            tauri::async_runtime::spawn(async move {
                let _ = h.set_voice(None).await;
            });
        }
    }
    Ok(true)
}

/// Passa a chamada para outra pessoa que lá está. Quem aloja pode sempre; o dono do grupo
/// também (o anfitrião da chamada ignora os outros).
#[tauri::command]
pub async fn pass_voice_host(voz: State<'_, Voz>, user_id: String) -> Res<bool> {
    let em = voz.atual.lock().await.clone().ok_or("Não estás numa chamada.")?;
    let c = em.chamada();
    let para = c
        .sala()
        .borrow()
        .iter()
        .find(|p| em.ativa.pessoa_id(&p.key) == user_id)
        .map(|p| p.id)
        .ok_or("Essa pessoa não está na chamada.")?;
    c.passar_host(para).await.map_err(|e| e.to_string())?;
    Ok(true)
}

/// Câmara ou partilha de ecrã ligada: os outros passam a ver o teu quadrado com vídeo.
#[tauri::command]
pub async fn set_video_state(voz: State<'_, Voz>, camera: bool, screen: bool) -> Res<bool> {
    let Some(em) = voz.atual.lock().await.clone() else { return Ok(false) };
    em.camera.store(camera, Ordering::Relaxed);
    em.screen.store(screen, Ordering::Relaxed);
    em.chamada().set_video(camera, screen).await.map_err(|e| e.to_string())?;
    Ok(true)
}

/// Abre o canal por onde chegam as imagens de vídeo dos outros (bytes: tamanho do id,
/// id da pessoa, imagem tal como quem a enviou a codificou).
#[tauri::command]
pub async fn video_subscribe(voz: State<'_, Voz>, channel: Channel<InvokeResponseBody>) -> Res<bool> {
    let Some(em) = voz.atual.lock().await.clone() else { return Ok(false) };
    *em.video.lock().unwrap() = Some(channel);
    Ok(true)
}

/// Uma imagem de vídeo da tua câmara ou do teu ecrã, já codificada pela interface.
/// O primeiro byte tem o bit 0 ligado nas imagens chave.
#[tauri::command]
pub async fn video_send(request: tauri::ipc::Request<'_>, voz: State<'_, Voz>) -> Res<bool> {
    let Some(body) = crate::corpo::bytes(request.body()) else { return Err("imagem inválida".into()) };
    if body.len() < 2 || body.len() > 8 * 1024 * 1024 {
        return Err("imagem com tamanho inválido".into());
    }
    let Some(em) = voz.atual.lock().await.clone() else { return Ok(false) };
    em.chamada().enviar_video(bytes::Bytes::copy_from_slice(&body));
    Ok(true)
}

/// Sai da chamada e fecha as ligações com cuidado, para os outros verem logo que saíste
/// (ao fechar a app).
pub async fn sair_tudo(voz: &Voz) {
    voz.atual.lock().await.take();
}

/// Sem microfone (`muted`) ou sem som (`deaf`): os outros veem os ícones.
#[tauri::command]
pub async fn set_voice_state(voz: State<'_, Voz>, muted: bool, deaf: bool) -> Res<bool> {
    let Some(em) = voz.atual.lock().await.clone() else { return Ok(false) };
    em.muted.store(muted || deaf, Ordering::Relaxed);
    em.deaf.store(deaf, Ordering::Relaxed);
    if deaf {
        em.mistura.lock().unwrap().manter(|_| false);
    }
    em.chamada().set_estado(muted || deaf, deaf).await.map_err(|e| e.to_string())?;
    Ok(true)
}

/// Volume de uma pessoa só para ti (0 a 2; 1 é o normal, 0 silencia).
#[tauri::command]
pub async fn set_user_volume(voz: State<'_, Voz>, user_id: String, volume: f32) -> Res<bool> {
    let Some(k) = crate::grupos::unhex32(&user_id) else { return Ok(false) };
    let Some(em) = voz.atual.lock().await.clone() else { return Ok(false) };
    em.mistura.lock().unwrap().set_volume(k, volume);
    Ok(true)
}

/// Muda a supressão de ruído ("deep", "light" ou "off") e a sensibilidade a meio da chamada.
#[tauri::command]
pub async fn set_voice_options(voz: State<'_, Voz>, noise: Option<String>, sensitivity: Option<u8>) -> Res<bool> {
    let Some(em) = voz.atual.lock().await.clone() else { return Ok(false) };
    if let Some(s) = sensitivity {
        em.captura.lock().unwrap().sensibilidade = s.min(100);
    }
    if let Some(n) = noise {
        let m = modo(Some(&n));
        if em.captura.lock().unwrap().modo() != m {
            let f = filtro(m).await;
            em.captura.lock().unwrap().set_filtro(f);
        }
    }
    Ok(true)
}

/// 20 ms de áudio: entra o microfone, sai a mistura dos outros.
///
/// Corpo (bytes): `flags: u8` (bit 0 = transmitir), 3 bytes a zero, 960 `f32` little-endian.
/// Resposta: `flags: u8` (bit 0 = estou a transmitir), nível do microfone (0-100), 2 bytes a
/// zero, 960 `f32` para tocar.
#[tauri::command]
pub async fn voice_frame(request: tauri::ipc::Request<'_>, voz: State<'_, Voz>) -> Res<tauri::ipc::Response> {
    let Some(body) = crate::corpo::bytes(request.body()) else { return Err("bloco de áudio inválido".into()) };
    let mut resp = vec![0u8; 4 + FRAME * 4];
    let Some(em) = voz.atual.lock().await.clone() else { return Ok(tauri::ipc::Response::new(resp)) };
    if body.len() != 4 + FRAME * 4 {
        return Err("bloco de áudio com tamanho errado".into());
    }
    let transmitir = body[0] & 1 == 1 && !em.muted.load(Ordering::Relaxed);
    let mut pcm: Vec<f32> = body[4..].chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();

    let cap = em.captura.lock().unwrap().processar(&mut pcm, transmitir);
    let mut a_falar = false;
    match cap {
        Ok(c) => {
            resp[1] = c.nivel;
            if let Some(p) = c.pacote {
                a_falar = true;
                em.chamada().enviar(&p);
            }
        }
        Err(e) => eprintln!("voz: microfone: {e:#}"),
    }
    resp[0] = a_falar as u8;

    let mut out = vec![0f32; FRAME];
    if !em.deaf.load(Ordering::Relaxed) {
        em.mistura.lock().unwrap().misturar(&mut out);
    }
    for (d, s) in resp[4..].chunks_exact_mut(4).zip(&out) {
        d.copy_from_slice(&s.to_le_bytes());
    }

    // Quem está a falar, cinco vezes por segundo.
    if em.blocos.fetch_add(1, Ordering::Relaxed) % 10 == 0 {
        let mut quem: Vec<String> = em.mistura.lock().unwrap().a_falar().iter().map(|k| em.ativa.pessoa_id(k)).collect();
        if a_falar {
            quem.push("me".into());
        }
        let _ = em.app.emit("voice_activity", json!({ "speaking": quem }));
    }
    Ok(tauri::ipc::Response::new(resp))
}
