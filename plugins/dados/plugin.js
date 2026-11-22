// Dados: /dados 2d6  →  rola dois dados de 6 faces e mostra o resultado a toda a gente.
// Também: /moeda (cara ou coroa) e /escolher pizza, sushi, francesinha.

const sorte = (n) => {
  const b = new Uint32Array(1);
  crypto.getRandomValues(b);
  return b[0] % n;
};

tasca.comando("dados", "Rola dados. Ex.: /dados 2d6", ({ args, channelId }) => {
  const m = /^(\d{0,2})d(\d{1,3})$/i.exec(args.trim() || "1d6");
  if (!m) return tasca.toast("Usa assim: /dados 2d6");
  const n = Math.min(Number(m[1] || 1), 20), f = Math.max(2, Math.min(Number(m[2]), 1000));
  const r = Array.from({ length: n }, () => 1 + sorte(f));
  tasca.enviarMensagem(channelId, `🎲 ${n}d${f}: ${r.join(" + ")}${n > 1 ? " = **" + r.reduce((a, b) => a + b, 0) + "**" : ""}`);
});

tasca.comando("moeda", "Cara ou coroa", ({ channelId }) => {
  tasca.enviarMensagem(channelId, "🪙 " + (sorte(2) ? "Cara" : "Coroa"));
});

tasca.comando("escolher", "Escolhe uma opção. Ex.: /escolher pizza, sushi", ({ args, channelId }) => {
  const ops = args.split(",").map((x) => x.trim()).filter(Boolean);
  if (ops.length < 2) return tasca.toast("Dá pelo menos duas opções, separadas por vírgulas.");
  tasca.enviarMensagem(channelId, "🤔 Escolhi: **" + ops[sorte(ops.length)] + "**");
});
