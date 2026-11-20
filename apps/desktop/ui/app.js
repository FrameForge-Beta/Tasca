// Tasca — interface. Sem framework nem passo de build: abre no browser e no Tauri tal como está.
(function () {
  "use strict";

  // ---------------- ícones ----------------
  const I = {
    hash: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M5 9h15M4 15h15M10 3 8 21M16 3l-2 18"/></svg>',
    speaker: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 5 6 9H3v6h3l5 4V5z"/><path d="M15.5 8.5a5 5 0 0 1 0 7M18.5 5.5a9 9 0 0 1 0 13"/></svg>',
    mic: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/></svg>',
    micOff: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M15 9.3V6a3 3 0 0 0-5.7-1.3M9 9v2a3 3 0 0 0 4.9 2.3M5 11a7 7 0 0 0 11.5 5.4M19 11a7 7 0 0 1-.6 2.8M12 18v3M3 3l18 18"/></svg>',
    head: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 18v-6a9 9 0 0 1 18 0v6"/><path d="M21 19a2 2 0 0 1-2 2h-1v-6h3zM3 19a2 2 0 0 0 2 2h1v-6H3z"/></svg>',
    headOff: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 18v-6a9 9 0 0 1 14.5-7.1M21 12v6"/><path d="M21 19a2 2 0 0 1-2 2h-1v-6h3zM3 19a2 2 0 0 0 2 2h1v-6H3zM3 3l18 18"/></svg>',
    gear: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg>',
    phoneOff: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.7 13.3a16 16 0 0 0 3.4 2.6l1.3-1.3a2 2 0 0 1 2.1-.4c.9.3 1.8.6 2.7.7a2 2 0 0 1 1.7 2v3a2 2 0 0 1-2.2 2A19.8 19.8 0 0 1 3 4.2 2 2 0 0 1 5 2h3a2 2 0 0 1 2 1.7c.1.9.4 1.8.7 2.7a2 2 0 0 1-.4 2.1L9 9.8"/><path d="M22 2 2 22"/></svg>',
    users: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="9" cy="8" r="4"/><path d="M2 21a7 7 0 0 1 14 0M16 3.1a4 4 0 0 1 0 7.8M22 21a7 7 0 0 0-4-6.3"/></svg>',
    plus: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>',
    send: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 2 11 13M22 2l-7 20-4-9-9-4z"/></svg>',
    clip: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="m21.4 11-8.6 8.6a6 6 0 0 1-8.5-8.5l8.6-8.6a4 4 0 0 1 5.7 5.7l-8.6 8.6a2 2 0 0 1-2.8-2.8l7.9-8"/></svg>',
    smile: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><path d="M8 14s1.5 2 4 2 4-2 4-2M9 9h.01M15 9h.01"/></svg>',
    puzzle: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M19.4 14.6a2.4 2.4 0 1 0 0-4.8H18V6a2 2 0 0 0-2-2h-3.8V2.6a2.4 2.4 0 1 0-4.8 0V4H4a2 2 0 0 0-2 2v3.6h1.4a2.4 2.4 0 1 1 0 4.8H2V18a2 2 0 0 0 2 2h3.6v-1.4a2.4 2.4 0 1 1 4.8 0V20H16a2 2 0 0 0 2-2v-3.4z"/></svg>',
    check: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 1.5 14.6 3.6l3.3-.2.9 3.2 2.8 1.8-1.2 3.1 1.2 3.1-2.8 1.8-.9 3.2-3.3-.2L12 22.5l-2.6-2.1-3.3.2-.9-3.2-2.8-1.8L3.6 12 2.4 8.9l2.8-1.8.9-3.2 3.3.2z"/><path d="m8 12 2.7 2.7L16.2 9" fill="none" stroke="var(--painel)" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg>',
    warn: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0zM12 9v4M12 17h.01"/></svg>',
    crown: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M3 7l4.5 4L12 4l4.5 7L21 7l-2 12H5z"/></svg>',
    menu: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 6h16M4 12h16M4 18h16"/></svg>',
    search: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/></svg>',
    chevron: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>',
    invite: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="9" cy="8" r="4"/><path d="M2 21a7 7 0 0 1 14 0M19 8v6M16 11h6"/></svg>',
    bell: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9M10.3 21a1.9 1.9 0 0 0 3.4 0"/></svg>',
    music: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>',
    grid: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M3 15h18M9 3v18M15 3v18"/></svg>',
    globe: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M2 12h20M12 2a15 15 0 0 1 0 20 15 15 0 0 1 0-20"/></svg>',
    palette: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22a10 10 0 1 1 10-10c0 2.8-2.2 4-4 4h-2a2 2 0 0 0-1.5 3.3A1.6 1.6 0 0 1 12 22z"/><circle cx="7.5" cy="10.5" r="1.2" fill="currentColor"/><circle cx="12" cy="7" r="1.2" fill="currentColor"/><circle cx="16.5" cy="10.5" r="1.2" fill="currentColor"/></svg>',
    close: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6 6 18M6 6l12 12"/></svg>',
    upload: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12"/></svg>',
    lock: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></svg>',
    cam: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m16 13 5.2 3.5a.5.5 0 0 0 .8-.4V7.9a.5.5 0 0 0-.8-.4L16 11"/><rect x="2" y="6" width="14" height="12" rx="2"/></svg>',
    camOff: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.7 6H14a2 2 0 0 1 2 2v3.3l1 1 4.2-3.5a.5.5 0 0 1 .8.4v8.2M16 16a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h2M2 2l20 20"/></svg>',
    screen: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4M12 13V7M9 10l3-3 3 3"/></svg>',
    screenOff: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4M9 7l6 6M15 7l-6 6"/></svg>',
    expand: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>',
    signal: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M4 20v-3M9 20v-7M14 20V9M19 20V4"/></svg>',
  };
  const LOGO = '<svg viewBox="0 0 512 512" aria-hidden="true"><rect width="512" height="512" rx="116" fill="#9B2C3F"/><path fill="#F6EEE1" d="M168,96 H344 A72,72 0 0 1 416,168 V300 A72,72 0 0 1 344,372 H226 L138,430 L152,364 A72,72 0 0 1 96,300 V168 A72,72 0 0 1 168,96Z"/><g transform="translate(256,234)"><path fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(90)" fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(180)" fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(270)" fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><circle r="16" fill="#E3A72F"/><circle r="7" fill="#F6EEE1"/></g></svg>';
  const TILE = "url(\"data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 120"><g transform="translate(60,60)" fill="#2A5CAA"><path d="M0,0 C24,-17.6 24,-39.3 0,-50.4 C-24,-39.3 -24,-17.6 0,0Z"/><path transform="rotate(90)" d="M0,0 C24,-17.6 24,-39.3 0,-50.4 C-24,-39.3 -24,-17.6 0,0Z"/><path transform="rotate(180)" d="M0,0 C24,-17.6 24,-39.3 0,-50.4 C-24,-39.3 -24,-17.6 0,0Z"/><path transform="rotate(270)" d="M0,0 C24,-17.6 24,-39.3 0,-50.4 C-24,-39.3 -24,-17.6 0,0Z"/><path transform="translate(-60,-60)" d="M0,0 A26.4,26.4 0 0 1 26.4,0 A26.4,26.4 0 0 1 0,26.4Z"/><path transform="translate(60,-60) rotate(90)" d="M0,0 A26.4,26.4 0 0 1 26.4,0 A26.4,26.4 0 0 1 0,26.4Z"/><path transform="translate(60,60) rotate(180)" d="M0,0 A26.4,26.4 0 0 1 26.4,0 A26.4,26.4 0 0 1 0,26.4Z"/><path transform="translate(-60,60) rotate(270)" d="M0,0 A26.4,26.4 0 0 1 26.4,0 A26.4,26.4 0 0 1 0,26.4Z"/><circle r="7.2" fill="#E3A72F"/></g></svg>') + "\")";
  document.documentElement.style.setProperty("--azulejo-tile", TILE);

  // ---------------- utilidades ----------------
  const $ = (s, el = document) => el.querySelector(s);
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  const initials = (n) => n.split(/\s+/).map((w) => w[0]).join("").slice(0, 2).toUpperCase();
  function fmt(text) {
    let s = esc(text);
    s = s.replace(/`([^`]+)`/g, "<code>$1</code>");
    s = s.replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>");
    s = s.replace(/(^|\s)@([\wÀ-ú]+)/g, '$1<span class="mention">@$2</span>');
    s = s.replace(/(^|\s)#([\w\-À-ú]+)/g, '$1<a href="#" data-chan-name="$2">#$2</a>');
    s = s.replace(/(https?:\/\/[^\s<]+)/g, '<a href="$1" target="_blank" rel="noopener">$1</a>');
    return s;
  }
  function avatar(p, cls = "", withStatus = false) {
    return `<span class="av ${cls}" style="background:${p.color}" aria-hidden="true">${esc(initials(p.name))}${
      withStatus ? `<i class="st ${p.status}"></i>` : ""}</span>`;
  }
  function toast(text) {
    const t = document.createElement("div");
    t.className = "toast"; t.textContent = text;
    $("#toasts").appendChild(t);
    setTimeout(() => t.remove(), 3200);
  }
  const store = {
    get(k, d) { try { const v = localStorage.getItem("tasca:" + k); return v == null ? d : JSON.parse(v); } catch { return d; } },
    set(k, v) { try { localStorage.setItem("tasca:" + k, JSON.stringify(v)); } catch {} },
  };

  // ---------------- estado ----------------
  const S = {
    me: null, people: {}, groups: [], channels: [], members: [], messages: [], plugins: [], settings: null,
    groupId: null, channelId: null,
    view: "chat",              // chat | voice | plugins | settings
    voice: null,               // { channelId, groupId, route, ping, codec }
    muted: false, deaf: false, speaking: [],
    camera: false,             // câmara ligada
    sharing: null,             // { source, quality, audio } quando estou a partilhar o ecrã
    focus: null,               // id do quadrado em destaque ("rui:screen", "me:cam", …); null = grelha
    callSidebar: false,        // mostrar grupos e canais durante a chamada
    showMembers: store.get("members", true),
    pluginTab: "loja", pluginQuery: "",
    settingsTab: "aparencia",
    typing: null, backupPct: null,
  };
  const group = () => S.groups.find((g) => g.id === S.groupId);
  const channel = () => S.channels.find((c) => c.id === S.channelId);

  // ---------------- render ----------------
  function render() {
    const app = $("#app");
    app.className = "app" + (S.showMembers && S.view === "chat" ? "" : " no-members") +
      (S.navOpen ? " show-nav" : "") + (S.membersOpen ? " show-members" : "") +
      (S.view === "voice" && !S.callSidebar ? " call" : "");
    $("#rail").innerHTML = renderRail();
    $("#sidebar").innerHTML = S.view === "settings" ? "" : renderSidebar();
    $("#sidebar").hidden = S.view === "settings";
    if (S.view === "settings") app.style.gridTemplateColumns = window.innerWidth > 720 ? "72px minmax(0,1fr)" : "";
    else app.style.gridTemplateColumns = "";
    $("#main").innerHTML = renderMain();
    fitCallGrid();
    $("#members").innerHTML = S.view === "chat" ? renderMembers() : "";
    const list = $(".messages");
    if (list) list.scrollTop = list.scrollHeight;
  }

  function renderRail() {
    const gs = S.groups.map((g) => {
      const on = S.groupId === g.id && (S.view === "chat" || S.view === "voice");
      const dot = `<i class="host-dot ${g.host.online ? "online" : ""}"></i>`;
      const badge = g.unread && !on ? `<span class="badge">${g.unread}</span>` : "";
      return `<button class="rail-btn ${on ? "on" : ""} ${g.unread ? "unread" : ""}" data-group="${g.id}" data-tip="${esc(g.name)}${g.host.online ? "" : " (anfitrião offline)"}" aria-label="${esc(g.name)}">${esc(g.short)}${badge || (g.owner === "me" ? "" : dot)}</button>`;
    }).join("");
    return `
      <button class="rail-btn logo" data-act="home" data-tip="Tasca" aria-label="Tasca">${LOGO}</button>
      <div class="rail-sep"></div>
      ${gs}
      <button class="rail-btn add" data-act="new-group" data-tip="Criar ou entrar num grupo" aria-label="Criar ou entrar num grupo">${I.plus}</button>
      <div class="rail-sep"></div>
      <button class="rail-btn store ${S.view === "plugins" ? "on" : ""}" data-act="plugins" data-tip="Plugins" aria-label="Plugins">${I.puzzle}</button>`;
  }

  function hostCard(g) {
    const owner = S.people[g.owner];
    if (g.owner === "me") return `<div class="host-card online"><i class="led"></i><b>Alojado neste PC</b><span>És o anfitrião. Mantém a Tasca aberta.</span></div>`;
    if (!g.host.online) return `<div class="host-card offline"><i class="led"></i><b>${esc(owner.name)} está offline</b><span>Visto ${esc(g.host.lastSeen)}. A ver cópia local.</span></div>`;
    return `<div class="host-card online"><i class="led"></i><b>Alojado no PC de ${esc(owner.name)}</b><span>Ligação ${esc(g.host.route)} · ${g.host.ping} ms</span></div>`;
  }

  function renderSidebar() {
    if (S.view === "plugins") {
      return `<div class="group-head"><span class="name">Plugins</span></div>
        <div class="channels">
          <div class="cat"><span>Loja</span></div>
          ${[["loja", "Explorar", I.search], ["instalados", "Instalados", I.puzzle], ["carregar", "Carregar ficheiro", I.upload]].map(([k, l, ic]) =>
            `<button class="chan ${S.pluginTab === k ? "on" : ""}" data-plugin-tab="${k}">${ic}<span class="label">${l}</span></button>`).join("")}
          <div class="host-card" style="margin-top:16px"><i class="led" style="background:var(--oliva)"></i><b>Como funciona a verificação</b><span>Cada plugin da loja é revisto pela equipa e assinado. A Tasca confirma a assinatura antes de instalar.</span></div>
        </div>${renderMe()}`;
    }
    const g = group();
    if (!g) return "";
    const cats = [];
    for (const c of S.channels) { if (!cats.includes(c.cat)) cats.push(c.cat); }
    const body = cats.map((cat) => `
      <div class="cat"><span>${esc(cat)}</span>${g.owner === "me" ? `<button class="icon-btn" style="width:22px;height:22px" data-act="new-channel" aria-label="Criar canal">${I.plus}</button>` : ""}</div>
      ${S.channels.filter((c) => c.cat === cat).map((c) => {
        if (c.kind === "text") {
          const on = c.id === S.channelId && S.view === "chat";
          return `<button class="chan ${on ? "on" : ""} ${c.unread && !on ? "unread" : ""}" data-chan="${c.id}">${I.hash}<span class="label">${esc(c.name)}</span>${c.unread && !on ? `<span class="pill">${c.unread}</span>` : ""}</button>`;
        }
        const on = S.voice && S.voice.channelId === c.id;
        const ppl = (c.inVoice || []).map((id) => S.people[id]);
        return `<button class="chan ${on && S.view === "voice" ? "on" : ""}" data-voice="${c.id}">${I.speaker}<span class="label">${esc(c.name)}</span>${ppl.length ? `<span class="pill" style="background:var(--caixa);color:var(--texto-2)">${ppl.length}</span>` : ""}</button>
          ${ppl.length ? `<ul class="voice-people">${ppl.map((p) => `<li class="${S.speaking.includes(p.id) && !(p.id === "me" && S.muted) ? "speaking" : ""}">${avatar(p, "")}${esc(p.name)}${p.id === "me" && S.muted ? `<span class="muted-ic">${I.micOff}</span>` : ""}</li>`).join("")}</ul>` : ""}`;
      }).join("")}`).join("");
    return `
      <div class="group-head"><span class="name">${esc(g.name)}</span>
        <button class="icon-btn" data-act="invite" aria-label="Convidar pessoas" title="Convidar pessoas">${I.invite}</button>
        <button class="icon-btn" data-act="group-menu" aria-label="Opções do grupo" title="Opções do grupo">${I.chevron}</button></div>
      ${hostCard(g)}
      <div class="channels">${body}</div>
      ${renderVoiceBar()}
      ${renderMe()}`;
  }

  function renderVoiceBar() {
    if (!S.voice) return "";
    const c = findChannel(S.voice.channelId);
    const g = S.groups.find((x) => x.id === S.voice.groupId);
    return `<div class="voice-bar">
      <span class="state">${I.signal.replace("<svg", '<svg width="14" height="14"')}${S.sharing ? "A partilhar ecrã" : S.camera ? "Voz e câmara" : "Ligado à voz"}</span>
      <button class="icon-btn danger" data-act="leave-voice" aria-label="Sair da chamada" title="Sair da chamada">${I.phoneOff}</button>
      <button class="where" data-voice="${c.id}" style="text-align:left">${esc(c.name)} / ${esc(g.name)} · ${esc(S.voice.route)}</button>
    </div>`;
  }

  function renderMe() {
    const me = S.me;
    return `<div class="me">${avatar(me, "s", true)}
      <div class="who"><b>${esc(me.name)}</b><small title="A tua chave pública">${esc(me.key.replace("ed25519:", ""))}</small></div>
      <button class="icon-btn ${S.muted ? "danger" : ""}" data-act="mute" aria-pressed="${S.muted}" aria-label="${S.muted ? "Ligar microfone" : "Desligar microfone"}" title="${S.muted ? "Ligar microfone" : "Desligar microfone"}">${S.muted ? I.micOff : I.mic}</button>
      <button class="icon-btn ${S.deaf ? "danger" : ""}" data-act="deaf" aria-pressed="${S.deaf}" aria-label="${S.deaf ? "Ligar som" : "Desligar som"}" title="${S.deaf ? "Ligar som" : "Desligar som"}">${S.deaf ? I.headOff : I.head}</button>
      <button class="icon-btn" data-act="settings" aria-label="Definições" title="Definições">${I.gear}</button>
    </div>`;
  }

  function renderMain() {
    if (S.view === "plugins") return renderPlugins();
    if (S.view === "settings") return renderSettings();
    if (S.view === "voice") return renderVoice();
    return renderChat();
  }

  const menuBtn = () => `<button class="icon-btn menu-btn" data-act="nav" aria-label="Grupos e canais">${I.menu}</button>`;

  function renderChat() {
    const c = channel(), g = group();
    if (!c) return "";
    let lastDay = null, lastFrom = null, lastAt = null;
    const rows = S.messages.map((m) => {
      const p = S.people[m.from];
      let out = "";
      if (m.day !== lastDay) { out += `<div class="day">${esc(m.day)}</div>`; lastDay = m.day; lastFrom = null; }
      const first = m.from !== lastFrom || m.file || minutes(m.at) - minutes(lastAt) > 7;
      lastFrom = m.from; lastAt = m.at;
      const reacts = m.reactions ? `<div class="reactions">${m.reactions.map((r, i) => `<button class="reaction ${r.mine ? "mine" : ""}" data-react="${m.id}:${i}">${r.e} ${r.n}</button>`).join("")}</div>` : "";
      const file = m.file ? `<div class="attach"><span class="ext">${esc(m.file.ext)}</span><div><b>${esc(m.file.name)}</b><small>${esc(m.file.size)} · enviado diretamente por ${esc(p.name)}</small></div></div>` : "";
      const state = m.pending ? `<span class="sent-state">a enviar ao anfitrião…</span>` : "";
      const cls = `msg ${first ? "first" : ""} ${m.mentionsMe ? "mentioned" : ""} ${m.pending ? "pending" : ""}`;
      if (first) {
        const tag = p.bot ? '<span class="tag bot">plugin</span>' : g.owner === p.id ? '<span class="tag">anfitrião</span>' : "";
        out += `<div class="${cls}">${avatar(p)}<div><div class="meta"><span class="author" style="color:${p.id === "me" ? "var(--texto)" : "inherit"}">${esc(p.name)}</span>${tag}<span class="time">${esc(m.day)} às ${esc(m.at)}</span></div><div class="body">${fmt(m.text)}${state}</div>${file}${reacts}</div></div>`;
      } else {
        out += `<div class="${cls}"><span class="gutter">${esc(m.at)}</span><div><div class="body">${fmt(m.text)}${state}</div>${file}${reacts}</div></div>`;
      }
      return out;
    }).join("");
    const offline = !g.host.online ? `<div class="banner">${I.warn}<div><b>${esc(S.people[g.owner].name)}</b>, o anfitrião, está offline. Estás a ver a tua cópia local. O que escreveres fica guardado e é enviado quando ele voltar, ou quando outro membro assumir o grupo.</div></div>` : "";
    const typing = S.typing && S.typing.channelId === c.id ? `<b>${esc(S.typing.who)}</b> está a escrever…` : "";
    return `
      <div class="main-head">${menuBtn()}
        <span class="title">${I.hash}${esc(c.name)}</span>
        <span class="topic">${esc(c.topic || "")}</span>
        <button class="icon-btn ${S.showMembers ? "on" : ""}" data-act="toggle-members" aria-label="Mostrar membros" title="Membros">${I.users}</button>
      </div>
      ${offline}
      <div class="messages" role="log" aria-live="polite">
        <div class="spacer"></div>
        <div class="welcome"><div class="tile"><span>#</span></div><h2>Bem-vindo a #${esc(c.name)}</h2><p>Este é o início do canal #${esc(c.name)} em ${esc(g.name)}. As mensagens ficam guardadas no PC do anfitrião e numa cópia em cada membro.</p></div>
        ${rows}
      </div>
      <div class="composer">
        <form id="composer">
          <button type="button" class="icon-btn" data-act="attach" aria-label="Anexar ficheiro" title="Anexar ficheiro">${I.clip}</button>
          <label class="sr" for="msg-input">Mensagem</label>
          <textarea id="msg-input" rows="1" placeholder="Escreve para #${esc(c.name)}"></textarea>
          <button type="button" class="icon-btn" data-act="emoji" aria-label="Emoji" title="Emoji">${I.smile}</button>
          <button type="submit" class="icon-btn send" aria-label="Enviar">${I.send}</button>
        </form>
        <div class="typing">${typing}</div>
      </div>`;
  }
  const minutes = (t) => { if (!t) return -999; const [h, m] = t.split(":").map(Number); return h * 60 + m; };

  function renderMembers() {
    if (!S.showMembers && !S.membersOpen) return "";
    const g = group();
    const on = S.members.filter((m) => m.status !== "offline");
    const off = S.members.filter((m) => m.status === "offline");
    const row = (m) => `<button class="member ${m.status === "offline" ? "off" : ""}" data-member="${m.id}">${avatar(m, "s", true)}<span class="nm"><b>${esc(m.name)}${m.owner ? `<span title="Anfitrião">${I.crown.replace("<svg", '<svg class="crown"')}</span>` : ""}</b><small>${m.bot ? "plugin" : m.owner ? "anfitrião" : m.status === "away" ? "ausente" : ""}</small></span></button>`;
    return `<div class="cat"><span>Online · ${on.length}</span></div>${on.map(row).join("")}
      <div class="cat"><span>Offline · ${off.length}</span></div>${off.map(row).join("")}`;
  }

  function findChannel(id) {
    for (const g of S.groups) { const list = S.allChannels[g.id] || []; const c = list.find((x) => x.id === id); if (c) return c; }
    return S.channels.find((x) => x.id === id);
  }

  // vídeo de teste: no Tauri isto passa a ser um <video> com o stream que chega do núcleo
  function fakeScreen(kind, who) {
    if (kind === "screen" && who === "rui") {
      return `<div class="fake fake-game"><div class="hud"><span>VOLTA 2/3</span><span>3.º</span></div><div class="road"></div><div class="kart"></div></div>`;
    }
    if (kind === "screen") {
      const name = S.sharing ? S.sharing.source : "Ecrã";
      return `<div class="fake fake-desk"><div class="win"><div class="bar"><i></i><i></i><i></i><span>${esc(name)}</span></div><div class="code">${Array.from({ length: 9 }, (_, i) => `<b style="width:${30 + ((i * 37) % 55)}%"></b>`).join("")}</div></div></div>`;
    }
    const p = S.people[who];
    return `<div class="fake fake-cam" style="--c:${p.color}"><span class="silhouette"></span></div>`;
  }

  // Chamada: ocupa a janela toda. Cada pessoa e cada ecrã partilhado é um quadrado na grelha.
  // Clicar num quadrado põe-no em destaque; clicar outra vez volta à grelha.
  function callGrid(n, w, h) {
    // escolhe o número de colunas que dá quadrados 16:9 maiores
    let best = { cols: 1, rows: n, size: 0 };
    for (let cols = 1; cols <= n; cols++) {
      const rows = Math.ceil(n / cols);
      const cw = (w - (cols - 1) * 8) / cols, ch = (h - (rows - 1) * 8) / rows;
      const tw = Math.min(cw, ch * 16 / 9);
      if (tw > best.size) best = { cols, rows, size: tw };
    }
    return best;
  }

  function renderVoice() {
    const c = findChannel(S.voice.channelId);
    const g = S.groups.find((x) => x.id === S.voice.groupId);
    const ppl = (c.inVoice || []).map((id) => S.people[id]);
    const items = [];
    if (c.inVoice.includes("rui") && c.id === "v-sala") items.push({ id: "rui:screen", who: "rui", kind: "screen", label: "Rui", meta: "1080p 60fps" });
    if (S.sharing) items.push({ id: "me:screen", who: "me", kind: "screen", label: "O teu ecrã", meta: S.sharing.quality.replace(" · ", " ") });
    for (const p of ppl) {
      const cam = p.id === "me" ? S.camera : p.id === "ines";
      items.push({ id: p.id + ":cam", who: p.id, kind: cam ? "cam" : "avatar", label: p.name + (p.id === "me" ? " (tu)" : "") });
    }
    if (S.focus && !items.some((x) => x.id === S.focus)) S.focus = null;

    const tile = (x, extra = "") => {
      const p = S.people[x.who];
      const muted = x.who === "me" && S.muted;
      const speaking = x.kind !== "screen" && S.speaking.includes(x.who) && !muted;
      const inner = x.kind === "avatar" ? `<span class="tile-bg" style="--c:${p.color}"></span>${avatar(p)}` : fakeScreen(x.kind === "screen" ? "screen" : "cam", x.who);
      const badge = x.kind === "screen" ? `<span class="live"><b>${esc(x.meta)}</b>AO VIVO</span>` : "";
      const host = x.kind !== "screen" && g.owner === x.who ? '<span class="host-tag">ANFITRIÃO</span>' : "";
      return `<button class="vtile ${speaking ? "speaking" : ""} ${x.kind !== "avatar" ? "has-video" : ""} ${extra}" data-focus="${x.id}" aria-label="${S.focus === x.id ? "Voltar à grelha" : "Pôr em destaque: " + esc(x.label)}" ${extra.includes("grid-cell") ? 'style="width:var(--tw);"' : ""}>${inner}${badge}${host}
        <span class="lbl">${x.kind === "screen" ? I.screen.replace("<svg", '<svg style="color:inherit"') : muted ? I.micOff : ""}${esc(x.label)}</span></button>`;
    };

    let body;
    if (S.focus) {
      const f = items.find((x) => x.id === S.focus);
      body = `<div class="call-focus">${tile(f, "big")}<div class="call-strip">${items.filter((x) => x !== f).map((x) => tile(x)).join("")}</div></div>`;
    } else {
      body = `<div class="call-grid" id="call-grid" data-n="${items.length}">${items.map((x) => tile(x, "grid-cell")).join("")}</div>`;
    }
    const up = (S.camera ? 1.2 : 0) + (S.sharing ? (S.sharing.quality.startsWith("1080p · 60") ? 6 : S.sharing.quality.startsWith("1080") ? 4 : 2.5) : 0) + 0.06;
    return `
      <div class="callview">
        <div class="call-top">
          <button class="icon-btn ${S.callSidebar ? "on" : ""}" data-act="call-sidebar" aria-label="${S.callSidebar ? "Esconder grupos e canais" : "Mostrar grupos e canais"}" title="${S.callSidebar ? "Esconder grupos e canais" : "Mostrar grupos e canais"}">${I.menu}</button>
          <span class="title">${I.speaker}${esc(c.name)}<small>${esc(g.name)}</small></span>
          <span class="call-chips">
            <span class="chip" title="Ligação ao anfitrião"><i class="dot"></i>${esc(S.voice.route)} · ${S.voice.ping} ms</span>
            <span class="chip" title="Áudio e vídeo cifrados ponta a ponta">${I.lock.replace("<svg", '<svg width="14" height="14"')}Cifrado</span>
            <span class="chip ${up > 5 ? "warn" : ""}" title="Quanto estás a enviar"><i class="dot"></i>${up.toFixed(1).replace(".", ",")} Mbps</span>
          </span>
          <button class="icon-btn" data-chan-open="${esc(c.id)}" data-act="call-chat" aria-label="Abrir o chat do grupo" title="Abrir o chat (a chamada continua)">${I.hash}</button>
          <button class="icon-btn" data-act="fullscreen" aria-label="Ecrã inteiro" title="Ecrã inteiro">${I.expand}</button>
        </div>
        ${body}
        <div class="call-controls">
          <div class="ctl-group">
            <button class="ctl ${S.muted ? "off" : ""}" data-act="mute" aria-pressed="${S.muted}" title="${S.muted ? "Ligar microfone" : "Desligar microfone"}" aria-label="${S.muted ? "Ligar microfone" : "Desligar microfone"}">${S.muted ? I.micOff : I.mic}</button>
            <button class="ctl ${S.deaf ? "off" : ""}" data-act="deaf" aria-pressed="${S.deaf}" title="${S.deaf ? "Ligar som" : "Desligar som"}" aria-label="${S.deaf ? "Ligar som" : "Desligar som"}">${S.deaf ? I.headOff : I.head}</button>
            <button class="ctl ${S.camera ? "on-media" : ""}" data-act="camera" aria-pressed="${S.camera}" title="${S.camera ? "Desligar câmara" : "Ligar câmara"}" aria-label="${S.camera ? "Desligar câmara" : "Ligar câmara"}">${S.camera ? I.cam : I.camOff}</button>
          </div>
          <div class="ctl-group">
            <button class="ctl ${S.sharing ? "on-media" : ""}" data-act="share" aria-pressed="${!!S.sharing}" title="${S.sharing ? "Parar partilha" : "Partilhar ecrã"}" aria-label="${S.sharing ? "Parar partilha" : "Partilhar ecrã"}">${S.sharing ? I.screenOff : I.screen}</button>
            <button class="ctl" data-act="settings-voz" title="Definições de voz e vídeo" aria-label="Definições de voz e vídeo">${I.gear}</button>
          </div>
          <button class="ctl leave" data-act="leave-voice" title="Sair da chamada" aria-label="Sair da chamada">${I.phoneOff}</button>
        </div>
      </div>`;
  }

  // ajusta o tamanho dos quadrados da grelha ao espaço disponível
  function fitCallGrid() {
    const el = document.getElementById("call-grid");
    if (!el) return;
    const n = +el.dataset.n || 1;
    const { cols, size } = callGrid(n, el.clientWidth - 24 - 2, el.clientHeight - 2);
    el.style.setProperty("--tw", Math.floor(size) + "px");
    el.style.setProperty("--cols", cols);
  }
  window.addEventListener("resize", fitCallGrid);

  function openSharePicker() {
    const sources = [
      ["Ecrã inteiro", "Monitor 1 · 2560 × 1440", "screen"],
      ["Monitor 2", "1920 × 1080", "screen"],
      ["Visual Studio Code", "Janela", "win"],
      ["Minecraft", "Janela · jogo", "win"],
      ["Google Chrome", "Janela", "win"],
    ];
    modal(`<h2>Partilhar ecrã</h2>
      <p>Escolhe o que os outros vão ver.</p>
      <div class="sources" role="radiogroup" aria-label="O que partilhar">${sources.map(([n, d, k], i) => `<button role="radio" aria-checked="${i === 0}" class="${i === 0 ? "on" : ""}" data-src="${esc(n)}"><span class="thumb ${k}"></span><b>${esc(n)}</b><small>${esc(d)}</small></button>`).join("")}</div>
      <div class="field"><label>Qualidade</label><div class="seg" role="radiogroup" aria-label="Qualidade">${["720p · 30 fps", "1080p · 30 fps", "1080p · 60 fps"].map((q, i) => `<button role="radio" aria-checked="${i === 1}" class="${i === 1 ? "on" : ""}" data-q="${q}">${q}</button>`).join("")}</div>
        <small style="color:var(--texto-3)">Mais qualidade gasta mais upload do anfitrião e do teu PC.</small></div>
      <div class="row" style="padding:0"><div class="txt"><b>Partilhar também o som</b><small>O som do jogo ou do vídeo vai junto.</small></div><label class="switch"><input type="checkbox" id="share-audio" checked><span></span><span class="sr">Partilhar som</span></label></div>
      <div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn primary" id="do-share">${I.screen}Partilhar</button></div>`,
    (el, close) => {
      let src = sources[0][0], q = "1080p · 30 fps";
      el.addEventListener("click", (e) => {
        const b = e.target.closest("[data-src], [data-q]"); if (!b) return;
        const sel = b.dataset.src ? "[data-src]" : "[data-q]";
        el.querySelectorAll(sel).forEach((x) => { x.classList.toggle("on", x === b); x.setAttribute("aria-checked", x === b); });
        if (b.dataset.src) src = b.dataset.src; else q = b.dataset.q;
      });
      $("#do-share", el).onclick = async () => {
        const audio = $("#share-audio", el).checked;
        await api.start_screen_share({ source: src, quality: q, audio });
        S.sharing = { source: src, quality: q, audio };
        close(); render(); toast(`Estás a partilhar «${src}».`);
      };
    });
  }

  // ---------------- plugins ----------------
  function pluginCard(p) {
    const badge = p.verified
      ? `<span class="verified" title="Revisto e assinado pela equipa Tasca">${I.check}Verificado</span>`
      : `<span class="unverified" title="Ainda não revisto pela equipa">${I.warn}Não verificado</span>`;
    let action;
    if (!p.installed) action = `<button class="btn ${p.verified ? "primary" : ""}" data-install="${p.id}">Instalar</button>`;
    else action = `<label class="switch" title="${p.enabled ? "Desativar" : "Ativar"}"><input type="checkbox" id="pl-${p.id}" data-plugin-toggle="${p.id}" ${p.enabled ? "checked" : ""}><span></span></label>`;
    return `<article class="plugin">
      <div class="top"><span class="pic" style="background:${p.color}">${I[p.icon] || I.puzzle}</span>
        <div style="min-width:0"><h3>${esc(p.name)} ${badge}</h3><div class="by">${esc(p.author)} · v${esc(p.version)} · ${esc(p.runtime)}</div></div></div>
      <p>${esc(p.desc)}</p>
      <div class="perms">${p.perms.map((x) => `<span class="perm ${(p.risky || []).includes(x) ? "risky" : ""}">${esc(x)}</span>`).join("")}</div>
      <div class="foot"><span>${p.installed ? (p.enabled ? "Ativo" : "Desativado") : `${esc(p.installs)} instalações`}</span>
        <span style="display:flex;gap:6px;align-items:center">${p.installed ? `<button class="btn ghost" data-uninstall="${p.id}">Remover</button>` : ""}${action}</span></div>
    </article>`;
  }

  function renderPlugins() {
    const q = S.pluginQuery.toLowerCase();
    let list = S.plugins.filter((p) => !q || (p.name + p.desc + p.author).toLowerCase().includes(q));
    if (S.pluginTab === "instalados") list = list.filter((p) => p.installed);
    const nInst = S.plugins.filter((p) => p.installed).length;
    let body;
    if (S.pluginTab === "carregar") {
      body = `<div class="box"><div class="row"><div class="txt"><b>Carregar um plugin a partir de ficheiro</b><small>Para testares plugins teus antes de os enviares para a loja. Aceita .tasca-plugin (WASM ou JS).</small></div>
        <label class="btn" for="sideload">${I.upload}Escolher ficheiro</label><input type="file" id="sideload" accept=".tasca-plugin,.wasm,.js,.zip" hidden></div>
        <div class="row" style="background:var(--mencao-bg)"><span style="color:var(--ocre);width:20px">${I.warn}</span><div class="txt"><b>Plugins carregados à mão não são verificados</b><small>Corre num ambiente isolado e só tem as permissões que aceitares, mas a equipa não reviu o código. Instala apenas o que confias.</small></div></div></div>`;
    } else {
      body = list.length ? `<div class="plugins">${list.map(pluginCard).join("")}</div>` : `<p class="lede">Nenhum plugin corresponde a «${esc(S.pluginQuery)}».</p>`;
    }
    return `
      <div class="main-head">${menuBtn()}<span class="title">${I.puzzle}Plugins</span><span class="topic">Catálogo no GitHub, revisto e assinado pela equipa</span></div>
      <div class="page"><div class="page-inner">
        <div style="display:grid;gap:6px"><h1>Plugins</h1><p class="lede">Junta funcionalidades à tua Tasca. Os plugins com o selo <span class="verified">${I.check}Verificado</span> foram revistos pela equipa e a assinatura é confirmada antes de instalar.</p></div>
        <div class="tabs" role="tablist">
          <button class="tab ${S.pluginTab === "loja" ? "on" : ""}" data-plugin-tab="loja">Explorar<span class="n">${S.plugins.length}</span></button>
          <button class="tab ${S.pluginTab === "instalados" ? "on" : ""}" data-plugin-tab="instalados">Instalados<span class="n">${nInst}</span></button>
          <button class="tab ${S.pluginTab === "carregar" ? "on" : ""}" data-plugin-tab="carregar">Carregar ficheiro</button>
        </div>
        ${S.pluginTab !== "carregar" ? `<div class="toolbar"><div class="search">${I.search}<label class="sr" for="plugin-q">Procurar plugins</label><input id="plugin-q" placeholder="Procurar plugins" value="${esc(S.pluginQuery)}"></div></div>` : ""}
        ${body}
      </div></div>`;
  }

  // ---------------- definições ----------------
  const SET_TABS = [
    ["Utilizador", [["conta", "A minha conta"], ["aparencia", "Aparência"], ["voz", "Voz e vídeo"]]],
    ["Tasca", [["rede", "Rede e relay"], ["backups", "Cópias de segurança"], ["grupos", "Grupos que aloja"], ["sobre", "Sobre"]]],
  ];
  function renderSettings() {
    const nav = SET_TABS.map(([cat, items]) => `<div class="cat"><span>${cat}</span></div>${items.map(([k, l]) => `<button class="${S.settingsTab === k ? "on" : ""}" data-set-tab="${k}">${l}</button>`).join("")}`).join("");
    const T = { conta: setConta, aparencia: setAparencia, voz: setVoz, rede: setRede, backups: setBackups, grupos: setGrupos, sobre: setSobre }[S.settingsTab];
    return `<div class="settings">
      <nav aria-label="Secções das definições">${nav}</nav>
      <div class="page"><div class="page-inner">
        <div style="display:flex;align-items:center;gap:12px">${menuBtn()}<h1 style="flex:1">${SET_TABS.flatMap((x) => x[1]).find((x) => x[0] === S.settingsTab)[1]}</h1>
          <button class="icon-btn" data-act="close-settings" aria-label="Fechar definições" title="Fechar (Esc)">${I.close}</button></div>
        ${T()}
      </div></div></div>`;
  }
  const sw = (id, on, label) => `<label class="switch"><input type="checkbox" id="${id}" data-setting="${id}" ${on ? "checked" : ""}><span></span><span class="sr">${label}</span></label>`;

  function setConta() {
    const me = S.me;
    return `<div class="section"><div class="box">
      <div class="row"><span class="av" style="width:64px;height:64px;font-size:24px;background:${me.color}" aria-hidden="true">${esc(initials(me.name))}</span>
        <div class="txt"><b style="font-size:18px">${esc(me.name)}</b><small>O teu nome pode ser diferente em cada grupo.</small></div><button class="btn">Editar perfil</button></div>
      <div class="row"><div class="txt"><b>A tua identidade</b><small>Não há contas nem palavras-passe num servidor. És identificado por esta chave, guardada só neste PC.</small><div class="mono" style="margin-top:6px;color:var(--texto-2)">${esc(me.key)}</div></div><button class="btn" data-act="copy-key">Copiar</button></div>
      <div class="row"><div class="txt"><b>Exportar identidade</b><small>Guarda a chave num ficheiro cifrado para usar a mesma conta noutro PC. Sem ela, perdes o acesso aos grupos.</small></div><button class="btn">Exportar…</button></div>
    </div></div>`;
  }
  function setAparencia() {
    const s = S.settings;
    return `<div class="section"><h2>Tema</h2><div class="box"><div class="row"><div class="txt"><b>Modo</b><small>Madeira escura é o tema da casa.</small></div>
      <div class="seg" role="radiogroup" aria-label="Tema">${[["dark", "Madeira (escuro)"], ["light", "Cal (claro)"], ["system", "Sistema"]].map(([k, l]) => `<button role="radio" aria-checked="${s.theme === k}" class="${s.theme === k ? "on" : ""}" data-theme-set="${k}">${l}</button>`).join("")}</div></div>
      <div class="row"><div class="txt"><b>Tamanho do texto</b><small>${s.fontSize} px</small></div><input class="range" style="max-width:240px" type="range" min="13" max="18" id="fontSize" data-setting="fontSize" value="${s.fontSize}"></div>
      <div class="row"><div class="txt"><b>Mensagens compactas</b><small>Menos espaço entre mensagens.</small></div>${sw("compact", s.compact, "Mensagens compactas")}</div>
    </div></div>`;
  }
  function setVoz() {
    const s = S.settings;
    return `<div class="section"><h2>Dispositivos</h2><div class="box">
      <div class="row"><div class="field" style="flex:1;min-width:200px"><label for="inputDevice">Entrada</label><select class="input" id="inputDevice" data-setting="inputDevice">${["Microfone (Realtek Audio)", "Microfone USB (Blue Yeti)", "Predefinição do Windows"].map((d) => `<option ${d === s.inputDevice ? "selected" : ""}>${d}</option>`).join("")}</select></div>
        <div class="field" style="flex:1;min-width:200px"><label for="outputDevice">Saída</label><select class="input" id="outputDevice" data-setting="outputDevice">${["Auscultadores (Realtek Audio)", "Colunas (Realtek Audio)", "Predefinição do Windows"].map((d) => `<option ${d === s.outputDevice ? "selected" : ""}>${d}</option>`).join("")}</select></div></div>
      <div class="row"><div class="txt"><b>Sensibilidade do microfone</b><small>Abaixo deste nível o microfone não transmite.</small><div class="meter" style="margin-top:8px"><i id="mic-meter" style="width:40%"></i></div></div><input class="range" style="max-width:200px" type="range" min="0" max="100" id="sensitivity" data-setting="sensitivity" value="${s.sensitivity}"></div>
      <div class="row"><div class="txt"><b>Carregar para falar</b><small>Só transmite enquanto carregas na tecla (V).</small></div>${sw("pushToTalk", s.pushToTalk, "Carregar para falar")}</div>
      <div class="row"><div class="txt"><b>Supressão de ruído</b><small>Remove ventoinhas e teclado. Corre no teu PC.</small></div>${sw("noiseSuppression", s.noiseSuppression, "Supressão de ruído")}</div>
      <div class="row"><div class="txt"><b>Cancelamento de eco</b></div>${sw("echoCancel", s.echoCancel, "Cancelamento de eco")}</div>
    </div><h2>Vídeo</h2><div class="box">
      <div class="row"><div class="field" style="flex:1;min-width:200px"><label for="camDevice">Câmara</label><select class="input" id="camDevice" data-setting="camDevice">${["Câmara integrada (HD)", "Logitech C920", "OBS Virtual Camera"].map((d) => `<option>${d}</option>`).join("")}</select></div>
        <div class="field" style="flex:1;min-width:200px"><label for="shareQuality">Qualidade da partilha de ecrã</label><select class="input" id="shareQuality" data-setting="shareQuality">${["720p · 30 fps", "1080p · 30 fps", "1080p · 60 fps"].map((d, i) => `<option ${i === 1 ? "selected" : ""}>${d}</option>`).join("")}</select></div></div>
      <div class="row"><div class="txt"><b>Usar a placa gráfica para codificar</b><small>Menos peso no processador quando partilhas um jogo (NVENC, AMF ou Quick Sync).</small></div>${sw("hwEncode", true, "Codificação por hardware")}</div>
    </div><p class="lede" style="font-size:13px">Codec Opus. Vídeo em AV1 ou H.264, conforme a placa gráfica. O anfitrião reencaminha áudio e vídeo sem os descodificar.</p></div>`;
  }
  function setRede() {
    const s = S.settings;
    return `<div class="section"><h2>Ligação entre PCs</h2><div class="box">
      <div class="row"><div class="txt"><b>Relay</b><small>Quando dois PCs não se conseguem ligar diretamente (CGNAT, firewall), o tráfego cifrado passa por um relay. O relay não consegue ler nada.</small></div></div>
      <div class="row"><div class="seg" role="radiogroup" aria-label="Relay">${[["public", "Relays públicos do iroh"], ["own", "O meu servidor"]].map(([k, l]) => `<button role="radio" aria-checked="${s.relay === k}" class="${s.relay === k ? "on" : ""}" data-relay="${k}">${l}</button>`).join("")}</div></div>
      ${s.relay === "own" ? `<div class="row"><div class="field" style="flex:1"><label for="relayUrl">Endereço do teu relay</label><input class="input mono" id="relayUrl" data-setting="relayUrl" value="${esc(s.relayUrl)}"></div><button class="btn" data-act="test-relay">Testar</button></div>` : ""}
      <div class="row"><dl class="kv"><dt>Estado</dt><dd><span style="color:var(--oliva);font-weight:700">● Ligado</span></dd><dt>Ligações diretas</dt><dd>3 de 4 grupos</dd><dt>Via relay</dt><dd>Clube de Xadrez (64 ms)</dd></dl></div>
    </div></div>`;
  }
  function setBackups() {
    const b = S.settings.backup;
    const dests = [
      ["gdrive", "Google Drive", "#1A73E8", "GD"],
      ["onedrive", "OneDrive", "#0F6CBD", "OD"],
      ["local", "Ficheiro local", "var(--vinho)", "PC"],
    ];
    const hosted = S.groups.filter((g) => g.owner === "me");
    const prog = S.backupPct != null ? `<div class="row"><div class="txt"><b>A fazer cópia… ${S.backupPct}%</b><small>Comprimir com zstd, cifrar, enviar.</small><div class="meter" style="margin-top:8px"><i style="width:${S.backupPct}%"></i></div></div></div>` : "";
    return `
      <p class="lede">Os grupos que alojas vivem no teu PC. Faz cópias para não os perderes se o disco falhar. Tudo é comprimido e cifrado aqui antes de sair: a Google ou a Microsoft só veem um ficheiro ilegível.</p>
      <div class="section"><h2>Destino</h2>
        <div class="dest" role="radiogroup" aria-label="Destino das cópias">${dests.map(([k, l, col, ab]) => `<button role="radio" aria-checked="${b.dest === k}" class="${b.dest === k ? "on" : ""}" data-dest="${k}"><span class="lg" style="background:${col}">${ab}</span><b>${l}</b><small>${b.connected[k] ? esc(b.connected[k]) : "Não ligado"}</small></button>`).join("")}</div>
        ${!b.connected[b.dest] ? `<div class="box"><div class="row"><div class="txt"><b>Ligar ${dests.find((d) => d[0] === b.dest)[1]}</b><small>Abre o browser para autorizares. A Tasca só pede acesso a uma pasta própria.</small></div><button class="btn blue" data-act="connect-dest">Ligar conta</button></div></div>` : ""}
      </div>
      <div class="section"><h2>Agendamento</h2><div class="box">
        <div class="row"><div class="txt"><b>Cópias automáticas</b><small>Só envia o que mudou desde a última cópia.</small></div>${sw("backup.auto", b.auto, "Cópias automáticas")}</div>
        <div class="row"><div class="txt"><b>Frequência</b></div><div class="seg" role="radiogroup" aria-label="Frequência">${[["hora", "De hora a hora"], ["diario", "Diária, 04:00"], ["semanal", "Semanal"]].map(([k, l]) => `<button role="radio" aria-checked="${b.every === k}" class="${b.every === k ? "on" : ""}" data-every="${k}">${l}</button>`).join("")}</div></div>
        <div class="row"><div class="txt"><b>Grupos incluídos</b><small>${hosted.map((g) => esc(g.name)).join(", ") || "Ainda não alojas nenhum grupo."}</small></div></div>
      </div></div>
      <div class="section"><h2>Compressão e cifra</h2><div class="box">
        <div class="row"><div class="txt"><b>Nível de compressão zstd</b><small>Nível ${b.zstdLevel}. Mais alto ocupa menos espaço mas demora mais.</small></div><input class="range" style="max-width:200px" type="range" min="3" max="22" id="zstdLevel" data-setting="backup.zstdLevel" value="${b.zstdLevel}"></div>
        <div class="row"><div class="txt"><b>Reduzir imagens para AVIF</b><small>As fotos ocupam cerca de metade, quase sem perda visível.</small></div>${sw("backup.avif", b.avif, "Reduzir imagens para AVIF")}</div>
        <div class="row"><span style="color:var(--oliva);width:20px">${I.lock}</span><div class="txt"><b>Palavra-passe das cópias</b><small>Definida. Guarda-a bem: sem ela ninguém, nem tu, consegue abrir as cópias.</small></div><button class="btn">Alterar</button></div>
      </div></div>
      <div class="section"><h2>Histórico</h2><div class="box">${prog}
        <ul class="history">${b.history.map((h) => `<li><span class="ok">✓</span><span>${esc(h.at)} · ${esc(h.note)}</span><span>${esc(h.size)}</span></li>`).join("")}</ul>
        <div class="row" style="justify-content:flex-end;gap:8px"><button class="btn ghost">Restaurar…</button><button class="btn primary" data-act="backup-now" ${S.backupPct != null ? "disabled" : ""}>Fazer cópia agora</button></div>
      </div></div>`;
  }
  function setGrupos() {
    const hosted = S.groups.filter((g) => g.owner === "me");
    return `<p class="lede">Grupos que estão alojados neste PC. Enquanto a Tasca estiver aberta, os membros conseguem falar contigo e entre si.</p>
      <div class="section"><div class="box">${hosted.map((g) => `<div class="row"><span class="rail-btn on" style="width:40px;height:40px;border-radius:12px;font-size:13px">${esc(g.short)}</span><div class="txt"><b>${esc(g.name)}</b><small>${(S.allMembers[g.id] || []).length} membros · 48,2 MB no disco</small></div><button class="btn" data-act="transfer" data-group-id="${g.id}">Transferir propriedade…</button></div>`).join("") || `<div class="row"><div class="txt"><small>Ainda não alojas nenhum grupo.</small></div></div>`}</div></div>
      <div class="section"><h2>Arranque</h2><div class="box"><div class="row"><div class="txt"><b>Abrir a Tasca quando o Windows arranca</b><small>Minimizada na área de notificação, para os teus grupos estarem sempre disponíveis.</small></div>${sw("autostart", true, "Abrir no arranque")}</div></div></div>`;
  }
  function setSobre() {
    return `<div class="section"><div class="box"><div class="row"><span style="width:56px;height:56px;display:block">${LOGO}</span><div class="txt"><b style="font-size:18px">Tasca</b><small>Versão 0.1.0 (pré-visualização) · ${window.api.isMock ? "dados de teste no browser" : "app desktop, núcleo P2P ainda por ligar"}</small></div></div>
      <div class="row"><div class="txt"><small>Conversa, voz e plugins sem servidores centrais. Cada grupo vive no PC de quem o criou. Feito com Rust, iroh, SQLite e Tauri.</small></div></div></div></div>`;
  }

  // ---------------- modais ----------------
  function modal(html, onMount) {
    const scrim = document.createElement("div");
    scrim.className = "scrim";
    scrim.innerHTML = `<div class="modal" role="dialog" aria-modal="true">${html}</div>`;
    scrim.addEventListener("click", (e) => { if (e.target === scrim || e.target.closest("[data-close]")) scrim.remove(); });
    document.body.appendChild(scrim);
    const first = scrim.querySelector("input, textarea, button.primary, button");
    if (first) first.focus();
    if (onMount) onMount(scrim, () => scrim.remove());
  }

  function openNewGroup() {
    modal(`<h2>Criar ou entrar num grupo</h2>
      <p>Um grupo novo fica alojado neste PC. Para entrar no grupo de alguém, cola o convite que te enviaram.</p>
      <div class="field"><label for="new-name">Nome do grupo novo</label><input class="input" id="new-name" placeholder="Ex.: Malta do Futsal" maxlength="40"></div>
      <div class="actions"><button class="btn primary" id="do-create">Criar grupo</button></div>
      <div class="day" style="margin:4px 0">ou</div>
      <div class="field"><label for="invite-code">Convite</label><textarea class="input mono" id="invite-code" rows="2" placeholder="tasca1…"></textarea></div>
      <p id="join-err" style="color:var(--perigo)" hidden></p>
      <div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn blue" id="do-join">Entrar</button></div>`,
    (el, close) => {
      $("#do-create", el).onclick = async () => {
        const name = $("#new-name", el).value.trim();
        if (!name) { $("#new-name", el).focus(); return; }
        const g = await api.create_group({ name });
        S.groups.push(g); close(); await openGroup(g.id); toast(`Grupo «${name}» criado. Está alojado neste PC.`);
      };
      $("#do-join", el).onclick = async (e) => {
        const err = $("#join-err", el); err.hidden = true;
        e.target.disabled = true; e.target.textContent = "A ligar…";
        try { await api.join_group({ invite: $("#invite-code", el).value }); close(); }
        catch (x) { err.textContent = x.message; err.hidden = false; }
        e.target.disabled = false; e.target.textContent = "Entrar";
      };
    });
  }

  async function openInvite() {
    const g = group();
    const inv = await api.create_invite({ groupId: g.id });
    modal(`<h2>Convidar para ${esc(g.name)}</h2>
      <p>Envia este código a quem quiseres convidar. Quem o tiver entra diretamente no teu PC${g.owner === "me" ? "" : " do anfitrião"}, sem passar por servidores.</p>
      <div class="field"><label for="inv">Convite · expira em ${inv.expires}</label><textarea class="input mono" id="inv" rows="3" readonly>${inv.code}</textarea></div>
      <div class="actions"><button class="btn ghost" data-close>Fechar</button><button class="btn primary" id="copy-inv">Copiar convite</button></div>`,
    (el) => {
      $("#copy-inv", el).onclick = () => copy($("#inv", el).value, $("#inv", el), "Convite copiado.");
    });
  }

  function openGroupMenu() {
    const g = group();
    const mine = g.owner === "me";
    modal(`<h2>${esc(g.name)}</h2>
      <div class="box">
        <div class="row"><div class="txt"><b>Anfitrião</b><small>${mine ? "Este PC" : esc(S.people[g.owner].name)}</small></div></div>
        <div class="row"><div class="txt"><b>Notificações</b><small>Só menções</small></div><button class="btn">Mudar</button></div>
        ${mine ? `<div class="row"><div class="txt"><b>Transferir propriedade</b><small>Passa o grupo para o PC de outro membro. Ele tem de aceitar.</small></div><button class="btn" data-act="transfer" data-group-id="${g.id}">Transferir…</button></div>` : ""}
        <div class="row"><div class="txt"><b>${mine ? "Apagar grupo" : "Sair do grupo"}</b><small>${mine ? "Apaga o grupo deste PC. Os membros ficam com a cópia deles." : "Apaga a tua cópia local."}</small></div><button class="btn danger">${mine ? "Apagar" : "Sair"}</button></div>
      </div>
      <div class="actions"><button class="btn ghost" data-close>Fechar</button></div>`);
  }

  function openTransfer(groupId) {
    const g = S.groups.find((x) => x.id === groupId);
    const others = (S.allMembers[groupId] || []).filter((m) => m.id !== "me" && !m.bot);
    document.querySelectorAll(".scrim").forEach((x) => x.remove());
    modal(`<h2>Transferir ${esc(g.name)}</h2>
      <p>O grupo passa a viver no PC da pessoa que escolheres. Assinas a transferência, ela aceita, os dados vão cifrados e só depois de confirmados é que deixas de ser anfitrião.</p>
      <div class="field"><label for="to-who">Novo anfitrião</label><select class="input" id="to-who">${others.map((m) => `<option value="${m.id}">${esc(m.name)} · ${esc(m.key)}</option>`).join("") || "<option disabled>Não há outros membros</option>"}</select></div>
      <div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn primary" id="do-transfer" ${others.length ? "" : "disabled"}>Pedir transferência</button></div>`,
    (el, close) => {
      $("#do-transfer", el).onclick = async () => {
        const who = S.people[$("#to-who", el).value];
        await api.transfer_ownership({ groupId, to: who.id });
        close(); toast(`Pedido enviado a ${who.name}. A transferência termina quando ele aceitar.`);
      };
    });
  }

  function copy(text, field, msg) {
    const done = () => toast(msg);
    try {
      navigator.clipboard.writeText(text).then(done, () => { field && field.select(); toast("Seleciona e copia com Ctrl+C."); });
    } catch { field && field.select(); toast("Seleciona e copia com Ctrl+C."); }
  }

  // ---------------- navegação ----------------
  async function openGroup(id) {
    S.groupId = id; S.view = "chat"; S.navOpen = S.navOpen && window.innerWidth <= 720;
    const g = group(); g.unread = 0;
    S.channels = await api.list_channels({ groupId: id });
    S.allChannels[id] = S.channels;
    S.members = await api.list_members({ groupId: id });
    S.allMembers[id] = S.members;
    const firstText = S.channels.find((c) => c.kind === "text");
    await openChannel(firstText.id, true);
    store.set("group", id);
  }
  async function openChannel(id, keepNav) {
    S.channelId = id; S.view = "chat";
    const c = channel(); if (c) c.unread = 0;
    S.messages = await api.list_messages({ channelId: id });
    if (!keepNav) S.navOpen = false;
    render();
    const inp = $("#msg-input"); if (inp && window.innerWidth > 720) inp.focus();
  }
  async function joinVoice(id) {
    if (S.voice && S.voice.channelId === id) { S.view = "voice"; S.navOpen = false; render(); return; }
    const info = await api.join_voice({ channelId: id });
    S.voice = { channelId: id, groupId: S.groupId, ...info };
    S.channels = await api.list_channels({ groupId: S.groupId });
    S.allChannels[S.groupId] = S.channels;
    S.view = "voice"; S.navOpen = false; render();
  }
  async function leaveVoice() {
    await api.leave_voice();
    const gid = S.voice.groupId; S.voice = null; S.camera = false; S.sharing = null;
    S.allChannels[gid] = await api.list_channels({ groupId: gid });
    if (S.groupId === gid) S.channels = S.allChannels[gid];
    if (S.view === "voice") S.view = "chat";
    render(); toast("Saíste da chamada.");
  }

  function applyTheme() {
    const t = S.settings.theme;
    if (t === "system") document.documentElement.removeAttribute("data-theme");
    else document.documentElement.setAttribute("data-theme", t);
    document.body.style.fontSize = S.settings.fontSize + "px";
  }
  async function patch(p) { S.settings = await api.set_settings({ patch: p }); }
  function pathPatch(path, value) {
    const keys = path.split("."); const out = {}; let o = out;
    keys.forEach((k, i) => { if (i === keys.length - 1) o[k] = value; else o = o[k] = {}; });
    return out;
  }

  // ---------------- eventos da UI ----------------
  document.addEventListener("click", async (e) => {
    const t = e.target.closest("button, a, [data-chan-name]");
    if (!t) return;
    const d = t.dataset;
    if (d.group) return openGroup(d.group);
    if (d.chan) return openChannel(d.chan);
    if (d.chanName) {
      e.preventDefault();
      const c = S.channels.find((x) => x.name === d.chanName);
      if (c) openChannel(c.id);
      return;
    }
    if (d.voice) return joinVoice(d.voice);
    if (d.focus) { S.focus = S.focus === d.focus ? null : d.focus; return render(); }
    if (d.pluginTab) { S.pluginTab = d.pluginTab; S.navOpen = false; return render(); }
    if (d.install) {
      t.disabled = true; t.textContent = "A verificar assinatura…";
      const p = await api.install_plugin({ id: d.install });
      Object.assign(S.plugins.find((x) => x.id === p.id), p);
      render(); return toast(`${p.name} instalado e ativo.`);
    }
    if (d.uninstall) {
      const p = await api.uninstall_plugin({ id: d.uninstall });
      Object.assign(S.plugins.find((x) => x.id === p.id), p);
      render(); return toast(`${p.name} removido.`);
    }
    if (d.react) {
      const [mid, i] = d.react.split(":");
      const m = S.messages.find((x) => String(x.id) === mid); const r = m.reactions[+i];
      r.mine = !r.mine; r.n += r.mine ? 1 : -1; return render();
    }
    if (d.setTab) { S.settingsTab = d.setTab; return render(); }
    if (d.themeSet) { await patch({ theme: d.themeSet }); applyTheme(); return render(); }
    if (d.relay) { await patch({ relay: d.relay }); return render(); }
    if (d.dest) { await patch({ backup: { dest: d.dest } }); return render(); }
    if (d.every) { await patch({ backup: { every: d.every } }); return render(); }
    switch (d.act) {
      case "home": return openGroup(S.groups[0].id);
      case "new-group": return openNewGroup();
      case "plugins": S.view = "plugins"; S.navOpen = window.innerWidth <= 720 ? S.navOpen : false; return render();
      case "settings": S.prevView = S.view === "settings" ? S.prevView : S.view; S.view = "settings"; S.navOpen = false; return render();
      case "settings-voz": S.prevView = S.view; S.settingsTab = "voz"; S.view = "settings"; return render();
      case "close-settings": S.view = S.prevView || "chat"; return render();
      case "mute": S.muted = !S.muted; if (!S.muted && S.deaf) S.deaf = false; api.set_voice_state({ muted: S.muted, deaf: S.deaf }); return render();
      case "deaf": S.deaf = !S.deaf; S.muted = S.deaf ? true : S.muted; api.set_voice_state({ muted: S.muted, deaf: S.deaf }); return render();
      case "leave-voice": return leaveVoice();
      case "camera":
        S.camera = !S.camera; await api.set_camera({ on: S.camera }); render();
        return toast(S.camera ? "Câmara ligada." : "Câmara desligada.");
      case "share":
        if (S.sharing) { await api.stop_screen_share(); S.sharing = null; render(); return toast("Deixaste de partilhar o ecrã."); }
        return openSharePicker();
      case "call-sidebar": S.callSidebar = !S.callSidebar; return render();
      case "call-chat": { const gid = S.voice.groupId; if (S.groupId !== gid) await openGroup(gid); else await openChannel(S.channels.find((x) => x.kind === "text").id); return; }
      case "fullscreen": { const el = $(".callview") || $(".spotlight"); try { el.requestFullscreen().catch(() => {}); } catch {} return; }
      case "invite": return openInvite();
      case "group-menu": return openGroupMenu();
      case "transfer": return openTransfer(d.groupId);
      case "toggle-members":
        if (window.innerWidth <= 1100) S.membersOpen = !S.membersOpen;
        else { S.showMembers = !S.showMembers; store.set("members", S.showMembers); }
        return render();
      case "nav": S.navOpen = !S.navOpen; return render();
      case "copy-key": return copy(S.me.key, null, "Chave copiada.");
      case "attach": return toast("No Tauri abre o seletor de ficheiros. O ficheiro vai direto para os PCs dos membros.");
      case "emoji": { const i = $("#msg-input"); i.value += "🍷"; i.focus(); return; }
      case "connect-dest": {
        const k = S.settings.backup.dest;
        await patch({ backup: { connected: { [k]: k === "local" ? "D:\\Backups\\Tasca" : "conta ligada" } } });
        render(); return toast("Conta ligada. A próxima cópia vai para lá.");
      }
      case "backup-now": {
        S.backupPct = 0; render();
        await api.run_backup();
        S.backupPct = null; S.settings = await api.get_settings(); render();
        return toast("Cópia feita, cifrada e enviada.");
      }
      case "test-relay": return toast("Relay respondeu em 12 ms.");
      case "new-channel": return toast("Criar canal: em breve.");
    }
  });

  // fechar painel de membros ao clicar fora (ecrãs estreitos)
  document.addEventListener("pointerdown", (e) => {
    if (S.membersOpen && !e.target.closest(".members") && !e.target.closest('[data-act="toggle-members"]')) { S.membersOpen = false; render(); }
  });

  document.addEventListener("submit", async (e) => {
    if (e.target.id !== "composer") return;
    e.preventDefault();
    const inp = $("#msg-input");
    const text = inp.value.trim();
    if (!text) return;
    inp.value = "";
    const m = await api.send_message({ channelId: S.channelId, text });
    S.messages.push(m); render(); $("#msg-input").focus();
  });
  document.addEventListener("keydown", (e) => {
    if (e.target.id === "msg-input" && e.key === "Enter" && !e.shiftKey) {
      e.preventDefault(); e.target.form.requestSubmit();
    }
    if (e.key === "Escape") {
      const sc = document.querySelector(".scrim");
      if (sc) sc.remove();
      else if (S.view === "settings") { S.view = S.prevView || "chat"; render(); }
    }
  });
  document.addEventListener("input", async (e) => {
    const t = e.target;
    if (t.id === "msg-input") { t.style.height = "auto"; t.style.height = Math.min(t.scrollHeight, 160) + "px"; return; }
    if (t.id === "plugin-q") {
      S.pluginQuery = t.value; const pos = t.selectionStart; render();
      const n = $("#plugin-q"); n.focus(); n.setSelectionRange(pos, pos); return;
    }
    if (t.dataset.setting && t.type === "range") {
      const v = +t.value;
      await patch(pathPatch(t.dataset.setting, v));
      if (t.dataset.setting === "fontSize") applyTheme();
      const small = t.closest(".row").querySelector(".txt small");
      if (t.dataset.setting === "fontSize") small.textContent = v + " px";
      if (t.dataset.setting === "backup.zstdLevel") small.textContent = `Nível ${v}. Mais alto ocupa menos espaço mas demora mais.`;
    }
  });
  document.addEventListener("change", async (e) => {
    const t = e.target;
    if (t.dataset.pluginToggle) {
      const p = await api.set_plugin_enabled({ id: t.dataset.pluginToggle, enabled: t.checked });
      Object.assign(S.plugins.find((x) => x.id === p.id), p); render(); return;
    }
    if (t.id === "sideload" && t.files[0]) { toast(`«${t.files[0].name}» carregado. Revê as permissões antes de ativar.`); return; }
    if (t.dataset.setting && t.type === "checkbox") return patch(pathPatch(t.dataset.setting, t.checked));
    if (t.dataset.setting && t.tagName === "SELECT") return patch(pathPatch(t.dataset.setting, t.value));
    if (t.dataset.setting && t.type !== "range") return patch(pathPatch(t.dataset.setting, t.value));
  });

  // tooltips da barra de grupos
  let tipEl;
  document.addEventListener("pointerover", (e) => {
    const b = e.target.closest("[data-tip]");
    if (!b) { if (tipEl) { tipEl.remove(); tipEl = null; } return; }
    if (!tipEl) { tipEl = document.createElement("div"); tipEl.className = "tip"; document.body.appendChild(tipEl); }
    const r = b.getBoundingClientRect();
    tipEl.textContent = b.dataset.tip;
    tipEl.style.left = r.right + 12 + "px"; tipEl.style.top = r.top + r.height / 2 - 15 + "px";
  });

  // ---------------- eventos do núcleo ----------------
  api.on("message", ({ channelId, message }) => {
    if (S.typing && S.typing.channelId === channelId) S.typing = null;
    if (channelId === S.channelId && S.view === "chat") { S.messages.push(message); render(); }
  });
  api.on("message_ack", ({ channelId, id }) => {
    const m = S.messages.find((x) => x.id === id);
    if (m && channelId === S.channelId) { m.pending = false; if (S.view === "chat") rerenderKeepInput(); }
  });
  api.on("typing", ({ channelId, who }) => { S.typing = { channelId, who }; if (S.view === "chat") rerenderKeepInput(); });
  api.on("voice_activity", ({ speaking }) => {
    S.speaking = speaking;
    if (S.voice || S.view === "chat") {
      if (S.view === "voice" && !document.fullscreenElement) { $("#main").innerHTML = renderVoice(); fitCallGrid(); }
      if (S.view !== "settings" && S.view !== "plugins") {
        const ch = $(".channels"); const st = ch ? ch.scrollTop : 0;
        $("#sidebar").innerHTML = renderSidebar();
        const ch2 = $(".channels"); if (ch2) ch2.scrollTop = st;
      }
    }
  });
  api.on("backup_progress", ({ pct }) => { S.backupPct = pct; if (S.view === "settings" && S.settingsTab === "backups") rerenderKeepInput(); });

  function rerenderKeepInput() {
    const inp = $("#msg-input");
    const val = inp ? inp.value : "", focused = document.activeElement === inp;
    const pg = $(".page"); const st = pg ? pg.scrollTop : 0;
    render();
    const n = $("#msg-input"); if (n) { n.value = val; if (focused) n.focus(); }
    const pg2 = $(".page"); if (pg2) pg2.scrollTop = st;
  }

  // ---------------- arranque ----------------
  (async function start() {
    S.me = await api.get_me();
    S.people = await api.list_people();
    S.groups = await api.list_groups();
    S.plugins = await api.list_plugins();
    S.settings = await api.get_settings();
    S.allChannels = {}; S.allMembers = {};
    for (const g of S.groups) {
      S.allChannels[g.id] = await api.list_channels({ groupId: g.id });
      S.allMembers[g.id] = await api.list_members({ groupId: g.id });
    }
    applyTheme();
    const saved = store.get("group", null);
    await openGroup(S.groups.some((g) => g.id === saved) ? saved : S.groups[0].id);
  })();
})();
