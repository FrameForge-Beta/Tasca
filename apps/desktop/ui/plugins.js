// Motor dos plugins: cada plugin ativo corre num iframe isolado (sem acesso à app, à rede
// nem aos ficheiros) e fala com a Tasca só por mensagens. Cada pedido é confirmado contra as
// permissões que o plugin declarou no catálogo. A API está em docs/plugins.md.
(function () {
  "use strict";
  // Código que corre dentro do iframe, antes do plugin: cria o objeto `tasca`.
  const BOOT = `
"use strict";
const pend = new Map(); let n = 0; const cmds = {}; const ouvintes = [];
const req = (op, a) => new Promise((res, rej) => { const id = ++n; pend.set(id, { res, rej }); parent.postMessage({ tasca: 1, id, op, a }, "*"); });
addEventListener("message", (e) => {
  if (e.source !== parent) return;
  const d = e.data || {}; if (!d.tasca) return;
  if (d.re) { const p = pend.get(d.re); if (p) { pend.delete(d.re); d.erro ? p.rej(new Error(d.erro)) : p.res(d.valor); } return; }
  const falhou = (x) => req("toast", { texto: "Erro no plugin: " + (x && x.message || x) });
  if (d.ev === "comando" && cmds[d.nome]) Promise.resolve().then(() => cmds[d.nome](d.dados)).catch(falhou);
  if (d.ev === "mensagem") for (const f of ouvintes) Promise.resolve().then(() => f(d.dados)).catch(falhou);
});
self.tasca = Object.freeze({
  versao: 1,
  comando(nome, descricao, fn) { if (typeof descricao === "function") { fn = descricao; descricao = ""; } cmds[String(nome).toLowerCase()] = fn; return req("comando", { nome: String(nome), descricao: String(descricao || "") }); },
  enviarMensagem(canal, texto) { return req("enviar", { canal, texto: String(texto) }); },
  toast(texto) { return req("toast", { texto: String(texto) }); },
  guardar(chave, valor) { return req("guardar", { chave: String(chave), valor }); },
  ler(chave) { return req("ler", { chave: String(chave) }); },
  aoReceberMensagem(fn) { ouvintes.push(fn); return req("ouvir", {}); },
  canalAtual() { return req("canal", {}); },
});
`;
  const ativos = new Map(); // janela do iframe → { id, nome, perms, el, ouve }
  const comandos = new Map(); // nome → { plugin, descricao }

  const tem = (p, perm) => (p.perms || []).includes(perm);
  const chaveLocal = (p, k) => "tasca:plugin:" + p.id + ":" + k;

  async function pedido(p, op, a) {
    switch (op) {
      case "comando": {
        if (!tem(p, "comandos")) throw new Error("sem permissão: comandos");
        const nome = String(a.nome).toLowerCase();
        if (!/^[a-z0-9à-ú_-]{1,32}$/.test(nome)) throw new Error("nome de comando inválido");
        const dono = comandos.get(nome);
        if (dono && dono.plugin !== p.id) throw new Error("o comando /" + nome + " já é de outro plugin");
        comandos.set(nome, { plugin: p.id, descricao: a.descricao.slice(0, 120) });
        return true;
      }
      case "enviar": {
        if (!tem(p, "enviar mensagens")) throw new Error("sem permissão: enviar mensagens");
        const canal = a.canal || (window.tascaApp && window.tascaApp.S.channelId);
        if (!canal) throw new Error("sem canal");
        const texto = String(a.texto).slice(0, 4000);
        if (!texto.trim()) throw new Error("mensagem vazia");
        await api.send_message({ channelId: canal, text: texto });
        return true;
      }
      case "toast": if (window.tascaToast) window.tascaToast(p.nome + ": " + String(a.texto).slice(0, 400)); return true;
      case "guardar": {
        if (!tem(p, "guardar dados")) throw new Error("sem permissão: guardar dados");
        const v = JSON.stringify(a.valor === undefined ? null : a.valor);
        if (v.length > 200000) throw new Error("dados demasiado grandes");
        localStorage.setItem(chaveLocal(p, a.chave), v); return true;
      }
      case "ler": {
        if (!tem(p, "guardar dados")) throw new Error("sem permissão: guardar dados");
        const v = localStorage.getItem(chaveLocal(p, a.chave)); return v == null ? null : JSON.parse(v);
      }
      case "ouvir": if (!tem(p, "ler mensagens")) throw new Error("sem permissão: ler mensagens"); p.ouve = true; return true;
      case "canal": return (window.tascaApp && window.tascaApp.S.channelId) || null;
      default: throw new Error("pedido desconhecido: " + op);
    }
  }

  window.addEventListener("message", async (e) => {
    const p = ativos.get(e.source);
    const d = e.data || {};
    if (!p || !d.tasca || !d.id) return;
    let resp;
    try { resp = { tasca: 1, re: d.id, valor: await pedido(p, d.op, d.a || {}) }; }
    catch (x) { resp = { tasca: 1, re: d.id, erro: String(x && x.message || x) }; }
    try { e.source.postMessage(resp, "*"); } catch {}
  });

  function arrancar(info) {
    const f = document.createElement("iframe");
    f.hidden = true;
    f.setAttribute("sandbox", "allow-scripts");
    f.title = "Plugin " + info.name;
    const seguro = (s) => String(s).replace(/<\/script/gi, "<\\/script");
    f.srcdoc = `<!doctype html><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'"><script>${seguro(BOOT)}</script><script>${seguro(info.code)}</script>`;
    document.body.appendChild(f);
    ativos.set(f.contentWindow, { id: info.id, nome: info.name, perms: info.perms || [], el: f, ouve: false });
  }

  function parar() {
    for (const p of ativos.values()) p.el.remove();
    ativos.clear(); comandos.clear();
  }

  async function carregar() {
    parar();
    if (api.isMock) return;
    let lista = [];
    try { lista = await api.list_plugins(); } catch (e) { console.warn("plugins:", e); return; }
    for (const p of lista.filter((x) => x.installed && x.enabled)) {
      try { arrancar(await api.plugin_code({ id: p.id })); } catch (e) { console.warn("plugin " + p.id + ":", e); }
    }
  }

  // "/comando argumentos" escrito no chat: se for de um plugin, vai para ele e não é enviado.
  function correr(texto, channelId) {
    const m = /^\/([^\s/]+)\s*([\s\S]*)$/.exec(texto.trim());
    if (!m) return false;
    const c = comandos.get(m[1].toLowerCase());
    if (!c) return false;
    for (const [w, p] of ativos) if (p.id === c.plugin) w.postMessage({ tasca: 1, ev: "comando", nome: m[1].toLowerCase(), dados: { args: m[2], channelId } }, "*");
    return true;
  }

  api.on("message", ({ channelId, message }) => {
    if (!message) return;
    for (const [w, p] of ativos) if (p.ouve) w.postMessage({ tasca: 1, ev: "mensagem", dados: { channelId, from: message.from, text: message.text || "" } }, "*");
  });

  window.tascaPlugins = { _arrancar: arrancar, carregar, correr, comandos: () => [...comandos].map(([nome, c]) => ({ nome, descricao: c.descricao })) };
  setTimeout(carregar, 1500);
})();
