// Tasca — definições do grupo, cargos, permissões e moderação.
//
// O dono do grupo cria cargos e dá-os às pessoas. Quem tem a permissão certa modera:
// ninguém ganha poderes por alojar uma chamada. Cada ação é um evento assinado que o
// anfitrião só aceita se a pessoa tiver mesmo a permissão (tasca_core::cargos).
//
// - tascaGrupo.openSettings(tab): página das definições do grupo (visão geral, cargos,
//   membros, convites, AutoMod, segurança, banidos, denúncias, registo de auditoria).
// - tascaGrupo.channelPerms(canal | "cat:<nome>"): exceções de permissões de um canal.
// - Botão direito numa pessoa: cargos, apelido, castigo, expulsar, banir e, numa chamada,
//   silenciar, ensurdecer, desligar e mover (via window.tascaHooks.userMenu).
// - Mensagens: fixar, ver as fixadas, denunciar.
(function () {
  "use strict";
  const A = () => window.tascaApp;
  const $ = (s, el = document) => el.querySelector(s);
  const esc = (s) => A().esc(s == null ? "" : s);

  // ---------------- permissões (os mesmos bits de tasca_core::cargos) ----------------
  const P = {
    VER_CANAIS: 2 ** 0, GERIR_CANAIS: 2 ** 1, GERIR_CARGOS: 2 ** 2, GERIR_EXPRESSOES: 2 ** 3, VER_REGISTO: 2 ** 4,
    GERIR_GRUPO: 2 ** 5, GERIR_INTEGRACOES: 2 ** 6, CRIAR_CONVITES: 2 ** 7, MUDAR_APELIDO: 2 ** 8, GERIR_APELIDOS: 2 ** 9,
    EXPULSAR: 2 ** 10, BANIR: 2 ** 11, CASTIGAR: 2 ** 12, ENVIAR_MENSAGENS: 2 ** 13, ANEXAR: 2 ** 14, PRE_VISUALIZAR: 2 ** 15,
    REAGIR: 2 ** 16, EMOJIS_DE_FORA: 2 ** 17, MENCIONAR_TODOS: 2 ** 18, GERIR_MENSAGENS: 2 ** 19, LER_HISTORICO: 2 ** 20,
    CRIAR_TOPICOS: 2 ** 21, LIGAR: 2 ** 22, FALAR: 2 ** 23, VIDEO: 2 ** 24, SILENCIAR_MEMBROS: 2 ** 25,
    ENSURDECER_MEMBROS: 2 ** 26, MOVER_MEMBROS: 2 ** 27, PRIORIDADE: 2 ** 28, ALOJAR_CHAMADAS: 2 ** 29,
    USAR_COMANDOS: 2 ** 30, ADMINISTRADOR: 2 ** 31,
  };
  const TODAS = 2 ** 32 - 1;
  const BASE = P.VER_CANAIS + P.CRIAR_CONVITES + P.MUDAR_APELIDO + P.ENVIAR_MENSAGENS + P.ANEXAR + P.PRE_VISUALIZAR + P.REAGIR +
    P.EMOJIS_DE_FORA + P.LER_HISTORICO + P.CRIAR_TOPICOS + P.LIGAR + P.FALAR + P.VIDEO + P.ALOJAR_CHAMADAS + P.USAR_COMANDOS;
  const has = (p, b) => Math.floor((p || 0) / b) % 2 === 1;
  const put = (p, b, on) => (has(p, b) === !!on ? p : on ? p + b : p - b);
  // Permissões que fazem sentido num canal (exceções).
  const CANAL_TXT = ["VER_CANAIS", "ENVIAR_MENSAGENS", "ANEXAR", "PRE_VISUALIZAR", "REAGIR", "MENCIONAR_TODOS", "GERIR_MENSAGENS", "LER_HISTORICO", "CRIAR_TOPICOS", "GERIR_CANAIS"];
  const CANAL_VOZ = ["VER_CANAIS", "LIGAR", "FALAR", "VIDEO", "SILENCIAR_MEMBROS", "ENSURDECER_MEMBROS", "MOVER_MEMBROS", "PRIORIDADE", "ALOJAR_CHAMADAS", "GERIR_CANAIS"];

  // A tua permissão no grupo (ou num canal). Dono: tudo.
  function can(bit, chan) {
    const { S, group } = A();
    const g = group(); if (!g || S.home) return false;
    if (g.owner === "me") return true;
    let p = g.perms != null ? g.perms : BASE;
    if (chan) { const c = typeof chan === "string" ? (S.channels.find((x) => x.id === chan) || {}) : chan; if (c.perms != null) p = c.perms; }
    return has(p, P.ADMINISTRADOR) || has(p, bit);
  }
  const me = () => (A().S.members || []).find((m) => m.id === "me") || {};
  const myRank = () => { const g = A().group(); return g && g.owner === "me" ? -1 : me().rank ?? 9999; };
  const member = (id) => (A().S.members || []).find((m) => m.id === id);
  // Posso mexer nesta pessoa? (o dono manda em todos; ninguém mexe no dono)
  function above(id) {
    const m = member(id); if (!m || id === "me" || m.owner) return false;
    return myRank() < (m.rank ?? 9999);
  }

  // ---------------- estado ----------------
  const G = { roles: {}, catalog: [], all: TODAS, base: BASE, pins: null };
  async function loadRoles(gid) {
    try { const r = await api.list_roles({ groupId: gid }); G.roles[gid] = r.roles; G.catalog = r.catalog; G.all = r.all; G.base = r.base; } catch (x) { console.warn(x); }
    return G.roles[gid] || [];
  }
  const roleOf = (gid, id) => (G.roles[gid] || []).find((r) => r.id === id);
  const fmtDate = (ms) => new Date(ms).toLocaleString("pt-PT", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
  const fmtIn = (ms) => {
    const s = Math.max(0, Math.round((ms - Date.now()) / 1000));
    if (s < 3600) return Math.max(1, Math.round(s / 60)) + " min";
    if (s < 86400) return Math.round(s / 3600) + " h";
    return Math.round(s / 86400) + " dias";
  };
  const nameOf = (id) => { const m = member(id); return (m && (m.nick || m.name)) || A().S.people[id]?.name || (id === "me" ? "Tu" : "Alguém"); };
  async function act(fn, ok) {
    try {
      const r = await fn();
      const { S } = A();
      if (S.groupId) await refreshGroup(S.groupId);
      if (ok) A().toast(r === false ? ok + " Vai valer quando o anfitrião voltar." : ok);
      return true;
    } catch (x) { A().toast(String(x.message || x)); return false; }
  }
  // Volta a ler o grupo (permissões, membros, canais) depois de uma mudança.
  async function refreshGroup(gid) {
    const { S, render } = A();
    try {
      const [gs, mem, ch] = await Promise.all([api.list_groups(), api.list_members({ groupId: gid }), api.list_channels({ groupId: gid })]);
      const g = gs.find((x) => x.id === gid);
      if (g) { const i = S.groups.findIndex((x) => x.id === gid); if (i >= 0) S.groups[i] = { ...S.groups[i], ...g }; }
      S.allMembers[gid] = mem; S.allChannels[gid] = ch;
      if (S.groupId === gid) { S.members = mem; S.channels = ch; }
      if (G.roles[gid]) await loadRoles(gid);
    } catch (x) { console.warn(x); }
    render();
    if (open) drawSettings();
  }

  // ---------------- janelas pequenas ----------------
  function ask(title, body, okLabel, danger, extra = "") {
    return new Promise((res) => {
      A().modal(`<h2>${title}</h2>${body ? `<p>${body}</p>` : ""}${extra}<div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn ${danger ? "danger" : "primary"}" id="ask-ok">${okLabel}</button></div>`,
        (el, close) => {
          let done = false;
          $("#ask-ok", el).onclick = () => { done = true; const vals = {}; el.querySelectorAll("[data-v]").forEach((x) => (vals[x.dataset.v] = x.type === "checkbox" ? x.checked : x.value)); close(); res(vals); };
          const obs = new MutationObserver(() => { if (!document.body.contains(el)) { obs.disconnect(); if (!done) res(null); } });
          obs.observe(document.body, { childList: true });
        });
    });
  }
  const reasonField = (ph = "Opcional. Fica no registo de auditoria.") => `<div class="field"><label>Motivo</label><input class="input" data-v="reason" maxlength="500" placeholder="${ph}"></div>`;

  async function kick(id) {
    const v = await ask("Expulsar " + esc(nameOf(id)) + "?", "Sai do grupo, mas pode voltar com um convite novo.", "Expulsar", true, reasonField());
    if (v) act(() => api.kick_member({ groupId: A().S.groupId, userId: id, reason: v.reason }), nameOf(id) + " foi expulso.");
  }
  async function ban(id) {
    const v = await ask("Banir " + esc(nameOf(id)) + "?", "Sai do grupo e não pode voltar, mesmo com convite, até levantares o ban.", "Banir", true,
      reasonField() + `<div class="field"><label>Apagar as mensagens dele</label><select class="input" data-v="del"><option value="0">Não apagar</option><option value="3600">Da última hora</option><option value="21600">Das últimas 6 horas</option><option value="86400">Das últimas 24 horas</option><option value="259200">Dos últimos 3 dias</option><option value="604800">Dos últimos 7 dias</option></select></div>`);
    if (v) act(() => api.ban_member({ groupId: A().S.groupId, userId: id, reason: v.reason, deleteSecs: +v.del }), nameOf(id) + " foi banido.");
  }
  const DURS = [[60, "60 segundos"], [300, "5 minutos"], [600, "10 minutos"], [3600, "1 hora"], [86400, "1 dia"], [604800, "1 semana"]];
  async function timeout(id, secs) {
    if (!secs) {
      const v = await ask("Pôr " + esc(nameOf(id)) + " de castigo", "Durante o castigo, a pessoa lê mas não escreve, não reage e não fala nas chamadas.", "Pôr de castigo", true,
        `<div class="field"><label>Durante</label><select class="input" data-v="secs">${DURS.map(([s, l]) => `<option value="${s}" ${s === 600 ? "selected" : ""}>${l}</option>`).join("")}</select></div>` + reasonField());
      if (!v) return; secs = +v.secs;
      return act(() => api.timeout_member({ groupId: A().S.groupId, userId: id, secs, reason: v.reason }), nameOf(id) + " está de castigo.");
    }
    act(() => api.timeout_member({ groupId: A().S.groupId, userId: id, secs, reason: "" }), nameOf(id) + " está de castigo.");
  }
  async function nick(id) {
    const m = member(id) || {};
    const v = await ask(id === "me" ? "O teu apelido neste grupo" : "Apelido de " + esc(m.name || nameOf(id)), "Toda a gente do grupo vê este nome. Deixa vazio para voltar ao nome da conta.", "Guardar", false,
      `<div class="field"><input class="input" data-v="nick" maxlength="32" value="${esc(m.nick || "")}" placeholder="${esc(m.name || "")}"></div>`);
    if (v) act(() => api.set_nick({ groupId: A().S.groupId, userId: id, nick: v.nick.trim() || null }), "Apelido guardado.");
  }
  function toggleRole(id, rid) {
    const m = member(id); if (!m) return;
    const cur = m.roles || [];
    const roles = cur.includes(rid) ? cur.filter((x) => x !== rid) : [...cur, rid];
    act(() => api.set_member_roles({ groupId: A().S.groupId, userId: id, roles }), null);
  }
  const vmod = (id, action, room) => act(() => api.voice_mod({ groupId: A().S.groupId, userId: id, action, room }), {
    mute: "Silenciaste " + nameOf(id) + " para todos.", unmute: nameOf(id) + " já pode falar.", deafen: "Cortaste o som a " + nameOf(id) + ".",
    undeafen: nameOf(id) + " já ouve.", disconnect: nameOf(id) + " saiu da chamada.", move: nameOf(id) + " mudou de sala.", stopvideo: "Desligaste o vídeo de " + nameOf(id) + ".",
  }[action]);

  // ---------------- botão direito numa pessoa ----------------
  (window.tascaHooks = window.tascaHooks || {}).userMenu = window.tascaHooks.userMenu || [];
  window.tascaHooks.userMenu.push((p, ctx) => {
    const { S, I } = A();
    const g = A().group();
    if (S.home || !g || !ctx.groupId || p.bot) return [];
    const m = member(p.id); if (!m) return [];
    const items = [];
    const self = p.id === "me";
    if (self ? can(P.MUDAR_APELIDO) : can(P.GERIR_APELIDOS) && above(p.id)) items.push({ label: self ? "Mudar o meu apelido no grupo" : "Mudar o apelido no grupo", hint: "Toda a gente vê", onClick: () => nick(p.id) });
    const roles = (G.roles[g.id] || []).filter((r) => r.id !== "todos");
    if (can(P.GERIR_CARGOS) && roles.length) {
      const mine = myRank();
      items.push({
        label: "Cargos", items: roles.map((r, i) => ({
          label: r.name, checked: (m.roles || []).includes(r.id), disabled: !(g.owner === "me" || i > mine) || (!self && !above(p.id) && g.owner !== "me"),
          onClick: () => toggleRole(p.id, r.id),
        })),
      });
    } else if (can(P.GERIR_CARGOS)) items.push({ label: "Criar cargos…", onClick: () => openSettings("cargos") });
    // numa chamada
    const inMyCall = S.voice && !S.voice.private && S.voice.groupId === g.id && (p.id === "me" || !!(S.voiceStates || {})[p.id]);
    if (inMyCall && !self && above(p.id)) {
      const vm = [];
      if (can(P.SILENCIAR_MEMBROS)) vm.push({ label: m.serverMuted ? "Tirar o silêncio" : "Silenciar para todos", danger: !m.serverMuted, onClick: () => vmod(p.id, m.serverMuted ? "unmute" : "mute") },
        { label: "Desligar câmara e ecrã", onClick: () => vmod(p.id, "stopvideo") });
      if (can(P.ENSURDECER_MEMBROS)) vm.push({ label: m.serverDeaf ? "Devolver o som" : "Cortar o som", onClick: () => vmod(p.id, m.serverDeaf ? "undeafen" : "deafen") });
      if (can(P.MOVER_MEMBROS)) {
        const salas = S.channels.filter((c) => c.kind === "voice" && c.id !== S.voice.channelId);
        if (salas.length) vm.push({ label: "Mover para", items: salas.map((c) => ({ label: c.name, onClick: () => vmod(p.id, "move", c.id) })) });
        vm.push({ label: "Tirar da chamada", danger: true, onClick: () => vmod(p.id, "disconnect") });
      }
      if (vm.length) items.push("-", ...vm);
    }
    if (!self && above(p.id)) {
      const mod = [];
      if (can(P.CASTIGAR)) {
        if (m.timeoutUntil && m.timeoutUntil > Date.now()) mod.push({ label: "Tirar o castigo", hint: "Acaba daqui a " + fmtIn(m.timeoutUntil), onClick: () => act(() => api.timeout_member({ groupId: g.id, userId: p.id, secs: 0 }), "Castigo tirado.") });
        else mod.push({ label: "Pôr de castigo", items: [...DURS.map(([s, l]) => ({ label: l, onClick: () => timeout(p.id, s) })), "-", { label: "Com motivo…", onClick: () => timeout(p.id) }] });
      }
      if (can(P.EXPULSAR)) mod.push({ label: "Expulsar " + nameOf(p.id), danger: true, onClick: () => kick(p.id) });
      if (can(P.BANIR)) mod.push({ label: "Banir " + nameOf(p.id), danger: true, onClick: () => ban(p.id) });
      if (mod.length) items.push("-", ...mod);
    }
    return items;
  });

  // ---------------- mensagens fixadas ----------------
  async function openPins() {
    const { S, modal } = A();
    const ch = S.channelId; if (!ch) return;
    let list = [];
    try { list = await api.list_pins({ channelId: ch }); } catch (x) { return A().toast(String(x.message || x)); }
    const c = S.channels.find((x) => x.id === ch) || {};
    modal(`<h2>Mensagens fixadas em #${esc(c.name || "")}</h2>
      <div class="pins">${list.length ? list.map((m) => { const p = A().S.people[m.from] || { name: "Alguém", color: "#555" }; return `<div class="pin-row"><div class="pin-head">${A().avatar(p, "s")}<b>${esc(nameOf(m.from))}</b><small>${fmtDate(m.ts)}</small><span style="flex:1"></span>
        <button class="btn ghost sm" data-pin-jump="${esc(m.id)}">Ir para</button>${can(P.GERIR_MENSAGENS, ch) ? `<button class="btn ghost sm" data-pin-off="${esc(m.id)}">Desafixar</button>` : ""}</div><div class="pin-body">${esc(m.text || ((m.files || [])[0] || {}).name || "")}</div></div>`; }).join("") : `<p class="muted">Ainda não há mensagens fixadas neste canal. Quem pode gerir mensagens fixa-as no menu de cada mensagem.</p>`}</div>
      <div class="actions"><button class="btn ghost" data-close>Fechar</button></div>`,
    (el, close) => {
      el.addEventListener("click", async (e) => {
        const j = e.target.closest("[data-pin-jump]"); if (j) { close(); const t = document.querySelector(`.msg[data-mid="${CSS.escape(j.dataset.pinJump)}"]`); if (t) { t.scrollIntoView({ block: "center", behavior: "smooth" }); t.classList.add("flash"); } else A().toast("Essa mensagem já não está carregada."); }
        const o = e.target.closest("[data-pin-off]"); if (o) { close(); await act(() => api.pin_message({ channelId: ch, id: o.dataset.pinOff, on: false }), "Mensagem desafixada."); }
      });
    });
  }
  function pin(m, on) { act(() => api.pin_message({ channelId: A().S.channelId, id: m.id, on }), on ? "Mensagem fixada." : "Mensagem desafixada.").then(() => window.tascaApp.refreshMessages && window.tascaApp.refreshMessages()); }
  async function report(m) {
    const v = await ask("Denunciar mensagem", "Os moderadores do grupo (quem vê o registo de auditoria) recebem a denúncia, com o teu nome.", "Denunciar", true,
      `<div class="field"><label>Porquê?</label><select class="input" data-v="reason"><option>Spam</option><option>Insultos ou assédio</option><option>Conteúdo impróprio</option><option>Burla ou link perigoso</option><option>Outro</option></select></div>`);
    if (v) act(() => api.report_message({ channelId: A().S.channelId, id: m.id, reason: v.reason }), "Denúncia enviada aos moderadores.");
  }

  // ---------------- convites ----------------
  async function openInvite() {
    const { S, modal, group, copy } = A();
    const g = group(); if (!g) return;
    if (!can(P.CRIAR_CONVITES)) return A().toast("Não tens permissão para convidar pessoas neste grupo.");
    modal(`<h2>Convidar para ${esc(g.name)}</h2>
      <p>Envia este código a quem quiseres convidar.</p>
      <div class="inv-opts"><div class="field"><label for="inv-exp">Expira</label><select class="input" id="inv-exp"><option value="1800">Daqui a 30 minutos</option><option value="3600">Daqui a 1 hora</option><option value="86400">Daqui a 1 dia</option><option value="604800" selected>Daqui a 7 dias</option><option value="0">Nunca</option></select></div>
      <div class="field"><label for="inv-max">Usos</label><select class="input" id="inv-max"><option value="0">Sem limite</option><option value="1">1 pessoa</option><option value="5">5 pessoas</option><option value="10">10 pessoas</option><option value="25">25 pessoas</option><option value="100">100 pessoas</option></select></div></div>
      <div class="field"><label for="inv">Convite</label><textarea class="input mono" id="inv" rows="3" readonly placeholder="A criar…"></textarea><small id="inv-note" class="muted"></small></div>
      <div class="actions"><button class="btn ghost" data-close>Fechar</button><button class="btn primary" id="copy-inv" disabled>Copiar convite</button></div>`,
    (el) => {
      const make = async () => {
        $("#copy-inv", el).disabled = true; $("#inv", el).value = "";
        try {
          const inv = await api.create_invite({ groupId: g.id, maxUses: +$("#inv-max", el).value, expiresSecs: +$("#inv-exp", el).value });
          $("#inv", el).value = inv.code; $("#copy-inv", el).disabled = false;
          $("#inv-note", el).textContent = inv.pending ? "O anfitrião está desligado: o convite começa a funcionar quando ele voltar." : "";
        } catch (x) { $("#inv-note", el).textContent = String(x.message || x); }
      };
      $("#inv-exp", el).onchange = make; $("#inv-max", el).onchange = make;
      $("#copy-inv", el).onclick = () => copy($("#inv", el).value, $("#inv", el), "Convite copiado.");
      make();
    });
  }

  // ---------------- exceções de um canal ----------------
  async function channelPerms(chan) {
    const { S, modal } = A();
    const g = A().group(); if (!g) return;
    const isCat = chan.startsWith("cat:");
    const c = isCat ? null : S.channels.find((x) => x.id === chan);
    const voice = c && c.kind === "voice";
    const keys = isCat ? [...new Set([...CANAL_TXT, ...CANAL_VOZ])] : voice ? CANAL_VOZ : CANAL_TXT;
    await loadRoles(g.id);
    let list = [];
    try { list = await api.get_channel_perms({ groupId: g.id, channel: chan }); } catch (x) { return A().toast(String(x.message || x)); }
    const roles = G.roles[g.id] || [];
    if (!list.some((o) => o.target === "role:todos")) list.unshift({ target: "role:todos", allow: 0, deny: 0 });
    let sel = list[0].target;
    const label = (t) => t.startsWith("role:") ? (roleOf(g.id, t.slice(5)) || { name: t === "role:todos" ? "@todos" : "cargo apagado" }).name : nameOf(t.slice(5));
    const catName = (k) => (G.catalog.find((x) => x.bit === P[k]) || { name: k }).name;
    const catDesc = (k) => (G.catalog.find((x) => x.bit === P[k]) || { desc: "" }).desc;
    const priv = () => has((list.find((o) => o.target === "role:todos") || {}).deny, P.VER_CANAIS);
    modal(`<h2>Permissões de ${isCat ? "categoria " + esc(chan.slice(4)) : (voice ? "" : "#") + esc(c ? c.name : "")}</h2>
      <p class="muted">${isCat ? "Os canais desta categoria herdam estas exceções; as do próprio canal vêm por cima." : "Exceções só para este canal. Por cima das permissões dos cargos."}</p>
      <div class="box"><div class="row"><div class="txt"><b>${isCat ? "Categoria privada" : "Canal privado"}</b><small>Só quem tiver um cargo (ou for adicionado) com «Ver canais» aqui o vê.</small></div><label class="switch"><input type="checkbox" id="cp-priv" ${priv() ? "checked" : ""}><span></span></label></div></div>
      <div class="cp-wrap"><div class="cp-targets" id="cp-targets"></div><div class="cp-perms" id="cp-perms"></div></div>
      <p id="cp-err" style="color:var(--perigo)" hidden></p>
      <div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn primary" id="cp-save">Guardar</button></div>`,
    (el, close) => {
      const draw = () => {
        $("#cp-targets", el).innerHTML = list.map((o) => `<button class="${o.target === sel ? "on" : ""}" data-cp-t="${esc(o.target)}">${o.target.startsWith("user:") ? "👤 " : "🛡️ "}${esc(label(o.target))}</button>`).join("") +
          `<select class="input" id="cp-add"><option value="">+ Adicionar cargo ou pessoa</option>${roles.filter((r) => !list.some((o) => o.target === "role:" + r.id)).map((r) => `<option value="role:${esc(r.id)}">🛡️ ${esc(r.name)}</option>`).join("")}${(S.members || []).filter((m) => !list.some((o) => o.target === "user:" + m.id)).map((m) => `<option value="user:${esc(m.id)}">👤 ${esc(m.nick || m.name)}</option>`).join("")}</select>`;
        const o = list.find((x) => x.target === sel) || list[0];
        $("#cp-perms", el).innerHTML = `<div class="cp-head"><b>${esc(label(o.target))}</b>${o.target !== "role:todos" ? `<button class="btn ghost sm" data-cp-rm>Tirar</button>` : ""}</div>` + keys.map((k) => {
          const st = has(o.deny, P[k]) ? "deny" : has(o.allow, P[k]) ? "allow" : "inherit";
          return `<div class="cp-row"><div class="txt"><b>${esc(catName(k))}</b><small>${esc(catDesc(k))}</small></div><div class="tri" role="radiogroup">
            <button class="${st === "deny" ? "on deny" : ""}" data-tri="${k}|deny" aria-label="Tirar" title="Tirar">✕</button>
            <button class="${st === "inherit" ? "on" : ""}" data-tri="${k}|inherit" aria-label="Como no cargo" title="Como no cargo">/</button>
            <button class="${st === "allow" ? "on allow" : ""}" data-tri="${k}|allow" aria-label="Dar" title="Dar">✓</button></div></div>`;
        }).join("");
        $("#cp-priv", el).checked = priv();
      };
      draw();
      el.addEventListener("change", (e) => {
        if (e.target.id === "cp-add" && e.target.value) { list.push({ target: e.target.value, allow: 0, deny: 0 }); sel = e.target.value; draw(); }
        if (e.target.id === "cp-priv") {
          const t = list.find((o) => o.target === "role:todos");
          t.deny = put(t.deny, P.VER_CANAIS, e.target.checked); if (e.target.checked) t.allow = put(t.allow, P.VER_CANAIS, false);
          draw();
        }
      });
      el.addEventListener("click", (e) => {
        const t = e.target.closest("[data-cp-t]"); if (t) { sel = t.dataset.cpT; return draw(); }
        if (e.target.closest("[data-cp-rm]")) { list = list.filter((o) => o.target !== sel); sel = list[0].target; return draw(); }
        const b = e.target.closest("[data-tri]"); if (b) {
          const [k, st] = b.dataset.tri.split("|"); const o = list.find((x) => x.target === sel);
          o.allow = put(o.allow, P[k], st === "allow"); o.deny = put(o.deny, P[k], st === "deny"); draw();
        }
      });
      $("#cp-save", el).onclick = async () => {
        try { await api.set_channel_perms({ groupId: g.id, channel: chan, overrides: list }); close(); await refreshGroup(g.id); A().toast("Permissões guardadas."); }
        catch (x) { const p = $("#cp-err", el); p.textContent = String(x.message || x); p.hidden = false; }
      };
    });
  }

  // ---------------- definições do grupo ----------------
  let open = false, tab = "geral", draft = null, saved = null, roleSel = null, roleTab = "visual", roleDraft = null, permQuery = "", auditFilter = { who: "", what: "" }, memberQuery = "";
  const TABS = () => {
    const t = [];
    if (can(P.GERIR_GRUPO)) t.push(["Grupo", [["geral", "Visão geral"], ["automod", "AutoMod"], ["seguranca", "Segurança e entrada"]]]);
    const ppl = [];
    if (can(P.GERIR_CARGOS) || can(P.GERIR_GRUPO)) ppl.push(["cargos", "Cargos"]);
    ppl.push(["membros", "Membros"]);
    if (can(P.CRIAR_CONVITES) || can(P.GERIR_GRUPO)) ppl.push(["convites", "Convites"]);
    if (can(P.BANIR)) ppl.push(["banidos", "Banidos"]);
    t.push(["Pessoas", ppl]);
    if (can(P.VER_REGISTO)) t.push(["Moderação", [["denuncias", "Denúncias"], ["registo", "Registo de auditoria"]]]);
    return t;
  };
  let cache = {};
  async function openSettings(t) {
    const g = A().group(); if (!g) return;
    open = true; tab = t || (can(P.GERIR_GRUPO) ? "geral" : can(P.GERIR_CARGOS) ? "cargos" : "membros");
    draft = null; roleSel = null; roleDraft = null; cache = {};
    await loadRoles(g.id);
    let el = $("#gset"); if (!el) { el = document.createElement("div"); el.id = "gset"; el.className = "gset"; document.body.appendChild(el); }
    el.addEventListener("click", onClick); el.addEventListener("input", onInput); el.addEventListener("change", onInput);
    await loadTab();
    drawSettings();
    requestAnimationFrame(() => el.classList.add("in"));
  }
  function closeSettings() {
    if (dirty() && !confirm2()) return;
    open = false; const el = $("#gset"); if (!el) return;
    el.classList.remove("in"); setTimeout(() => el.remove(), 180);
  }
  let warned = false;
  function confirm2() { if (warned) return true; warned = true; A().toast("Tens alterações por guardar. Carrega outra vez para sair sem guardar."); setTimeout(() => (warned = false), 3000); return false; }
  const dirty = () => (draft && JSON.stringify(draft) !== saved) || (roleDraft && JSON.stringify(roleDraft) !== JSON.stringify(G.roles[A().S.groupId]));
  async function loadTab() {
    const g = A().group(); if (!g) return;
    try {
      if (["geral", "automod", "seguranca"].includes(tab) && !draft) { draft = await api.get_group_settings({ groupId: g.id }); saved = JSON.stringify(draft); }
      if (tab === "convites") cache.invites = await api.list_invites({ groupId: g.id });
      if (tab === "banidos") cache.bans = await api.list_bans({ groupId: g.id });
      if (tab === "registo" || tab === "denuncias") cache.audit = await api.audit_log({ groupId: g.id });
      if (tab === "cargos" && !roleDraft) { roleDraft = JSON.parse(JSON.stringify(G.roles[g.id] || [])); if (!roleSel) roleSel = (roleDraft[0] || {}).id; }
    } catch (x) { A().toast(String(x.message || x)); }
  }
  function drawSettings() {
    const el = $("#gset"); if (!el || !open) return;
    const g = A().group(); if (!g) return closeSettings();
    const tabs = TABS();
    if (!tabs.some(([, items]) => items.some(([k]) => k === tab))) tab = tabs[0][1][0][0];
    const nav = tabs.map(([cat, items]) => items.length ? `<div class="cat"><span>${cat}</span></div>${items.map(([k, l]) => `<button class="${tab === k ? "on" : ""}" data-gs-tab="${k}">${l}</button>`).join("")}` : "").join("") +
      `<div class="cat"><span></span></div>${g.owner === "me" ? `<button class="danger-t" data-gs-act="delete">Apagar grupo</button>` : `<button class="danger-t" data-gs-act="leave">Sair do grupo</button>`}`;
    const title = tabs.flatMap((x) => x[1]).find((x) => x[0] === tab)[1];
    const body = { geral: tGeral, cargos: tCargos, membros: tMembros, convites: tConvites, automod: tAutoMod, seguranca: tSeguranca, banidos: tBanidos, registo: tRegisto, denuncias: tDenuncias }[tab]();
    const keep = el.querySelector(".page") ? el.querySelector(".page").scrollTop : 0;
    const navLeft = el.querySelector("nav") ? el.querySelector("nav").scrollLeft : 0;
    const focus = document.activeElement && el.contains(document.activeElement) && document.activeElement.id;
    el.innerHTML = `<div class="settings"><nav aria-label="Definições do grupo"><div class="gs-name">${esc(g.name)}</div>${nav}</nav>
      <div class="page"><div class="page-inner"><div class="set-head" style="display:flex;align-items:center;gap:12px"><h1 style="flex:1">${title}</h1>
      <button class="icon-btn" data-gs-act="close" aria-label="Fechar" title="Fechar (Esc)">${A().I.close}</button></div>${body}</div></div></div>
      ${dirty() ? `<div class="save-bar"><span>Cuidado: tens alterações por guardar.</span><button class="btn ghost" data-gs-act="reset">Repor</button><button class="btn primary" data-gs-act="save">Guardar alterações</button></div>` : ""}`;
    const pg = el.querySelector(".page"); if (pg) pg.scrollTop = keep;
    const nv = el.querySelector("nav"); if (nv) nv.scrollLeft = navLeft;
    if (focus) { const f = document.getElementById(focus); if (f) { f.focus(); if (f.setSelectionRange && f.value != null && f.type !== "checkbox" && f.type !== "color" && f.tagName !== "SELECT") try { f.setSelectionRange(f.value.length, f.value.length); } catch {} } }
  }
  const sw = (key, on, label) => `<label class="switch"><input type="checkbox" data-gs="${key}" ${on ? "checked" : ""}><span></span><span class="sr">${label}</span></label>`;
  const chanOpts = (kind, cur, none = "Nenhum") => `<option value="">${none}</option>` + A().S.channels.filter((c) => c.kind === kind).map((c) => `<option value="${esc(c.id.split("/").pop())}" ${cur === c.id.split("/").pop() ? "selected" : ""}>${kind === "text" ? "#" : "🔊 "}${esc(c.name)}</option>`).join("");

  function tGeral() {
    const d = draft || {};
    const icon = d.icon ? `<img src="${esc(d.icon)}" alt="">` : `<span>${esc(A().initials(d.name || "?"))}</span>`;
    return `<div class="section"><div class="gs-ov"><button class="gs-icon" data-gs-act="icon" aria-label="Mudar o ícone">${icon}<small>Mudar</small></button>
        <div style="flex:1;display:grid;gap:10px"><div class="field"><label>Nome do grupo</label><input class="input" id="gs-name" data-gs="name" maxlength="40" value="${esc(d.name || "")}"></div>
        ${d.icon ? `<button class="btn ghost sm" data-gs-act="icon-rm" style="justify-self:start">Tirar o ícone</button>` : ""}</div></div></div>
      <div class="section"><div class="field"><label>Descrição</label><textarea class="input" id="gs-desc" data-gs="description" rows="3" maxlength="1024" placeholder="Do que é este grupo? Aparece a quem recebe um convite.">${esc(d.description || "")}</textarea></div>
        <div class="field"><label>Cor de destaque</label><div class="swatches">${["", "#9B2C3F", "#2A5CAA", "#5E8C4A", "#B8791A", "#6B4C9A", "#2E7D7A", "#E3A72F", "#7A3B2E"].map((c) => `<button class="sw ${(d.color || "") === c ? "on" : ""}" style="--c:${c || "var(--caixa)"}" data-gs-color="${c}" aria-label="${c || "Sem cor"}">${c ? "" : "∅"}</button>`).join("")}</div></div>
        <div class="field"><label>Etiqueta do grupo</label><input class="input" id="gs-tag" data-gs="tag" maxlength="6" value="${esc(d.tag || "")}" placeholder="Ex.: MALTA" style="max-width:160px"><small class="muted">Até 6 letras. Os membros podem mostrá-la ao lado do nome.</small></div></div>
      <div class="section"><h2>Mensagens do sistema</h2><div class="box">
        <div class="row"><div class="txt"><b>Canal das mensagens do sistema</b><small>Onde aparecem as boas-vindas e quem saiu.</small></div><select class="input" data-gs="system_channel" style="max-width:220px">${chanOpts("text", d.system_channel)}</select></div>
        <div class="row"><div class="txt"><b>Dar as boas-vindas a quem entra</b></div>${sw("welcome", d.welcome, "Boas-vindas")}</div>
        <div class="row"><div class="txt"><b>Avisar quando alguém sai, é expulso ou banido</b></div>${sw("leave_notices", d.leave_notices, "Avisos de saída")}</div></div></div>
      <div class="section"><h2>Inatividade</h2><div class="box">
        <div class="row"><div class="txt"><b>Canal de inatividade</b><small>Quem está calado e sem som numa chamada durante este tempo vai para lá.</small></div><select class="input" data-gs="afk_channel" style="max-width:200px">${chanOpts("voice", d.afk_channel)}</select></div>
        <div class="row"><div class="txt"><b>Tempo</b></div><select class="input" data-gs="afk_secs" data-num style="max-width:200px">${[[60, "1 minuto"], [300, "5 minutos"], [900, "15 minutos"], [1800, "30 minutos"], [3600, "1 hora"]].map(([s, l]) => `<option value="${s}" ${d.afk_secs === s ? "selected" : ""}>${l}</option>`).join("")}</select></div></div></div>
      <div class="section"><h2>Avisos</h2><div class="box"><div class="row"><div class="txt"><b>Para quem entra de novo</b><small>Cada pessoa pode mudar isto para si.</small></div>
        <div class="seg">${[[0, "Todas as mensagens"], [1, "Só menções"]].map(([v, l]) => `<button class="${(d.default_notify || 0) === v ? "on" : ""}" data-gs-notify="${v}">${l}</button>`).join("")}</div></div></div></div>
      <div class="section"><h2>Regras</h2><div class="field"><textarea class="input" id="gs-rules" data-gs="rules" rows="5" maxlength="4000" placeholder="Uma regra por linha. Quem entra vê-as antes de escrever.">${esc(d.rules || "")}</textarea></div></div>`;
  }

  function tSeguranca() {
    const d = draft || {};
    return `<p class="lede">Quem pode entrar e quando pode começar a escrever.</p>
      <div class="section"><h2>Verificação</h2><div class="box">
        <div class="row"><div class="txt"><b>Só contas com @nome de utilizador</b><small>Quem não tiver @ID pode ler, mas não escreve.</small></div>${sw("need_username", d.need_username, "Exigir @ID")}</div>
        <div class="row"><div class="txt"><b>Esperar antes de escrever</b><small>Tempo que alguém novo tem de ser membro antes da primeira mensagem.</small></div><select class="input" data-gs="min_member_secs" data-num style="max-width:200px">${[[0, "Sem espera"], [300, "5 minutos"], [600, "10 minutos"], [1800, "30 minutos"], [3600, "1 hora"], [86400, "1 dia"]].map(([s, l]) => `<option value="${s}" ${(d.min_member_secs || 0) === s ? "selected" : ""}>${l}</option>`).join("")}</select></div></div></div>
      <div class="section"><h2>Proteção contra ataques</h2><div class="box">
        <div class="row"><div class="txt"><b>Limite de entradas por minuto</b><small>Quando muita gente entra de repente (um ataque), as entradas seguintes são recusadas durante um minuto.</small></div><select class="input" data-gs="raid_limit" data-num style="max-width:200px">${[[0, "Sem limite"], [5, "5 por minuto"], [10, "10 por minuto"], [20, "20 por minuto"], [50, "50 por minuto"]].map(([s, l]) => `<option value="${s}" ${(d.raid_limit || 0) === s ? "selected" : ""}>${l}</option>`).join("")}</select></div>
        <div class="row"><div class="txt"><b>Pôr os convites em pausa</b><small>Ninguém entra, nem com convite. Os membros continuam.</small></div>${sw("invites_paused", d.invites_paused, "Pausar convites")}</div></div></div>`;
  }

  // ---- AutoMod ----
  const RULE_TYPES = {
    Palavras: ["Palavras proibidas", "Bloqueia palavras ou expressões. Usa * para apanhar partes de palavras (ex.: *merd*)."],
    Links: ["Links", "Bloqueia links, menos os domínios que permitires."],
    Mencoes: ["Demasiadas menções", "Mensagens com muitas @menções (spam de menções)."],
    Repetidas: ["Mensagens repetidas", "A mesma mensagem várias vezes seguidas."],
    Maiusculas: ["Tudo em maiúsculas", "Mensagens que são quase só maiúsculas."],
    Convites: ["Convites de outros grupos", "Mensagens com convites da Tasca (tasca1…)."],
    Emojis: ["Demasiados emojis", "Mensagens com emojis a mais."],
  };
  const ruleKind = (r) => (typeof r.tipo === "string" ? r.tipo : Object.keys(r.tipo || {})[0]);
  function newRule(kind) {
    const tipo = { Palavras: { Palavras: [] }, Links: { Links: { permitidos: ["youtube.com", "youtu.be", "tenor.com"] } }, Mencoes: { Mencoes: 5 }, Repetidas: { Repetidas: 3 }, Maiusculas: { Maiusculas: 70 }, Convites: "Convites", Emojis: { Emojis: 10 } }[kind];
    return { id: Math.random().toString(16).slice(2, 10), name: RULE_TYPES[kind][0], on: true, tipo, block: true, alert_channel: null, timeout_secs: 0, exempt_roles: [], exempt_channels: [] };
  }
  function tAutoMod() {
    const d = draft || {}; const rules = d.automod || [];
    const g = A().group();
    const roles = (G.roles[g.id] || []).filter((r) => r.id !== "todos");
    return `<p class="lede">O AutoMod vê cada mensagem antes de ela aparecer no grupo. Quem tem «Gerir o grupo» ou é administrador não é apanhado.</p>
      ${rules.map((r, i) => { const k = ruleKind(r); const val = typeof r.tipo === "object" ? r.tipo[k] : null; return `<div class="section"><div class="box rule">
        <div class="row"><div class="txt"><b>${esc(r.name)}</b><small>${esc((RULE_TYPES[k] || [k, ""])[1])}</small></div>${sw("automod." + i + ".on", r.on, "Ligada")}</div>
        ${k === "Palavras" ? `<div class="row col"><label class="muted">Palavras (separadas por vírgulas)</label><textarea class="input" id="am-${i}-w" data-am="${i}|words" rows="2" placeholder="palavrão, *ofensa*, outra expressão">${esc((val || []).join(", "))}</textarea></div>` : ""}
        ${k === "Links" ? `<div class="row col"><label class="muted">Domínios permitidos (separados por vírgulas)</label><input class="input" id="am-${i}-l" data-am="${i}|allow" value="${esc((val.permitidos || []).join(", "))}"></div>` : ""}
        ${["Mencoes", "Repetidas", "Emojis", "Maiusculas"].includes(k) ? `<div class="row"><div class="txt"><b>${k === "Maiusculas" ? "Percentagem máxima" : k === "Repetidas" ? "Quantas vezes seguidas" : "Máximo por mensagem"}</b></div><input class="input" type="number" min="1" max="${k === "Maiusculas" ? 100 : 50}" style="max-width:100px" data-am="${i}|n" value="${val}"></div>` : ""}
        <div class="row"><div class="txt"><b>Bloquear a mensagem</b><small>Quem escreveu é avisado; ninguém mais a vê.</small></div>${sw("automod." + i + ".block", r.block, "Bloquear")}</div>
        <div class="row"><div class="txt"><b>Mandar alerta para</b></div><select class="input" data-gs="automod.${i}.alert_channel" style="max-width:200px">${chanOpts("text", r.alert_channel, "Não mandar")}</select></div>
        <div class="row"><div class="txt"><b>Pôr de castigo</b></div><select class="input" data-gs="automod.${i}.timeout_secs" data-num style="max-width:200px"><option value="0">Não</option>${DURS.map(([s, l]) => `<option value="${s}" ${r.timeout_secs === s ? "selected" : ""}>${l}</option>`).join("")}</select></div>
        ${roles.length ? `<div class="row col"><label class="muted">Cargos isentos</label><div class="chips">${roles.map((ro) => `<button class="chip ${r.exempt_roles.includes(ro.id) ? "on" : ""}" data-am-ex="${i}|${esc(ro.id)}" style="--c:${ro.color || "var(--texto-3)"}">${esc(ro.name)}</button>`).join("")}</div></div>` : ""}
        <div class="row"><span style="flex:1"></span><button class="btn ghost sm danger-t" data-am-rm="${i}">Apagar regra</button></div></div></div>`; }).join("")}
      <div class="section"><h2>Adicionar regra</h2><div class="rule-add">${Object.entries(RULE_TYPES).map(([k, [n, dsc]]) => `<button class="kind-card" data-am-add="${k}"><b>${n}</b><small>${dsc}</small></button>`).join("")}</div></div>`;
  }

  // ---- cargos ----
  const PALETTE = ["#1ABC9C", "#2ECC71", "#3498DB", "#9B59B6", "#E91E63", "#F1C40F", "#E67E22", "#E74C3C", "#95A5A6", "#607D8B", "#9B2C3F", "#2A5CAA", "#5E8C4A", "#E3A72F"];
  const TEMPLATES = {
    Moderador: { color: "#2ECC71", hoist: true, perms: () => BASE + P.EXPULSAR + P.CASTIGAR + P.GERIR_MENSAGENS + P.GERIR_APELIDOS + P.SILENCIAR_MEMBROS + P.ENSURDECER_MEMBROS + P.MOVER_MEMBROS + P.VER_REGISTO + P.MENCIONAR_TODOS },
    Admin: { color: "#E74C3C", hoist: true, perms: () => P.ADMINISTRADOR },
    Membro: { color: "#3498DB", hoist: false, perms: () => BASE },
    Convidado: { color: "#95A5A6", hoist: false, perms: () => P.VER_CANAIS + P.LER_HISTORICO + P.REAGIR + P.LIGAR + P.FALAR },
    Castigo: { color: "#607D8B", hoist: false, perms: () => P.VER_CANAIS + P.LER_HISTORICO },
  };
  function tCargos() {
    const g = A().group();
    const rd = roleDraft || [];
    const mine = myRank();
    const editable = (i) => g.owner === "me" || i > mine;
    const sel = rd.find((r) => r.id === roleSel) || rd[0];
    const idx = rd.indexOf(sel);
    const canEdit = sel && editable(idx) && can(P.GERIR_CARGOS);
    const list = rd.map((r, i) => `<button class="role-item ${r.id === (sel || {}).id ? "on" : ""}" data-role="${esc(r.id)}" ${r.id !== "todos" && editable(i) && can(P.GERIR_CARGOS) && !window.TASCA_MOBILE ? `draggable="true" data-role-drag="${esc(r.id)}"` : ""}>
      <i class="dot" style="background:${r.color || "var(--texto-3)"}"></i><span>${esc(r.name)}</span>${r.icon && !r.icon.startsWith("data:") ? `<em>${esc(r.icon)}</em>` : ""}<small>${r.members ?? ""}</small>${!editable(i) ? "🔒" : ""}</button>`).join("");
    let right = "";
    if (sel) {
      const rtabs = sel.id === "todos" ? [["permissoes", "Permissões"]] : [["visual", "Aparência"], ["permissoes", "Permissões"], ["membros", "Membros"]];
      if (!rtabs.some(([k]) => k === roleTab)) roleTab = rtabs[0][0];
      const tabsHtml = `<div class="seg role-tabs">${rtabs.map(([k, l]) => `<button class="${roleTab === k ? "on" : ""}" data-role-tab="${k}">${l}</button>`).join("")}</div>`;
      const dis = canEdit ? "" : "disabled";
      let inner = "";
      if (roleTab === "visual") inner = `<div class="field"><label>Nome do cargo</label><input class="input" id="role-name" data-rd="name" maxlength="64" value="${esc(sel.name)}" ${dis}></div>
        <div class="field"><label>Cor</label><div class="swatches">${["", ...PALETTE].map((c) => `<button class="sw ${(sel.color || "") === c ? "on" : ""}" style="--c:${c || "var(--caixa)"}" data-rd-color="${c}" ${dis} aria-label="${c || "Sem cor"}">${c ? "" : "∅"}</button>`).join("")}<input type="color" data-rd="color" value="${sel.color || "#999999"}" ${dis} aria-label="Outra cor"></div></div>
        <div class="field"><label>Ícone</label><div style="display:flex;gap:8px;align-items:center"><input class="input" id="role-icon" data-rd="icon" maxlength="8" value="${esc(sel.icon && !sel.icon.startsWith("data:") ? sel.icon : "")}" placeholder="Um emoji, ex.: 🛡️" style="max-width:160px" ${dis}><small class="muted">Aparece ao lado do nome das pessoas com este cargo.</small></div></div>
        <div class="box"><div class="row"><div class="txt"><b>Mostrar à parte na lista de membros</b><small>As pessoas com este cargo aparecem num grupo próprio.</small></div><label class="switch"><input type="checkbox" data-rd="hoist" ${sel.hoist ? "checked" : ""} ${dis}><span></span></label></div>
        <div class="row"><div class="txt"><b>Qualquer pessoa pode mencionar @${esc(sel.name)}</b><small>Sem isto, só quem tem «Mencionar @todos e cargos».</small></div><label class="switch"><input type="checkbox" data-rd="mentionable" ${sel.mentionable ? "checked" : ""} ${dis}><span></span></label></div></div>
        ${canEdit ? `<div class="actions" style="justify-content:flex-start;margin-top:16px"><button class="btn danger" data-role-del>Apagar cargo</button></div>` : ""}`;
      if (roleTab === "permissoes") {
        const q = permQuery.toLowerCase();
        const cats = {};
        for (const p of G.catalog) if (!q || (p.name + " " + p.desc).toLowerCase().includes(q)) (cats[p.cat] = cats[p.cat] || []).push(p);
        inner = `${sel.id === "todos" ? `<p class="lede">O que toda a gente pode fazer. Os outros cargos só acrescentam.</p>` : ""}<input class="input" id="perm-q" data-perm-q placeholder="Procurar permissões" value="${esc(permQuery)}">
          ${canEdit ? `<div style="margin:8px 0"><button class="btn ghost sm" data-perm-clear>Tirar todas</button></div>` : ""}
          ${Object.entries(cats).map(([cat, ps]) => `<div class="section"><h2>${esc(cat)}</h2><div class="box">${ps.map((p) => `<div class="row"><div class="txt"><b>${esc(p.name)}</b><small>${esc(p.desc)}</small></div><label class="switch"><input type="checkbox" data-perm-bit="${p.bit}" ${has(sel.perms, p.bit) ? "checked" : ""} ${dis}><span></span></label></div>`).join("")}</div></div>`).join("") || `<p class="muted">Nenhuma permissão com esse nome.</p>`}`;
      }
      if (roleTab === "membros") {
        const ms = (A().S.members || []).filter((m) => (m.roles || []).includes(sel.id));
        const others = (A().S.members || []).filter((m) => !(m.roles || []).includes(sel.id) && !m.bot);
        inner = `${canEdit && others.length ? `<div class="field"><select class="input" data-role-addm><option value="">+ Adicionar pessoa a este cargo</option>${others.map((m) => `<option value="${esc(m.id)}">${esc(m.nick || m.name)}</option>`).join("")}</select></div>` : ""}
          <div class="box">${ms.map((m) => `<div class="row">${A().avatar(m, "s")}<div class="txt"><b>${esc(m.nick || m.name)}</b>${m.username ? `<small>@${esc(m.username)}</small>` : ""}</div>${canEdit ? `<button class="btn ghost sm" data-role-rmm="${esc(m.id)}">Tirar</button>` : ""}</div>`).join("") || `<div class="row"><div class="txt"><small>Ninguém tem este cargo.</small></div></div>`}</div>
          ${dirty() ? `<p class="muted">Guarda primeiro os cargos para os poderes dar a pessoas.</p>` : ""}`;
      }
      right = `<div class="role-edit"><div class="role-title"><i class="dot" style="background:${sel.color || "var(--texto-3)"}"></i><b>${esc(sel.name)}</b>${!canEdit ? `<small class="muted">Só podes mexer em cargos abaixo do teu.</small>` : ""}</div>${tabsHtml}${inner}</div>`;
    }
    return `<p class="lede">Quem está mais acima na lista manda mais: só se mexe em cargos e pessoas abaixo do próprio cargo. Arrasta para mudar a ordem.</p>
      <div class="roles-wrap"><div class="role-list">${can(P.GERIR_CARGOS) ? `<button class="btn primary" data-role-new>Criar cargo</button><div class="tpl">${Object.keys(TEMPLATES).map((k) => `<button class="chip" data-role-tpl="${k}">+ ${k}</button>`).join("")}</div>` : ""}${list}</div>${right}</div>`;
  }

  function tMembros() {
    const { S } = A();
    const g = A().group();
    const q = memberQuery.toLowerCase();
    const ms = (S.members || []).filter((m) => !q || (m.name + " " + (m.nick || "") + " " + (m.username || "")).toLowerCase().includes(q));
    const fmtD = (ms) => (ms ? new Date(ms).toLocaleDateString("pt-PT", { day: "numeric", month: "short", year: "numeric" }) : "");
    return `<p class="lede">${(S.members || []).length} membros. Toca numa pessoa para ver as ações.</p>
      <input class="input" id="mem-q" data-mem-q placeholder="Procurar membros" value="${esc(memberQuery)}">
      <div class="box" style="margin-top:12px">${ms.map((m) => `<div class="row member-row">${A().avatar(m, "s")}<div class="txt"><b>${esc(m.nick || m.name)}${m.owner ? " 👑" : ""}${m.timeoutUntil && m.timeoutUntil > Date.now() ? ` <span class="tag warn">castigo · ${fmtIn(m.timeoutUntil)}</span>` : ""}</b>
        <small>${[m.username ? "@" + esc(m.username) : "", m.since ? "membro desde " + fmtD(m.since) : ""].filter(Boolean).join(" · ")}</small>
        <div class="chips">${(m.roles || []).map((r) => roleOf(g.id, r)).filter(Boolean).map((r) => `<span class="chip on" style="--c:${r.color || "var(--texto-3)"}">${esc(r.name)}</span>`).join("")}</div></div>
        <button class="icon-btn" data-mem-menu="${esc(m.id)}" aria-label="Ações para ${esc(m.name)}">${A().I.more}</button></div>`).join("")}</div>`;
  }

  function tConvites() {
    const l = cache.invites || [];
    return `<p class="lede">Convites ativos deste grupo. Revogar um convite impede que mais alguém entre com ele.</p>
      ${can(P.CRIAR_CONVITES) ? `<div style="margin-bottom:12px"><button class="btn primary" data-gs-act="invite">Criar convite</button></div>` : ""}
      ${can(P.GERIR_GRUPO) ? `<div class="box"><div class="row"><div class="txt"><b>Pôr os convites em pausa</b><small>Ninguém entra, nem com convite.</small></div><button class="btn ${draft && draft.invites_paused ? "primary" : "ghost"}" data-gs-act="pause">${draft && draft.invites_paused ? "Retomar convites" : "Pausar"}</button></div></div>` : ""}
      <div class="box" style="margin-top:12px">${l.map((i) => `<div class="row"><div class="txt"><b>Criado por ${esc(nameOf(i.by))}</b><small>${fmtDate(i.ts)} · ${i.uses}${i.maxUses ? "/" + i.maxUses : ""} usos · ${i.expires ? "expira daqui a " + fmtIn(i.expires) : "não expira"}</small></div>
        ${i.by === "me" || can(P.GERIR_GRUPO) ? `<button class="btn ghost sm danger-t" data-inv-rm="${esc(i.hash)}">Revogar</button>` : ""}</div>`).join("") || `<div class="row"><div class="txt"><small>Ainda não há convites ativos. Os convites antigos (de antes desta versão) continuam a funcionar.</small></div></div>`}</div>`;
  }
  function tBanidos() {
    const l = cache.bans || [];
    return `<p class="lede">Quem foi banido não volta a entrar, nem com convite.</p>
      <div class="box">${l.map((b) => `<div class="row"><div class="txt"><b>${esc(b.name || "Alguém")}</b><small>Banido por ${esc(nameOf(b.by))} · ${fmtDate(b.ts)}${b.reason ? " · «" + esc(b.reason) + "»" : ""}</small></div><button class="btn ghost sm" data-unban="${esc(b.id)}">Levantar o ban</button></div>`).join("") || `<div class="row"><div class="txt"><small>Ninguém está banido.</small></div></div>`}</div>`;
  }
  const ACTIONS = {
    channels: "mudou os canais", roles: "mudou os cargos", member_roles: "mudou os cargos de", channel_perms: "mudou as permissões de um canal", kick: "expulsou", ban: "baniu",
    unban: "levantou o ban de", timeout: "pôs de castigo", nick: "mudou o apelido de", pin: "fixou ou desafixou uma mensagem", settings: "mudou as definições do grupo",
    voice: "moderou na chamada", invite: "criou um convite", invite_revoked: "revogou um convite", report: "denunciou uma mensagem de", join: "entrou no grupo", leave: "saiu do grupo",
  };
  function auditRow(e) {
    let det = "";
    const d = e.detail || {};
    if (d.reason) det += " · «" + esc(d.reason) + "»";
    if (e.action === "timeout") det += d.until ? " até " + fmtDate(d.until) : " (tirou o castigo)";
    if (e.action === "nick") det += d.nick ? " para «" + esc(d.nick) + "»" : " (tirou)";
    if (e.action === "member_roles") det += ": " + (d.roles || []).map((r) => esc((roleOf(A().S.groupId, r) || { name: r }).name)).join(", ");
    if (e.action === "voice") det += " (" + esc(d.action) + ")";
    if (e.action === "report" && d.text) det += `<div class="quote">${esc(d.text)}</div>`;
    return `<div class="row audit"><div class="txt"><b>${esc(nameOf(e.by))}</b> ${ACTIONS[e.action] || e.action}${e.target ? " <b>" + esc(nameOf(e.target)) + "</b>" : ""}${det}<small>${fmtDate(e.ts)}</small></div></div>`;
  }
  function tRegisto() {
    const l = (cache.audit || []).filter((e) => e.action !== "report" && (!auditFilter.who || e.by === auditFilter.who) && (!auditFilter.what || e.action === auditFilter.what));
    const pessoas = [...new Set((cache.audit || []).map((e) => e.by))];
    return `<div class="audit-filter"><select class="input" data-audit="who"><option value="">Toda a gente</option>${pessoas.map((p) => `<option value="${esc(p)}" ${auditFilter.who === p ? "selected" : ""}>${esc(nameOf(p))}</option>`).join("")}</select>
      <select class="input" data-audit="what"><option value="">Todas as ações</option>${Object.entries(ACTIONS).filter(([k]) => k !== "report").map(([k, l]) => `<option value="${k}" ${auditFilter.what === k ? "selected" : ""}>${l}</option>`).join("")}</select></div>
      <div class="box">${l.map(auditRow).join("") || `<div class="row"><div class="txt"><small>Nada a mostrar.</small></div></div>`}</div>`;
  }
  function tDenuncias() {
    const l = (cache.audit || []).filter((e) => e.action === "report");
    return `<p class="lede">Mensagens que os membros denunciaram.</p><div class="box">${l.map((e) => auditRow(e) + (e.target ? `<div class="row"><span style="flex:1"></span>${can(P.CASTIGAR) && above(e.target) ? `<button class="btn ghost sm" data-rep-to="${esc(e.target)}">Castigar</button>` : ""}${can(P.GERIR_MENSAGENS) ? `<button class="btn ghost sm danger-t" data-rep-del="${esc((e.detail || {}).target || "")}">Apagar mensagem</button>` : ""}</div>` : "")).join("") || `<div class="row"><div class="txt"><small>Sem denúncias. 🎉</small></div></div>`}</div>`;
  }

  // ---- eventos da página ----
  function setPath(obj, path, val) { const ks = path.split("."); let o = obj; for (let i = 0; i < ks.length - 1; i++) o = o[isNaN(ks[i]) ? ks[i] : +ks[i]]; o[ks[ks.length - 1]] = val; }
  function onInput(e) {
    const t = e.target;
    if (t.dataset.gs && draft) {
      let v = t.type === "checkbox" ? t.checked : t.value;
      if (t.dataset.num !== undefined) v = +v;
      if (t.tagName === "SELECT" && v === "" && /channel/.test(t.dataset.gs)) v = null;
      setPath(draft, t.dataset.gs, v);
      if (e.type === "change" || t.type === "checkbox") drawSettings(); else redrawBar();
    }
    if (t.dataset.am && draft) {
      const [i, k] = t.dataset.am.split("|"); const r = draft.automod[+i]; const kind = ruleKind(r);
      if (k === "words") r.tipo = { Palavras: t.value.split(",").map((x) => x.trim()).filter(Boolean) };
      if (k === "allow") r.tipo = { Links: { permitidos: t.value.split(",").map((x) => x.trim()).filter(Boolean) } };
      if (k === "n") r.tipo = { [kind]: Math.max(1, +t.value || 1) };
      redrawBar();
    }
    if (t.dataset.rd !== undefined && roleDraft) {
      const r = roleDraft.find((x) => x.id === roleSel); if (!r) return;
      const k = t.dataset.rd; r[k] = t.type === "checkbox" ? t.checked : t.value;
      if (k === "icon" && !t.value) r.icon = null;
      if (e.type === "change" || t.type === "checkbox" || k === "color") drawSettings(); else redrawBar();
    }
    if (t.dataset.permBit && roleDraft) {
      const r = roleDraft.find((x) => x.id === roleSel); r.perms = put(r.perms, +t.dataset.permBit, t.checked); drawSettings();
    }
    if (t.dataset.permQ !== undefined) { permQuery = t.value; drawSettings(); }
    if (t.dataset.memQ !== undefined) { memberQuery = t.value; drawSettings(); }
    if (t.dataset.audit && e.type === "change") { auditFilter[t.dataset.audit] = t.value; drawSettings(); }
    if (t.dataset.roleAddm !== undefined && t.value && e.type === "change") {
      const r = roleSel; const m = member(t.value);
      if (m) act(() => api.set_member_roles({ groupId: A().S.groupId, userId: m.id, roles: [...(m.roles || []), r] }), (m.nick || m.name) + " tem agora o cargo.");
    }
  }
  function redrawBar() {
    const el = $("#gset"); if (!el) return;
    const bar = el.querySelector(".save-bar");
    if (dirty() && !bar) { const b = document.createElement("div"); b.className = "save-bar"; b.innerHTML = `<span>Cuidado: tens alterações por guardar.</span><button class="btn ghost" data-gs-act="reset">Repor</button><button class="btn primary" data-gs-act="save">Guardar alterações</button>`; el.appendChild(b); }
    if (!dirty() && bar) bar.remove();
  }
  let dragRole = null;
  document.addEventListener("dragstart", (e) => { const r = e.target.closest && e.target.closest("[data-role-drag]"); if (r) { dragRole = r.dataset.roleDrag; e.dataTransfer.effectAllowed = "move"; } });
  document.addEventListener("dragover", (e) => { if (dragRole && e.target.closest && e.target.closest(".role-item")) e.preventDefault(); });
  document.addEventListener("drop", (e) => {
    if (!dragRole || !roleDraft) return;
    const over = e.target.closest && e.target.closest(".role-item"); if (!over) return;
    e.preventDefault();
    const from = roleDraft.findIndex((r) => r.id === dragRole); let to = roleDraft.findIndex((r) => r.id === over.dataset.role);
    dragRole = null;
    if (from < 0 || to < 0 || from === to) return;
    if (roleDraft[to].id === "todos") to = roleDraft.length - 2;
    const g = A().group(); if (g.owner !== "me" && to <= myRank()) return A().toast("Só podes pôr cargos abaixo do teu.");
    const [x] = roleDraft.splice(from, 1); roleDraft.splice(to, 0, x); drawSettings();
  });

  async function save() {
    const g = A().group();
    try {
      if (draft && JSON.stringify(draft) !== saved) {
        const r = await api.save_group_settings({ groupId: g.id, settings: draft }); saved = JSON.stringify(draft);
        A().toast(r === false ? "Guardado. Vale quando o anfitrião voltar." : "Definições guardadas.");
      }
      if (roleDraft && JSON.stringify(roleDraft) !== JSON.stringify(G.roles[g.id])) {
        const r = await api.save_roles({ groupId: g.id, roles: roleDraft });
        await loadRoles(g.id); roleDraft = JSON.parse(JSON.stringify(G.roles[g.id] || []));
        A().toast(r === false ? "Cargos guardados. Valem quando o anfitrião voltar." : "Cargos guardados.");
      }
      await refreshGroup(g.id);
    } catch (x) { A().toast(String(x.message || x)); }
  }
  async function onClick(e) {
    const { S } = A();
    const g = A().group(); if (!g) return;
    const t = e.target;
    const tb = t.closest("[data-gs-tab]"); if (tb) { tab = tb.dataset.gsTab; await loadTab(); return drawSettings(); }
    const a = t.closest("[data-gs-act]");
    if (a) {
      const k = a.dataset.gsAct;
      if (k === "close") return closeSettings();
      if (k === "save") return save();
      if (k === "reset") { if (saved) draft = JSON.parse(saved); roleDraft = JSON.parse(JSON.stringify(G.roles[g.id] || [])); return drawSettings(); }
      if (k === "invite") { await openInvite(); return; }
      if (k === "pause") { if (!draft) draft = await api.get_group_settings({ groupId: g.id }); draft.invites_paused = !draft.invites_paused; await save(); return; }
      if (k === "leave" || k === "delete") { open = false; $("#gset") && $("#gset").remove(); return A().confirmLeave(g.id); }
      if (k === "icon") {
        const inp = document.createElement("input"); inp.type = "file"; inp.accept = "image/*";
        inp.onchange = async () => { const f = inp.files[0]; if (!f) return; try { draft.icon = await A().squarePhoto(f); drawSettings(); } catch (x) { A().toast(String(x.message || x)); } };
        return inp.click();
      }
      if (k === "icon-rm") { draft.icon = null; return drawSettings(); }
    }
    const col = t.closest("[data-gs-color]"); if (col && draft) { draft.color = col.dataset.gsColor; return drawSettings(); }
    const nt = t.closest("[data-gs-notify]"); if (nt && draft) { draft.default_notify = +nt.dataset.gsNotify; return drawSettings(); }
    // AutoMod
    const add = t.closest("[data-am-add]"); if (add && draft) { draft.automod = draft.automod || []; draft.automod.push(newRule(add.dataset.amAdd)); return drawSettings(); }
    const rm = t.closest("[data-am-rm]"); if (rm && draft) { draft.automod.splice(+rm.dataset.amRm, 1); return drawSettings(); }
    const ex = t.closest("[data-am-ex]"); if (ex && draft) { const [i, rid] = ex.dataset.amEx.split("|"); const r = draft.automod[+i]; r.exempt_roles = r.exempt_roles.includes(rid) ? r.exempt_roles.filter((x) => x !== rid) : [...r.exempt_roles, rid]; return drawSettings(); }
    // cargos
    const ro = t.closest("[data-role]"); if (ro) { roleSel = ro.dataset.role; return drawSettings(); }
    const rt = t.closest("[data-role-tab]"); if (rt) { roleTab = rt.dataset.roleTab; return drawSettings(); }
    if (t.closest("[data-role-new]") || t.closest("[data-role-tpl]")) {
      const tpl = t.closest("[data-role-tpl]") ? t.closest("[data-role-tpl]").dataset.roleTpl : null;
      const T = tpl ? TEMPLATES[tpl] : null;
      const r = { id: Math.random().toString(16).slice(2, 14), name: tpl || "cargo novo", color: T ? T.color : "", icon: null, hoist: T ? T.hoist : false, mentionable: false, perms: T ? T.perms() : BASE, members: 0 };
      // fica logo abaixo do meu cargo (ou no topo, se for o dono)
      const at = g.owner === "me" ? 0 : Math.min(roleDraft.length - 1, myRank() + 1);
      roleDraft.splice(at, 0, r); roleSel = r.id; roleTab = "visual"; return drawSettings();
    }
    const rc = t.closest("[data-rd-color]"); if (rc) { const r = roleDraft.find((x) => x.id === roleSel); r.color = rc.dataset.rdColor; return drawSettings(); }
    if (t.closest("[data-role-del]")) { roleDraft = roleDraft.filter((x) => x.id !== roleSel); roleSel = (roleDraft[0] || {}).id; return drawSettings(); }
    if (t.closest("[data-perm-clear]")) { const r = roleDraft.find((x) => x.id === roleSel); r.perms = 0; return drawSettings(); }
    const rmm = t.closest("[data-role-rmm]"); if (rmm) { const m = member(rmm.dataset.roleRmm); if (m) act(() => api.set_member_roles({ groupId: g.id, userId: m.id, roles: (m.roles || []).filter((x) => x !== roleSel) }), "Cargo tirado."); return; }
    // membros
    const mm = t.closest("[data-mem-menu]"); if (mm) { const r = mm.getBoundingClientRect(); window.tascaAmigos && window.tascaAmigos.userMenu(r.left, r.bottom, mm.dataset.memMenu, mm); return; }
    const ir = t.closest("[data-inv-rm]"); if (ir) { await act(() => api.revoke_invite({ groupId: g.id, hash: ir.dataset.invRm }), "Convite revogado."); cache.invites = await api.list_invites({ groupId: g.id }); return drawSettings(); }
    const ub = t.closest("[data-unban]"); if (ub) { await act(() => api.unban_member({ groupId: g.id, userId: ub.dataset.unban }), "Ban levantado."); cache.bans = await api.list_bans({ groupId: g.id }); return drawSettings(); }
    const rto = t.closest("[data-rep-to]"); if (rto) return timeout(rto.dataset.repTo);
    const rdel = t.closest("[data-rep-del]"); if (rdel) {
      const id = rdel.dataset.repDel; const ch = S.channels.find((c) => c.kind === "text");
      // o canal certo descobre-se pela mensagem: o núcleo aceita qualquer canal do grupo
      return act(() => api.delete_message({ channelId: ch ? ch.id : g.id + "/geral", id }), "Mensagem apagada.");
    }
  }
  document.addEventListener("keydown", (e) => { if (e.key === "Escape" && open && !document.querySelector(".scrim, .ctx-menu")) { e.stopPropagation(); closeSettings(); } }, true);

  // ---------------- eventos da app ----------------
  if (window.api && api.on) {
    api.on("group_error", (e) => A().toast(e.message || "O anfitrião recusou."));
    api.on("timed_out", (e) => {
      const g = A().S.groups.find((x) => x.id === e.groupId);
      if (e.until && e.until > Date.now()) A().toast(`Estás de castigo em ${g ? g.name : "um grupo"} durante ${fmtIn(e.until)}${e.reason ? ": " + e.reason : ""}.`);
      else A().toast(`Já não estás de castigo${g ? " em " + g.name : ""}.`);
    });
    api.on("pins_changed", () => { if (A().refreshMessages) A().refreshMessages(); });
    api.on("group_removed", async (e) => {
      A().toast(e.banned ? `Foste banido de ${e.name}${e.reason ? ": " + e.reason : ""}.` : `Foste expulso de ${e.name}${e.reason ? ": " + e.reason : ""}.`);
      try { await api.leave_group({ groupId: e.groupId }); } catch {}
      const { S } = A();
      S.groups = S.groups.filter((x) => x.id !== e.groupId);
      if (S.groupId === e.groupId) { if (S.groups.length) A().openGroup(S.groups[0].id); else { S.home = true; S.groupId = null; S.view = "friends"; A().render(); } }
      else A().render();
    });
    api.on("voice_mod", (e) => { if (A().voiceMod) A().voiceMod(e); });
  }

  window.tascaGrupo = { P, has, put, can, above, openSettings, closeSettings, channelPerms, openInvite, openPins, pin, report, nick, refreshGroup, loadRoles, roleOf, roles: G.roles, nameOf, BASE, TODAS };
})();
