//! Partilha de ecrã feita pela própria Tasca (Windows).
//!
//! A janela de escolha do Windows/WebView2 não se pode mudar (mostra «tauri.localhost»),
//! por isso a Tasca lista os ecrãs e as janelas ela mesma (`list_share_sources`, com
//! miniaturas), captura o que escolheres (Windows Graphics Capture) e codifica em H.264
//! (OpenH264). As imagens codificadas vão para a interface por um canal: primeiro byte com
//! o bit 0 ligado nas imagens chave, depois a imagem (Annex B). A interface manda-as aos
//! outros pelo mesmo caminho da partilha nativa do Android (`sendRaw`) e mostra-as no
//! teu quadrado.
//!
//! Noutros sistemas estes comandos dizem que não dá, e a interface usa a janela do sistema.

use serde::Deserialize;
use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::AppHandle;

type Res<T> = Result<T, String>;

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pedido {
    /// "m:<índice>" para um ecrã, "w:<hwnd>" para uma janela.
    pub id: String,
    pub height: u32,
    pub fps: u32,
    pub bitrate: u32,
}

/// Ecrãs e janelas que se podem partilhar, com miniatura (data URL JPEG).
#[tauri::command]
pub async fn list_share_sources() -> Res<Vec<Value>> {
    #[cfg(windows)]
    {
        tauri::async_runtime::spawn_blocking(win::fontes)
            .await
            .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        Err("unsupported".into())
    }
}

/// Começa a partilhar. As imagens chegam por `channel`; se a janela partilhada fechar,
/// a interface recebe o evento `share_state` com `{ state: "stopped" }`.
#[tauri::command]
pub async fn start_share(
    app: AppHandle,
    source: Pedido,
    channel: Channel<InvokeResponseBody>,
) -> Res<bool> {
    #[cfg(windows)]
    {
        tauri::async_runtime::spawn_blocking(move || win::comecar(app, source, channel))
            .await
            .map_err(|e| e.to_string())??;
        Ok(true)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, source, channel);
        Err("unsupported".into())
    }
}

#[tauri::command]
pub fn stop_share() {
    #[cfg(windows)]
    win::parar();
}

/// A ligação atrasou-se e deitou imagens fora: a próxima vai ser uma imagem chave.
#[tauri::command]
pub fn share_keyframe() {
    #[cfg(windows)]
    win::pedir_chave();
}

/// Reduz uma imagem BGRA (com `pitch` bytes por linha) para `ow`×`oh` e converte para
/// I420 (Y, depois U, depois V). Amostragem do ponto mais próximo; a cor de cada bloco 2×2
/// é a média dos quatro pontos.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn bgra_para_i420(
    src: &[u8],
    sw: usize,
    sh: usize,
    pitch: usize,
    ow: usize,
    oh: usize,
) -> Vec<u8> {
    let mut out = vec![0u8; ow * oh * 3 / 2];
    let (y_plane, uv) = out.split_at_mut(ow * oh);
    let (u_plane, v_plane) = uv.split_at_mut(ow * oh / 4);
    let xs: Vec<usize> = (0..ow).map(|x| ((x * sw) / ow).min(sw - 1) * 4).collect();
    let px = |row: usize, x: usize| -> (i32, i32, i32) {
        let i = row * pitch + xs[x];
        (src[i + 2] as i32, src[i + 1] as i32, src[i] as i32)
    };
    for y in (0..oh).step_by(2) {
        let r0 = ((y * sh) / oh).min(sh - 1);
        let r1 = (((y + 1) * sh) / oh).min(sh - 1);
        for x in (0..ow).step_by(2) {
            let mut su = 0;
            let mut sv = 0;
            for (dy, row) in [(0, r0), (1, r1)] {
                for dx in 0..2 {
                    let (r, g, b) = px(row, x + dx);
                    y_plane[(y + dy) * ow + x + dx] =
                        (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16) as u8;
                    su += ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                    sv += ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
                }
            }
            let c = (y / 2) * (ow / 2) + x / 2;
            u_plane[c] = (su / 4).clamp(0, 255) as u8;
            v_plane[c] = (sv / 4).clamp(0, 255) as u8;
        }
    }
    out
}

