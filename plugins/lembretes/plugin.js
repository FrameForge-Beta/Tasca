// Lembretes: /lembrar 10m comprar pão  →  daqui a 10 minutos aparece «⏰ comprar pão» no canal.
// Unidades: s, m, h, d. Os lembretes ficam guardados e sobrevivem a fechar a Tasca.

const UN = { s: 1e3, m: 6e4, h: 36e5, d: 864e5 };

async function lista() { return (await tasca.ler("lista")) || []; }
async function guardar(l) { await tasca.guardar("lista", l); }

function agendar(l) {
  const r = l.reduce((a, b) => (!a || b.quando < a.quando ? b : a), null);
  clearTimeout(globalThis.__lembrete);
  if (!r) return;
  globalThis.__lembrete = setTimeout(async () => {
    const atual = await lista();
    const agora = Date.now();
    const vencidos = atual.filter((x) => x.quando <= agora);
    for (const x of vencidos) await tasca.enviarMensagem(x.canal, "⏰ Lembrete: " + x.texto);
    const fica = atual.filter((x) => x.quando > agora);
    await guardar(fica);
    agendar(fica);
  }, Math.max(0, Math.min(r.quando - Date.now(), 2 ** 31 - 1)));
}

tasca.comando("lembrar", "Lembra o canal de alguma coisa. Ex.: /lembrar 10m jantar", async ({ args, channelId }) => {
  const m = /^(\d+)\s*([smhd])\s+(.+)$/i.exec(args.trim());
  if (!m) return tasca.toast("Usa assim: /lembrar 10m texto (s, m, h ou d)");
  const quando = Date.now() + Number(m[1]) * UN[m[2].toLowerCase()];
  const l = await lista();
  l.push({ quando, texto: m[3], canal: channelId });
  await guardar(l);
  agendar(l);
  tasca.toast("Combinado. Lembro o canal às " + new Date(quando).toLocaleTimeString("pt-PT", { hour: "2-digit", minute: "2-digit" }) + ".");
});

tasca.comando("lembretes", "Mostra os teus lembretes por cumprir", async () => {
  const l = await lista();
  tasca.toast(l.length ? l.map((x) => new Date(x.quando).toLocaleString("pt-PT") + " · " + x.texto).join("\n") : "Não tens lembretes.");
});

lista().then(agendar);
