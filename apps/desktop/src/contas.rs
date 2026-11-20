//! Comandos de conta chamados pelos ecrãs de `ui/auth.js`.
//!
//! A conta vive num cofre encriptado (`conta.cofre`) na pasta de dados da app. Enquanto a
//! app está aberta e a conta desbloqueada, a `Account` (com a chave privada) fica em
//! memória no estado da app; `lock` larga-a.
//!
//! Os erros do `tasca-core` vêm em português e chegam à interface como texto.

use std::time::Duration;

use serde::Serialize;
use tasca_core::google::{GoogleConfig, GoogleSession};
use tasca_core::servidor::{AccountServer, normalize_username};
use tasca_core::{Account, AccountStore};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

/// Estado das contas guardado na app.
///
/// É registado com `Builder::manage` antes de qualquer janela abrir, porque no Windows a
/// interface pode chamar comandos antes de o `setup` correr.
#[derive(Default)]
pub struct Contas {
    conta: Mutex<Option<Account>>,
    google: Mutex<Option<GoogleSession>>,
}

impl Contas {
    /// A conta desbloqueada, se houver. Para usar a identidade (por exemplo no `Node`).
    #[allow(dead_code)]
    pub async fn identidade(&self) -> Option<tasca_core::Identity> {
        self.conta.lock().await.as_ref().map(|a| a.identity.clone())
    }
}

type Res<T> = Result<T, String>;

/// O cofre vive na pasta de dados da app (no Windows, `%APPDATA%\pt.tasca.desktop`).
fn store(app: &AppHandle) -> Res<AccountStore> {
    let dir = app.path().app_data_dir().map_err(|e| format!("Pasta de dados indisponível: {e}"))?;
    Ok(AccountStore::new(dir))
}

/// Converte o erro para texto a mostrar, com a primeira letra maiúscula.
fn txt(e: anyhow::Error) -> String {
    eprintln!("erro de conta: {e:#}");
    let s = e.to_string();
    let mut c = s.chars();
    match c.next() {
        Some(p) => p.to_uppercase().chain(c).collect(),
        None => "Erro desconhecido".into(),
    }
}

/// Corre trabalho pesado (Argon2, disco) fora da thread da interface.
async fn bloqueante<T: Send + 'static>(f: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?.map_err(txt)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estado {
    exists: bool,
    unlocked: bool,
    display_name: Option<String>,
    public_key: Option<String>,
    /// Email da conta Google ligada nesta sessão.
    google: Option<String>,
    /// Esta versão tem servidor de contas (utilizador + palavra-passe).
    server: bool,
    /// Nome de utilizador no servidor de contas, se a conta lá estiver.
    username: Option<String>,
}

