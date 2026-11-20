//! Aplicação desktop da Tasca.
//!
//! A interface (pasta `ui/`) chama comandos Rust com `invoke`. Por agora só existe
//! `app_info`; os restantes comandos listados em `ui/api.js` ainda usam dados de teste
//! no próprio JavaScript até serem ligados ao `tasca-core`.

// Sem janela de consola no Windows em modo release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[derive(serde::Serialize)]
struct AppInfo {
    version: &'static str,
    core: bool,
}

/// Versão da app e se o núcleo P2P já está ligado.
#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        core: false,
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("erro ao arrancar a Tasca");
}
