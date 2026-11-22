//! Aplicação da Tasca (Windows e Android; Linux e macOS compilam também).
//!
//! No PC, ao abrir aparece o ecrã de arranque (`ui/splash.html`), que procura uma versão nova
//! no GitHub. Se houver, transfere-a, instala-a e reinicia a app; se não houver (ou não
//! houver internet), fecha o ecrã de arranque e mostra a janela principal.
//!
//! A interface (pasta `ui/`) chama comandos Rust com `invoke`: `app_info`, os comandos de
//! conta (`contas.rs`) e os de grupos, canais e mensagens (`grupos.rs`, ligados ao
//! `tasca-core`), e os de voz (`voz.rs`, com o `tasca-voz`). Os restantes comandos
//! listados em `ui/api.js` (plugins, cópias) ainda usam dados de teste no próprio JavaScript.

mod amigos;
mod avisos;
mod contas;
mod dms;
mod grupos;
mod mensagens;
mod plugins;
mod presenca;
mod sincronia;
mod voz;

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
#[cfg(desktop)]
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
        core: true,
    }
}

/// Telemóvel: abre o APK mais recente (o Android descarrega-o e pergunta se instala).
#[tauri::command]
fn abrir_atualizacao(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url("https://github.com/RubenFernandes2004/Tasca/releases/latest/download/Tasca.apk", None::<&str>)
        .map_err(|e| format!("Não consegui abrir o link da atualização: {e}"))
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
/// No Android não há atualização silenciosa: instala-se o APK novo por cima.
#[cfg(mobile)]
async fn atualizar(_app: &AppHandle) -> Result<bool, String> {
    Ok(false)
}

/// Procura e instala uma atualização. Devolve `true` se a app vai reiniciar.
#[cfg(desktop)]
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

/// Fecha a app (no Android: botão de voltar no ecrã principal).
#[tauri::command]
fn sair_app(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(avisos::Avisos::default())
        .manage(plugins::Loja::default());
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None));
    builder
        // Antes das janelas: no Windows a interface chama comandos de conta logo ao abrir.
        // Microfone, câmara, avisos e área de transferência: quem decide é o ecrã de
        // permissões da própria Tasca (Definições > Privacidade e permissões), por isso o
        // WebView não volta a perguntar.
        .on_permission_request(|_, kind| {
            use tauri::webview::{PermissionKind as K, PermissionResponse as R};
            match kind {
                K::Microphone | K::Camera | K::Notifications | K::ClipboardRead => R::Allow,
                _ => R::Default,
            }
        })
        .manage(contas::Contas::default())
        .manage(grupos::Rede::default())
        .manage(amigos::Amigos::default())
        .manage(voz::Voz::default())
        .invoke_handler(tauri::generate_handler![
            app_info,
            abrir_atualizacao,
            sair_app,
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
            contas::server_diagnostics,
            contas::save_phrase_desktop,
            contas::list_accounts,
            contas::select_account,
            contas::sign_out,
            contas::suggest_usernames,
            contas::set_display_name,
            contas::change_username,
            grupos::list_group_people,
            grupos::list_groups,
            grupos::list_members,
            grupos::list_channels,
            grupos::list_categories,
            grupos::list_messages,
            avisos::set_notify_state,
            plugins::list_plugins,
            plugins::install_plugin,
            plugins::sideload_plugin,
            plugins::uninstall_plugin,
            plugins::set_plugin_enabled,
            plugins::plugin_code,
            mensagens::send_message,
            mensagens::edit_message,
            mensagens::delete_message,
            mensagens::react_message,
            mensagens::search_messages,
            mensagens::unread_counts,
            mensagens::upload_file,
            mensagens::get_file,
            mensagens::save_file,
            grupos::create_group,
            grupos::import_local_group,
            grupos::join_group,
            grupos::create_invite,
            grupos::create_channel,
            grupos::update_channel,
            grupos::delete_channel,
            grupos::create_category,
            grupos::rename_category,
            grupos::delete_category,
            grupos::reorder_categories,
            grupos::reorder_channels,
            voz::join_voice,
            voz::leave_voice,
            voz::set_voice_state,
            voz::pass_voice_host,
            voz::set_voice_options,
            voz::set_user_volume,
            voz::voice_frame,
            voz::set_video_state,
            voz::video_subscribe,
            voz::video_send,
            grupos::publish_profile,
            amigos::list_friends,
            amigos::send_friend_request,
            amigos::accept_friend,
            amigos::decline_friend,
            amigos::cancel_friend,
            amigos::remove_friend,
            amigos::block_user,
            amigos::unblock_user,
            amigos::set_presence,
            amigos::list_dms,
            amigos::list_dm_messages,
            amigos::close_dm,
            amigos::send_dm,
            presenca::idle_seconds,
            presenca::set_autostart,
            presenca::get_autostart,
        ])
        .setup(|app| {
            let app = app.handle().clone();
            #[cfg(mobile)]
            {
                let a = app.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    avisos::pedir_autorizacao(&a);
                });
            }
            tauri::async_runtime::spawn(async move {
                // deixa o ecrã de arranque desenhar-se antes de começar
                tokio::time::sleep(Duration::from_millis(400)).await;
                estado(&app, Arranque::AProcurar);
                // em modo de desenvolvimento não há versões publicadas para comparar
                let reinicia = if cfg!(debug_assertions) || cfg!(mobile) {
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
        .build(tauri::generate_context!())
        .expect("erro ao arrancar a Tasca")
        .run(|app, ev| {
            // Ao fechar: sai da chamada e fecha as ligações P2P com cuidado, para os outros
            // verem logo que saíste (sem isto, só davam por isso quando a ligação expirasse).
            if let tauri::RunEvent::Exit = ev {
                let app = app.clone();
                tauri::async_runtime::block_on(async move {
                    let _ = tokio::time::timeout(Duration::from_secs(3), async {
                        voz::sair_tudo(&app.state::<voz::Voz>()).await;
                        grupos::desligar(&app.state::<grupos::Rede>()).await;
                    })
                    .await;
                });
            }
        });
}
