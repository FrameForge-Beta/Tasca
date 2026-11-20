//! Aplicação desktop da Tasca.
//!
//! Ao abrir aparece o ecrã de arranque (`ui/splash.html`), que procura uma versão nova
//! no GitHub. Se houver, transfere-a, instala-a e reinicia a app; se não houver (ou não
//! houver internet), fecha o ecrã de arranque e mostra a janela principal.
//!
//! A interface (pasta `ui/`) chama comandos Rust com `invoke`. Já existem `app_info` e
//! os comandos de conta (`contas.rs`); os restantes comandos listados em `ui/api.js`
//! ainda usam dados de teste no próprio JavaScript até serem ligados ao `tasca-core`.

// Sem janela de consola no Windows em modo release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod contas;

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
struct AppInfo {
    version: String,
    core: bool,
}

/// Versão da app e se o núcleo P2P já está ligado.
#[tauri::command]
fn app_info(app: AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        core: false,
    }
}

/// Estado mostrado no ecrã de arranque.
#[derive(Clone, Serialize)]
#[serde(tag = "fase", rename_all = "snake_case")]
enum Arranque {
    AProcurar,
    ATransferir { versao: String, feito: u64, total: Option<u64> },
    AInstalar { versao: String },
    Pronto,
}

fn estado(app: &AppHandle, e: Arranque) {
    let _ = app.emit_to("splash", "arranque", e);
}

/// Procura e instala uma atualização. Devolve `true` se a app vai reiniciar.
async fn atualizar(app: &AppHandle) -> tauri_plugin_updater::Result<bool> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(false);
    };
    let versao = update.version.clone();
    let mut feito = 0u64;
    update
        .download_and_install(
            |n, total| {
                feito += n as u64;
                estado(app, Arranque::ATransferir { versao: versao.clone(), feito, total });
            },
            || estado(app, Arranque::AInstalar { versao: versao.clone() }),
        )
        .await?;
    Ok(true)
}

fn mostrar_app(app: &AppHandle) {
    estado(app, Arranque::Pronto);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        // Antes das janelas: no Windows a interface chama comandos de conta logo ao abrir.
        .manage(contas::Contas::default())
        .invoke_handler(tauri::generate_handler![
            app_info,
            contas::account_status,
            contas::create_account,
            contas::unlock,
            contas::lock,
            contas::recover,
            contas::google_sign_in,
            contas::google_restore,
            contas::google_backup,
            contas::username_available,
            contas::server_register,
            contas::server_login,
            contas::save_phrase_desktop,
        ])
        .setup(|app| {
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // deixa o ecrã de arranque desenhar-se antes de começar
                tokio::time::sleep(Duration::from_millis(400)).await;
                estado(&app, Arranque::AProcurar);
                // em modo de desenvolvimento não há versões publicadas para comparar
                let reinicia = if cfg!(debug_assertions) {
                    false
                } else {
                    match tokio::time::timeout(Duration::from_secs(20), atualizar(&app)).await {
                        Ok(Ok(r)) => r,
                        // sem internet, GitHub em baixo, etc.: arranca na mesma
                        Ok(Err(e)) => {
                            eprintln!("atualização falhou: {e}");
                            false
                        }
                        Err(_) => false,
                    }
                };
                if reinicia {
                    app.restart();
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                mostrar_app(&app);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao arrancar a Tasca");
}
