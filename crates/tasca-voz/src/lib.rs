//! Áudio das chamadas da Tasca.
//!
//! Tudo a 48 kHz, mono, em blocos de 20 ms ([`FRAME`] amostras `f32` entre -1 e 1):
//! - [`Captura`]: microfone → supressão de ruído → porta de ruído (sensibilidade) → Opus;
//! - [`Mistura`]: pacotes Opus de cada pessoa → buffer de jitter → descodificação (com FEC e
//!   ocultação de perdas) → soma de todos → saída.
//!
//! A rede fica de fora: quem usa isto recebe e entrega bytes Opus.

use std::collections::HashMap;
use std::hash::Hash;

use anyhow::{Context, Result};

/// Taxa de amostragem de tudo.
pub const RATE: u32 = 48_000;
/// Amostras num bloco de 20 ms.
pub const FRAME: usize = 960;
/// Metade de um bloco: o tamanho com que o DeepFilterNet e o RNNoise trabalham (10 ms).
const HOP: usize = 480;

/// Supressão de ruído.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ruido {
    Desligado,
    /// RNNoise (nnnoiseless): leve, para PCs fracos.
    Leve,
    /// DeepFilterNet 3: melhor, gasta mais CPU.
    Forte,
}

/// Um filtro de ruído já carregado. Carregar o DeepFilterNet demora um pouco, por isso
/// pode ser criado fora da linha do áudio e trocado depois com [`Captura::set_filtro`].
pub enum Filtro {
    Desligado,
    Leve(Box<nnnoiseless::DenoiseState<'static>>),
    Forte(Box<deep_filter_rt::tract::DfTract>),
}

impl Filtro {
    pub fn novo(modo: Ruido) -> Result<Self> {
        Ok(match modo {
            Ruido::Desligado => Filtro::Desligado,
            Ruido::Leve => Filtro::Leve(nnnoiseless::DenoiseState::new()),
            Ruido::Forte => {
                let mut df = deep_filter_rt::tract::DfTract::default();
                // Não apaga a voz por completo quando há muito ruído: soa mais natural.
                df.set_atten_lim(40.0);
                Filtro::Forte(Box::new(df))
            }
        })
    }

    pub fn modo(&self) -> Ruido {
        match self {
            Filtro::Desligado => Ruido::Desligado,
            Filtro::Leve(_) => Ruido::Leve,
            Filtro::Forte(_) => Ruido::Forte,
        }
    }

    /// Filtra 10 ms em sítio.
    fn hop(&mut self, x: &mut [f32]) -> Result<()> {
        debug_assert_eq!(x.len(), HOP);
        match self {
            Filtro::Desligado => {}
            Filtro::Leve(st) => {
                // O RNNoise trabalha na escala de 16 bits.
                let input: Vec<f32> = x.iter().map(|v| v * 32768.0).collect();
                let mut out = [0f32; HOP];
                st.process_frame(&mut out, &input);
                for (d, s) in x.iter_mut().zip(out) {
                    *d = s / 32768.0;
                }
            }
            Filtro::Forte(df) => {
                let noisy = ndarray::ArrayView2::from_shape((1, HOP), &*x)?.to_owned();
                let mut enh = ndarray::Array2::<f32>::zeros((1, HOP));
                df.process(noisy.view(), enh.view_mut())?;
                for (d, s) in x.iter_mut().zip(enh.iter()) {
                    *d = *s;
                }
            }
        }
        Ok(())
    }
}

/// O que sai de [`Captura::processar`].
pub struct Capturado {
    /// Nível do microfone (0 a 100), na mesma escala do medidor das definições.
    pub nivel: u8,
    /// Pacote Opus para enviar, ou `None` quando a porta está fechada (silêncio ou mudo).
    pub pacote: Option<Vec<u8>>,
}

/// Microfone → pacotes Opus.
pub struct Captura {
    enc: opus::Encoder,
    filtro: Filtro,
    /// Abaixo deste nível (0 a 100) não transmite. 0 transmite sempre.
    pub sensibilidade: u8,
    /// Blocos que a porta ainda fica aberta depois do último som acima da sensibilidade.
    aberta: u32,
    buf: Vec<u8>,
}

/// A porta fica aberta 400 ms depois de parares de falar, para não cortar o fim das palavras.
const SEGURA: u32 = 20;

impl Captura {
    pub fn new(filtro: Filtro) -> Result<Self> {
        let mut enc = opus::Encoder::new(RATE, opus::Channels::Mono, opus::Application::Voip)
            .context("não consegui criar o codificador Opus")?;
        enc.set_bitrate(opus::Bitrate::Bits(48_000))?;
        enc.set_inband_fec(true)?;
        enc.set_packet_loss_perc(10)?;
        Ok(Self { enc, filtro, sensibilidade: 0, aberta: 0, buf: vec![0; 1500] })
    }

    pub fn set_filtro(&mut self, filtro: Filtro) {
        self.filtro = filtro;
    }

    pub fn modo(&self) -> Ruido {
        self.filtro.modo()
    }