/// Tamanho de saída: a altura pedida (sem aumentar), mesma proporção, lados pares.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn tamanho(sw: u32, sh: u32, altura: u32) -> (usize, usize) {
    let h = altura.min(sh).max(2) & !1;
    let w = ((sw as u64 * h as u64) / sh.max(1) as u64) as u32 & !1;
    (w.max(2) as usize, h as usize)
}

#[cfg(windows)]
mod win {
    use super::{bgra_para_i420, tamanho, Pedido, Res};
    use base64::Engine as _;
    use openh264::encoder::{
        BitRate, Encoder, EncoderConfig, FrameRate, FrameType, RateControlMode, UsageType,
    };
    use openh264::formats::YUVBuffer;
    use openh264::OpenH264API;
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use tauri::ipc::{Channel, InvokeResponseBody};
    use tauri::{AppHandle, Emitter};
    use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
    use windows_capture::encoder::{ImageEncoder, ImageEncoderPixelFormat, ImageFormat};
    use windows_capture::frame::Frame;
    use windows_capture::graphics_capture_api::{GraphicsCaptureApi, InternalCaptureControl};
    use windows_capture::monitor::Monitor;
    use windows_capture::settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    };
    use windows_capture::window::Window;

    type Erro = Box<dyn std::error::Error + Send + Sync>;

    // ---------------- lista com miniaturas ----------------

    const MINI_LARGURA: usize = 320;

    struct Mini {
        tx: Option<SyncSender<String>>,
    }

    impl GraphicsCaptureApiHandler for Mini {
        type Flags = SyncSender<String>;
        type Error = Erro;

        fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
            Ok(Self {
                tx: Some(ctx.flags),
            })
        }

        fn on_frame_arrived(
            &mut self,
            frame: &mut Frame,
            ctl: InternalCaptureControl,
        ) -> Result<(), Self::Error> {
            if let Some(tx) = self.tx.take() {
                let mut b = frame.buffer()?;
                let (sw, sh, pitch) = (
                    b.width() as usize,
                    b.height() as usize,
                    b.row_pitch() as usize,
                );
                let ow = MINI_LARGURA.min(sw).max(1);
                let oh = ((sh * ow) / sw.max(1)).max(1);
                let raw = b.as_raw_buffer();
                let mut px = vec![0u8; ow * oh * 4];
                for y in 0..oh {
                    let sy = (y * sh) / oh;
                    for x in 0..ow {
                        let s = sy * pitch + ((x * sw) / ow) * 4;
                        let d = (y * ow + x) * 4;
                        px[d..d + 3].copy_from_slice(&raw[s..s + 3]);
                        px[d + 3] = 255;
                    }
                }
                let jpg = ImageEncoder::new(ImageFormat::Jpeg, ImageEncoderPixelFormat::Bgra8)?
                    .encode(&px, ow as u32, oh as u32)?;
                let _ = tx.send(format!(
                    "data:image/jpeg;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(jpg)
                ));
            }
            ctl.stop();
            Ok(())
        }
    }

    fn borda() -> DrawBorderSettings {
        if GraphicsCaptureApi::is_border_settings_supported().unwrap_or(false) {
            DrawBorderSettings::WithoutBorder
        } else {
            DrawBorderSettings::Default
        }
    }

    /// Tira uma imagem pequena de um ecrã ou janela. Janelas minimizadas não dão imagem.
    fn miniatura<T>(item: T) -> Option<String>
    where
        T: TryInto<windows_capture::settings::GraphicsCaptureItemType> + Send + 'static,
    {
        let (tx, rx) = sync_channel(1);
        let settings = Settings::new(
            item,
            CursorCaptureSettings::Default,
            borda(),
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,
            tx,
        );
        let ctl = Mini::start_free_threaded(settings).ok()?;
        let r = rx.recv_timeout(Duration::from_millis(1500)).ok();
        if !ctl.is_finished() {
            let _ = ctl.stop();
        }
        r
    }

    fn eu() -> u32 {
        std::process::id()
    }

    pub fn fontes() -> Res<Vec<Value>> {
        type Tirar = Box<dyn FnOnce() -> Option<String> + Send>;
        let mut lista: Vec<(Value, Tirar)> = Vec::new();
        let monitores = Monitor::enumerate().map_err(|e| e.to_string())?;
        for (i, m) in monitores.into_iter().enumerate() {
            let (w, h) = (m.width().unwrap_or(0), m.height().unwrap_or(0));
            // posição na lista do Windows, a mesma que Monitor::from_index usa (a contar de 1)
            let id = format!("m:{}", i + 1);
            lista.push((json!({ "id": id, "kind": "screen", "name": format!("Ecrã {}", i + 1), "size": [w, h] }), Box::new(move || miniatura(m))));
        }
        for w in Window::enumerate().unwrap_or_default() {
            if w.process_id().map(|p| p == eu()).unwrap_or(true) || escondida(w.as_raw_hwnd()) {
                continue;
            }
            let titulo = w.title().unwrap_or_default();
            if titulo.trim().is_empty() {
                continue;
            }
            let app = w.process_name().unwrap_or_default();
            let id = format!("w:{}", w.as_raw_hwnd() as usize);
            lista.push((json!({ "id": id, "kind": "window", "name": titulo, "app": app.trim_end_matches(".exe") }), Box::new(move || miniatura(w))));
        }
        // Miniaturas aos poucos: cada uma abre uma captura (e um dispositivo gráfico).
        let mut out = Vec::new();
        let mut it = lista.into_iter().peekable();
        while it.peek().is_some() {
            let lote: Vec<_> = it
                .by_ref()
                .take(6)
                .map(|(info, f)| (info, std::thread::spawn(f)))
                .collect();
            for (mut info, t) in lote {
                info["thumb"] = json!(t.join().ok().flatten());
                out.push(info);
            }
        }
        Ok(out)
    }

    /// Janelas que o Windows tem escondidas (apps da loja em segundo plano, por exemplo).
    fn escondida(hwnd: *mut std::ffi::c_void) -> bool {
        use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
        let mut cloaked: u32 = 0;
        let r = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED as u32,
                &mut cloaked as *mut u32 as *mut _,
                std::mem::size_of::<u32>() as u32,
            )
        };
        r == 0 && cloaked != 0
    }

    // ---------------- partilha ----------------

    struct Imagem {
        yuv: Vec<u8>,
        w: usize,
        h: usize,
    }

    struct Flags {
        tx: SyncSender<Imagem>,
        altura: u32,
        intervalo: Duration,
        fim: Arc<AtomicBool>,
        app: AppHandle,
    }

    struct Cap {
        f: Flags,
        ultima: Option<Instant>,
    }

    impl GraphicsCaptureApiHandler for Cap {
        type Flags = Flags;
        type Error = Erro;

        fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
            Ok(Self {
                f: ctx.flags,
                ultima: None,
            })
        }

        fn on_frame_arrived(
            &mut self,
            frame: &mut Frame,
            ctl: InternalCaptureControl,
        ) -> Result<(), Self::Error> {
            if self.f.fim.load(Ordering::Relaxed) {
                ctl.stop();
                return Ok(());
            }
            let agora = Instant::now();
            if let Some(u) = self.ultima {
                if agora.duration_since(u) < self.f.intervalo {
                    return Ok(());
                }
            }
            self.ultima = Some(agora);
            let mut b = frame.buffer()?;
            let (sw, sh, pitch) = (
                b.width() as usize,
                b.height() as usize,
                b.row_pitch() as usize,
            );
            if sw < 2 || sh < 2 {
                return Ok(());
            }
            let (w, h) = tamanho(sw as u32, sh as u32, self.f.altura);
            let yuv = bgra_para_i420(b.as_raw_buffer(), sw, sh, pitch, w, h);
            match self.f.tx.try_send(Imagem { yuv, w, h }) {
                // o codificador ainda está ocupado com a anterior: esta fica de fora
                Ok(()) | Err(TrySendError::Full(_)) => Ok(()),
                Err(TrySendError::Disconnected(_)) => {
                    ctl.stop();
                    Ok(())
                }
            }
        }

        fn on_closed(&mut self) -> Result<(), Self::Error> {
            // a janela partilhada fechou
            if !self.f.fim.swap(true, Ordering::Relaxed) {
                let _ = self
                    .f
                    .app
                    .emit("share_state", json!({ "state": "stopped" }));
            }
            Ok(())
        }
    }

    struct Partilha {
        ctl: Option<CaptureControl<Cap, Erro>>,
        fim: Arc<AtomicBool>,
        chave: Arc<AtomicBool>,
    }

    fn atual() -> &'static Mutex<Option<Partilha>> {
        static P: OnceLock<Mutex<Option<Partilha>>> = OnceLock::new();
        P.get_or_init(|| Mutex::new(None))
    }

    pub fn parar() {
        let p = atual().lock().unwrap().take();
        if let Some(mut p) = p {
            p.fim.store(true, Ordering::Relaxed);
            if let Some(c) = p.ctl.take() {
                std::thread::spawn(move || {
                    let _ = c.stop();
                });
            }
        }
    }

    pub fn pedir_chave() {
        if let Some(p) = atual().lock().unwrap().as_ref() {
            p.chave.store(true, Ordering::Relaxed);
        }
    }

    pub fn comecar(app: AppHandle, p: Pedido, canal: Channel<InvokeResponseBody>) -> Res<()> {
        parar();
        let fps = p.fps.clamp(5, 60);
        let fim = Arc::new(AtomicBool::new(false));
        let chave = Arc::new(AtomicBool::new(true));
        let (tx, rx) = sync_channel::<Imagem>(1);
        let flags = Flags {
            tx,
            altura: p.height.clamp(360, 2160),
            intervalo: Duration::from_micros(1_000_000 / fps as u64 - 2_000),
            fim: fim.clone(),
            app: app.clone(),
        };
        let intervalo =
            if GraphicsCaptureApi::is_minimum_update_interval_supported().unwrap_or(false) {
                MinimumUpdateIntervalSettings::Custom(Duration::from_millis(1000 / fps as u64))
            } else {
                MinimumUpdateIntervalSettings::Default
            };
        let (tipo, resto) = p.id.split_once(':').ok_or("fonte inválida")?;
        let ctl = match tipo {
            "m" => {
                let i: usize = resto.parse().map_err(|_| "fonte inválida")?;
                let m = Monitor::from_index(i).map_err(|_| "Esse ecrã já não está ligado.")?;
                Cap::start_free_threaded(Settings::new(
                    m,
                    CursorCaptureSettings::Default,
                    borda(),
                    SecondaryWindowSettings::Default,
                    intervalo,
                    DirtyRegionSettings::Default,
                    ColorFormat::Bgra8,
                    flags,
                ))
            }
            "w" => {
                let h: usize = resto.parse().map_err(|_| "fonte inválida")?;
                let w = Window::from_raw_hwnd(h as *mut std::ffi::c_void);
                if !w.is_valid() {
                    return Err("Essa janela já fechou.".into());
                }
                Cap::start_free_threaded(Settings::new(
                    w,
                    CursorCaptureSettings::Default,
                    borda(),
                    SecondaryWindowSettings::Default,
                    intervalo,
                    DirtyRegionSettings::Default,
                    ColorFormat::Bgra8,
                    flags,
                ))
            }
            _ => return Err("fonte inválida".into()),
        }
        .map_err(|e| format!("Não deu para partilhar: {e}"))?;
        let (f2, k2) = (fim.clone(), chave.clone());
        let bitrate = p.bitrate.clamp(500_000, 12_000_000);
        std::thread::spawn(move || codificar(rx, canal, fps, bitrate, f2, k2, app));
        *atual().lock().unwrap() = Some(Partilha {
            ctl: Some(ctl),
            fim,
            chave,
        });
        Ok(())
    }

    fn codificar(
        rx: Receiver<Imagem>,
        canal: Channel<InvokeResponseBody>,
        fps: u32,
        bitrate: u32,
        fim: Arc<AtomicBool>,
        chave: Arc<AtomicBool>,
        app: AppHandle,
    ) {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
            .clamp(1, 4) as u16;
        let cfg = EncoderConfig::new()
            .usage_type(UsageType::CameraVideoRealTime)
            .rate_control_mode(RateControlMode::Bitrate)
            .bitrate(BitRate::from_bps(bitrate))
            .max_frame_rate(FrameRate::from_hz(fps as f32))
            .skip_frames(true)
            .num_threads(threads);
        let mut enc = match Encoder::with_api_config(OpenH264API::from_source(), cfg) {
            Ok(e) => e,
            Err(e) => {
                let _ = app.emit(
                    "share_state",
                    json!({ "state": "error", "message": format!("Não deu para partilhar: {e}") }),
                );
                return;
            }
        };
        let mut n: u64 = 0;
        let mut dims = (0, 0);
        while !fim.load(Ordering::Relaxed) {
            let Ok(img) = rx.recv_timeout(Duration::from_millis(500)) else {
                continue;
            };
            if (img.w, img.h) != dims {
                dims = (img.w, img.h);
                chave.store(true, Ordering::Relaxed);
            }
            // uma imagem chave a cada 2 s, para quem entra a meio
            if chave.swap(false, Ordering::Relaxed) || n % (fps as u64 * 2) == 0 {
                enc.force_intra_frame();
            }
            n += 1;
            let yuv = YUVBuffer::from_vec(img.yuv, img.w, img.h);
            let Ok(bs) = enc.encode(&yuv) else { continue };
            let e_chave = matches!(bs.frame_type(), FrameType::IDR | FrameType::I);
            if matches!(bs.frame_type(), FrameType::Skip | FrameType::Invalid) {
                continue;
            }
            let mut buf = Vec::with_capacity(64 * 1024);
            buf.push(e_chave as u8);
            bs.write_vec(&mut buf);
            if buf.len() <= 1 {
                continue;
            }
            if canal.send(InvokeResponseBody::Raw(buf)).is_err() {
                break;
            }
        }
        fim.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tamanho_par_e_sem_aumentar() {
        assert_eq!(tamanho(2560, 1440, 1080), (1920, 1080));
        assert_eq!(tamanho(1280, 720, 1080), (1280, 720));
        assert_eq!(tamanho(1001, 701, 1080), (998, 700));
    }

    #[test]
    fn converte_branco_e_preto() {
        // 4x2 BGRA: metade branca, metade preta, com 4 bytes de sobra por linha
        let pitch = 4 * 4 + 4;
        let mut src = vec![0u8; pitch * 2];
        for y in 0..2 {
            for x in 0..2 {
                src[y * pitch + x * 4..y * pitch + x * 4 + 4]
                    .copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let out = bgra_para_i420(&src, 4, 2, pitch, 4, 2);
        assert_eq!(out.len(), 12);
        assert_eq!(out[0], 235); // Y do branco
        assert_eq!(out[3], 16); // Y do preto
        assert_eq!(out[8], 128); // U sem cor
        assert_eq!(out[10], 128); // V sem cor
    }
}
