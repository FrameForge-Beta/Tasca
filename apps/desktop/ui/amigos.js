// Tasca — amigos, cartões de perfil e menu de botão direito nas pessoas.
//
// - Página "Amigos" (botão da Tasca no topo da barra de grupos): online, todos, pedidos,
//   bloqueados e "Adicionar amigo" pelo @ID.
// - Clique numa pessoa (lista de membros, nome ou foto numa mensagem, lista de amigos):
//   abre o cartão de perfil ao lado. "Ver perfil completo" abre o perfil grande.
// - Botão direito numa pessoa: perfil, mencionar, mensagem, volume e silêncio só para ti,
//   amizade, bloquear, ações de anfitrião e copiar o ID.
//
// Plugins: `window.tascaHooks` tem listas onde um plugin pode pôr funções:
//   userMenu:     (pessoa, ctx) => [itens do menu]     (itens no formato do tascaMenu)
//   profileCard:  (pessoa, ctx) => "<html>"            (secção extra no cartão e no perfil)
//   friendsTabs:  () => [{ id, label, render() }]      (separadores extra na página Amigos)
// `ctx` = { where: "card" | "full" | "menu", groupId }.
(function () {
  "use strict";
  const A = () => window.tascaApp;
  const hooks = (window.tascaHooks = window.tascaHooks || {});
  for (const k of ["userMenu", "profileCard", "friendsTabs"]) hooks[k] = hooks[k] || [];

  const IC = {
    users: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="9" cy="8" r="4"/><path d="M2 21a7 7 0 0 1 14 0M16 3.1a4 4 0 0 1 0 7.8M22 21a7 7 0 0 0-4-6.3"/></svg>',
    add: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="9" cy="8" r="4"/><path d="M2 21a7 7 0 0 1 14 0M19 8v6M16 11h6"/></svg>',
    msg: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20.5l1.4-4.9A8 8 0 1 1 21 12z"/></svg>',
    ok: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5 9-10"/></svg>',
    no: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M18 6 6 18M6 6l12 12"/></svg>',
    more: '<svg viewBox="0 0 24 24" fill="currentColor"><circle cx="5" cy="12" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="19" cy="12" r="2"/></svg>',
    block: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M5.6 5.6l12.8 12.8"/></svg>',
    edit: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z"/></svg>',
    at: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="4"/><path d="M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-4 8"/></svg>',
    id: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="5" width="18" height="14" rx="3"/><path d="M8 9v6M12 9h2a3 3 0 0 1 0 6h-2z"/></svg>',
    note: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6M8 13h8M8 17h5"/></svg>',
    tag: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20.6 13.4 13.4 20.6a2 2 0 0 1-2.8 0L3 13V3h10l7.6 7.6a2 2 0 0 1 0 2.8z"/><circle cx="7.5" cy="7.5" r="1.5"/></svg>',
    user: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/></svg>',
    mute: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 5 6 9H3v6h3l5 4V5zM22 9l-6 6M16 9l6 6"/></svg>',
    crown: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M3 7l4.5 4L12 4l4.5 7L21 7l-2 12H5z"/></svg>',
  };

  // ---------------- estado ----------------
  const F = { list: [], tab: "online", query: "", addOpen: false, addMsg: null, addOk: false, server: true, card: null };
  const byId = (id) => F.list.find((f) => f.id === id);
  const relOf = (id) => (byId(id) || {}).rel || null;
  const pendingCount = () => F.list.filter((f) => f.rel === "recebido").length;
  const esc = (s) => A().esc(s == null ? "" : s);
  const DIA = 86400000;
  const fmtDate = (ms) => (ms ? new Date(ms).toLocaleDateString("pt-PT", { day: "numeric", month: "short", year: "numeric" }) : "");
  const statusLabel = (st) => ({ online: "Online", away: "Ausente", dnd: "Não incomodar", offline: "Offline" }[st] || "Offline");

  // Uma pessoa com tudo o que se sabe dela: grupos, amizade e o perfil que ela mandou.
  function person(id) {
    const { S } = A();
    if (id === "me" || (S.me && id === S.me.id)) return { ...S.me, id: "me", status: A().myStatus(), statusText: S.customStatus || "" };
    const p = S.people[id] || {};
    const f = byId(id) || {};
    const pick = (k) => (f[k] != null && f[k] !== "" ? f[k] : p[k]);
    const st = f.rel === "amigo" && f.status && f.status !== "offline" ? f.status : p.status || "offline";
    return {
      id, name: pick("name") || "Alguém", color: pick("color") || "#4A4542", photo: pick("photo") || null,
      realName: p.realName || f.realName || null, username: pick("username") || null, bio: pick("bio") || "", banner: pick("banner") || null,
      status: st, statusText: f.statusText || p.statusText || "", bot: p.bot, rel: f.rel || null, friendSince: f.rel === "amigo" ? f.since : null,
    };
  }

  // Amigos que não estão em nenhum grupo também têm de existir em S.people (conversas, cartões).
  function mergePeople() {
    const { S } = A();
    for (const f of F.list) {
      const old = S.people[f.id];
      if (!old) S.people[f.id] = { id: f.id, name: f.name, color: f.color, photo: f.photo, username: f.username, bio: f.bio, banner: f.banner, status: f.status || "offline", statusText: f.statusText || "" };
      else {
        for (const k of ["photo", "username", "bio", "banner"]) if (f[k] && !old[k]) old[k] = f[k];
        if (f.rel === "amigo" && f.status && f.status !== "offline") old.status = f.status;
        old.statusText = f.statusText || old.statusText || "";
      }
    }
  }

  async function refresh() {
    try { F.list = await api.list_friends(); } catch (x) { console.warn("amigos:", x); return; }
    mergePeople();
    A().rerenderKeepInput();
    if (F.card) redrawCard();
  }

  let started = false;
  async function start() {
    if (started) return; started = true;
    try { const st = await api.account_status(); F.server = !!st.server; } catch {}
    await refresh();
    api.on("friends_changed", refresh);
    api.on("friend_request", ({ name }) => {
      A().toast(`${name || "Alguém"} quer ser teu amigo.`);
      if (window.sfx && sfx.join && A().S.myStatus !== "dnd") try { sfx.join(); } catch {}
    });
    api.on("friend_presence", ({ id, status, text }) => {
      const f = byId(id); if (f) { f.status = status; f.statusText = text || ""; }
      const p = A().S.people[id]; if (p) { if (status !== "offline" || f) p.status = status; p.statusText = text || ""; }
      // só redesenha o que mostra pessoas (evita piscar a conversa)
      const { S } = A();
      if (S.home || (S.view === "chat" && S.showMembers)) A().rerenderKeepInput();
      if (F.card && F.card.id === id) redrawCard();
    });
    // conversas privadas: mensagem nova noutra conversa, ou a lista mudou
    const dmsUpdate = async () => {
      const { S } = A();
      try { S.dms = await api.list_dms(); } catch { return; }
      if (S.home || S.view === "chat") A().rerenderKeepInput();
    };
    api.on("dms_changed", dmsUpdate);
    api.on("message", ({ channelId }) => { if (String(channelId).startsWith("dm-") && channelId !== A().S.channelId) dmsUpdate(); });
    // presença inicial (com a mensagem de estado guardada)
    api.set_presence({ status: A().myStatus(), text: A().S.customStatus || "" }).catch(() => {});
    askUsername();
  }

  // Quem ainda não tem @ID: pede uma vez por arranque (sem ID ninguém o encontra).
  function askUsername() {
    const { S, modal } = A();
    if (S.me.username || !F.server || sessionStorage.getItem("tasca:ask-id")) return;
    try { sessionStorage.setItem("tasca:ask-id", "1"); } catch {}
    modal(`<h2>Escolhe o teu ID</h2>
      <p>O teu ID é um nome único em minúsculas, como <b>@tiago.f</b>. É com ele que te adicionam como amigo. O nome que aparece nas conversas pode ser igual ao de outras pessoas; o ID não.</p>
      <div class="actions"><button class="btn ghost" data-close>Mais tarde</button><button class="btn primary" id="id-now">Escolher agora</button></div>`,
    (el, close) => { el.querySelector("#id-now").onclick = () => { close(); A().openProfile(); }; });
  }

  // ---------------- página Amigos ----------------
  function sidebarButton() {
    const { S } = A();
    const on = S.view === "friends";
    const n = pendingCount();
    return `<button class="dm friends-btn ${on ? "on" : ""}" data-fr-open>${IC.users}<span class="dm-txt"><b>Amigos</b></span>${n ? `<span class="dm-side"><span class="pill">${n}</span></span>` : ""}</button>`;
  }

  const TABS = [["online", "Online"], ["todos", "Todos"], ["pendentes", "Pedidos"], ["bloqueados", "Bloqueados"]];
  function renderFriends() {
    const { S, I, LOGO } = A();
    const pend = pendingCount();
    const extra = hooks.friendsTabs.flatMap((fn) => { try { return fn() || []; } catch (x) { console.warn(x); return []; } });
    const tabs = [...TABS, ...extra.map((t) => [t.id, t.label])].map(([k, l]) =>
      `<button class="fr-tab ${F.tab === k && !F.addOpen ? "on" : ""}" data-fr-tab="${k}">${esc(l)}${k === "pendentes" && pend ? `<span class="pill">${pend}</span>` : ""}</button>`).join("");
    const head = `<div class="main-head fr-head"><button class="icon-btn menu-btn" data-act="nav" aria-label="Grupos e canais">${I.menu}</button>
      <span class="title">${IC.users}Amigos</span><span class="fr-sep"></span>
      <nav class="fr-tabs" aria-label="Amigos">${tabs}<button class="fr-tab add ${F.addOpen ? "on" : ""}" data-fr-add>Adicionar amigo</button></nav></div>`;
    let body;
    const tabX = extra.find((t) => t.id === F.tab);
    if (F.addOpen) body = renderAdd();
    else if (tabX) body = `<div class="fr-body">${tabX.render()}</div>`;
    else body = renderList();
    const noId = !S.me.username && F.server ? `<div class="fr-noid">${IC.at}<div><b>Ainda não tens ID</b><small>Sem ID ninguém te consegue adicionar. Escolhe um nome único, como @tiago.f.</small></div><button class="btn primary" data-act="edit-profile">Escolher ID</button></div>` : "";
    return `${head}<div class="fr-page">${noId}${body}</div>`;
  }

  function renderAdd() {
    const { S, LOGO } = A();
    return `<div class="fr-add">
      <h2>Adicionar amigo</h2>
      <p>Escreve o ID da pessoa, o nome único que começa por @. ${S.me.username ? `O teu é <b>@${esc(S.me.username)}</b> <button class="linkish" data-act="copy-key">copiar</button>.` : ""}</p>
      <form id="fr-add-form" class="fr-add-in ${F.addMsg ? (F.addOk ? "ok" : "bad") : ""}" autocomplete="off">
        <span aria-hidden="true">@</span><label class="sr" for="fr-user">ID da pessoa</label>
        <input id="fr-user" maxlength="25" spellcheck="false" autocapitalize="off" placeholder="nome.da.pessoa" value="${esc(F.addValue || "")}">
        <button class="btn primary" type="submit" id="fr-send" ${F.addValue ? "" : "disabled"}>Enviar pedido</button>
      </form>
      ${F.addMsg ? `<p class="fr-add-msg ${F.addOk ? "ok" : "bad"}">${esc(F.addMsg)}</p>` : ""}
      <div class="fr-add-how"><span class="fr-flor">${LOGO}</span><div><b>Como funciona</b><small>O pedido vai cifrado pelo servidor da Tasca: só a outra pessoa o consegue ler. Quando ela aceitar, passam a ver se estão online e o perfil um do outro, diretamente entre os vossos PCs.</small></div></div>
    </div>`;
  }

  function row(f) {
    const { avatar } = A();
    const p = person(f.id);
    const sub = f.rel === "recebido" ? "Pedido de amizade recebido" : f.rel === "enviado" ? "Pedido enviado" : f.rel === "bloqueado" ? "Bloqueado"
      : p.statusText ? p.statusText : statusLabel(p.status);
    const acts = f.rel === "recebido"
      ? `<button class="fr-act ok" data-fr-accept="${f.id}" title="Aceitar" aria-label="Aceitar o pedido de ${esc(p.name)}">${IC.ok}</button><button class="fr-act no" data-fr-decline="${f.id}" title="Recusar" aria-label="Recusar">${IC.no}</button>`
      : f.rel === "enviado" ? `<button class="fr-act no" data-fr-cancel="${f.id}" title="Cancelar pedido" aria-label="Cancelar pedido">${IC.no}</button>`
      : f.rel === "bloqueado" ? `<button class="btn ghost" data-fr-unblock="${f.id}">Desbloquear</button>`
      : `<button class="fr-act" data-fr-msg="${f.id}" title="Enviar mensagem" aria-label="Mensagem para ${esc(p.name)}">${IC.msg}</button><button class="fr-act" data-fr-more="${f.id}" title="Mais" aria-label="Mais opções">${IC.more}</button>`;
    return `<div class="fr-row" data-friend="${f.id}" tabindex="0">${avatar({ ...p, status: f.rel === "amigo" ? p.status : "offline" }, "", f.rel === "amigo")}
      <div class="fr-txt"><b>${esc(p.name)}${p.username ? ` <span>@${esc(p.username)}</span>` : ""}</b><small>${esc(sub)}</small></div><span class="fr-acts">${acts}</span></div>`;
  }

  function renderList() {
    const { LOGO } = A();
    const q = F.query.trim().toLowerCase().replace(/^@/, "");
    let list = F.list.filter((f) =>
      F.tab === "online" ? f.rel === "amigo" && person(f.id).status !== "offline"
        : F.tab === "todos" ? f.rel === "amigo"
        : F.tab === "pendentes" ? f.rel === "recebido" || f.rel === "enviado"
        : f.rel === "bloqueado");
    if (q) list = list.filter((f) => { const p = person(f.id); return p.name.toLowerCase().includes(q) || (p.username || "").includes(q); });
    list.sort((a, b) => (a.rel === "recebido") - (b.rel === "recebido") || person(a.id).name.localeCompare(person(b.id).name, "pt"));
    if (F.tab === "pendentes") list.sort((a, b) => (b.rel === "recebido") - (a.rel === "recebido"));
    const titles = { online: "Online", todos: "Todos os amigos", pendentes: "Pedidos", bloqueados: "Bloqueados" };
    const empty = {
      online: ["Ninguém online agora", "Quando um amigo abrir a Tasca, aparece aqui."],
      todos: ["Ainda não tens amigos na Tasca", "Adiciona alguém pelo ID, ou carrega com o botão direito numa pessoa de um grupo."],
      pendentes: ["Sem pedidos", "Os pedidos que recebes e envias ficam aqui até alguém responder."],
      bloqueados: ["Ninguém bloqueado", "Quem bloqueares não te consegue pedir amizade nem ver se estás online."],
    }[F.tab];
    const search = `<div class="fr-search search"><label class="sr" for="fr-q">Procurar amigos</label>${A().I.search}<input id="fr-q" placeholder="Procurar" value="${esc(F.query)}"></div>`;
    if (!list.length) {
      return `<div class="fr-body">${F.list.length ? search : ""}<div class="fr-empty"><span class="fr-flor">${LOGO}</span><b>${empty[0]}</b><small>${empty[1]}</small>
        ${F.tab !== "bloqueados" ? `<button class="btn primary" data-fr-add>Adicionar amigo</button>` : ""}</div></div>`;
    }
    return `<div class="fr-body">${search}<div class="fr-count">${titles[F.tab]} · ${list.length}</div><div class="fr-list">${list.map(row).join("")}</div></div>`;
  }

  // ---------------- ações ----------------
  async function act(fn, ok) {
    try { await fn(); if (ok) A().toast(ok); await refresh(); }
    catch (x) { A().toast(String((x && x.message) || x)); }
  }
  const addFriend = (id) => act(() => api.send_friend_request({ userId: id, name: person(id).name }), relOf(id) === "recebido" ? "Agora são amigos." : "Pedido de amizade enviado.");
  const removeFriend = (id) => confirmBox(`Remover ${person(id).name}?`, "Deixam de ser amigos e de ver se o outro está online. Podem voltar a pedir amizade quando quiserem.", "Remover amigo", () => act(() => api.remove_friend({ userId: id }), "Amigo removido."));
  const blockUser = (id) => confirmBox(`Bloquear ${person(id).name}?`, "Deixam de ser amigos, os pedidos dela são ignorados e ela deixa de ver se estás online. Nos grupos continuas a ver as mensagens dela.", "Bloquear", () => act(() => api.block_user({ userId: id, name: person(id).name }), "Bloqueado."));
  function confirmBox(title, text, btn, then) {
    A().modal(`<h2>${esc(title)}</h2><p>${esc(text)}</p><div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn danger" id="cf-ok">${esc(btn)}</button></div>`,
      (el, close) => { el.querySelector("#cf-ok").onclick = () => { close(); then(); }; });
  }

  async function message(id) {
    const { S, openDm } = A();
    closeCard();
    const dmId = "dm-" + id;
    if (!S.dms.some((d) => d.id === dmId)) {
      try { S.dms = await api.list_dms(); } catch {}
      if (!S.dms.some((d) => d.id === dmId)) S.dms.unshift({ id: dmId, with: id, unread: 0, last: null });
    }
    if (!S.people[id]) S.people[id] = person(id);
    openDm(dmId);
  }

  function mention(id) {
    const p = person(id);
    const inp = document.getElementById("msg-input");
    if (!inp) return A().toast("Abre um canal de texto para mencionar.");
    const tag = "@" + (p.username || p.name.split(/\s+/)[0]) + " ";
    const a = inp.selectionStart ?? inp.value.length;
    const pre = inp.value.slice(0, a);
    inp.value = pre + (pre && !/\s$/.test(pre) ? " " : "") + tag + inp.value.slice(inp.selectionEnd ?? a);
    inp.focus(); inp.dispatchEvent(new Event("input", { bubbles: true }));
  }

  // volume e silêncio só para ti (a voz lê S.settings.userVolume / userMuted)
  let volTimer = 0;
  function setVolume(id, v) {
    const { S } = A();
    S.settings.userVolume = { ...(S.settings.userVolume || {}), [id]: v };
    clearTimeout(volTimer);
    volTimer = setTimeout(() => A().patch({ userVolume: { [id]: v } }).catch(() => {}), 120);
  }
  async function toggleMuted(id) {
    const { S } = A();
    const on = !(S.settings.userMuted || {})[id];
    await A().patch({ userMuted: { [id]: on } });
    A().toast(on ? `${person(id).name} silenciado só para ti.` : `${person(id).name} volta a ouvir-se.`);
  }

  // ---- coisas só tuas sobre uma pessoa: nota, apelido, não ver o vídeo ----
  async function toggleLocal(k, id, msg) {
    const { S } = A();
    const on = !(S.settings[k] || {})[id];
    await A().patch({ [k]: { [id]: on } });
    A().render(); if (msg) A().toast(msg(on));
  }
  function editNote(id) {
    const { S, modal } = A();
    const p = person(id);
    modal(`<h2>Nota sobre ${esc(p.name)}</h2><p>Só tu a vês. Fica guardada neste PC.</p>
      <div class="field"><label class="sr" for="note-t">Nota</label><textarea class="input" id="note-t" rows="4" maxlength="256" placeholder="Ex.: conheci no torneio de xadrez">${esc((S.settings.userNotes || {})[id] || "")}</textarea></div>
      <div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn primary" id="note-ok">Guardar</button></div>`,
    (el, close) => { el.querySelector("#note-ok").onclick = async () => {
      await A().patch({ userNotes: { [id]: el.querySelector("#note-t").value.trim() } }); close(); if (F.card) redrawCard(); A().toast("Nota guardada.");
    }; });
  }
  function editNick(id) {
    const { S, modal } = A();
    const p = person(id);
    modal(`<h2>Apelido para ${esc(p.realName || p.name)}</h2><p>Só tu vês este nome. Os outros continuam a ver o nome que a pessoa escolheu.</p>
      <div class="field"><label class="sr" for="nick-t">Apelido</label><input class="input" id="nick-t" maxlength="32" value="${esc((S.settings.userNicks || {})[id] || "")}" placeholder="${esc(p.realName || p.name)}"></div>
      <div class="actions"><button class="btn ghost" data-close>Cancelar</button>${(S.settings.userNicks || {})[id] ? '<button class="btn ghost" id="nick-rm">Tirar apelido</button>' : ""}<button class="btn primary" id="nick-ok">Guardar</button></div>`,
    (el, close) => {
      const save = async (v) => {
        await A().patch({ userNicks: { [id]: v } });
        S.people = await api.list_people(); await refresh();
        for (const gid of Object.keys(S.allMembers || {})) S.allMembers[gid] = await api.list_members({ groupId: gid });
        if (S.groupId && S.allMembers[S.groupId]) S.members = S.allMembers[S.groupId];
        close(); A().rerenderKeepInput();
      };
      el.querySelector("#nick-ok").onclick = () => save(el.querySelector("#nick-t").value.trim());
      const rm = el.querySelector("#nick-rm"); if (rm) rm.onclick = () => save("");
      el.querySelector("#nick-t").addEventListener("keydown", (e) => { if (e.key === "Enter") el.querySelector("#nick-ok").click(); });
    });
  }

  // ---------------- menu de botão direito ----------------
  function userMenu(x, y, id, from) {
    const { S, group, copy, openProfile } = A();
    const me = id === "me";
    const p = person(id);
    const rel = p.rel;
    const g = !S.home ? group() : null;
    const inGroup = g && (S.members || []).some((m) => m.id === id);
    const inCall = !!(from && from.closest && from.closest(".callview, .voice-people, .dock"));
    const local = (k) => (S.settings[k] || {})[id];
    const items = [
      { label: "Perfil", icon: IC.user, onClick: () => openCard(id, from) },
    ];
    if (!me && !p.bot) items.push(
      { label: local("userNotes") ? "Editar nota" : "Adicionar nota", hint: "Só tu a vês", icon: IC.note, onClick: () => editNote(id) },
      { label: local("userNicks") ? "Mudar apelido" : "Dar um apelido", hint: local("userNicks") ? "Nome verdadeiro: " + (p.realName || p.name) : "Só para ti", icon: IC.tag, onClick: () => editNick(id) });
    if (me) {
      items.push({ label: "Editar perfil", icon: IC.edit, onClick: () => openProfile() });
    } else {
      if (S.view === "chat" && document.getElementById("msg-input")) items.push({ label: "Mencionar", icon: IC.at, onClick: () => mention(id) });
      if (!p.bot) items.push({ label: "Enviar mensagem", icon: IC.msg, disabled: rel === "bloqueado", onClick: () => message(id) });
      if (!p.bot) {
        const vol = (S.settings.userVolume || {})[id] ?? 100;
        items.push("-",
          { label: "Volume", range: { value: vol, min: 0, max: 200, step: 5, fmt: (v) => v + "%", onInput: (v) => setVolume(id, v) } },
          { label: "Silenciar", hint: "Só para ti, nas chamadas", checked: !!(S.settings.userMuted || {})[id], icon: IC.mute, onClick: () => toggleMuted(id) });
        if (inCall) items.push(
          { label: "Não ver o vídeo", hint: "Mostra só a foto desta pessoa", checked: !!local("userNoVideo"), onClick: () => toggleLocal("userNoVideo", id, (on) => on ? "Já não vês o vídeo de " + p.name + "." : "Voltas a ver o vídeo de " + p.name + ".") });
        items.push("-");
        if (rel === "amigo") items.push({ label: "Remover amigo", onClick: () => removeFriend(id) });
        else if (rel === "recebido") items.push({ label: "Aceitar pedido de amizade", onClick: () => act(() => api.accept_friend({ userId: id }), "Agora são amigos.") }, { label: "Recusar pedido", onClick: () => act(() => api.decline_friend({ userId: id }), "Pedido recusado.") });
        else if (rel === "enviado") items.push({ label: "Cancelar pedido de amizade", onClick: () => act(() => api.cancel_friend({ userId: id }), "Pedido cancelado.") });
        else if (rel !== "bloqueado") items.push({ label: "Adicionar amigo", icon: IC.add, onClick: () => addFriend(id) });
        items.push(rel === "bloqueado"
          ? { label: "Desbloquear", onClick: () => act(() => api.unblock_user({ userId: id }), "Desbloqueado.") }
          : { label: "Bloquear", danger: true, icon: IC.block, onClick: () => blockUser(id) });
      }
    }
    // Quem aloja a chamada pode passá-la a qualquer pessoa da chamada; o dono do grupo
    // também (incluindo a si mesmo).
    if (inCall && S.voice && S.voice.hostId && S.voice.hostId !== id && !p.bot) {
      const naChamada = id === "me" || !!(S.voiceStates || {})[id];
      const possoPassar = S.voice.hostId === "me" || (g && g.owner === "me");
      if (naChamada && possoPassar) items.push("-", {
        label: me ? "Ficar a alojar a chamada" : "Passar a chamada para " + p.name,
        hint: "O áudio e o vídeo passam pelo dispositivo dessa pessoa", icon: IC.crown,
        onClick: () => act(() => api.pass_voice_host({ userId: id }), me ? "A chamada vai passar para ti…" : "A chamada vai passar para " + p.name + "…"),
      });
    }
    if (inCall) items.push("-", { label: "Mostrar quem está sem vídeo", checked: S.settings.callShowNoVideo !== false, onClick: async () => { await A().patch({ callShowNoVideo: S.settings.callShowNoVideo === false }); A().render(); } });
    for (const fn of hooks.userMenu) { try { const extra = fn(p, { where: "menu", groupId: g && g.id }); if (extra && extra.length) items.push("-", ...extra); } catch (x) { console.warn(x); } }
    items.push("-");
    if (p.username) items.push({ label: "Copiar ID (@" + p.username + ")", icon: IC.id, onClick: () => copy("@" + p.username, null, "ID copiado.") });
    else if (!me) items.push({ label: "Copiar chave", icon: IC.id, hint: "Esta pessoa ainda não tem @ID", onClick: () => copy(id, null, "Chave copiada.") });
    tascaMenu.open(x, y, items);
  }

  // ---------------- cartão de perfil ----------------
  function roles(id) {
    const { S, group } = A();
    const g = !S.home ? group() : null;
    const m = g && (S.members || []).find((x) => x.id === id);
    const out = [];
    if (m && m.owner) out.push(["Anfitrião", "var(--ocre)"]);
    if (m && m.bot) out.push(["Plugin", "var(--azulejo)"]);
    if (m && !m.owner && !m.bot) out.push(["Membro", "var(--oliva)"]);
    return { g, m, out };
  }

  function hookSections(p, where) {
    const { S } = A();
    return hooks.profileCard.map((fn) => { try { return fn(p, { where, groupId: S.groupId }) || ""; } catch (x) { console.warn(x); return ""; } }).join("");
  }

  function cardButtons(p, full) {
    if (p.id === "me") return `<button class="btn" data-pc-edit>${IC.edit}Editar perfil</button>`;
    if (p.bot) return "";
    const msg = `<button class="btn primary" data-fr-msg="${p.id}" ${p.rel === "bloqueado" ? "disabled" : ""}>${IC.msg}Mensagem</button>`;
    const friend = p.rel === "amigo" ? "" : p.rel === "recebido" ? `<button class="btn blue" data-fr-accept="${p.id}">${IC.ok}Aceitar amizade</button>`
      : p.rel === "enviado" ? `<button class="btn" data-fr-cancel="${p.id}">Pedido enviado · cancelar</button>`
      : p.rel === "bloqueado" ? `<button class="btn" data-fr-unblock="${p.id}">Desbloquear</button>`
      : `<button class="btn blue" data-pc-add="${p.id}">${IC.add}Adicionar amigo</button>`;
    return `${friend}${msg}<button class="icon-btn" data-fr-more="${p.id}" aria-label="Mais opções" title="Mais">${IC.more}</button>`;
  }

  function cardHtml(id) {
    const { avatar, bannerStyle } = A();
    const p = person(id);
    const { g, m, out } = roles(id);
    const since = m && m.since ? `<div class="pc-sec"><h4>Na ${esc(g.name)} desde</h4><p>${fmtDate(m.since)}</p></div>` : "";
    const friends = p.friendSince ? `<div class="pc-sec"><h4>Amigos desde</h4><p>${fmtDate(p.friendSince)}</p></div>` : "";
    const statusTxt = p.statusText ? `<div class="pc-status"><span>${esc(p.statusText)}</span></div>` : "";
    const note = (A().S.settings.userNotes || {})[id];
    const noteSec = note ? `<div class="pc-sec"><h4>A tua nota</h4><p class="pc-bio">${esc(note)}</p></div>` : "";
    return `<div class="pc-banner" style="${bannerStyle(p.banner, p.color)}"></div>
      <div class="pc-av">${avatar(p, "xl", true)}</div>
      ${statusTxt}
      <div class="pc-body">
        <div class="pc-names"><b>${esc(p.name)}</b>${p.realName && p.realName !== p.name ? `<small class="pc-real">${esc(p.realName)}</small>` : ""}<span>${p.username ? "@" + esc(p.username) : p.id === "me" ? "sem ID" : ""}${p.rel === "amigo" ? '<i class="pc-tag">amigo</i>' : ""}</span></div>
        <div class="pc-state"><i class="st-dot ${p.status}">${A().FLOR}</i>${statusLabel(p.status)}</div>
        ${p.bio ? `<div class="pc-sec"><h4>Sobre mim</h4><p class="pc-bio">${esc(p.bio)}</p></div>` : ""}
        <div class="pc-row2">${since}${friends}</div>
        ${noteSec}
        ${out.length ? `<div class="pc-sec"><h4>Papéis</h4><div class="pc-roles">${out.map(([l, c]) => `<span style="--r:${c}"><i></i>${l}</span>`).join("")}</div></div>` : ""}
        ${hookSections(p, "card")}
        <div class="pc-btns">${cardButtons(p)}</div>
        <button class="linkish pc-full" data-pc-full="${p.id}">Ver perfil completo</button>
      </div>`;
  }

  // fecha com animação; no telemóvel o cartão é uma folha que desce, com fundo escuro
  function closeCard() {
    if (!F.card) return;
    const { el, scrim } = F.card; F.card = null;
    el.classList.add("out"); if (scrim) scrim.classList.add("out");
    setTimeout(() => { el.remove(); if (scrim) scrim.remove(); }, 200);
  }
  function redrawCard() { if (F.card) { F.card.el.innerHTML = (F.card.scrim ? '<span class="sheet-grip" aria-hidden="true"></span>' : "") + cardHtml(F.card.id); } }

  function openCard(id, anchor) {
    if (F.card && F.card.id === id && F.card.anchor === anchor) return closeCard();
    closeCard();
    const el = document.createElement("div");
    el.className = "pcard"; el.setAttribute("role", "dialog"); el.setAttribute("aria-label", "Perfil de " + person(id).name);
    el.innerHTML = cardHtml(id);
    const sheet = innerWidth <= 720;
    let scrim = null;
    if (sheet) {
      // telemóvel: folha que sobe do fundo; fecha a lista de membros por baixo
      const { S, render } = A();
      if (S.membersOpen) { S.membersOpen = false; render(); }
      scrim = document.createElement("div"); scrim.className = "pcard-scrim";
      scrim.addEventListener("click", () => closeCard());
      document.body.appendChild(scrim);
      el.insertAdjacentHTML("afterbegin", '<span class="sheet-grip" aria-hidden="true"></span>');
      let y0 = null, dy = 0;
      el.addEventListener("touchstart", (e) => { y0 = el.scrollTop <= 0 ? e.touches[0].clientY : null; dy = 0; }, { passive: true });
      el.addEventListener("touchmove", (e) => { if (y0 == null) return; dy = Math.max(0, e.touches[0].clientY - y0); if (dy > 4) { el.style.transition = "none"; el.style.transform = `translateY(${dy}px)`; } }, { passive: true });
      el.addEventListener("touchend", () => {
        if (y0 == null) return; y0 = null;
        el.style.transition = "transform .2s cubic-bezier(.2,.8,.2,1)";
        if (dy > 100) closeCard(); else el.style.transform = "";
      });
    }
    document.body.appendChild(el);
    if (sheet) { F.card = { id, el, anchor, scrim }; return; }
    const w = el.offsetWidth, h = el.offsetHeight;
    let left, top;
    const r = anchor && anchor.getBoundingClientRect ? anchor.getBoundingClientRect() : null;
    if (r && r.width) {
      const inMembers = anchor.closest && anchor.closest(".members");
      left = inMembers ? r.left - w - 12 : r.right + 12;
      if (left + w > innerWidth - 8) left = r.left - w - 12;
      top = r.top - 20;
    } else { left = innerWidth / 2 - w / 2; top = innerHeight / 2 - h / 2; }
    el.style.left = Math.max(8, Math.min(left, innerWidth - w - 8)) + "px";
    el.style.top = Math.max(8, Math.min(top, innerHeight - h - 8)) + "px";
    F.card = { id, el, anchor, scrim };
  }

  function openFullProfile(id) {
    closeCard();
    const { S, modal, avatar, bannerStyle } = A();
    const p = person(id);
    const common = S.groups.filter((g) => (S.allMembers[g.id] || []).some((m) => m.id === id) && id !== "me");
    const sinceRows = S.groups.map((g) => { const m = (S.allMembers[g.id] || []).find((x) => x.id === id); return m && m.since ? `<li><b>${esc(g.name)}</b><span>desde ${fmtDate(m.since)}</span></li>` : ""; }).join("");
    let tab = "sobre";
    const body = () => tab === "sobre" ? `
        ${p.bio ? `<div class="pc-sec"><h4>Sobre mim</h4><p class="pc-bio">${esc(p.bio)}</p></div>` : `<p class="pf-none">${id === "me" ? "Ainda não escreveste nada sobre ti." : "Ainda não escreveu nada sobre si."}</p>`}
        ${p.friendSince ? `<div class="pc-sec"><h4>Amigos desde</h4><p>${fmtDate(p.friendSince)}</p></div>` : ""}
        ${sinceRows ? `<div class="pc-sec"><h4>Membro desde</h4><ul class="pf-groups plain">${sinceRows}</ul></div>` : ""}
        ${hookSections(p, "full")}`
      : common.length ? `<ul class="pf-groups">${common.map((g) => `<li><button data-pf-group="${g.id}"><span class="pf-g" style="background:${g.color || "var(--vinho)"}">${esc(g.short)}</span><b>${esc(g.name)}</b><small>${(S.allMembers[g.id] || []).length} membros</small></button></li>`).join("")}</ul>`
      : `<p class="pf-none">Não estão em nenhum grupo juntos.</p>`;
    modal(`<div class="pfull">
        <div class="pc-banner big" style="${bannerStyle(p.banner, p.color)}"></div>
        <div class="pfull-top"><div class="pc-av">${avatar(p, "xl", true)}</div><div class="pc-btns">${cardButtons(p, true)}</div></div>
        <div class="pfull-names"><b>${esc(p.name)}</b><span>${p.username ? "@" + esc(p.username) : ""}${p.rel === "amigo" ? '<i class="pc-tag">amigo</i>' : ""}</span>
          <div class="pc-state"><i class="st-dot ${p.status}">${A().FLOR}</i>${statusLabel(p.status)}${p.statusText ? " · " + esc(p.statusText) : ""}</div></div>
        <nav class="pfull-tabs"><button class="on" data-pf-tab="sobre">Sobre</button>${id === "me" ? "" : `<button data-pf-tab="grupos">Grupos em comum · ${common.length}</button>`}</nav>
        <div class="pfull-body" id="pf-body">${body()}</div>
      </div>`, (el, close) => {
      el.querySelector(".modal").classList.add("pfull-modal");
      el.addEventListener("click", (e) => {
        const t = e.target.closest("[data-pf-tab]");
        if (t) { tab = t.dataset.pfTab; el.querySelectorAll("[data-pf-tab]").forEach((b) => b.classList.toggle("on", b === t)); el.querySelector("#pf-body").innerHTML = body(); return; }
        const gb = e.target.closest("[data-pf-group]"); if (gb) { close(); A().openGroup(gb.dataset.pfGroup); return; }
        if (e.target.closest("[data-fr-msg], [data-pc-edit], [data-pc-add], [data-fr-accept], [data-fr-cancel], [data-fr-unblock]")) setTimeout(close, 0);
      });
    });
  }

  // ---------------- eventos ----------------
  const personOf = (el) => el.dataset.member || el.dataset.person || el.dataset.friend || el.dataset.spk || el.dataset.who;
  document.addEventListener("click", (e) => {
    const t = e.target.closest("[data-fr-open], [data-fr-tab], [data-fr-add], [data-fr-accept], [data-fr-decline], [data-fr-cancel], [data-fr-unblock], [data-fr-msg], [data-fr-more], [data-pc-full], [data-pc-edit], [data-pc-add], [data-member], [data-person], .fr-row[data-friend]");
    if (!t) { if (F.card && !e.target.closest(".pcard") && !e.target.closest(".ctx-menu")) closeCard(); return; }
    const d = t.dataset, { S, render } = A();
    if (d.frOpen != null) { S.home = true; S.view = "friends"; S.channelId = null; S.navOpen = false; return render(); }
    if (d.frTab) { F.tab = d.frTab; F.addOpen = false; return render(); }
    if (d.frAdd != null) { F.addOpen = !F.addOpen || t.classList.contains("btn"); F.addMsg = null; S.home = true; S.view = "friends"; render(); const i = document.getElementById("fr-user"); if (i) i.focus(); return; }
    if (d.frAccept) { closeCard(); return act(() => api.accept_friend({ userId: d.frAccept }), "Agora são amigos."); }
    if (d.frDecline) return act(() => api.decline_friend({ userId: d.frDecline }), "Pedido recusado.");
    if (d.frCancel) { closeCard(); return act(() => api.cancel_friend({ userId: d.frCancel }), "Pedido cancelado."); }
    if (d.frUnblock) { closeCard(); return act(() => api.unblock_user({ userId: d.frUnblock }), "Desbloqueado."); }
    if (d.frMsg) return message(d.frMsg);
    if (d.frMore) { const r = t.getBoundingClientRect(); return userMenu(r.left, r.bottom + 4, d.frMore, F.card ? F.card.anchor : t); }
    if (d.pcFull) return openFullProfile(d.pcFull);
    if (d.pcEdit != null) { closeCard(); return A().openProfile(); }
    if (d.pcAdd) { closeCard(); return addFriend(d.pcAdd); }
    if (e.target.closest(".fr-acts")) return;
    const id = personOf(t);
    if (id && S.people[id] || id === "me" || byId(id)) { e.preventDefault(); openCard(id, t); }
  });
  document.addEventListener("contextmenu", (e) => {
    const t = e.target.closest && e.target.closest("[data-member], [data-person], [data-friend], [data-who], .vtile[data-spk], .voice-people [data-spk], .me-av[data-spk]");
    if (!t) return;
    const id = personOf(t);
    if (!id) return;
    e.preventDefault();
    userMenu(e.clientX, e.clientY, id, t);
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && F.card && !document.querySelector(".ctx-menu")) { e.stopPropagation(); closeCard(); }
    if (e.key === "Enter" && e.target.classList && e.target.classList.contains("fr-row")) openCard(e.target.dataset.friend, e.target);
  }, true);
  document.addEventListener("input", (e) => {
    if (e.target.id === "fr-user") {
      F.addValue = e.target.value; F.addMsg = null;
      const b = document.getElementById("fr-send"); if (b) b.disabled = !e.target.value.trim();
      const f = document.getElementById("fr-add-form"); if (f) f.classList.remove("ok", "bad");
      const m = document.querySelector(".fr-add-msg"); if (m) m.remove();
    }
    if (e.target.id === "fr-q") {
      F.query = e.target.value; const pos = e.target.selectionStart;
      A().render(); const n = document.getElementById("fr-q"); if (n) { n.focus(); n.setSelectionRange(pos, pos); }
    }
  });
  document.addEventListener("submit", async (e) => {
    if (e.target.id !== "fr-add-form") return;
    e.preventDefault();
    const v = (F.addValue || "").trim().replace(/^@/, "").toLowerCase();
    if (!v) return;
    const b = document.getElementById("fr-send"); if (b) { b.disabled = true; b.textContent = "A enviar…"; }
    try {
      const f = await api.send_friend_request({ username: v });
      F.addOk = true; F.addMsg = f && f.rel === "amigo" ? `Tu e @${v} já são amigos.` : `Pedido enviado a @${v}. Quando aceitar, aparece na tua lista.`;
      F.addValue = "";
      await refresh();
    } catch (x) { F.addOk = false; F.addMsg = String((x && x.message) || x); }
    A().render(); const i = document.getElementById("fr-user"); if (i) i.focus();
  });
  window.addEventListener("resize", () => { if (innerWidth > 720) closeCard(); });
  document.addEventListener("scroll", (e) => { if (F.card && innerWidth > 720 && !(e.target.closest && e.target.closest(".pcard"))) closeCard(); }, true);

  // a app pode já ter arrancado antes deste ficheiro carregar (no browser tudo é imediato)
  if (window.tascaApp && window.tascaApp.S.me) start().then(() => window.tascaApp.render());

  window.tascaAmigos = { start, renderFriends, sidebarButton, pendingCount, openCard, openFullProfile, userMenu, person, refresh, closeCard, cardOpen: () => !!F.card };
})();
