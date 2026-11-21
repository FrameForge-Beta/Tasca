//! Comandos de conta chamados pelos ecrãs de `ui/auth.js`.
//!
//! Cada conta vive num cofre encriptado em `contas/<chave pública>/conta.cofre`, na pasta
//! de dados da app; `conta-atual.txt` diz qual está escolhida. Enquanto a app está aberta e
//! a conta desbloqueada, a `Account` (com a chave privada) fica em memória no estado da app.
//!
//! "Lembrar esta conta" guarda as 12 palavras no cofre de credenciais do sistema (no
//! Windows, o Gestor de Credenciais, cifrado com o login do Windows). Ao abrir a app, a
//! conta escolhida desbloqueia-se sozinha a partir daí. "Terminar sessão" apaga-as.
//!
//! Os erros do `tasca-core` vêm em português e chegam à interface como texto.

use std::path::{Path, PathBuf};
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

// ---------------- várias contas no mesmo PC ----------------

/// Pasta de dados da app (no Windows, `%APPDATA%\pt.tasca.desktop`).
fn base(app: &AppHandle) -> Res<PathBuf> {
    app.path().app_data_dir().map_err(|e| format!("Pasta de dados indisponível: {e}"))
}

fn pasta_contas(app: &AppHandle) -> Res<PathBuf> {
    Ok(base(app)?.join("contas"))
}

/// Id da conta escolhida (a sua chave pública), se houver.
fn ativa(app: &AppHandle) -> Option<String> {
    let s = std::fs::read_to_string(base(app).ok()?.join("conta-atual.txt")).ok()?;
    let s = s.trim().to_string();
    let valida = !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric());
    (valida && pasta_contas(app).ok()?.join(&s).join(COFRE).exists()).then_some(s)
}

fn definir_ativa(app: &AppHandle, id: Option<&str>) -> Res<()> {
    let f = base(app)?.join("conta-atual.txt");
    match id {
        Some(id) => {
            std::fs::create_dir_all(base(app)?).map_err(|e| e.to_string())?;
            std::fs::write(f, id).map_err(|e| format!("Não consegui guardar a conta escolhida: {e}"))
        }
        None => {
            let _ = std::fs::remove_file(f);
            Ok(())
        }
    }
}

const COFRE: &str = "conta.cofre";

/// Versões antigas guardavam uma só conta na raiz da pasta de dados: passa-a para `contas/`.
fn migrar(app: &AppHandle) -> Res<()> {
    let base = base(app)?;
    let antigo = base.join(COFRE);
    if !antigo.exists() {
        return Ok(());
    }
    let vault = AccountStore::new(&base).load().map_err(txt)?.ok_or("cofre antigo vazio")?;
    let id = id_de(&vault)?;
    let destino = pasta_contas(app)?.join(&id);
    std::fs::create_dir_all(&destino).map_err(|e| e.to_string())?;
    std::fs::rename(&antigo, destino.join(COFRE)).map_err(|e| e.to_string())?;
    let _ = std::fs::rename(base.join("utilizador.txt"), destino.join("utilizador.txt"));
    if ativa(app).is_none() {
        definir_ativa(app, Some(&id))?;
    }
    Ok(())
}

fn id_de(vault: &tasca_core::Vault) -> Res<String> {
    let pk = vault.public_key().map_err(txt)?;
    Ok(pk.as_bytes().iter().map(|b| format!("{b:02x}")).collect())
}

/// O cofre da conta escolhida. Sem conta escolhida, uma pasta vazia (não há conta).
fn store(app: &AppHandle) -> Res<AccountStore> {
    migrar(app)?;
    let id = ativa(app).unwrap_or_else(|| "_nenhuma".into());
    Ok(AccountStore::new(pasta_contas(app)?.join(id)))
}

/// Pasta temporária para uma conta que está a ser criada ou trazida de fora.
fn store_nova(app: &AppHandle) -> Res<AccountStore> {
    migrar(app)?;
    let dir = pasta_contas(app)?.join("_nova");
    let _ = std::fs::remove_dir_all(&dir);
    Ok(AccountStore::new(dir))
}

/// Põe a conta acabada de criar no seu lugar (substitui a mesma conta, se já existir) e
/// escolhe-a.
fn instalar(app: &AppHandle) -> Res<String> {
    let contas = pasta_contas(app)?;
    let nova = contas.join("_nova");
    let vault = AccountStore::new(&nova).load().map_err(txt)?.ok_or("a conta nova não ficou gravada")?;
    let id = id_de(&vault)?;
    let destino = contas.join(&id);
    std::fs::create_dir_all(&destino).map_err(|e| e.to_string())?;
    std::fs::rename(nova.join(COFRE), destino.join(COFRE)).map_err(|e| format!("Não consegui gravar a conta: {e}"))?;
    let _ = std::fs::remove_dir_all(&nova);
    definir_ativa(app, Some(&id))?;
    Ok(id)
}

