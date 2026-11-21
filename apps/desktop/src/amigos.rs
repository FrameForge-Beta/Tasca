//! Amigos: pedidos de amizade, lista, bloquear e estado online (ver `tasca_core::amigos`).
//!
//! Os pedidos vão cifrados pelo servidor de contas (caixa de pedidos de cada pessoa). O
//! estado online e o perfil vão diretamente entre os PCs dos amigos, pelo iroh. A lista
//! fica em `contas/<chave>/amigos.json`.
//!
//! Eventos para a interface: `friends_changed` (a lista mudou), `friend_request` (pedido
//! novo, `{ id, name }`) e `friend_presence` (`{ id, status, text }`).

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tasca_core::{
    Identity, PublicKey,
    amigos::{Aviso, AvisoAssinado, Estado, Perfil},
    event::now_ms,
    servidor::AccountServer,
};
use tauri::{AppHandle, Emitter, State, async_runtime::JoinHandle};
use tokio::sync::{Mutex, Notify};

use crate::{
    contas::{Contas, ler_utilizador, pasta_contas},
    grupos::{Ativa, Rede, ativa, cor, hex, txt, unhex32},
};

type Res<T> = Result<T, String>;

/// De quanto em quanto tempo pergunto aos amigos como estão.
const PING: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct Amigos {
    ativos: Mutex<Option<Arc<Ativos>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Rel {
    Amigo,
    /// Pedi eu, à espera.
    Enviado,
    /// Pediram-me, à espera de mim.
    Recebido,
    Bloqueado,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Pessoa {
    rel: Rel,
    nome: String,
    utilizador: Option<String>,
    /// Desde quando (amigos desde, ou quando chegou o pedido).
    desde: u64,
    /// Último aviso aplicado (ms do autor), para ignorar repetidos.
    #[serde(default)]
    ultimo: u64,
    #[serde(default)]
    perfil: Option<Perfil>,
    /// Quando mudou por ação minha (aqui ou noutro dispositivo da conta), em ms.
    #[serde(default)]
    mudou: u64,
}

impl Pessoa {
    /// Momento da última mudança, venha de um aviso ou de um dispositivo meu.
    fn marca(&self) -> u64 {
        self.ultimo.max(self.mudou)
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Dados {
    /// Último número lido da caixa de pedidos.
    seq: u64,
    pessoas: BTreeMap<String, Pessoa>,
    /// Último número lido da caixa da conta (outros dispositivos).
    #[serde(default)]
    seq_conta: u64,
    /// Já pus a lista toda na caixa da conta (na primeira vez com sincronização).
    #[serde(default)]
    publicado: bool,
    /// Quem saiu da lista e quando (para avisos antigos não o trazerem de volta).
    #[serde(default)]
    apagados: BTreeMap<String, u64>,
}

struct Ativos {
    app: AppHandle,
    rede: Arc<Ativa>,
    id: Identity,
    servidor: Option<AccountServer>,
    caminho: PathBuf,
    dados: StdMutex<Dados>,
    online: StdMutex<HashMap<[u8; 32], Estado>>,
    /// Acorda o ciclo de perguntas (amigo novo).
    acordar: Notify,
    conversas: Arc<crate::dms::Conversas>,
    tarefas: StdMutex<Vec<JoinHandle<()>>>,
}

fn chave(id: &str) -> Res<PublicKey> {
    let b = unhex32(id).ok_or("Pessoa desconhecida.")?;
    PublicKey::from_bytes(&b).map_err(|_| "Pessoa desconhecida.".to_string())
}

impl Ativos {
    fn guardar(&self) {
        let d = self.dados.lock().unwrap();
        match serde_json::to_vec_pretty(&*d) {
            Ok(b) => {
                if let Err(e) = std::fs::write(&self.caminho, b) {
                    eprintln!("não consegui guardar os amigos: {e}");
                }
            }
            Err(e) => eprintln!("amigos: {e}"),
        }
    }

    /// Passa a lista de amigos ao nó (quem pode ver o meu estado).
    fn sincronizar_lista(&self) {
        let lista: Vec<[u8; 32]> = self
            .dados
            .lock()
            .unwrap()
            .pessoas
            .iter()
            .filter(|(_, p)| p.rel == Rel::Amigo)
            .filter_map(|(k, _)| unhex32(k))
            .collect();
        self.rede.node.amigos().definir_lista(lista.clone());
        if let Some(srv) = &self.servidor {
            let amigos: Vec<(String, PublicKey)> =
                lista.iter().filter_map(|b| Some((hex(b), PublicKey::from_bytes(b).ok()?))).collect();
            self.conversas.vigiar(&self.app, srv, &self.id, &amigos);
        }
    }

    fn mudou(&self) {
        self.guardar();
        self.sincronizar_lista();
        let _ = self.app.emit("friends_changed", json!({}));
    }

    fn json(&self, k: &str, p: &Pessoa) -> Value {
        let b = unhex32(k).unwrap_or_default();
        let estado = self.online.lock().unwrap().get(&b).cloned();
        let perfil = p.perfil.clone().unwrap_or_default();
        json!({
            "id": k,
            "rel": p.rel,
            "name": if perfil.nome.is_empty() { p.nome.clone() } else { perfil.nome.clone() },
            "username": p.utilizador.clone().or(perfil.utilizador.clone()),
            "since": p.desde,
            "status": if p.rel == Rel::Amigo { estado.as_ref().map(|e| e.estado.as_str()).unwrap_or("offline") } else { "offline" },
            "statusText": estado.map(|e| e.mensagem).unwrap_or_default(),
            "color": if perfil.cor.is_empty() { cor(&b).to_string() } else { perfil.cor.clone() },
            "photo": perfil.foto,
            "banner": perfil.banner,
            "bio": perfil.bio,
        })
    }

    async fn enviar(&self, para: &PublicKey, aviso: Aviso) -> Res<()> {
        let srv = self.servidor.as_ref().ok_or("Os pedidos de amizade precisam do servidor de contas.")?;
        let nome = self.rede.node.amigos().perfil_eu().nome;
        let a = AvisoAssinado::assinar(&self.id, para, &nome, ler_utilizador(&self.app), aviso).map_err(txt)?;
        srv.amizade_enviar(&a, para).await.map_err(txt)?;
        Ok(())
    }

    /// Confirma no servidor que o @nome é mesmo desta chave.
    async fn confirmar_utilizador(&self, quem: &PublicKey, u: Option<String>) -> Option<String> {
        let (srv, u) = (self.servidor.as_ref()?, u?);
        let info = srv.lookup(&u).await.ok()??;
        (info.public_key == quem.to_string()).then_some(info.username)
    }

    async fn aplicar(&self, a: AvisoAssinado) {
        let Ok(quem) = a.autor() else { return };
        let k = hex(&a.corpo.de);
        let utilizador = self.confirmar_utilizador(&quem, a.corpo.utilizador.clone()).await;
        let mut responder = None;
        let mut pedido_novo = false;
        {
            let mut d = self.dados.lock().unwrap();
            let atual = d.pessoas.get(&k).cloned();
            let marca = atual.as_ref().map(Pessoa::marca).or_else(|| d.apagados.get(&k).copied()).unwrap_or(0);
            if marca >= a.corpo.em {
                return; // repetido, ou mais antigo do que a última mudança
            }
            let rel = atual.as_ref().map(|p| p.rel);
            let novo = match (a.corpo.aviso, rel) {
                (_, Some(Rel::Bloqueado)) => Some(Rel::Bloqueado),
                (Aviso::Pedido, Some(Rel::Enviado)) => {
                    responder = Some(Aviso::Aceite);
                    Some(Rel::Amigo)
                }
                (Aviso::Pedido, Some(Rel::Amigo)) => {
                    // perdeu a lista (PC novo?): volto a confirmar
                    responder = Some(Aviso::Aceite);
                    Some(Rel::Amigo)
                }
                (Aviso::Pedido, _) => {
                    pedido_novo = true;
                    Some(Rel::Recebido)
                }
                (Aviso::Aceite, Some(Rel::Enviado | Rel::Amigo)) => Some(Rel::Amigo),
                (Aviso::Aceite, other) => other,
                (Aviso::Recusado, Some(Rel::Enviado)) => None,
                (Aviso::Cancelado, Some(Rel::Recebido)) => None,
                (Aviso::Removido, Some(Rel::Amigo | Rel::Enviado)) => None,
                (_, other) => other,
            };
            match novo {
                Some(rel) => {
                    let desde = match &atual {
                        Some(p) if p.rel == rel => p.desde,
                        _ => now_ms(),
                    };
                    let p = d.pessoas.entry(k.clone()).or_insert(Pessoa {
                        rel,
                        nome: a.corpo.nome.clone(),
                        utilizador: None,
                        desde,
                        ultimo: 0,
                        perfil: None,
                        mudou: 0,
                    });
                    p.rel = rel;
                    p.desde = desde;
                    p.ultimo = a.corpo.em;
                    if !a.corpo.nome.trim().is_empty() {
                        p.nome = a.corpo.nome.clone();
                    }
                    if utilizador.is_some() {
                        p.utilizador = utilizador;
                    }
                }
                None => {
                    d.pessoas.remove(&k);
                }
            }
        }
        if let Some(r) = responder {
            if let Err(e) = self.enviar(&quem, r).await {
                eprintln!("amigos: resposta não enviada: {e}");
            }
        }
        if pedido_novo {
            let nome = self.dados.lock().unwrap().pessoas.get(&k).map(|p| p.nome.clone()).unwrap_or_default();
            let _ = self.app.emit("friend_request", json!({ "id": k, "name": nome }));
        }
        self.mudou();
        self.acordar.notify_one();
    }

    /// Mudei alguém da lista neste dispositivo: conto aos outros dispositivos da conta.
    fn partilhar(&self, k: &str) {
        let em = now_ms();
        let dados = {
            let mut d = self.dados.lock().unwrap();
            match d.pessoas.get_mut(k) {
                Some(p) => {
                    p.mudou = em.max(p.ultimo + 1);
                    let mut copia = p.clone();
                    copia.perfil = None; // o perfil vem do próprio amigo
                    serde_json::to_value(copia).ok()
                }
                None => {
                    d.apagados.insert(k.to_string(), em);
                    None
                }
            }
        };
        self.guardar();
        if let Some(srv) = self.servidor.clone() {
            crate::sincronia::publicar_depois(srv, self.id.clone(), crate::sincronia::Item::Pessoa { chave: k.to_string(), dados, em });
        }
    }

    /// Aplica uma mudança feita noutro dispositivo da conta. Devolve `true` se mudou algo.
    fn aplicar_conta(&self, chave: &str, dados: Option<Value>, em: u64) -> bool {
        if unhex32(chave).is_none() || chave == hex(self.id.public().as_bytes()) {
            return false;
        }
        let mut d = self.dados.lock().unwrap();
        let marca = d.pessoas.get(chave).map(Pessoa::marca).or_else(|| d.apagados.get(chave).copied()).unwrap_or(0);
        if marca >= em {
            return false;
        }
        match dados.and_then(|v| serde_json::from_value::<Pessoa>(v).ok()) {
            Some(mut p) => {
                p.perfil = d.pessoas.get(chave).and_then(|x| x.perfil.clone());
                p.mudou = em;
                d.apagados.remove(chave);
                d.pessoas.insert(chave.to_string(), p);
            }
            None => {
                d.pessoas.remove(chave);
                d.apagados.insert(chave.to_string(), em);
            }
        }
        true
    }

    /// Lê a caixa da conta (o que os outros dispositivos mudaram na lista) sem parar.
    async fn ler_conta(self: Arc<Self>) {
        let Some(srv) = self.servidor.clone() else { return };
        // primeira vez com sincronização: a lista deste dispositivo vai para a caixa
        let por_publicar: Option<Vec<(String, Pessoa)>> = {
            let d = self.dados.lock().unwrap();
            (!d.publicado).then(|| d.pessoas.iter().map(|(k, p)| (k.clone(), p.clone())).collect())
        };
        if let Some(lista) = por_publicar {
            let mut tudo = true;
            for (k, mut p) in lista {
                p.perfil = None;
                // O PC é a referência: a lista dele vale como sendo de agora. No telemóvel
                // fica com a data real, para não estragar o que o PC já disse.
                let em = if cfg!(mobile) { p.marca().max(1) } else { now_ms() };
                let item = crate::sincronia::Item::Pessoa { chave: k, dados: serde_json::to_value(p).ok(), em };
                if let Err(e) = crate::sincronia::publicar(&srv, &self.id, &item).await {
                    eprintln!("amigos: lista não publicada: {e:#}");
                    tudo = false;
                    break;
                }
            }
            if tudo {
                self.dados.lock().unwrap().publicado = true;
                self.guardar();
            }
        }
        loop {
            let desde = self.dados.lock().unwrap().seq_conta;
            match crate::sincronia::ler(&srv, &self.id, desde, 25).await {
                Ok((ultimo, itens)) => {
                    let mut mudou = false;
                    for it in itens {
                        if let crate::sincronia::Item::Pessoa { chave, dados, em } = it {
                            mudou |= self.aplicar_conta(&chave, dados, em);
                        }
                    }
                    if ultimo != desde {
                        self.dados.lock().unwrap().seq_conta = ultimo;
                        self.guardar();
                    }
                    if mudou {
                        self.mudou();
                        self.acordar.notify_one();
                    }
                }
                Err(e) => {
                    eprintln!("amigos: caixa da conta: {e:#}");
                    tokio::time::sleep(Duration::from_secs(15)).await;
                }
            }
        }
    }

    /// Lê a caixa de pedidos sem parar (o servidor segura o pedido até haver novidades).
    async fn ler_caixa(self: Arc<Self>) {
        let Some(srv) = self.servidor.clone() else { return };
        loop {
            let desde = self.dados.lock().unwrap().seq;
            match srv.amizade_ler(&self.id, desde, 25).await {
                Ok((ultimo, avisos)) => {
                    for a in avisos {
                        self.aplicar(a).await;
                    }
                    if ultimo != desde {
                        self.dados.lock().unwrap().seq = ultimo;
                        self.guardar();
                    }
                }
                Err(e) => {
                    eprintln!("amigos: caixa de pedidos: {e:#}");
                    tokio::time::sleep(Duration::from_secs(15)).await;
                }
            }
        }
    }

    /// Pergunta a cada amigo como está, de minuto a minuto (ou logo, com amigo novo).
    async fn perguntar(self: Arc<Self>) {
        loop {
            let amigos: Vec<PublicKey> = self
                .dados
                .lock()
                .unwrap()
                .pessoas
                .iter()
                .filter(|(_, p)| p.rel == Rel::Amigo)
                .filter_map(|(k, _)| chave(k).ok())
                .collect();
            for k in amigos {
                let node = self.rede.node.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = node.perguntar_amigo(k).await;
                });
            }
            tokio::select! {
                _ = tokio::time::sleep(PING) => {}
                _ = self.acordar.notified() => tokio::time::sleep(Duration::from_millis(300)).await,
            }
        }
    }

    /// Novidades dos amigos (estado e perfil) para a interface.
    async fn ouvir(self: Arc<Self>) {
        let mut rx = self.rede.node.amigos().subscrever();
        loop {
            let n = match rx.recv().await {
                Ok(n) => n,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => return,
            };
            let k = hex(&n.quem);
            let antes = self.online.lock().unwrap().get(&n.quem).cloned();
            match &n.estado {
                Some(e) => {
                    self.online.lock().unwrap().insert(n.quem, e.clone());
                }
                None => {
                    self.online.lock().unwrap().remove(&n.quem);
                }
            }
            if let Some(p) = n.perfil {
                if let Some(x) = self.dados.lock().unwrap().pessoas.get_mut(&k) {
                    x.perfil = Some(p);
                }
                self.guardar();
                let _ = self.app.emit("friends_changed", json!({}));
            }
            if antes != n.estado {
                let e = n.estado.unwrap_or_default();
                let status = if e.estado.is_empty() { "offline" } else { e.estado.as_str() };
                let _ = self.app.emit("friend_presence", json!({ "id": k, "status": status, "text": e.mensagem }));
            }
        }
    }
}

async fn ativos(app: &AppHandle, am: &Amigos, rede: &Rede, contas: &Contas) -> Res<Arc<Ativos>> {
    let r = ativa(app, rede, contas).await?;
    let mut g = am.ativos.lock().await;
    if let Some(a) = g.as_ref() {
        if a.rede.conta == r.conta && Arc::ptr_eq(&a.rede, &r) {
            return Ok(a.clone());
        }
    }
    if let Some(velho) = g.take() {
        for t in velho.tarefas.lock().unwrap().drain(..) {
            t.abort();
        }
        velho.conversas.parar();
    }
    let id = contas.identidade().await.ok_or("Desbloqueia a conta primeiro.")?;
    let caminho = pasta_contas(app)?.join(hex(&r.conta)).join("amigos.json");
    let dados: Dados = std::fs::read(&caminho)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    // perfis guardados: o nó não os volta a pedir se não mudaram
    for (k, p) in &dados.pessoas {
        if let (Some(b), Some(perfil)) = (unhex32(k), &p.perfil) {
            r.node.amigos().lembrar_perfil(b, perfil.clone());
        }
    }
    // até a interface mandar o perfil completo, pelo menos o nome e o @
    let mut eu = r.node.amigos().perfil_eu();
    if eu.nome.is_empty() {
        eu.nome = contas.nome().await.unwrap_or_default();
        eu.utilizador = ler_utilizador(app);
        r.node.amigos().definir_perfil(eu);
    }
    let conversas = Arc::new(crate::dms::Conversas::abrir(pasta_contas(app)?.join(hex(&r.conta)).join("conversas.json")));
    let a = Arc::new(Ativos {
        app: app.clone(),
        rede: r,
        id,
        servidor: AccountServer::from_env(),
        caminho,
        dados: StdMutex::new(dados),
        online: Default::default(),
        acordar: Notify::new(),
        conversas,
        tarefas: Default::default(),
    });
    a.sincronizar_lista();
    {
        let mut t = a.tarefas.lock().unwrap();
        t.push(tauri::async_runtime::spawn(a.clone().ler_caixa()));
        t.push(tauri::async_runtime::spawn(a.clone().ler_conta()));
        t.push(tauri::async_runtime::spawn(a.clone().perguntar()));
        t.push(tauri::async_runtime::spawn(a.clone().ouvir()));
    }
    *g = Some(a.clone());
    Ok(a)
}

/// O meu perfil para os amigos (chamado por `publish_profile`).
pub(crate) fn definir_meu_perfil(node: &tasca_core::Node, perfil: Perfil) {
    node.amigos().definir_perfil(perfil);
}

// ---------------- comandos ----------------

/// Amigos, pedidos (recebidos e enviados) e bloqueados.
#[tauri::command]
pub async fn list_friends(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>) -> Res<Vec<Value>> {
    let a = ativos(&app, &am, &rede, &c).await?;
    let d = a.dados.lock().unwrap();
    Ok(d.pessoas.iter().map(|(k, p)| a.json(k, p)).collect())
}

/// Pede amizade por @nome de utilizador ou pelo id (chave) de alguém de um grupo.
#[tauri::command]
pub async fn send_friend_request(
    app: AppHandle,
    am: State<'_, Amigos>,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    username: Option<String>,
    user_id: Option<String>,
    name: Option<String>,
) -> Res<Value> {
    let a = ativos(&app, &am, &rede, &c).await?;
    let srv = a.servidor.clone().ok_or("Os pedidos de amizade precisam do servidor de contas.")?;
    let (pk, utilizador) = match (username.filter(|u| !u.trim().is_empty()), user_id) {
        (Some(u), _) => {
            let info = srv
                .lookup(u.trim())
                .await
                .map_err(txt)?
                .ok_or_else(|| format!("Não há ninguém com o nome @{}.", u.trim().trim_start_matches('@').to_lowercase()))?;
            let pk: PublicKey = info.public_key.parse().map_err(|_| "Resposta estranha do servidor.".to_string())?;
            (pk, Some(info.username))
        }
        (None, Some(id)) => (chave(&id)?, None),
        _ => return Err("Escreve o nome de utilizador.".into()),
    };
    if pk == a.id.public() {
        return Err("Não te podes adicionar a ti próprio.".into());
    }
    let k = hex(pk.as_bytes());
    let rel = a.dados.lock().unwrap().pessoas.get(&k).map(|p| p.rel);
    let aviso = match rel {
        Some(Rel::Amigo) => return Err("Já são amigos.".into()),
        Some(Rel::Enviado) => return Err("Já pediste. Agora é esperar que aceite.".into()),
        Some(Rel::Bloqueado) => return Err("Bloqueaste esta pessoa. Desbloqueia primeiro.".into()),
        Some(Rel::Recebido) => Aviso::Aceite,
        None => Aviso::Pedido,
    };
    a.enviar(&pk, aviso).await?;
    {
        let mut d = a.dados.lock().unwrap();
        let p = d.pessoas.entry(k.clone()).or_insert(Pessoa {
            rel: Rel::Enviado,
            nome: name.clone().or(utilizador.clone()).unwrap_or_else(|| "Alguém".into()),
            utilizador: None,
            desde: now_ms(),
            ultimo: 0,
            perfil: None,
            mudou: 0,
        });
        p.rel = if aviso == Aviso::Aceite { Rel::Amigo } else { Rel::Enviado };
        p.desde = now_ms();
        if utilizador.is_some() {
            p.utilizador = utilizador;
        }
        if let Some(n) = name.filter(|n| !n.trim().is_empty()) {
            p.nome = n;
        }
    }
    a.partilhar(&k);
    a.mudou();
    a.acordar.notify_one();
    let d = a.dados.lock().unwrap();
    Ok(a.json(&k, &d.pessoas[&k]))
}

async fn responder(a: &Ativos, user_id: &str, quero: Rel, aviso: Aviso, depois: Option<Rel>) -> Res<bool> {
    let pk = chave(user_id)?;
    let rel = a.dados.lock().unwrap().pessoas.get(user_id).map(|p| p.rel);
    if rel != Some(quero) {
        return Err("Esse pedido já não existe.".into());
    }
    a.enviar(&pk, aviso).await?;
    {
        let mut d = a.dados.lock().unwrap();
        match depois {
            Some(r) => {
                if let Some(p) = d.pessoas.get_mut(user_id) {
                    p.rel = r;
                    p.desde = now_ms();
                }
            }
            None => {
                d.pessoas.remove(user_id);
            }
        }
    }
    a.partilhar(user_id);
    a.mudou();
    a.acordar.notify_one();
    Ok(true)
}

#[tauri::command]
pub async fn accept_friend(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>, user_id: String) -> Res<bool> {
    let a = ativos(&app, &am, &rede, &c).await?;
    responder(&a, &user_id, Rel::Recebido, Aviso::Aceite, Some(Rel::Amigo)).await
}

#[tauri::command]
pub async fn decline_friend(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>, user_id: String) -> Res<bool> {
    let a = ativos(&app, &am, &rede, &c).await?;
    responder(&a, &user_id, Rel::Recebido, Aviso::Recusado, None).await
}

#[tauri::command]
pub async fn cancel_friend(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>, user_id: String) -> Res<bool> {
    let a = ativos(&app, &am, &rede, &c).await?;
    responder(&a, &user_id, Rel::Enviado, Aviso::Cancelado, None).await
}

#[tauri::command]
pub async fn remove_friend(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>, user_id: String) -> Res<bool> {
    let a = ativos(&app, &am, &rede, &c).await?;
    a.online.lock().unwrap().remove(&unhex32(&user_id).unwrap_or_default());
    responder(&a, &user_id, Rel::Amigo, Aviso::Removido, None).await
}

/// Bloqueia: deixa de ser amigo, os pedidos dele são ignorados e não vê o teu estado.
#[tauri::command]
pub async fn block_user(
    app: AppHandle,
    am: State<'_, Amigos>,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    user_id: String,
    name: Option<String>,
) -> Res<bool> {
    let a = ativos(&app, &am, &rede, &c).await?;
    let pk = chave(&user_id)?;
    let antes = a.dados.lock().unwrap().pessoas.get(&user_id).map(|p| p.rel);
    let aviso = match antes {
        Some(Rel::Amigo) => Some(Aviso::Removido),
        Some(Rel::Enviado) => Some(Aviso::Cancelado),
        Some(Rel::Recebido) => Some(Aviso::Recusado),
        _ => None,
    };
    if let Some(av) = aviso {
        let _ = a.enviar(&pk, av).await;
    }
    {
        let mut d = a.dados.lock().unwrap();
        let p = d.pessoas.entry(user_id.clone()).or_insert(Pessoa {
            rel: Rel::Bloqueado,
            nome: name.unwrap_or_else(|| "Alguém".into()),
            utilizador: None,
            desde: now_ms(),
            ultimo: 0,
            perfil: None,
            mudou: 0,
        });
        p.rel = Rel::Bloqueado;
        p.desde = now_ms();
    }
    a.online.lock().unwrap().remove(pk.as_bytes());
    a.partilhar(&user_id);
    a.mudou();
    Ok(true)
}

#[tauri::command]
pub async fn unblock_user(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>, user_id: String) -> Res<bool> {
    let a = ativos(&app, &am, &rede, &c).await?;
    let mut d = a.dados.lock().unwrap();
    let estava = d.pessoas.get(&user_id).is_some_and(|p| p.rel == Rel::Bloqueado);
    if estava {
        d.pessoas.remove(&user_id);
    }
    drop(d);
    if estava {
        a.partilhar(&user_id);
    }
    a.mudou();
    Ok(true)
}

/// O meu estado para os amigos (online, ausente, não incomodar, invisível) e a mensagem.
#[tauri::command]
pub async fn set_presence(
    app: AppHandle,
    am: State<'_, Amigos>,
    rede: State<'_, Rede>,
    c: State<'_, Contas>,
    status: String,
    text: Option<String>,
) -> Res<bool> {
    let a = ativos(&app, &am, &rede, &c).await?;
    let estado = match status.as_str() {
        "online" | "away" | "dnd" | "offline" => status,
        _ => "online".into(),
    };
    let mensagem: String = text.unwrap_or_default().chars().take(60).collect();
    a.rede.node.amigos().definir_estado(Estado { estado, mensagem });
    a.acordar.notify_one(); // os amigos ficam a saber já
    Ok(true)
}

// ---------------- mensagens privadas ----------------

/// Conversas privadas (com mensagens), mais recentes primeiro.
#[tauri::command]
pub async fn list_dms(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>) -> Res<Vec<Value>> {
    let a = ativos(&app, &am, &rede, &c).await?;
    let amigos: Vec<String> = a.dados.lock().unwrap().pessoas.iter().filter(|(_, p)| p.rel == Rel::Amigo).map(|(k, _)| k.clone()).collect();
    Ok(a.conversas.lista(&amigos))
}

/// Mensagens da conversa com um amigo (e marca-as como lidas).
#[tauri::command]
pub async fn list_dm_messages(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>, user_id: String) -> Res<Vec<Value>> {
    let a = ativos(&app, &am, &rede, &c).await?;
    chave(&user_id)?;
    Ok(a.conversas.mensagens(&user_id))
}

/// A interface saiu da conversa (as mensagens novas voltam a contar como não lidas).
#[tauri::command]
pub async fn close_dm(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>) -> Res<bool> {
    let a = ativos(&app, &am, &rede, &c).await?;
    a.conversas.fechar();
    Ok(true)
}

#[tauri::command]
pub async fn send_dm(app: AppHandle, am: State<'_, Amigos>, rede: State<'_, Rede>, c: State<'_, Contas>, user_id: String, text: String) -> Res<Value> {
    let a = ativos(&app, &am, &rede, &c).await?;
    let pk = chave(&user_id)?;
    let rel = a.dados.lock().unwrap().pessoas.get(&user_id).map(|p| p.rel);
    if rel != Some(Rel::Amigo) {
        return Err("Só podes mandar mensagens privadas a amigos.".into());
    }
    let srv = a.servidor.clone().ok_or("As mensagens privadas precisam do servidor de contas.")?;
    a.conversas.enviar(&srv, &a.id, &user_id, &pk, &text).await
}
