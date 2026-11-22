//! Loja de plugins.
//!
//! O catálogo vive no repositório da Tasca (`plugins/catalogo.json`), assinado pela equipa
//! com Ed25519 (`catalogo.json.sig`). A app só aceita um catálogo com a assinatura certa e
//! só instala um plugin se o SHA-256 do ficheiro for o do catálogo. Também se podem
//! instalar plugins de um ficheiro (ficam marcados como não verificados).
//!
//! Os plugins correm na interface, cada um num iframe isolado (ver `ui/plugins.js`).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, State};

type Res<T> = Result<T, String>;

const BASE: &str = "https://raw.githubusercontent.com/RubenFernandes2004/Tasca/main/plugins/";
/// Chave pública da equipa que assina o catálogo.
const CHAVE_LOJA: &str = "2b5b939fdbad3ee65bec303160c15170773775644e739c387d3031154248a73b";
const MAX_PLUGIN: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Entrada {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub color: String,
    pub icon: String,
    pub desc: String,
    pub perms: Vec<String>,
    pub file: String,
    pub sha256: String,
    pub size: usize,
}

#[derive(Deserialize)]
struct Catalogo {
    plugins: Vec<Entrada>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct Instalado {
    entrada: Entrada,
    enabled: bool,
    verified: bool,
}

#[derive(Default)]
pub struct Loja {
    catalogo: Mutex<Option<(std::time::Instant, Vec<Entrada>)>>,
}

fn pasta(app: &AppHandle) -> Res<PathBuf> {
    let p = app.path().app_data_dir().map_err(|e| e.to_string())?.join("plugins");
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    Ok(p)
}

fn ler_estado(app: &AppHandle) -> BTreeMap<String, Instalado> {
    pasta(app)
        .ok()
        .and_then(|p| std::fs::read(p.join("instalados.json")).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn gravar_estado(app: &AppHandle, e: &BTreeMap<String, Instalado>) -> Res<()> {
    let p = pasta(app)?.join("instalados.json");
    std::fs::write(&p, serde_json::to_vec_pretty(e).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn id_valido(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

async fn obter(url: &str) -> Res<Vec<u8>> {
    let c = tasca_core::http_builder().timeout(std::time::Duration::from_secs(15)).build().map_err(|e| e.to_string())?;
    let r = c.get(url).send().await.map_err(|e| format!("Sem ligação à loja: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("A loja respondeu {}", r.status()));
    }
    Ok(r.bytes().await.map_err(|e| e.to_string())?.to_vec())
}

/// Confirma que o catálogo foi assinado pela equipa.
fn verificar(texto: &[u8], sig_hex: &str) -> Res<()> {
    let dec = |s: &str| -> Option<Vec<u8>> {
        let s = s.trim();
        (0..s.len()).step_by(2).map(|i| s.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok())).collect()
    };
    let chave: [u8; 32] = dec(CHAVE_LOJA).and_then(|v| v.try_into().ok()).ok_or("chave da loja inválida")?;
    let sig: [u8; 64] = dec(sig_hex).and_then(|v| v.try_into().ok()).ok_or("assinatura do catálogo inválida")?;
    let pk = tasca_core::PublicKey::from_bytes(&chave).map_err(|e| e.to_string())?;
    pk.verify(texto, &tasca_core::Signature::from_bytes(&sig))
        .map_err(|_| "O catálogo da loja não tem a assinatura da equipa. Não instalei nada.".to_string())
}

async fn catalogo(loja: &Loja) -> Res<Vec<Entrada>> {
    if let Some((quando, l)) = loja.catalogo.lock().unwrap().clone() {
        if quando.elapsed() < std::time::Duration::from_secs(300) {
            return Ok(l);
        }
    }
    let texto = obter(&format!("{BASE}catalogo.json")).await?;
    let sig = obter(&format!("{BASE}catalogo.json.sig")).await?;
    verificar(&texto, &String::from_utf8_lossy(&sig))?;
    let c: Catalogo = serde_json::from_slice(&texto).map_err(|e| format!("Catálogo ilegível: {e}"))?;
    let l: Vec<Entrada> = c.plugins.into_iter().filter(|p| id_valido(&p.id)).collect();
    *loja.catalogo.lock().unwrap() = Some((std::time::Instant::now(), l.clone()));
    Ok(l)
}

fn json_de(e: &Entrada, inst: Option<&Instalado>, catalogo: Option<&Entrada>) -> Value {
    json!({
        "id": e.id, "name": e.name, "author": e.author, "version": e.version,
        "color": if e.color.is_empty() { "#2A5CAA" } else { &e.color },
        "icon": if e.icon.is_empty() { "puzzle" } else { &e.icon },
        "desc": e.desc, "perms": e.perms, "runtime": "JS",
        "verified": inst.map(|i| i.verified).unwrap_or(true),
        "installed": inst.is_some(),
        "enabled": inst.is_some_and(|i| i.enabled),
        "update": match (inst, catalogo) { (Some(i), Some(c)) => i.entrada.version != c.version, _ => false },
        "installs": "",
    })
}

#[tauri::command]
pub async fn list_plugins(app: AppHandle, loja: State<'_, Loja>) -> Res<Vec<Value>> {
    let est = ler_estado(&app);
    let cat = match catalogo(&loja).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("loja: {e}");
            Vec::new()
        }
    };
    let mut out: Vec<Value> = cat.iter().map(|c| json_de(c, est.get(&c.id), Some(c))).collect();
    for (id, i) in &est {
        if !cat.iter().any(|c| &c.id == id) {
            out.push(json_de(&i.entrada, Some(i), None));
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn install_plugin(app: AppHandle, loja: State<'_, Loja>, id: String) -> Res<Value> {
    // sempre a lista fresca: a versão e o hash têm de ser os de agora
    *loja.catalogo.lock().unwrap() = None;
    let cat = catalogo(&loja).await?;
    let e = cat.iter().find(|c| c.id == id).cloned().ok_or("Esse plugin já não está na loja.")?;
    if e.file.contains("..") || e.file.starts_with('/') {
        return Err("Plugin com caminho inválido.".into());
    }
    let codigo = obter(&format!("{BASE}{}", e.file)).await?;
    if codigo.len() > MAX_PLUGIN {
        return Err("Plugin demasiado grande.".into());
    }
    let hash: String = Sha256::digest(&codigo).iter().map(|b| format!("{b:02x}")).collect();
    if hash != e.sha256.to_lowercase() {
        return Err("O ficheiro do plugin não bate certo com o catálogo assinado. Não o instalei.".into());
    }
    std::fs::write(pasta(&app)?.join(format!("{id}.js")), &codigo).map_err(|e| e.to_string())?;
    let mut est = ler_estado(&app);
    est.insert(id.clone(), Instalado { entrada: e.clone(), enabled: true, verified: true });
    gravar_estado(&app, &est)?;
    Ok(json_de(&e, est.get(&id), Some(&e)))
}

/// Instala um plugin a partir de um ficheiro (não verificado pela equipa).
#[tauri::command]
pub async fn sideload_plugin(app: AppHandle, name: String, code: String) -> Res<Value> {
    if code.len() > MAX_PLUGIN {
        return Err("Plugin demasiado grande.".into());
    }
    let base: String = name
        .trim_end_matches(".js")
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(40)
        .collect();
    let id = format!("local-{}", if base.is_empty() { "plugin".into() } else { base });
    std::fs::write(pasta(&app)?.join(format!("{id}.js")), code.as_bytes()).map_err(|e| e.to_string())?;
    let e = Entrada {
        id: id.clone(),
        name: name.trim_end_matches(".js").to_string(),
        author: "instalado de um ficheiro".into(),
        version: "local".into(),
        color: "#6B4C9A".into(),
        icon: "puzzle".into(),
        desc: "Plugin instalado à mão. A equipa da Tasca não o reviu.".into(),
        perms: ["comandos", "ler mensagens", "enviar mensagens", "guardar dados"].map(String::from).to_vec(),
        ..Default::default()
    };
    let mut est = ler_estado(&app);
    est.insert(id.clone(), Instalado { entrada: e.clone(), enabled: true, verified: false });
    gravar_estado(&app, &est)?;
    Ok(json_de(&e, est.get(&id), None))
}

#[tauri::command]
pub async fn uninstall_plugin(app: AppHandle, id: String) -> Res<Value> {
    let mut est = ler_estado(&app);
    let i = est.remove(&id).ok_or("Esse plugin não está instalado.")?;
    gravar_estado(&app, &est)?;
    if id_valido(&id) {
        let _ = std::fs::remove_file(pasta(&app)?.join(format!("{id}.js")));
    }
    Ok(json_de(&i.entrada, None, None))
}

#[tauri::command]
pub async fn set_plugin_enabled(app: AppHandle, id: String, enabled: bool) -> Res<Value> {
    let mut est = ler_estado(&app);
    let i = est.get_mut(&id).ok_or("Esse plugin não está instalado.")?;
    i.enabled = enabled;
    let v = json_de(&i.entrada.clone(), Some(&i.clone()), None);
    gravar_estado(&app, &est)?;
    Ok(v)
}

/// O código de um plugin instalado e ativo, com as permissões que pediu.
#[tauri::command]
pub async fn plugin_code(app: AppHandle, id: String) -> Res<Value> {
    if !id_valido(&id) {
        return Err("Plugin inválido.".into());
    }
    let est = ler_estado(&app);
    let i = est.get(&id).filter(|i| i.enabled).ok_or("Esse plugin não está ativo.")?;
    let code = std::fs::read_to_string(pasta(&app)?.join(format!("{id}.js"))).map_err(|e| e.to_string())?;
    Ok(json!({ "id": id, "code": code, "perms": i.entrada.perms, "name": i.entrada.name }))
}

#[cfg(test)]
mod testes {
    #[test]
    fn catalogo_do_repositorio_esta_assinado() {
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugins/");
        let texto = std::fs::read(format!("{base}catalogo.json")).unwrap();
        let sig = std::fs::read_to_string(format!("{base}catalogo.json.sig")).unwrap();
        super::verificar(&texto, &sig).unwrap();
        let mut mexido = texto.clone();
        mexido[10] ^= 1;
        assert!(super::verificar(&mexido, &sig).is_err());
    }
}