#[tauri::command]
pub async fn account_status(app: AppHandle, c: State<'_, Contas>) -> Res<Estado> {
    let store = store(&app)?;
    let vault = bloqueante(move || store.load()).await?;
    let unlocked = c.conta.lock().await.is_some();
    let google = c.google.lock().await.as_ref().map(|s| s.email.clone());
    Ok(Estado {
        exists: vault.is_some(),
        unlocked,
        display_name: vault.as_ref().map(|v| v.display_name.clone()),
        public_key: vault.as_ref().and_then(|v| v.public_key().ok()).map(|k| k.to_string()),
        google,
        server: AccountServer::from_env().is_some(),
        username: ler_utilizador(&app),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContaNova {
    recovery_phrase: String,
}

#[tauri::command]
pub async fn create_account(app: AppHandle, c: State<'_, Contas>, display_name: String, password: String) -> Res<ContaNova> {
    let store = store(&app)?;
    let (conta, frase) = bloqueante(move || store.create(display_name.trim(), &password)).await?;
    *c.conta.lock().await = Some(conta);
    Ok(ContaNova { recovery_phrase: frase.as_str().to_string() })
}

#[tauri::command]
pub async fn unlock(app: AppHandle, c: State<'_, Contas>, password: String) -> Res<bool> {
    let store = store(&app)?;
    let conta = bloqueante(move || store.unlock(&password)).await?;
    *c.conta.lock().await = Some(conta);
    Ok(true)
}

#[tauri::command]
pub async fn lock(c: State<'_, Contas>) -> Res<bool> {
    *c.conta.lock().await = None;
    Ok(true)
}

#[tauri::command]
pub async fn recover(app: AppHandle, c: State<'_, Contas>, phrase: String, display_name: String, new_password: String) -> Res<bool> {
    let store = store(&app)?;
    let conta = bloqueante(move || store.recover(&phrase, display_name.trim(), &new_password)).await?;
    *c.conta.lock().await = Some(conta);
    Ok(true)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginGoogle {
    email: String,
    has_backup: bool,
}

/// Abre o browser no login da Google e espera (até 5 minutos) que o utilizador termine.
async fn entrar_google(app: &AppHandle) -> Res<GoogleSession> {
    let config = GoogleConfig::from_env().ok_or("O login com Google ainda não está configurado nesta versão.")?;
    let pendente = config.begin_sign_in().await.map_err(txt)?;
    app.opener()
        .open_url(pendente.url.clone(), None::<&str>)
        .map_err(|e| format!("Não consegui abrir o browser: {e}"))?;
    pendente.finish(Duration::from_secs(300)).await.map_err(txt)
}

#[tauri::command]
pub async fn google_sign_in(app: AppHandle, c: State<'_, Contas>) -> Res<LoginGoogle> {
    let mut sessao = entrar_google(&app).await?;
    let has_backup = sessao.has_vault().await.map_err(txt)?;
    let email = sessao.email.clone();
    *c.google.lock().await = Some(sessao);
    Ok(LoginGoogle { email, has_backup })
}

/// Traz a conta guardada no Google Drive para este PC.
#[tauri::command]
pub async fn google_restore(app: AppHandle, c: State<'_, Contas>, password: String) -> Res<bool> {
    let vault = {
        let mut g = c.google.lock().await;
        let sessao = g.as_mut().ok_or("Entra primeiro com o Google.")?;
        sessao.download_vault().await.map_err(txt)?
    }
    .ok_or("Não há nenhuma conta da Tasca guardada neste Google Drive.")?;
    let store = store(&app)?;
    let conta = bloqueante(move || store.import(&vault, &password)).await?;
    *c.conta.lock().await = Some(conta);
    Ok(true)
}

#[derive(Serialize)]
pub struct Copia {
    email: String,
}

/// Guarda o cofre (já encriptado) no Google Drive.
#[tauri::command]
pub async fn google_backup(app: AppHandle, c: State<'_, Contas>) -> Res<Copia> {
    let store = store(&app)?;
    let vault = bloqueante(move || store.load())
        .await?
        .ok_or("Ainda não há conta neste PC.")?;
    let mut g = c.google.lock().await;
    // Conta criada na Tasca (sem Google): liga a conta Google agora.
    if g.is_none() {
        *g = Some(entrar_google(&app).await?);
    }
    let sessao = g.as_mut().expect("sessão Google acabada de criar");
    sessao.upload_vault(&vault).await.map_err(txt)?;
    Ok(Copia { email: sessao.email.clone() })
}

// ---------------- servidor de contas ----------------
//
// O servidor só guarda o cofre já cifrado e um verificador da palavra-passe. Abrir o
// cofre acontece sempre aqui no PC.

fn servidor() -> Res<AccountServer> {
    AccountServer::from_env().ok_or_else(|| "O servidor de contas ainda não está configurado nesta versão.".into())
}

fn ficheiro_utilizador(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("utilizador.txt"))
}

fn ler_utilizador(app: &AppHandle) -> Option<String> {
    let s = std::fs::read_to_string(ficheiro_utilizador(app)?).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

fn guardar_utilizador(app: &AppHandle, nome: &str) {
    if let Some(p) = ficheiro_utilizador(app) {
        if let Err(e) = std::fs::write(&p, nome) {
            eprintln!("não consegui guardar o nome de utilizador: {e}");
        }
    }
}

/// `true` se o nome de utilizador estiver livre. Erro se o nome for inválido.
#[tauri::command]
pub async fn username_available(username: String) -> Res<bool> {
    let nome = normalize_username(&username).map_err(txt)?;
    Ok(servidor()?.lookup(&nome).await.map_err(txt)?.is_none())
}

/// Põe a conta deste PC no servidor com um nome de utilizador. A palavra-passe tem de ser a
/// da conta (o cofre enviado é o que já está neste PC).
#[tauri::command]
pub async fn server_register(app: AppHandle, username: String, password: String) -> Res<String> {
    let srv = servidor()?;
    let nome = normalize_username(&username).map_err(txt)?;
    let store = store(&app)?;
    let pw = password.clone();
    let vault = bloqueante(move || {
        let v = store.load()?.ok_or_else(|| anyhow::anyhow!("ainda não há conta neste PC"))?;
        v.open(&pw)?; // confirma a palavra-passe antes de enviar
        Ok(v)
    })
    .await?;
    srv.register(&nome, &password, &vault).await.map_err(txt)?;
    guardar_utilizador(&app, &nome);
    Ok(nome)
}

/// Entra com utilizador e palavra-passe: traz a conta do servidor para este PC.
#[tauri::command]
pub async fn server_login(app: AppHandle, c: State<'_, Contas>, username: String, password: String) -> Res<bool> {
    let srv = servidor()?;
    let nome = normalize_username(&username).map_err(txt)?;
    let (vault, _) = srv.login(&nome, &password).await.map_err(txt)?;
    let store = store(&app)?;
    // Não apagar sem aviso uma conta que só existe neste PC.
    let local = { let s = store.clone(); bloqueante(move || s.load()).await? };
    if let Some(local) = local {
        let outra = local.public_key().ok() != vault.public_key().ok();
        if outra && ler_utilizador(&app).is_none() {
            return Err(format!(
                "Este PC já tem a conta {} e ela só existe aqui. Põe-na primeiro no servidor ou guarda as 12 palavras.",
                local.display_name
            ));
        }
    }
    let conta = bloqueante(move || store.import(&vault, &password)).await?;
    guardar_utilizador(&app, &nome);
    *c.conta.lock().await = Some(conta);
    Ok(true)
}

/// Grava as 12 palavras num `.txt` no ambiente de trabalho, a pedido do utilizador.
/// Nunca substitui um ficheiro que já exista. Devolve o caminho.
#[tauri::command]
pub async fn save_phrase_desktop(app: AppHandle, phrase: String, display_name: String) -> Res<String> {
    let palavras: Vec<&str> = phrase.split_whitespace().collect();
    if palavras.len() != 12 {
        return Err("Não há 12 palavras para guardar.".into());
    }
    let dir = app
        .path()
        .desktop_dir()
        .map_err(|e| format!("Não encontrei o ambiente de trabalho: {e}"))?;
    let mut texto = String::from("Tasca: palavras de recuperação\r\n");
    if !display_name.trim().is_empty() {
        texto += &format!("Conta: {}\r\n", display_name.trim());
    }
    texto += "\r\nGuarda-as em papel e apaga este ficheiro. Quem tiver estas palavras fica com a tua conta.\r\n\r\n";
    for (i, p) in palavras.iter().enumerate() {
        texto += &format!("{:>2}. {p}\r\n", i + 1);
    }
    for n in 1..100 {
        let nome = if n == 1 { "Tasca - 12 palavras.txt".to_string() } else { format!("Tasca - 12 palavras ({n}).txt") };
        let caminho = dir.join(nome);
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&caminho) {
            Ok(mut f) => {
                use std::io::Write;
                f.write_all(texto.as_bytes()).map_err(|e| format!("Não consegui escrever o ficheiro: {e}"))?;
                return Ok(caminho.display().to_string());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Não consegui criar o ficheiro: {e}")),
        }
    }
    Err("Já há demasiados ficheiros de palavras no ambiente de trabalho.".into())
}
