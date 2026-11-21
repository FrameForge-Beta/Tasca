// Corre na linha de áudio do navegador. Junta o microfone em blocos de 20 ms (960
// amostras a 48 kHz) e manda-os para a página; toca os blocos misturados que voltam.
const FRAME = 960;

class TascaVoz extends AudioWorkletProcessor {
  constructor() {
    super();
    this.mic = new Float32Array(FRAME);
    this.n = 0;
    this.fila = []; // blocos para tocar
    this.pos = 0; // posição no primeiro bloco da fila
    this.a_tocar = false;
    this.port.onmessage = (e) => {
      this.fila.push(e.data);
      // Atraso a crescer (o relógio da placa e o da rede não batem certo): deita fora.
      while (this.fila.length > 6) { this.fila.shift(); this.pos = 0; }
    };
  }

  process(inputs, outputs) {
    const inp = inputs[0] && inputs[0][0];
    const out = outputs[0];
    const len = out[0].length;
    for (let i = 0; i < len; i++) {
      this.mic[this.n++] = inp ? inp[i] : 0;
      if (this.n === FRAME) {
        this.port.postMessage(this.mic, [this.mic.buffer]);
        this.mic = new Float32Array(FRAME);
        this.n = 0;
      }
    }
    // Espera por dois blocos antes de começar, para aguentar variações da IPC.
    if (!this.a_tocar && this.fila.length >= 2) this.a_tocar = true;
    for (let i = 0; i < len; i++) {
      let v = 0;
      if (this.a_tocar) {
        const b = this.fila[0];
        if (b) {
          v = b[this.pos++];
          if (this.pos >= b.length) { this.fila.shift(); this.pos = 0; }
        } else {
          this.a_tocar = false; // ficou sem nada: volta a encher
        }
      }
      for (const ch of out) ch[i] = v;
    }
    return true;
  }
}

registerProcessor("tasca-voz", TascaVoz);