    /// Processa 20 ms do microfone. `transmitir` é falso quando estás sem microfone (ou com
    /// carregar-para-falar sem a tecla): nesse caso só mede o nível.
    pub fn processar(&mut self, pcm: &mut [f32], transmitir: bool) -> Result<Capturado> {
        anyhow::ensure!(pcm.len() == FRAME, "bloco de áudio com tamanho errado");
        let pico = pcm.iter().fold(0f32, |m, v| m.max(v.abs()));
        let nivel = (pico * 160.0).min(100.0) as u8;
        if !transmitir {
            self.aberta = 0;
            return Ok(Capturado { nivel, pacote: None });
        }
        if nivel >= self.sensibilidade {
            self.aberta = SEGURA;
        } else if self.aberta > 0 {
            self.aberta -= 1;
        }
        // O filtro corre sempre, para não perder o estado entre frases.
        for hop in pcm.chunks_mut(HOP) {
            self.filtro.hop(hop)?;
        }
        if self.aberta == 0 {
            return Ok(Capturado { nivel, pacote: None });
        }
        let n = self.enc.encode_float(pcm, &mut self.buf)?;
        Ok(Capturado { nivel, pacote: Some(self.buf[..n].to_vec()) })
    }
}

/// Blocos guardados antes de começar a tocar (60 ms).
const ALVO: usize = 3;
/// A partir daqui deita fora os mais antigos para baixar o atraso.
const MAXIMO: usize = 10;
/// Blocos perdidos seguidos até dar o fluxo por parado (a pessoa calou-se).
const PARADO: u32 = 10;

struct Fonte {
    dec: opus::Decoder,
    buf: HashMap<u16, Vec<u8>>,
    proximo: Option<u16>,
    a_tocar: bool,
    perdidos: u32,
    nivel: f32,
    volume: f32,
}

impl Fonte {
    fn new() -> Result<Self> {
        Ok(Self {
            dec: opus::Decoder::new(RATE, opus::Channels::Mono)?,
            buf: HashMap::new(),
            proximo: None,
            a_tocar: false,
            perdidos: 0,
            nivel: 0.0,
            volume: 1.0,
        })
    }

    fn receber(&mut self, seq: u16, data: Vec<u8>) {
        let Some(prox) = self.proximo else {
            self.proximo = Some(seq);
            self.buf.insert(seq, data);
            return;
        };
        let dist = seq.wrapping_sub(prox) as i16;
        if dist < 0 {
            return; // chegou tarde demais
        }
        if dist as usize > 4 * MAXIMO {
            // Salto grande (a pessoa voltou a entrar, por exemplo): recomeça.
            self.buf.clear();
            self.proximo = Some(seq);
            self.a_tocar = false;
        }
        self.buf.insert(seq, data);
    }

    /// Os próximos 20 ms desta pessoa (somados a `out`). Devolve falso se não tocou nada.
    fn tocar(&mut self, out: &mut [f32], tmp: &mut [f32]) -> bool {
        let Some(mut prox) = self.proximo else { return false };
        if !self.a_tocar {
            if self.buf.len() < ALVO {
                return false;
            }
            self.a_tocar = true;
            self.perdidos = 0;
        }
        while self.buf.len() > MAXIMO {
            self.buf.remove(&prox);
            prox = prox.wrapping_add(1);
        }
        let n = if let Some(p) = self.buf.remove(&prox) {
            self.perdidos = 0;
            self.dec.decode_float(&p, tmp, false)
        } else {
            self.perdidos += 1;
            if self.perdidos > PARADO {
                self.a_tocar = false;
                self.proximo = None;
                self.buf.clear();
                self.nivel = 0.0;
                return false;
            }
            match self.buf.get(&prox.wrapping_add(1)) {
                // O pacote seguinte traz uma cópia de baixa qualidade deste (FEC).
                Some(p) => self.dec.decode_float(p, tmp, true),
                // Ocultação de perdas: o Opus inventa uma continuação plausível.
                None => self.dec.decode_float(&[], tmp, false),
            }
        };
        self.proximo = Some(prox.wrapping_add(1));
        let n = n.unwrap_or(0).min(out.len());
        let mut soma = 0.0;
        for (o, s) in out.iter_mut().zip(&tmp[..n]) {
            *o += s * self.volume;
            soma += s * s;
        }
        let rms = if n > 0 { (soma / n as f32).sqrt() } else { 0.0 };
        self.nivel = if self.perdidos > 0 { 0.0 } else { rms };
        n > 0
    }
}

/// Junta o áudio de todos os outros numa só saída.
pub struct Mistura<K> {
    fontes: HashMap<K, Fonte>,
    tmp: Vec<f32>,
}

impl<K: Hash + Eq + Clone> Default for Mistura<K> {
    fn default() -> Self {
        Self { fontes: HashMap::new(), tmp: vec![0.0; FRAME * 6] }
    }
}

impl<K: Hash + Eq + Clone> Mistura<K> {
    /// Guarda um pacote Opus que chegou de `de`.
    pub fn receber(&mut self, de: K, seq: u16, data: Vec<u8>) -> Result<()> {
        let f = match self.fontes.entry(de) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => e.insert(Fonte::new()?),
        };
        f.receber(seq, data);
        Ok(())
    }

