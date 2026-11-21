//! Mensagens privadas entre amigos, cifradas de ponta a ponta.
//!
//! Cada conversa tem uma caixa no servidor de contas cuja chave sai de uma troca
//! Diffie-Hellman entre as identidades dos dois (`ChaveCaixa::privada`): o servidor guarda
//! blocos cifrados e não sabe de quem são. Cada mensagem vai assinada pelo autor. Só se
//! leem as caixas das conversas com amigos, por isso só amigos te escrevem.
//!
//! As mensagens ficam guardadas em `contas/<chave>/conversas.json`. Para a interface, uma
//! conversa é `dm-<chave do amigo>`; mensagens novas chegam pelo evento `message`.

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Mutex as StdMutex,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tasca_core::{ChaveCaixa, EventKind, Identity, PublicKey, SignedEvent, servidor::AccountServer};
use tauri::{AppHandle, Emitter, async_runtime::JoinHandle};

use crate::grupos::{hex, txt};

type Res<T> = Result<T, String>;

const CANAL: &str = "dm";
const MAX_TEXTO: usize = 4000;

#[derive(Clone, Serialize, Deserialize)]
struct Msg {
    id: String,
    ts: u64,
    /// `me` ou a chave (hex) do amigo.
    from: String,
    text: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Conversa {
    /// Último número lido da caixa.
    seq: u64,
    /// Até onde já li (número de mensagens).
    lidas: usize,
    msgs: Vec<Msg>,
}

pub(crate) struct Conversas {
    caminho: PathBuf,
    dados: StdMutex<BTreeMap<String, Conversa>>,
    vigias: StdMutex<HashMap<String, JoinHandle<()>>>,
    /// Conversa aberta agora na interface (não conta como não lida).
    aberta: StdMutex<Option<String>>,
}

impl Conversas {
    /// Para de ler as caixas (troca de conta).
    pub(crate) fn parar(&self) {
        for (_, t) in self.vigias.lock().unwrap().drain() {
            t.abort();
        }
    }

    pub(crate) fn abrir(caminho: PathBuf) -> Self {
        let dados = std::fs::read(&caminho).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self { caminho, dados: StdMutex::new(dados), vigias: Default::default(), aberta: Default::default() }
    }

    fn guardar(&self) {
        let d = self.dados.lock().unwrap();
        if let Ok(b) = serde_json::to_vec(&*d) {
            if let Err(e) = std::fs::write(&self.caminho, b) {
                eprintln!("não consegui guardar as conversas: {e}");
            }
        }
    }

    fn msg_json(m: &Msg) -> Value {
        json!({ "id": m.id, "ts": m.ts, "from": m.from, "text": m.text })
    }

    /// Junta uma mensagem (se ainda não estiver). Devolve `true` se era nova.
    fn juntar(&self, amigo: &str, seq: Option<u64>, m: Msg) -> bool {
        let mut d = self.dados.lock().unwrap();
        let c = d.entry(amigo.to_string()).or_default();
        if let Some(s) = seq {
            c.seq = c.seq.max(s);
        }
        if c.msgs.iter().any(|x| x.id == m.id) {
            return false;
        }
        let aberta = self.aberta.lock().unwrap().as_deref() == Some(amigo);
        let pos = c.msgs.iter().rposition(|x| x.ts <= m.ts).map(|i| i + 1).unwrap_or(0);
        c.msgs.insert(pos, m);
        if aberta {
            c.lidas = c.msgs.len();
        }
        true
    }

    /// Conversas para a barra lateral: amigos e quem já tem mensagens.
    pub(crate) fn lista(&self, amigos: &[String]) -> Vec<Value> {
        let d = self.dados.lock().unwrap();
        let mut out: Vec<(u64, Value)> = Vec::new();
        let mut ids: Vec<&String> = d.keys().collect();
        for a in amigos {
            if !d.contains_key(a) {
                ids.push(a);
            }
        }
        for k in ids {
            let c = d.get(k);
            let last = c.and_then(|c| c.msgs.last());
            // amigo sem mensagens só aparece na lista de amigos, não aqui
            let Some(last) = last else { continue };
            let unread = c
                .map(|c| c.msgs[c.lidas.min(c.msgs.len())..].iter().filter(|m| m.from != "me").count())
                .unwrap_or(0);
            out.push((
                last.ts,
                json!({ "id": format!("dm-{k}"), "with": k, "unread": unread, "last": Self::msg_json(last) }),
            ));
        }
        out.sort_by(|a, b| b.0.cmp(&a.0));
        out.into_iter().map(|(_, v)| v).collect()
    }

