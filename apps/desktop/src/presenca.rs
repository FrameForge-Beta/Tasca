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
