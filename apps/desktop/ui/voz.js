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

  window.vozAudio = {
    start, stop,
    get active() { return !!A.ctx; },
    set transmit(f) { A.transmit = f; },
    set onFrame(f) { A.onFrame = f; },
  };
})();