    /// Volume de uma pessoa (1.0 é o normal).
    pub fn set_volume(&mut self, de: &K, volume: f32) {
        if let Some(f) = self.fontes.get_mut(de) {
            f.volume = volume.clamp(0.0, 3.0);
        }
    }

    /// Esquece quem já não está na chamada.
    pub fn manter(&mut self, f: impl Fn(&K) -> bool) {
        self.fontes.retain(|k, _| f(k));
    }

    /// Os próximos 20 ms, com toda a gente somada.
    pub fn misturar(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        for f in self.fontes.values_mut() {
            f.tocar(out, &mut self.tmp);
        }
        // Limita suavemente quando várias pessoas falam alto ao mesmo tempo.
        for s in out.iter_mut() {
            if s.abs() > 0.9 {
                *s = s.signum() * (0.9 + 0.1 * ((s.abs() - 0.9) / 0.1).tanh());
            }
        }
    }

    /// Quem está a falar agora (nível acima de cerca de -40 dBFS).
    pub fn a_falar(&self) -> Vec<K> {
        self.fontes.iter().filter(|(_, f)| f.nivel > 0.01).map(|(k, _)| k.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tom(freq: f32, t0: usize, amp: f32) -> Vec<f32> {
        (0..FRAME)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * (t0 + i) as f32 / RATE as f32).sin())
            .collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn voz_vai_e_volta() {
        let mut cap = Captura::new(Filtro::novo(Ruido::Desligado).unwrap()).unwrap();
        let mut mix = Mistura::<u8>::default();
        let mut out = vec![0.0; FRAME];
        let mut tocou = 0.0f32;
        for i in 0..50 {
            let mut x = tom(440.0, i * FRAME, 0.5);
            let c = cap.processar(&mut x, true).unwrap();
            assert!(c.nivel > 60);
            // Perde um pacote em cada dez: o FEC/ocultação tapa o buraco.
            if i % 10 != 5 {
                mix.receber(1, i as u16, c.pacote.unwrap()).unwrap();
            }
            mix.misturar(&mut out);
            if i > 10 {
                tocou = tocou.max(rms(&out));
            }
        }
        assert!(tocou > 0.2, "rms {tocou}");
        assert_eq!(mix.a_falar(), vec![1]);
    }

    #[test]
    fn porta_fecha_no_silencio() {
        let mut cap = Captura::new(Filtro::Desligado).unwrap();
        cap.sensibilidade = 40;
        let mut x = tom(440.0, 0, 0.5);
        assert!(cap.processar(&mut x, true).unwrap().pacote.is_some());
        let mut enviados = 0;
        for _ in 0..40 {
            let mut s = vec![0.001; FRAME];
            if cap.processar(&mut s, true).unwrap().pacote.is_some() {
                enviados += 1;
            }
        }
        assert_eq!(enviados, SEGURA as usize - 1);
        let mut x = tom(440.0, 0, 0.5);
        assert!(cap.processar(&mut x, false).unwrap().pacote.is_none());
    }

    #[test]
    fn mistura_para_quando_a_pessoa_se_cala() {
        let mut cap = Captura::new(Filtro::Desligado).unwrap();
        let mut mix = Mistura::<u8>::default();
        let mut out = vec![0.0; FRAME];
        for i in 0..10 {
            let mut x = tom(300.0, i * FRAME, 0.5);
            mix.receber(7, i as u16, cap.processar(&mut x, true).unwrap().pacote.unwrap()).unwrap();
            mix.misturar(&mut out);
        }
        for _ in 0..30 {
            mix.misturar(&mut out);
        }
        assert!(rms(&out) < 1e-4);
        assert!(mix.a_falar().is_empty());
    }

    fn ruido_branco(n: usize, amp: f32, semente: &mut u32) -> Vec<f32> {
        (0..n)
            .map(|_| {
                *semente ^= *semente << 13;
                *semente ^= *semente >> 17;
                *semente ^= *semente << 5;
                amp * ((*semente as f32 / u32::MAX as f32) * 2.0 - 1.0)
            })
            .collect()
    }

    fn reduz_ruido(modo: Ruido) -> f32 {
        let mut f = Filtro::novo(modo).unwrap();
        let mut s = 1u32;
        let mut depois = 0.0;
        for i in 0..100 {
            let mut x = ruido_branco(HOP, 0.05, &mut s);
            f.hop(&mut x).unwrap();
            if i > 50 {
                depois += rms(&x);
            }
        }
        depois / 49.0 / (0.05 / 3f32.sqrt())
    }

    #[test]
    fn rnnoise_tira_ruido() {
        let r = reduz_ruido(Ruido::Leve);
        assert!(r < 0.5, "ficou {r} do ruído");
    }

    #[test]
    fn deepfilternet_tira_ruido() {
        let r = reduz_ruido(Ruido::Forte);
        assert!(r < 0.5, "ficou {r} do ruído");
    }
}
