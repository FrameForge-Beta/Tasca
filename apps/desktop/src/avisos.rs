//! Notificações do sistema (PC e Android) para mensagens novas.
//!
//! A interface diz o que está a ver e as preferências (`set_notify_state`); aqui decide-se
//! se uma mensagem merece aviso: não quando estás a ver esse canal com a janela à frente,
//! não em "Não incomodar", e respeitando o modo de cada grupo (tudo, só menções, nada) e
//! os canais silenciados.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Deserialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::NotificationExt;

#[derive(Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct EstadoUi {
    /// A janela está à frente e visível.
    pub focused: bool,
    /// Canal aberto (ou conversa `dm-…`).
    pub channel_id: Option<String>,
    pub dnd: bool,
    /// Notificações ligadas nas definições.
    pub enabled: bool,
    /// Mostrar o texto da mensagem no aviso.
    pub preview: bool,
    /// Modo por grupo: "all", "mentions" ou "none".
    pub group_modes: HashMap<String, String>,
    pub muted_channels: Vec<String>,
    /// O meu @nome, para as menções.
    pub username: Option<String>,
}

#[derive(Default)]
pub struct Avisos(Mutex<Option<EstadoUi>>);

#[tauri::command]
pub fn set_notify_state(avisos: State<'_, Avisos>, state: EstadoUi) {
    *avisos.0.lock().unwrap() = Some(state);
}

/// Uma mensagem nova chegou. `grupo`: id do grupo (`None` numa conversa privada).
pub fn mensagem(app: &AppHandle, canal: &str, grupo: Option<&str>, titulo: &str, texto: &str) {
    let Some(avisos) = app.try_state::<Avisos>() else { return };
    // Antes de a interface dizer alguma coisa (a arrancar), não avisa.
    let Some(e) = avisos.0.lock().unwrap().clone() else { return };
    if !e.enabled || e.dnd || (e.focused && e.channel_id.as_deref() == Some(canal)) {
        return;
    }
    if e.muted_channels.iter().any(|c| c == canal) {
        return;
    }
    if let Some(g) = grupo {
        let mencao = e.username.as_ref().is_some_and(|u| texto.to_lowercase().contains(&format!("@{}", u.to_lowercase())));
        match e.group_modes.get(g).map(String::as_str) {
            Some("none") => return,
            Some("mentions") if !mencao => return,
            _ => {}
        }
    }
    let corpo: String = if e.preview { texto.chars().take(180).collect() } else { "Mensagem nova".into() };
    if let Err(err) = app.notification().builder().title(titulo).body(corpo).show() {
        eprintln!("aviso não mostrado: {err}");
    }
}

#[cfg(mobile)]
/// Telemóvel: pede autorização para mostrar avisos (Android 13+), se ainda não foi dada.
pub fn pedir_autorizacao(app: &AppHandle) {
    use tauri_plugin_notification::PermissionState;
    let n = app.notification();
    if matches!(n.permission_state(), Ok(PermissionState::Prompt | PermissionState::PromptWithRationale)) {
        let _ = n.request_permission();
    }
}