// ---------------- lembrar a conta ----------------

fn chaveiro(id: &str) -> Option<keyring::Entry> {
    keyring::Entry::new("pt.tasca.desktop", id).ok()
}

/// Guarda (ou esquece) as palavras da conta no cofre de credenciais do sistema.
fn lembrar(app: &AppHandle, conta: &Account, sim: bool) {
    let Some(id) = ativa(app) else { return };
    let Some(e) = chaveiro(&id) else { return };
    if sim {
        match conta.recovery_phrase() {
            Ok(f) => {
                if let Err(x) = e.set_password(f.as_str()) {
                    eprintln!("não consegui lembrar a conta: {x}");
                }
            }
            Err(x) => eprintln!("não consegui lembrar a conta: {x}"),
        }
    } else {
        let _ = e.delete_credential();
    }
}

fn lembrada(id: &str) -> bool {
    chaveiro(id).is_some_and(|e| e.get_password().is_ok())
}

/// Desbloqueia sozinha a conta escolhida, se estiver lembrada.
fn abrir_lembrada(app: &AppHandle, vault: &tasca_core::Vault) -> Option<Account> {
    let id = ativa(app)?;
    let frase = zeroize::Zeroizing::new(chaveiro(&id)?.get_password().ok()?);
    let conta = Account::recover(&frase, &vault.display_name).ok()?;
    // só serve se as palavras forem mesmo desta conta
    (vault.public_key().ok()? == conta.identity.public()).then_some(conta)
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
    /// A conta abre sozinha neste PC.
    remembered: bool,
}