    pub(crate) fn mensagens(&self, amigo: &str) -> Vec<Value> {
        *self.aberta.lock().unwrap() = Some(amigo.to_string());
        let mut d = self.dados.lock().unwrap();
        let Some(c) = d.get_mut(amigo) else { return Vec::new() };
        c.lidas = c.msgs.len();
        let out = c.msgs.iter().map(Self::msg_json).collect();
        drop(d);
        self.guardar();
        out
    }

    pub(crate) fn fechar(&self) {
        *self.aberta.lock().unwrap() = None;
    }

    pub(crate) async fn enviar(&self, srv: &AccountServer, eu: &Identity, amigo: &str, outro: &PublicKey, texto: &str) -> Res<Value> {
        let texto = texto.trim();
        if texto.is_empty() {
            return Err("Mensagem vazia.".into());
        }
        if texto.chars().count() > MAX_TEXTO {
            return Err("Mensagem demasiado longa.".into());
        }
        let chave = ChaveCaixa::privada(eu, outro).map_err(txt)?;
        let ev = SignedEvent::sign(eu, None, EventKind::Message { channel: CANAL.into(), text: texto.into() });
        let seq = srv.caixa_enviar(&chave, &ev).await.map_err(txt)?;
        let m = Msg { id: hex(&ev.id()), ts: ev.body.timestamp_ms, from: "me".into(), text: texto.into() };
        self.juntar(amigo, None, m.clone());
        // o número desta mensagem não conta como lido: a vigia ainda tem de ler as anteriores
        let _ = seq;
        self.guardar();
        Ok(Self::msg_json(&m))
    }

    /// Garante uma vigia (leitura contínua da caixa) para cada amigo, e só para amigos.
    pub(crate) fn vigiar(self: &std::sync::Arc<Self>, app: &AppHandle, srv: &AccountServer, eu: &Identity, amigos: &[(String, PublicKey)]) {
        let mut v = self.vigias.lock().unwrap();
        v.retain(|k, t| {
            let fica = amigos.iter().any(|(a, _)| a == k);
            if !fica {
                t.abort();
            }
            fica
        });
        for (k, pk) in amigos {
            if v.contains_key(k) {
                continue;
            }
            let (me, app, srv, eu, k2, pk) = (self.clone(), app.clone(), srv.clone(), eu.clone(), k.clone(), *pk);
            v.insert(k.clone(), tauri::async_runtime::spawn(async move { me.ler(app, srv, eu, k2, pk).await }));
        }
    }

    async fn ler(self: std::sync::Arc<Self>, app: AppHandle, srv: AccountServer, eu: Identity, amigo: String, outro: PublicKey) {
        let Ok(chave) = ChaveCaixa::privada(&eu, &outro) else { return };
        let meu = *eu.public().as_bytes();
        loop {
            let desde = self.dados.lock().unwrap().get(&amigo).map(|c| c.seq).unwrap_or(0);
            match srv.caixa_ler(&chave, desde, 25).await {
                Ok(evs) => {
                    let mut novas = false;
                    for (seq, ev) in evs {
                        let autor = ev.body.author;
                        let EventKind::Message { channel, text } = &ev.body.kind else { continue };
                        if channel != CANAL || (autor != meu && autor != *outro.as_bytes()) || ev.body.group.is_some() {
                            continue;
                        }
                        let from = if autor == meu { "me".to_string() } else { amigo.clone() };
                        let m = Msg { id: hex(&ev.id()), ts: ev.body.timestamp_ms, from: from.clone(), text: text.chars().take(MAX_TEXTO).collect() };
                        let json = Self::msg_json(&m);
                        // as minhas também: podem ter sido escritas noutro dispositivo da conta (se foram
                        // escritas aqui, `juntar` já as tinha e devolve `false`)
                        if self.juntar(&amigo, Some(seq), m) {
                            let _ = app.emit("message", json!({ "channelId": format!("dm-{amigo}"), "message": json }));
                        }
                        novas = true;
                    }
                    if novas {
                        self.guardar();
                        let _ = app.emit("dms_changed", json!({}));
                    }
                }
                Err(e) => {
                    eprintln!("conversa com {}: {e:#}", &amigo[..8.min(amigo.len())]);
                    tokio::time::sleep(Duration::from_secs(15)).await;
                }
            }
        }
    }
}
