//! Esconde a barra que o WebView2 mostra ao partilhar o ecrã
//! («http://tauri.localhost está a partilhar uma janela · Parar partilha · Ocultar»).
//!
//! O WebView2 não tem opção para a tirar, por isso procuramos a janela dela e
//! escondemo-la. É uma janela pequena do processo do WebView2 (filho direto da
//! Tasca) com o endereço da app no título. A Tasca já mostra que estás a partilhar
//! e tem o seu próprio botão para parar.

/// Chamado pela interface logo que a partilha começa.
#[tauri::command]
pub fn esconder_barra_partilha() {
    #[cfg(windows)]
    std::thread::spawn(win::vigiar);
}

#[cfg(windows)]
mod win {
    use std::time::{Duration, Instant};
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM, RECT};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
        ShowWindowAsync, SW_HIDE,
    };

    /// A janela de escolha do Windows pode ficar aberta um bocado; a barra só aparece depois.
    const DURANTE: Duration = Duration::from_secs(20);
    const INTERVALO: Duration = Duration::from_millis(150);
    /// Altura máxima da barra, em píxeis reais (chega para 300% de escala).
    const ALTURA_MAX: i32 = 240;

    pub fn vigiar() {
        let fim = Instant::now() + DURANTE;
        while Instant::now() < fim {
            let filhos = processos_filhos();
            if !filhos.is_empty() {
                // Continua até ao fim, caso o WebView2 a volte a mostrar.
                let mut ctx = Ctx { filhos };
                unsafe { EnumWindows(Some(visitar), &mut ctx as *mut Ctx as LPARAM) };
            }
            std::thread::sleep(INTERVALO);
        }
    }

    struct Ctx {
        filhos: Vec<u32>,
    }

    /// Processos lançados pela Tasca (o processo principal do WebView2 é um deles).
    fn processos_filhos() -> Vec<u32> {
        let eu = unsafe { GetCurrentProcessId() };
        let mut v = Vec::new();
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE || snap.is_null() {
                return v;
            }
            let mut e: PROCESSENTRY32W = std::mem::zeroed();
            e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut ok = Process32FirstW(snap, &mut e);
            while ok != 0 {
                if e.th32ParentProcessID == eu {
                    v.push(e.th32ProcessID);
                }
                ok = Process32NextW(snap, &mut e);
            }
            CloseHandle(snap);
        }
        v
    }

    pub(super) fn e_a_barra(titulo: &str, altura: i32) -> bool {
        altura > 0 && altura <= ALTURA_MAX && titulo.contains("tauri.localhost")
    }

    unsafe extern "system" fn visitar(hwnd: HWND, l: LPARAM) -> BOOL {
        let ctx = &mut *(l as *mut Ctx);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if !ctx.filhos.contains(&pid) || IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut r) == 0 {
            return 1;
        }
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        let titulo = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        if e_a_barra(&titulo, r.bottom - r.top) {
            // Assíncrono: se o WebView2 estiver ocupado, não ficamos à espera dele.
            ShowWindowAsync(hwnd, SW_HIDE);
        }
        1
    }
}

#[cfg(all(test, windows))]
mod testes {
    #[test]
    fn reconhece_a_barra() {
        assert!(super::win::e_a_barra(
            "http://tauri.localhost está a partilhar uma janela e áudio.",
            48
        ));
        assert!(!super::win::e_a_barra("Tasca", 48));
        assert!(!super::win::e_a_barra("http://tauri.localhost", 900));
    }
}