#[tauri::command]
pub async fn account_status(app: AppHandle, c: State<'_, Contas>) -> Res<Estado> {
    let store = store(&app)?;
    let vault = bloqueante(move || store.load()).await?;
    {
        let mut g = c.conta.lock().await;
        if g.is_none() {
            if let Some(v) = &vault {
                *g = abrir_lembrada(&app, v);
            }
        }
    }
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
        remembered: ativa(&app).is_some_and(|id| lembrada(&id)),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContaNova {
    recovery_phrase: String,
}

#[tauri::command]
pub async fn create_account(
    app: AppHandle,
    c: State<'_, Contas>,
    display_name: String,
    password: String,
    remember: Option<bool>,
) -> Res<ContaNova> {
    let store = store_nova(&app)?;
    let (conta, frase) = bloqueante(move || store.create(display_name.trim(), &password)).await?;
    instalar(&app)?;
    lembrar(&app, &conta, remember.unwrap_or(false));
    *c.conta.lock().await = Some(conta);
    Ok(ContaNova { recovery_phrase: frase.as_str().to_string() })
}

#[tauri::command]
pub async fn unlock(app: AppHandle, c: State<'_, Contas>, password: String, remember: Option<bool>) -> Res<bool> {
    let store = store(&app)?;
    let conta = bloqueante(move || store.unlock(&password)).await?;
    lembrar(&app, &conta, remember.unwrap_or(false));
    *c.conta.lock().await = Some(conta);
    Ok(true)
}

#[tauri::command]
pub async fn lock(c: State<'_, Contas>) -> Res<bool> {
    *c.conta.lock().await = None;
    Ok(true)
}

#[tauri::command]
pub async fn recover(
    app: AppHandle,
    c: State<'_, Contas>,
    phrase: String,
    display_name: String,
    new_password: String,
    remember: Option<bool>,
) -> Res<bool> {
    let store = store_nova(&app)?;
    let conta = bloqueante(move || store.recover(&phrase, display_name.trim(), &new_password)).await?;
    instalar(&app)?;
    lembrar(&app, &conta, remember.unwrap_or(false));
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
pub async fn google_restore(app: AppHandle, c: State<'_, Contas>, password: String, remember: Option<bool>) -> Res<bool> {
    let vault = {
        let mut g = c.google.lock().await;
        let sessao = g.as_mut().ok_or("Entra primeiro com o Google.")?;
        sessao.download_vault().await.map_err(txt)?
    }
    .ok_or("Não há nenhuma conta da Tasca guardada neste Google Drive.")?;
    let store = store_nova(&app)?;
    let conta = bloqueante(move || store.import(&vault, &password)).await?;
    instalar(&app)?;
    lembrar(&app, &conta, remember.unwrap_or(false));
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

fn ficheiro_utilizador(app: &AppHandle) -> Option<PathBuf> {
    Some(pasta_contas(app).ok()?.join(ativa(app)?).join("utilizador.txt"))
}

fn utilizador_em(dir: &Path) -> Option<String> {
    let s = std::fs::read_to_string(dir.join("utilizador.txt")).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
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
pub async fn server_login(
    app: AppHandle,
    c: State<'_, Contas>,
    username: String,
    password: String,
    remember: Option<bool>,
) -> Res<bool> {
    let srv = servidor()?;
    let nome = normalize_username(&username).map_err(txt)?;
    let (vault, _) = srv.login(&nome, &password).await.map_err(txt)?;
    // A conta entra ao lado das que já existem neste PC; nenhuma é apagada.
    let store = store_nova(&app)?;
    let conta = bloqueante(move || store.import(&vault, &password)).await?;
    instalar(&app)?;
    guardar_utilizador(&app, &nome);
    lembrar(&app, &conta, remember.unwrap_or(false));
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

// ---------------- trocar de conta e terminar sessão ----------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContaGuardada {
    id: String,
    display_name: String,
    username: Option<String>,
    remembered: bool,
    active: bool,
}

/// As contas guardadas neste PC.
#[tauri::command]
pub async fn list_accounts(app: AppHandle) -> Res<Vec<ContaGuardada>> {
    migrar(&app)?;
    let dir = pasta_contas(&app)?;
    let atual = ativa(&app);
    let mut out = Vec::new();
    let Ok(entradas) = std::fs::read_dir(&dir) else { return Ok(out) };
    for e in entradas.flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        if id.starts_with('_') {
            continue;
        }
        let Ok(Some(v)) = AccountStore::new(e.path()).load() else { continue };
        out.push(ContaGuardada {
            display_name: v.display_name.clone(),
            username: utilizador_em(&e.path()),
            remembered: lembrada(&id),
            active: atual.as_deref() == Some(id.as_str()),
            id,
        });
    }
    out.sort_by_key(|c| c.display_name.to_lowercase());
    Ok(out)
}

/// Escolhe outra conta deste PC (ou nenhuma, para entrar com uma nova). Fecha a atual.
#[tauri::command]
pub async fn select_account(app: AppHandle, c: State<'_, Contas>, id: Option<String>) -> Res<bool> {
    if let Some(id) = &id {
        let ok = !id.starts_with('_')
            && id.chars().all(|ch| ch.is_ascii_alphanumeric())
            && pasta_contas(&app)?.join(id).join(COFRE).exists();
        if !ok {
            return Err("Essa conta já não está neste PC.".into());
        }
    }
    definir_ativa(&app, id.as_deref())?;
    *c.conta.lock().await = None;
    *c.google.lock().await = None;
    Ok(true)
}

/// Termina a sessão: fecha a conta e esquece-a (da próxima vez pede a palavra-passe).
/// A conta continua guardada neste PC.
#[tauri::command]
pub async fn sign_out(app: AppHandle, c: State<'_, Contas>) -> Res<bool> {
    if let Some(e) = ativa(&app).and_then(|id| chaveiro(&id)) {
        let _ = e.delete_credential();
    }
    *c.conta.lock().await = None;
    *c.google.lock().await = None;
    Ok(true)
}

/// Nomes de utilizador livres parecidos com o que a pessoa escreveu (quando está ocupado).
#[tauri::command]
pub async fn suggest_usernames(username: String) -> Res<Vec<String>> {
    let srv = servidor()?;
    let mut base: String = username
        .trim()
        .trim_start_matches('@')
        .to_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '.')
        .collect();
    base = base.trim_matches('.').chars().take(19).collect();
    if base.chars().count() < 3 {
        base = format!("{base}tasca");
    }
    let mut r = [0u8; 6];
    let _ = getrandom::fill(&mut r);
    let mut candidatos = vec![
        format!("{base}_"),
        format!("{base}.pt"),
        format!("{base}{}", 10 + r[0] % 90),
        format!("o.{base}"),
        format!("{base}_{}", 100 + (u16::from(r[1]) * 4) % 900),
        format!("{base}{}", 2000 + u16::from(r[2] % 27)),
        format!("{base}.{}", 10 + r[3] % 90),
    ];
    candidatos.retain(|c| normalize_username(c).is_ok());
    candidatos.dedup();
    let mut livres = Vec::new();
    for c in candidatos {
        if livres.len() == 4 {
            break;
        }
        if let Ok(None) = srv.lookup(&c).await {
            livres.push(c);
        }
    }
    Ok(livres)
}
