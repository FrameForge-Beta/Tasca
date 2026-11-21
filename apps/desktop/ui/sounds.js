// Tasca — sons da interface, sintetizados com Web Audio (sem ficheiros).
// Cada som é uma sequência curta de notas com um ataque suave e cauda curta,
// num timbre de "sino de madeira" (seno + uma harmónica discreta).
(function () {
  let ctx = null;
  let volume = 0.6;
  let enabled = true;
  let sink = "default";

  function audio() {
    if (!ctx) {
      const AC = window.AudioContext || window.webkitAudioContext;
      if (!AC) return null;
      ctx = new AC();
      if (sink !== "default" && ctx.setSinkId) ctx.setSinkId(sink).catch(() => {});
    }
    if (ctx.state === "suspended") ctx.resume();
    return ctx;
  }

  // notes: [frequência Hz, início s, duração s]
  function play(notes, { gain = 0.22, bright = 0.25 } = {}) {
    if (!enabled || volume <= 0) return;
    const ac = audio();
    if (!ac) return;
    const t0 = ac.currentTime + 0.01;
    const master = ac.createGain();
    master.gain.value = gain * volume;
    master.connect(ac.destination);
    for (const [f, at, dur] of notes) {
      for (const [mult, amp, type] of [[1, 1, "sine"], [2, bright, "triangle"]]) {
        const o = ac.createOscillator();
        const g = ac.createGain();
        o.type = type;
        o.frequency.value = f * mult;
        const s = t0 + at;
        g.gain.setValueAtTime(0.0001, s);
        g.gain.exponentialRampToValueAtTime(amp, s + 0.012);
        g.gain.exponentialRampToValueAtTime(0.0001, s + dur);
        o.connect(g).connect(master);
        o.start(s);
        o.stop(s + dur + 0.02);
      }
    }
  }

  const N = { G4: 392, A4: 440, C5: 523.25, D5: 587.33, E5: 659.25, G5: 783.99, A5: 880, C6: 1046.5 };

  window.sfx = {
    setVolume(v) { volume = Math.max(0, Math.min(1, v)); },
    setEnabled(on) { enabled = !!on; },
    // colunas/auscultadores escolhidos nas definições
    setOutput(id) { sink = id || "default"; if (ctx && ctx.setSinkId) ctx.setSinkId(sink === "default" ? "" : sink).catch(() => {}); },
    // microfone
    mute()     { play([[N.E5, 0, 0.09], [N.A4, 0.07, 0.14]], { gain: 0.18 }); },
    unmute()   { play([[N.A4, 0, 0.09], [N.E5, 0.07, 0.14]], { gain: 0.18 }); },
    // som (auscultadores)
    deafen()   { play([[N.D5, 0, 0.09], [N.G4, 0.07, 0.18]], { gain: 0.18, bright: 0.1 }); },
    undeafen() { play([[N.G4, 0, 0.09], [N.D5, 0.07, 0.18]], { gain: 0.18, bright: 0.1 }); },
    // partilha de ecrã
    shareStart() { play([[N.C5, 0, 0.12], [N.E5, 0.08, 0.12], [N.A5, 0.16, 0.26]], { gain: 0.2, bright: 0.35 }); },
    shareStop()  { play([[N.A5, 0, 0.12], [N.E5, 0.08, 0.12], [N.C5, 0.16, 0.22]], { gain: 0.18, bright: 0.2 }); },
    // entrar e sair da chamada
    join()  { play([[N.G4, 0, 0.16], [N.C5, 0.09, 0.16], [N.E5, 0.18, 0.16], [N.G5, 0.27, 0.38]], { gain: 0.2 }); },
    leave() { play([[N.G5, 0, 0.14], [N.E5, 0.09, 0.14], [N.C5, 0.18, 0.14], [N.G4, 0.27, 0.34]], { gain: 0.18 }); },
    // alguém entra ou sai da chamada em que estás
    otherJoin()  { play([[N.C5, 0, 0.1], [N.G5, 0.08, 0.2]], { gain: 0.12 }); },
    otherLeave() { play([[N.G5, 0, 0.1], [N.C5, 0.08, 0.2]], { gain: 0.12 }); },
  };
})();
