//! Há quanto tempo ninguém mexe no PC (rato ou teclado), para o estado "Ausente" automático.

/// Segundos desde a última vez que o rato ou o teclado foram usados em qualquer app.
/// `None` onde o sistema não o diz; aí a interface conta só a atividade na janela da Tasca.
#[tauri::command]
pub fn idle_seconds() -> Option<u64> {
    imp::idle()
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    pub fn idle() -> Option<u64> {
        let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        // SAFETY: `info` é uma LASTINPUTINFO válida com o cbSize certo.
        if unsafe { GetLastInputInfo(&mut info) } == 0 {
            return None;
        }
        // SAFETY: sem argumentos.
        let agora = unsafe { GetTickCount() };
        Some(u64::from(agora.wrapping_sub(info.dwTime)) / 1000)
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn idle() -> Option<u64> {
        None
    }
}

/// No Android não há "abrir com o sistema".
#[cfg(mobile)]
#[tauri::command]
pub fn set_autostart(_app: tauri::AppHandle, _enabled: bool) -> Result<bool, String> {
    Err("Não disponível no telemóvel.".into())
}

#[cfg(mobile)]
#[tauri::command]
pub fn get_autostart(_app: tauri::AppHandle) -> Result<bool, String> {
    Ok(false)
}

/// Abrir a Tasca quando o Windows arranca (Definições > Privacidade e permissões).
#[cfg(desktop)]
#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let r = if enabled { al.enable() } else { al.disable() };
    r.map_err(|e| e.to_string())?;
    al.is_enabled().map_err(|e| e.to_string())
}

/// Se a Tasca está marcada para abrir com o sistema.
#[cfg(desktop)]
#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}
