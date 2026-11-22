// Áudio das chamadas na app instalada.
//
// O microfone e as colunas ficam aqui (Web Audio), com os dispositivos escolhidos nas
// definições e o cancelamento de eco do sistema. A cada 20 ms o bloco do microfone vai
// para o Rust (`voice_frame`), que trata da supressão de ruído, do Opus e da rede, e
// devolve o som dos outros já misturado para tocar.
//
// window.vozAudio.start({ inputDevice, outputDevice, echoCancel, noise, mic })
// window.vozAudio.stop()
// window.vozAudio.transmit = () => bool   (falso: sem microfone ou carregar-para-falar solto)
// window.vozAudio.onFrame = ({ speaking, level }) => {}
(() => {
  const FRAME = 960;
  const core = window.__TAURI__ && window.__TAURI__.core;
  const A = {
    ctx: null, node: null, stream: null, gen: 0, emVoo: 0,
    transmit: () => true,
    onFrame: null,
  };

  async function enviar(mic, gen) {
    // Se a IPC se atrasar, não acumula: salta blocos em vez de aumentar o atraso.
    if (A.emVoo > 3) return;
    const body = new Uint8Array(4 + FRAME * 4);
    body[0] = A.transmit() ? 1 : 0;
    new Float32Array(body.buffer, 4, FRAME).set(mic);
    A.emVoo++;
    let r;
    try { r = await core.invoke("voice_frame", body); }
    catch (e) { console.warn("voz:", e); return; }
    finally { A.emVoo--; }
    if (gen !== A.gen || !A.node) return;
    const buf = r instanceof ArrayBuffer ? r : Array.isArray(r) ? new Uint8Array(r).buffer : r.buffer;
    const head = new Uint8Array(buf, 0, 4);
    const out = new Float32Array(buf.slice(4, 4 + FRAME * 4));
    A.node.port.postMessage(out, [out.buffer]);
    if (A.onFrame) A.onFrame({ speaking: !!(head[0] & 1), level: head[1] });
  }

  async function start(o) {
    stop();
    const gen = ++A.gen;
    if (!core) return;
    const ctx = new AudioContext({ sampleRate: 48000, latencyHint: "interactive" });
    A.ctx = ctx;
    try {
      if (o.outputDevice && o.outputDevice !== "default" && ctx.setSinkId) await ctx.setSinkId(o.outputDevice);
    } catch (e) { console.warn("saída de som indisponível, a usar a predefinida:", e); }
    await ctx.audioWorklet.addModule("voz-worklet.js");
    if (gen !== A.gen) { ctx.close().catch(() => {}); return; }
    const node = new AudioWorkletNode(ctx, "tasca-voz", { numberOfInputs: 1, numberOfOutputs: 1, outputChannelCount: [2] });
    node.port.onmessage = (e) => enviar(e.data, gen);
    node.connect(ctx.destination);
    A.node = node;
    if (o.mic && navigator.mediaDevices && navigator.mediaDevices.getUserMedia) {
      try {
        const dev = o.inputDevice;
        const stream = await navigator.mediaDevices.getUserMedia({ audio: {
          ...(dev && dev !== "default" ? { deviceId: { exact: dev } } : {}),
          channelCount: 1,
          echoCancellation: o.echoCancel !== false,
          // a supressão de ruído é a nossa (DeepFilterNet/RNNoise); a do sistema só sem ela
          noiseSuppression: o.noise === "off",
          autoGainControl: true,
        } });
        if (gen !== A.gen) { stream.getTracks().forEach((t) => t.stop()); return; }
        A.stream = stream;
        ctx.createMediaStreamSource(stream).connect(node);
      } catch (e) { console.warn("microfone indisponível:", e); }
    }
    if (ctx.state === "suspended") ctx.resume().catch(() => {});
  }

  function stop() {
    A.gen++;
    if (A.stream) A.stream.getTracks().forEach((t) => t.stop());
    if (A.node) { A.node.port.onmessage = null; A.node.disconnect(); }
    if (A.ctx) A.ctx.close().catch(() => {});
    Object.assign(A, { ctx: null, node: null, stream: null });
  }

  // ---------------- vídeo: câmara e partilha de ecrã ----------------
  // Cada imagem é codificada aqui (WebCodecs: H.264 se o PC o fizer, senão VP8) e vai
  // para o Rust (`video_send`), que a manda pelo anfitrião. Formato: byte 0 = bit 0
  // imagem chave, bit 1 ecrã (senão câmara); byte 1 = codec (0 VP8, 1 H.264); depois a
  // imagem. As imagens dos outros chegam por um canal (`video_subscribe`), com o id da
  // pessoa à frente, e são desenhadas em <canvas>.
  const CODECS = ["avc1.42E034", "vp8"];
  const V = { pubs: {}, decs: {}, canvases: new Map(), emVoo: 0, onStreams: null };

  async function escolherCodec(cfg) {
    if (!window.VideoEncoder) return null;
    for (const codec of CODECS) {
      const c = { ...cfg, codec, ...(codec.startsWith("avc") ? { avc: { format: "annexb" } } : {}) };
      try { const r = await VideoEncoder.isConfigSupported(c); if (r.supported) return c; } catch {}
    }
    return null;
  }

  function enviarImagem(kind, codecId, chunk, pub) {
    const key = chunk.type === "key";
    // Rede ou IPC atrasada: deita fora e pede uma imagem chave para recuperar.
    if (V.emVoo > 4 && !key) { pub.forceKey = true; return; }
    const buf = new Uint8Array(2 + chunk.byteLength);
    buf[0] = (key ? 1 : 0) | (kind === "screen" ? 2 : 0);
    buf[1] = codecId;
    chunk.copyTo(buf.subarray(2));
    V.emVoo++;
    core.invoke("video_send", buf).catch((e) => console.warn("vídeo:", e)).finally(() => V.emVoo--);
  }

  // Imagem já codificada fora do browser (partilha de ecrã nativa do Android, Ecra.kt).
  // Devolve false quando a ligação vai atrasada, para quem manda pedir uma imagem chave.
  function sendRaw(kind, codecId, bytes, key) {
    if (!core) return false;
    if (V.emVoo > 4 && !key) return false;
    const buf = new Uint8Array(2 + bytes.length);
    buf[0] = (key ? 1 : 0) | (kind === "screen" ? 2 : 0);
    buf[1] = codecId;
    buf.set(bytes, 2);
    V.emVoo++;
    core.invoke("video_send", buf).catch((e) => console.warn("vídeo:", e)).finally(() => V.emVoo--);
    return true;
  }

  async function publish(kind, stream, o) {
    const track = stream && stream.getVideoTracks()[0];
    if (!core || !track) return;
    const old = V.pubs[kind];
    if (old && old.track === track) return;
    unpublish(kind);
    if (!window.MediaStreamTrackProcessor || !window.VideoEncoder) { console.warn("este sistema não codifica vídeo"); return; }
    const pub = { track, stop: false, forceKey: true, enc: null, w: 0, h: 0 };
    V.pubs[kind] = pub;
    const reader = new MediaStreamTrackProcessor({ track }).readable.getReader();
    pub.reader = reader;
    let n = 0, codecId = 0;
    while (!pub.stop) {
      let r;
      try { r = await reader.read(); } catch { break; }
      if (r.done) break;
      const frame = r.value;
      try {
        // tamanho de saída: a altura pedida, sem aumentar, com lados pares
        const sw = frame.displayWidth, sh = frame.displayHeight;
        const h = Math.min(o.height, sh) & ~1, w = Math.round((sw * h) / sh) & ~1;
        if (!pub.enc || w !== pub.w || h !== pub.h) {
          if (pub.enc) try { pub.enc.close(); } catch {}
          const cfg = await escolherCodec({ width: w, height: h, bitrate: o.bitrate, framerate: o.fps, latencyMode: "realtime", hardwareAcceleration: "no-preference" });
          if (!cfg) { console.warn("nenhum codec de vídeo disponível"); frame.close(); break; }
          codecId = cfg.codec.startsWith("avc") ? 1 : 0;
          const id = codecId;
          pub.enc = new VideoEncoder({ output: (chunk) => enviarImagem(kind, id, chunk, pub), error: (e) => { console.warn("codificador:", e); pub.enc = null; } });
          pub.enc.configure(cfg);
          Object.assign(pub, { w, h, forceKey: true });
        }
        if (pub.enc.encodeQueueSize > 2) { pub.forceKey = true; continue; }
        const keyFrame = pub.forceKey || n % (o.fps * 2) === 0; // uma imagem chave a cada 2 s
        pub.forceKey = false;
        pub.enc.encode(frame, { keyFrame });
        n++;
      } catch (e) { console.warn("vídeo:", e); }
      finally { frame.close(); }
    }
    try { reader.releaseLock(); } catch {}
  }

  function unpublish(kind) {
    const pub = V.pubs[kind];
    if (!pub) return;
    pub.stop = true;
    try { pub.reader && pub.reader.cancel(); } catch {}
    try { pub.enc && pub.enc.close(); } catch {}
    delete V.pubs[kind];
  }

  function canvas(key) {
    let c = V.canvases.get(key);
    if (!c) {
      c = document.createElement("canvas");
      c.className = "live-video";
      V.canvases.set(key, c);
    }
    return c;
  }

  function receber(data) {
    const buf = data instanceof ArrayBuffer ? new Uint8Array(data) : new Uint8Array(data);
    const idLen = buf[0];
    const id = new TextDecoder().decode(buf.subarray(1, 1 + idLen));
    const flags = buf[1 + idLen], codecId = buf[2 + idLen];
    const img = buf.subarray(3 + idLen);
    const key = id + ":" + (flags & 2 ? "screen" : "cam");
    const isKey = !!(flags & 1);
    let d = V.decs[key];
    if (d && d.codecId !== codecId) { try { d.dec.close(); } catch {} d = null; }
    if (!d || d.dec.state === "closed") {
      if (!isKey) return; // espera por uma imagem chave
      if (!window.VideoDecoder) return;
      const dec = new VideoDecoder({
        output: (frame) => {
          const c = canvas(key);
          if (c.width !== frame.displayWidth || c.height !== frame.displayHeight) { c.width = frame.displayWidth; c.height = frame.displayHeight; }
          c.getContext("2d").drawImage(frame, 0, 0);
          frame.close();
        },
        error: (e) => { console.warn("descodificador:", e); },
      });
      dec.configure({ codec: codecId === 1 ? "avc1.42E034" : "vp8", optimizeForLatency: true, ...(codecId === 1 ? { avc: { format: "annexb" } } : {}) });
      d = V.decs[key] = { dec, codecId };
    }
    if (d.dec.decodeQueueSize > 4 && !isKey) return;
    try { d.dec.decode(new EncodedVideoChunk({ type: isKey ? "key" : "delta", timestamp: performance.now() * 1000, data: img })); }
    catch (e) { console.warn("vídeo recebido:", e); try { d.dec.close(); } catch {} delete V.decs[key]; }
  }

  function subscribe() {
    if (!core || !core.Channel) return;
    const ch = new core.Channel();
    ch.onmessage = (m) => { try { receber(m); } catch (e) { console.warn("vídeo:", e); } };
    core.invoke("video_subscribe", { channel: ch }).catch((e) => console.warn("vídeo:", e));
  }

  function limparVideo() {
    for (const k of Object.keys(V.pubs)) unpublish(k);
    for (const d of Object.values(V.decs)) try { d.dec.close(); } catch {}
    V.decs = {};
    V.canvases.clear();
  }

  window.vozAudio = {
    start, stop,
    video: { publish, unpublish, sendRaw, canvas, subscribe, clear: limparVideo },
    get active() { return !!A.ctx; },
    set transmit(f) { A.transmit = f; },
    set onFrame(f) { A.onFrame = f; },
  };
})();
