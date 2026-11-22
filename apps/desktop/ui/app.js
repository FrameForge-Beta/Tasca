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
    wave: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M2 12h2M6 8v8M10 5v14M14 9v6M18 7v10M22 12h-2"/></svg>',
    phone: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 16.9v3a2 2 0 0 1-2.2 2A19.8 19.8 0 0 1 3.1 4.2 2 2 0 0 1 5.1 2h3a2 2 0 0 1 2 1.7c.1.9.4 1.8.7 2.7a2 2 0 0 1-.4 2.1L9.1 9.8a16 16 0 0 0 6 6l1.3-1.3a2 2 0 0 1 2.1-.4c.9.3 1.8.6 2.7.7a2 2 0 0 1 1.7 2z"/></svg>',
    cam: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m16 13 5.2 3.5a.5.5 0 0 0 .8-.4V7.9a.5.5 0 0 0-.8-.4L16 11"/><rect x="2" y="6" width="14" height="12" rx="2"/></svg>',
    camOff: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.7 6H14a2 2 0 0 1 2 2v3.3l1 1 4.2-3.5a.5.5 0 0 1 .8.4v8.2M16 16a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h2M2 2l20 20"/></svg>',
    screen: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4M12 13V7M9 10l3-3 3 3"/></svg>',
    screenOff: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4M9 7l6 6M15 7l-6 6"/></svg>',
    shrink: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7"/></svg>',
    swap: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 3l4 4-4 4M20 7H8M8 21l-4-4 4-4M4 17h12"/></svg>',
    logout: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9"/></svg>',
    retry: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-3-6.7"/><path d="M21 3v6h-6"/></svg>',
    expand: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>',
    signal: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M4 20v-3M9 20v-7M14 20V9M19 20V4"/></svg>',
  };
  const LOGO = '<svg viewBox="0 0 512 512" aria-hidden="true"><rect width="512" height="512" rx="116" fill="#9B2C3F"/><path fill="#F6EEE1" d="M168,96 H344 A72,72 0 0 1 416,168 V300 A72,72 0 0 1 344,372 H226 L138,430 L152,364 A72,72 0 0 1 96,300 V168 A72,72 0 0 1 168,96Z"/><g transform="translate(256,234)"><path fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(90)" fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(180)" fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(270)" fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><circle r="16" fill="#E3A72F"/><circle r="7" fill="#F6EEE1"/></g></svg>';
  const TILE = "url(\"data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 120"><g transform="translate(60,60)" fill="#2A5CAA"><path d="M0,0 C24,-17.6 24,-39.3 0,-50.4 C-24,-39.3 -24,-17.6 0,0Z"/><path transform="rotate(90)" d="M0,0 C24,-17.6 24,-39.3 0,-50.4 C-24,-39.3 -24,-17.6 0,0Z"/><path transform="rotate(180)" d="M0,0 C24,-17.6 24,-39.3 0,-50.4 C-24,-39.3 -24,-17.6 0,0Z"/><path transform="rotate(270)" d="M0,0 C24,-17.6 24,-39.3 0,-50.4 C-24,-39.3 -24,-17.6 0,0Z"/><path transform="translate(-60,-60)" d="M0,0 A26.4,26.4 0 0 1 26.4,0 A26.4,26.4 0 0 1 0,26.4Z"/><path transform="translate(60,-60) rotate(90)" d="M0,0 A26.4,26.4 0 0 1 26.4,0 A26.4,26.4 0 0 1 0,26.4Z"/><path transform="translate(60,60) rotate(180)" d="M0,0 A26.4,26.4 0 0 1 26.4,0 A26.4,26.4 0 0 1 0,26.4Z"/><path transform="translate(-60,60) rotate(270)" d="M0,0 A26.4,26.4 0 0 1 26.4,0 A26.4,26.4 0 0 1 0,26.4Z"/><circle r="7.2" fill="#E3A72F"/></g></svg>') + "\")";
  document.documentElement.style.setProperty("--azulejo-tile", TILE);
  // flor "fantasma": só o contorno, fina e esbatida, para fundos de cartões
  const petal = "M0,0 C24,-17.6 24,-39.3 0,-50.4 C-24,-39.3 -24,-17.6 0,0Z";
  const GHOST = "url(\"data:image/svg+xml," + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="-60 -60 120 120"><g fill="none" stroke="#F6EEE1" stroke-opacity=".16" stroke-width="1.6">${[0, 90, 180, 270].map((r) => `<path transform="rotate(${r})" d="${petal}"/>`).join("")}<circle r="7" /></g></svg>`) + "\")";
  document.documentElement.style.setProperty("--flor-fantasma", GHOST);

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
  // Estado: a flor da Tasca em miniatura, com as pétalas da cor do estado.
  const PETAL = "M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z";
  const FLOR = `<svg viewBox="-100 -100 200 200" aria-hidden="true"><g class="petals">${[0, 90, 180, 270].map((r) => `<path transform="rotate(${r})" d="${PETAL}"/>`).join("")}</g><circle class="miolo" r="20"/></svg>`;
  function avatar(p, cls = "", withStatus = false) {
    return `<span class="av ${cls} ${p.photo ? "has-photo" : ""}" style="background:${p.color}" aria-hidden="true">${p.photo ? `<img src="${esc(p.photo)}" alt="">` : esc(initials(p.name))}${
      withStatus ? `<i class="st ${p.status}">${FLOR}</i>` : ""}</span>`;
  }
  const shortKey = (k) => (k && k.length > 22 ? k.slice(0, 14) + "…" + k.slice(-6) : k || "");
  function toast(text) {
    const t = document.createElement("div");
    t.className = "toast"; t.textContent = text;
    $("#toasts").appendChild(t);
    setTimeout(() => { t.classList.add("out"); setTimeout(() => t.remove(), 220); }, 3200);
  }
  // vibração curta ao tocar (só no telemóvel)
  function haptic(ms = 8) { if (window.TASCA_MOBILE && navigator.vibrate) { try { navigator.vibrate(ms); } catch {} } }
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
    callSidebar: false,
    myStatus: "online", customStatus: "", statusOpen: false,
    home: false,               // true = mensagens privadas (botão da Tasca)
    dms: [], dmQuery: "",        // mostrar grupos e canais durante a chamada
    showMembers: store.get("members", true),
    pluginTab: "loja", pluginQuery: "",
    settingsTab: "aparencia",
    typing: null, backupPct: null,
  };
  const group = () => S.groups.find((g) => g.id === S.groupId);
  // pessoa por id, mesmo que ainda não se saiba nada dela
  const who = (id) => S.people[id] || { id, name: "Alguém", color: "#4A4542", status: "offline" };
  const channel = () => S.channels.find((c) => c.id === S.channelId);

  // ---------------- permissões ----------------
  // Escolhidas ao criar a conta e em Definições > Privacidade e permissões.
  // Desligar uma permissão desliga mesmo a funcionalidade.
  const PERM_DEF = { mic: true, camera: true, notifications: true, autostart: false };
  const perm = (k) => { const p = (S.settings && S.settings.perms) || {}; return p[k] ?? PERM_DEF[k]; };
  const micAllowed = () => perm("mic");
  const camAllowed = () => perm("camera");

  // ---------------- câmara e partilha de ecrã (a sério) ----------------
  // O teu vídeo aparece na chamada e vai para os outros PCs pelo anfitrião (voz.js).
  const media = { cam: null, screen: null };
  function attachMedia() {
    document.querySelectorAll("video[data-stream]").forEach((v) => {
      const st = media[v.dataset.stream];
      if (st && v.srcObject !== st) { v.srcObject = st; v.play().catch(() => {}); }
    });
    // vídeo dos outros: o mesmo <canvas> passa para o quadrado novo a cada render
    if (window.vozAudio) document.querySelectorAll("[data-remote]").forEach((slot) => {
      const c = vozAudio.video.canvas(slot.dataset.remote);
      if (c.parentNode !== slot) slot.appendChild(c);
    });
  }
  // Manda a câmara e o ecrã aos outros da chamada, e diz-lhes o que está ligado.
  function syncVideo() {
    if (api.isMock || !S.voice || !window.vozAudio) return;
    const v = vozAudio.video;
    if (media.cam) v.publish("cam", media.cam, { height: 720, fps: 30, bitrate: 1_500_000 }); else v.unpublish("cam");
    if (media.screen && S.sharing) {
      const q = S.sharing.quality || "";
      const [h, fps] = q.startsWith("720") ? [720, 30] : q.includes("60") ? [1080, 60] : [1080, 30];
      v.publish("screen", media.screen, { height: h, fps, bitrate: h === 720 ? 2_500_000 : fps === 60 ? 6_000_000 : 4_000_000 });
    } else v.unpublish("screen");
    api.set_video_state({ camera: !!media.cam, screen: !!media.screen }).catch(() => {});
  }
  // Estado de voz de alguém na chamada onde estou (micro, som, câmara, ecrã).
  const vs = (id) => (id === "me" ? { muted: S.muted, deaf: S.deaf, camera: S.camera, screen: !!S.sharing } : (S.voiceStates && S.voiceStates[id]) || {});
  const voiceIc = (id) => { const v = vs(id); return v.deaf ? I.headOff : v.muted ? I.micOff : ""; };
  function mediaError(x, what) {
    const n = x && x.name;
    if (n === "NotAllowedError") return `O Windows não deixou a Tasca usar ${what}. Vê Definições do Windows > Privacidade e segurança.`;
    if (n === "NotFoundError" || n === "OverconstrainedError") return what === "a câmara" ? "Não encontrei nenhuma câmara ligada." : `Não encontrei ${what}.`;
    if (n === "NotReadableError") return `${what[0].toUpperCase() + what.slice(1)} está a ser usada por outra app.`;
    return `Não consegui usar ${what}: ${x && x.message || x}`;
  }
  async function startCamera() {
    if (!camAllowed()) { toast("A câmara está desligada em Definições > Privacidade e permissões."); return false; }
    if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) { toast("Este sistema não deixa usar a câmara."); return false; }
    const dev = S.settings.camDevice;
    try {
      media.cam = await navigator.mediaDevices.getUserMedia({ video: {
        ...(dev && dev !== "default" ? { deviceId: { exact: dev } } : {}), width: { ideal: 1280 }, height: { ideal: 720 },
      } });
    } catch (x) { toast(mediaError(x, "a câmara")); return false; }
    const st = media.cam;
    st.getVideoTracks()[0].addEventListener("ended", () => { if (media.cam === st) { stopCamera(); render(); } });
    return true;
  }
  function stopCamera() {
    if (media.cam) media.cam.getTracks().forEach((t) => t.stop());
    media.cam = null; S.camera = false;
    syncVideo();
  }
  function stopShare(quiet) {
    if (media.screen) media.screen.getTracks().forEach((t) => t.stop());
    media.screen = null;
    if (!S.sharing) return;
    S.sharing = null; sfx.shareStop(); api.stop_screen_share().catch(() => {});
    syncVideo();
    if (!quiet) toast("Deixaste de partilhar o ecrã.");
  }

  // ---------------- render ----------------
  function render() {
    const app = $("#app");
    app.className = "app" + (S.showMembers && S.view === "chat" && !S.home ? "" : " no-members") +
      (S.navOpen ? " show-nav" : "") + (S.membersOpen && S.view === "chat" && !S.home ? " show-members" : "") +
      (S.view === "voice" && !S.callSidebar ? " call" : "") + (S.view === "settings" ? " no-sidebar" : "");
    $("#rail").innerHTML = renderRail();
    const tbp = document.getElementById("tb-plugins"); if (tbp) tbp.classList.toggle("on", S.view === "settings" && S.settingsTab === "plugins");
    document.body.classList.add("ready");
    $("#sidebar").innerHTML = S.view === "settings" ? "" : renderSidebar();
    $("#sidebar").hidden = S.view === "settings";
    if (S.view === "settings") app.style.gridTemplateColumns = window.innerWidth > 720 ? "72px minmax(0,1fr)" : "";
    else app.style.gridTemplateColumns = "";
    $("#main").innerHTML = renderMain();
    pageTransition();
    fitCallGrid();
    $("#members").innerHTML = S.view === "chat" && !S.home ? renderMembers() : "";
    const list = $(".messages");
    if (list) list.scrollTop = list.scrollHeight;
    if (S.view === "settings" && S.settingsTab === "voz") { if (!meter && DEV.loaded) startMicMeter(); }
    else stopMicMeter();
    attachMedia();
  }

  // Troca de página (canal, conversa, amigos, definições, chamada): a página nova entra a deslizar.
  let lastPage = null, pageTimer = 0;
  const PAGE_DEPTH = { friends: 0, chat: 1, voicepre: 2, voice: 3, settings: 4 };
  function pageTransition() {
    const key = [S.view, S.home, S.groupId, S.channelId, S.view === "settings" ? S.settingsTab : ""].join("|");
    if (lastPage && key !== lastPage) {
      const main = $("#main");
      const prevView = lastPage.split("|")[0];
      main.classList.remove("enter", "back"); void main.offsetWidth;
      main.classList.add("enter");
      if ((PAGE_DEPTH[S.view] ?? 1) < (PAGE_DEPTH[prevView] ?? 1)) main.classList.add("back");
      clearTimeout(pageTimer); pageTimer = setTimeout(() => main.classList.remove("enter", "back"), 320);
    }
    lastPage = key;
  }

  function renderRail() {
    const pend = window.tascaAmigos ? window.tascaAmigos.pendingCount() : 0;
    const dmUnread = S.dms.reduce((a, d) => a + (d.unread || 0), 0) + pend;
    const gs = S.groups.map((g) => {
      const on = !S.home && S.groupId === g.id && (S.view === "chat" || S.view === "voice");
      const dot = `<i class="host-dot ${g.host.online ? "online" : ""}"></i>`;
      const badge = g.unread && !on ? `<span class="badge">${g.unread}</span>` : "";
      return `<button class="rail-btn ${on ? "on" : ""} ${g.unread ? "unread" : ""}" data-group="${g.id}" data-tip="${esc(g.name)}${g.host.online ? "" : " (anfitrião offline)"}" aria-label="${esc(g.name)}" style="--g:${g.color || "var(--vinho)"}">${esc(g.short)}${badge || (g.owner === "me" ? "" : dot)}</button>`;
    }).join("");
    return `
      <button class="rail-btn logo ${S.home && (S.view === "chat" || S.view === "friends") ? "on" : ""}" data-act="home" data-tip="Amigos e mensagens privadas" aria-label="Amigos e mensagens privadas">${LOGO}${dmUnread && !(S.home && !pend) ? `<span class="badge">${dmUnread}</span>` : ""}</button>
      <div class="rail-sep"></div>
      ${gs}
      <button class="rail-btn add" data-act="new-group" data-tip="Criar ou entrar num grupo" aria-label="Criar ou entrar num grupo">${I.plus}</button>`;
  }

  function hostCard(g) {
    const owner = who(g.owner);
    // grupo teu alojado no PC, visto do telemóvel
    if (g.owner === "me" && g.host.route !== "este PC") return g.host.online
      ? `<div class="host-card online"><i class="led"></i><b>Alojado no teu PC</b><span>Ligação ${esc(g.host.route)}</span></div>`
      : `<div class="host-card offline"><i class="led"></i><b>O teu PC está desligado</b><span>A ver cópia local. Abre a Tasca no PC.</span></div>`;
    if (g.owner === "me") return `<div class="host-card online"><i class="led"></i><b>Alojado neste PC</b><span>És o anfitrião. Mantém a Tasca aberta.</span></div>`;
    if (!g.host.online) return `<div class="host-card offline"><i class="led"></i><b>${esc(owner.name)} está offline</b><span>Visto ${esc(g.host.lastSeen)}. A ver cópia local.</span></div>`;
    return `<div class="host-card online"><i class="led"></i><b>Alojado no PC de ${esc(owner.name)}</b><span>Ligação ${esc(g.host.route)}${g.host.ping != null ? ` · ${g.host.ping} ms` : ""}</span></div>`;
  }

  function renderSidebar() {
    if (S.home) return renderDmSidebar();
    const g = group();
    if (!g) return "";
    const cats = [...(S.cats[g.id] || [])];
    for (const c of S.channels) { if (!cats.includes(c.cat)) cats.push(c.cat); }
    const own = g.owner === "me";
    const editBtn = (c) => own ? `<button class="chan-edit" data-edit-chan="${c.id}" aria-label="Editar ${esc(c.name)}" title="Editar canal">${I.gear}</button>` : "";
    const drag = (attr) => (own ? `draggable="true" ${attr}` : "");
    const body = cats.map((cat) => `
      <div class="cat" ${drag(`data-drag-cat="${esc(cat)}"`)}><span>${esc(cat)}</span>${own ? `<button class="icon-btn cat-edit" style="width:22px;height:22px" data-edit-cat="${esc(cat)}" aria-label="Editar categoria ${esc(cat)}" title="Editar categoria">${I.gear}</button><button class="icon-btn" style="width:22px;height:22px" data-act="new-channel" data-cat="${esc(cat)}" aria-label="Criar canal em ${esc(cat)}" title="Criar canal">${I.plus}</button>` : ""}</div>
      ${S.channels.filter((c) => c.cat === cat).map((c) => {
        if (c.kind === "text") {
          const on = c.id === S.channelId && S.view === "chat";
          return `<div class="chan-wrap ${isMuted(c.id) ? "muted" : ""}" ${drag(`data-drag-chan="${c.id}"`)}><button class="chan ${on ? "on" : ""} ${c.unread && !on && !isMuted(c.id) ? "unread" : ""}" data-chan="${c.id}">${I.hash}<span class="label">${esc(c.name)}</span>${c.unread && !on && !isMuted(c.id) ? `<span class="pill">${c.unread}</span>` : ""}</button>${editBtn(c)}</div>`;
        }
        const on = S.voice && S.voice.channelId === c.id;
        const ppl = (c.inVoice || []).map((id) => S.people[id]);
        return `<div class="chan-wrap ${isMuted(c.id) ? "muted" : ""}" ${drag(`data-drag-chan="${c.id}"`)}><button class="chan ${on && S.view === "voice" ? "on" : ""}" data-voice="${c.id}">${I.speaker}<span class="label">${esc(c.name)}</span>${ppl.length ? `<span class="pill" style="background:var(--caixa);color:var(--texto-2)">${ppl.length}</span>` : ""}</button>${editBtn(c)}</div>
          ${ppl.length ? `<ul class="voice-people">${ppl.map((p) => `<li data-spk="${p.id}" class="${S.speaking.includes(p.id) && !(p.id === "me" && S.muted) ? "speaking" : ""}">${avatar(p, "")}${esc(p.name)}${(p.id === "me" || (S.voice && S.voice.channelId === c.id)) && voiceIc(p.id) ? `<span class="muted-ic">${voiceIc(p.id)}</span>` : ""}</li>`).join("")}</ul>` : ""}`;
      }).join("")}`).join("");
    return `
      <div class="group-head"><span class="name">${esc(g.name)}</span>
        <button class="icon-btn" data-act="invite" aria-label="Convidar pessoas" title="Convidar pessoas">${I.invite}</button>
        <button class="icon-btn" data-act="group-menu" aria-label="Opções do grupo" title="Opções do grupo">${I.chevron}</button></div>
      ${hostCard(g)}
      <div class="channels">${body || (g.owner === "me" ? `<button class="btn ghost" style="margin:8px" data-act="new-channel">${I.plus}Criar canal</button>` : "")}</div>
      ${renderMe()}`;
  }

  function renderDmSidebar() {
    const q = S.dmQuery.toLowerCase();
    const list = S.dms.filter((d) => !q || who(d.with).name.toLowerCase().includes(q));
    const rows = list.map((d) => {
      const p = who(d.with);
      const on = d.id === S.channelId && S.view === "chat";
      const last = d.last ? (d.last.from === "me" ? "Tu: " : "") + d.last.text : "";
      return `<button class="dm ${on ? "on" : ""} ${d.unread && !on ? "unread" : ""}" data-dm="${d.id}">${avatar(p, "s", true)}
        <span class="dm-txt"><b>${esc(p.name)}</b><small>${esc(last)}</small></span>
        <span class="dm-side"><small>${d.last ? esc(d.last.day === "Hoje" ? d.last.at : d.last.day) : ""}</small>${d.unread && !on ? `<span class="pill">${d.unread}</span>` : ""}</span></button>`;
    }).join("");
    return `
      <div class="group-head"><span class="name">Mensagens privadas</span>
        <button class="icon-btn" data-act="new-dm" aria-label="Nova conversa" title="Nova conversa">${I.plus}</button></div>
      <div class="dm-search search"><label class="sr" for="dm-q">Procurar conversa</label>${I.search}<input id="dm-q" placeholder="Procurar pessoa" value="${esc(S.dmQuery)}"></div>
      ${window.tascaAmigos ? window.tascaAmigos.sidebarButton() : ""}
      <div class="channels dms">${rows || `<p class="empty">${q ? "Ninguém com esse nome." : "Ainda não tens conversas."}</p>`}</div>
      ${renderMe()}`;
  }

  // Balcão: o painel de baixo da barra lateral. Mostra a chamada (se estiveres numa),
  // o que estás a partilhar e o teu perfil com estado.
  const STATUS = [
    ["online", "Online", "Toda a gente vê que estás cá."],
    ["away", "Ausente", "Fica assim sozinho depois de 10 min parado."],
    ["dnd", "Não incomodar", "Sem notificações nem sons de mensagens."],
    ["offline", "Invisível", "Pareces offline, mas continuas a ver tudo."],
  ];
  function renderVoiceBar() {
    if (!S.voice) return "";
    const c = findChannel(S.voice.channelId);
    const g = S.groups.find((x) => x.id === S.voice.groupId);
    const q = S.voice.ping < 60 ? 3 : S.voice.ping < 120 ? 2 : 1;
    const bars = [1, 2, 3].map((i) => `<i class="${i <= q ? "on" : ""}" style="height:${4 + i * 3}px"></i>`).join("");
    const share = S.sharing ? `
      <div class="dock-share">
        <button class="dock-thumb" data-voice="${c.id}" aria-label="Ver a tua partilha">${fakeScreen("screen", "me")}</button>
        <div class="dock-share-txt"><b>A partilhar</b><small>${esc(S.sharing.source)} · ${esc(S.sharing.quality.replace(" · ", " "))}</small></div>
        <button class="icon-btn danger" data-act="share" aria-label="Parar partilha" title="Parar partilha">${I.screenOff}</button>
      </div>` : "";
    const tgl = (act, on, icon, offIcon, label, cls = "") =>
      `<button class="dock-tgl ${on ? "on" : ""} ${cls}" data-act="${act}" aria-pressed="${on}" aria-label="${label}" title="${label}">${on ? icon : offIcon}</button>`;
    return `<div class="dock-call" style="--g:${g.color || "var(--vinho)"}">
      <button class="dock-where" data-voice="${c.id}" title="Abrir a chamada">
        <span class="sig" aria-label="Qualidade da ligação">${bars}</span>
        <span class="dock-where-txt"><b>Em chamada</b><small>${esc(g.name)} · ${esc(c.name)} · ${S.voice.ping} ms</small></span>
      </button>
      ${share}
      <div class="dock-tgls">
        ${tgl("camera", S.camera, I.cam, I.camOff, S.camera ? "Desligar câmara" : "Ligar câmara")}
        ${tgl("share", !!S.sharing, I.screen, I.screen, S.sharing ? "Parar partilha" : "Partilhar ecrã")}
        ${tgl("noise", S.settings.noiseSuppression, I.wave, I.wave, S.settings.noiseSuppression ? "Desligar supressão de ruído" : "Ligar supressão de ruído")}
        <button class="dock-tgl leave" data-act="leave-voice" aria-label="Sair da chamada" title="Sair da chamada">${I.phoneOff}</button>
      </div>
    </div>`;
  }

  // Botão de ecrã inteiro: muda para "sair" enquanto a chamada está em ecrã inteiro.
  const fsBtn = () => document.fullscreenElement
    ? `<button class="icon-btn" data-act="fullscreen" aria-label="Sair do ecrã inteiro" title="Sair do ecrã inteiro (Esc)">${I.shrink}</button>`
    : `<button class="icon-btn" data-act="fullscreen" aria-label="Ecrã inteiro" title="Ecrã inteiro">${I.expand}</button>`;
  document.addEventListener("fullscreenchange", () => {
    document.querySelectorAll('[data-act="fullscreen"]').forEach((b) => { b.outerHTML = fsBtn(); });
    fitCallGrid();
  });

  // Ausente sozinho: só quando escolheste "Online" e não mexes no PC há 10 minutos.
  // Se escolheres Ausente, Não incomodar ou Invisível, fica como escolheste.
  const IDLE_AWAY = 10 * 60;
  const myStatus = () => (S.myStatus === "online" && S.autoAway ? "away" : S.myStatus || "online");
  let lastInput = Date.now();
  for (const ev of ["pointermove", "pointerdown", "keydown", "wheel"]) window.addEventListener(ev, () => { lastInput = Date.now(); }, { passive: true });
  async function idleSeconds() {
    try { const n = await api.idle_seconds(); if (typeof n === "number") return n; } catch {}
    return (Date.now() - lastInput) / 1000; // sem o sistema: conta só a atividade nesta janela
  }
  setInterval(async () => {
    if (!S.me) return;
    const away = S.myStatus === "online" && (await idleSeconds()) >= IDLE_AWAY;
    if (away !== !!S.autoAway) {
      S.autoAway = away;
      api.set_presence({ status: myStatus(), text: S.customStatus || "" }).catch(() => {});
      if (!document.querySelector("#custom-status:focus")) render();
    }
  }, 15000);

  function renderMe() {
    const me = S.me;
    const st = myStatus();
    const label = STATUS.find((x) => x[0] === st)[1];
    const speaking = S.voice && S.speaking.includes("me") && !S.muted;
    const pop = S.statusOpen ? `<div class="status-pop" role="menu">
        <div class="status-pop-head">${avatar(me)}<div><b>${esc(me.name)}</b><small>${me.username ? "@" + esc(me.username) : "sem ID: escolhe um em Editar perfil"}</small></div></div>
        <label class="status-custom" for="custom-status"><span>Mensagem de estado</span><input id="custom-status" data-sugg="estado" maxlength="60" placeholder="Ex.: a jogar Mario Kart" value="${esc(S.customStatus || "")}"></label>
        ${STATUS.map(([k, l, d]) => `<button role="menuitemradio" aria-checked="${st === k}" class="${st === k ? "on" : ""}" data-status="${k}"><i class="st-dot ${k}">${FLOR}</i><span><b>${l}</b><small>${d}</small></span></button>`).join("")}
        <button data-act="${me.username ? "copy-key" : "edit-profile"}" class="status-copy">${I.lock}<span><b>${me.username ? "Copiar o meu ID (@" + esc(me.username) + ")" : "Escolher o meu ID"}</b><small>É com ele que te adicionam como amigo.</small></span></button>
        <button data-act="my-card" class="status-copy">${I.users}<span><b>Ver o meu perfil</b><small>Como os outros te veem.</small></span></button>
        <div class="status-acct">
          <button data-act="switch-account">${I.swap}<span><b>Trocar de conta</b></span></button>
          <button data-act="sign-out" class="danger">${I.logout}<span><b>Terminar sessão</b></span></button>
        </div>
      </div>` : "";
    const vg = S.voice ? S.groups.find((x) => x.id === S.voice.groupId) : null;
    return `<div class="dock ${S.voice ? "in-call" : ""}" ${vg ? `style="--g:${vg.color || "var(--vinho)"}"` : ""}>${renderVoiceBar()}<div class="me">
      <button class="me-who" data-act="status" aria-expanded="${!!S.statusOpen}" aria-label="O teu estado: ${label}">
        <span class="me-av ${speaking ? "speaking" : ""}" data-spk="me">${avatar({ ...me, status: st }, "s", true)}</span>
        <span class="who"><b>${esc(me.name)}</b><small>${esc(S.customStatus || label)}</small></span>
      </button>
      <span class="me-btns">
        <button class="icon-btn ${S.muted ? "danger" : ""}" data-act="mute" aria-pressed="${S.muted}" aria-label="${S.muted ? "Ligar microfone" : "Desligar microfone"}" title="${S.muted ? "Ligar microfone" : "Desligar microfone"}">${S.muted ? I.micOff : I.mic}<i class="lvl ${speaking ? "on" : ""}" data-spk="me" data-spk-cls="on"></i></button>
        <button class="icon-btn ${S.deaf ? "danger" : ""}" data-act="deaf" aria-pressed="${S.deaf}" aria-label="${S.deaf ? "Ligar som" : "Desligar som"}" title="${S.deaf ? "Ligar som" : "Desligar som"}">${S.deaf ? I.headOff : I.head}</button>
        <button class="icon-btn" data-act="settings" aria-label="Definições" title="Definições">${I.gear}</button>
      </span>
    </div>${pop}</div>`;
  }

  function renderMain() {
    if (S.view === "settings") return renderSettings();
    if (S.view === "voice") return renderVoice();
    if (S.view === "voicepre") return renderVoicePre();
    if (S.view === "friends" && window.tascaAmigos) return window.tascaAmigos.renderFriends();
    return renderChat();
  }

  const narrow = () => window.innerWidth <= 720;
  // telemóvel: com a chamada a decorrer, uma barra por cima da caixa de escrita leva de volta
  const callPill = () => {
    if (!S.voice || !narrow()) return "";
    const c = findChannel(S.voice.channelId); if (!c) return "";
    const n = (c.inVoice || []).length;
    return `<button class="call-pill" data-voice="${esc(c.id)}"><span class="cp-dot"></span><span class="cp-txt"><b>Em chamada</b><small>${esc(c.name)}${n ? " · " + n + (n === 1 ? " pessoa" : " pessoas") : ""}</small></span><span class="cp-mic ${S.muted ? "off" : ""}" data-act="mute" role="button" aria-label="${S.muted ? "Ligar microfone" : "Desligar microfone"}">${S.muted ? I.micOff : I.mic}</span><span class="cp-go">Voltar</span></button>`;
  };
  const menuBtn = () => `<button class="icon-btn menu-btn" data-act="nav" aria-label="Grupos e canais">${I.menu}</button>`;

  // Sem grupos nem conversas (conta nova).
  function renderEmpty() {
    return `<div class="main-head">${menuBtn()}<span class="title">Olá, ${esc(S.me.name)}</span></div>
      <div class="empty-home">
        <span class="empty-logo">${LOGO}</span>
        <h2>Ainda não estás em nenhum grupo</h2>
        <p>Cria um grupo para a tua malta, ou entra num com o convite que alguém te mandou.</p>
        <button class="btn primary" data-act="new-group">Criar ou entrar num grupo</button>
      </div>`;
  }

  let seen = new Set(), seenCh = null;
  function renderChat() {
    const dm = S.home ? S.dms.find((d) => d.id === S.channelId) : null;
    const c = dm ? { id: dm.id, name: who(dm.with).name } : channel();
    const g = dm ? { owner: null, host: { online: true }, name: "" } : group();
    if (!c) return renderEmpty();
    let lastDay = null, lastFrom = null, lastAt = null;
    const rows = S.messages.map((m) => {
      const p = who(m.from);
      let out = "";
      if (m.day !== lastDay) { out += `<div class="day">${esc(m.day)}</div>`; lastDay = m.day; lastFrom = null; }
      const first = m.from !== lastFrom || m.file || minutes(m.at) - minutes(lastAt) > 7;
      lastFrom = m.from; lastAt = m.at;
      const reacts = m.reactions ? `<div class="reactions">${m.reactions.map((r, i) => `<button class="reaction ${r.mine ? "mine" : ""}" data-react="${m.id}:${i}">${r.e} ${r.n}</button>`).join("")}</div>` : "";
      const file = m.file ? `<div class="attach"><span class="ext">${esc(m.file.ext)}</span><div><b>${esc(m.file.name)}</b><small>${esc(m.file.size)} · enviado diretamente por ${esc(p.name)}</small></div></div>` : "";
      const state = ""; // a enviar: a mensagem só fica mais escura (classe "pending")
      const mine = m.from === "me";
      const fresh = seenCh === S.channelId && !seen.has(String(m.id));
      const cls = `msg ${mine ? "mine" : ""} ${first ? "first" : ""} ${m.mentionsMe ? "mentioned" : ""} ${m.pending ? "pending" : ""} ${m.failed ? "failed" : ""} ${fresh ? "new" : ""}`;
      const fail = m.failed ? `<div class="msg-fail"><small>Não foi enviada. Volta a tentar.</small><button class="retry" data-retry="${esc(m.id)}" aria-label="Tentar enviar outra vez" title="Tentar enviar outra vez">${I.retry}</button></div>` : "";
      if (first) {
        const tag = p.bot ? '<span class="tag bot">plugin</span>' : g.owner === p.id ? '<span class="tag">anfitrião</span>' : "";
        out += `<div class="${cls}" data-mid="${esc(m.id)}"><button type="button" class="msg-av" data-person="${esc(p.id)}" aria-label="Perfil de ${esc(p.name)}">${avatar(p)}</button><div><div class="bubble"><div class="meta"><button type="button" class="author" data-person="${esc(p.id)}" style="--a:${p.color}">${esc(p.name)}</button>${tag}<span class="time">${esc(m.at)}</span></div><div class="body">${fmt(m.text)}${state}</div>${file}</div>${reacts}${fail}</div></div>`;
      } else {
        out += `<div class="${cls}" data-mid="${esc(m.id)}"><span class="gutter">${esc(m.at)}</span><div><div class="bubble"><div class="body">${fmt(m.text)}${state}</div>${file}</div>${reacts}${fail}</div></div>`;
      }
      return out;
    }).join("");
    // as que já estavam no ecrã não voltam a animar
    seen = new Set(S.messages.map((m) => String(m.id))); seenCh = S.channelId;
    if (dm) {
      const p = who(dm.with);
      const st = { online: "online", away: "ausente", dnd: "não incomodar", offline: "offline" }[p.status] || "offline";
      return `
      <div class="main-head">${menuBtn()}
        <button type="button" class="title dm-who" data-person="${esc(p.id || dm.with)}">${avatar(p, "s", true)}${esc(p.name)}</button>
        <span class="topic">${p.statusText ? esc(p.statusText) : st}${p.username ? " · @" + esc(p.username) : ""}</span>
        <button class="icon-btn" data-act="dm-call" aria-label="Ligar a ${esc(p.name)}" title="Chamada de voz">${I.phone}</button>
        <button class="icon-btn" data-act="dm-video" aria-label="Videochamada com ${esc(p.name)}" title="Videochamada">${I.cam}</button>
      </div>
      <div class="messages ${S.settings.msgStyle === "list" ? "" : "bubbles"}" role="log" aria-live="polite">
        <div class="spacer"></div>
        <div class="welcome">${avatar(p).replace('class="av ', 'class="av big ')}<h2>${esc(p.name)}</h2><p>Esta conversa é só entre ti e ${esc(p.name)}, cifrada de ponta a ponta: o servidor da Tasca guarda-a para chegar mesmo com um de vocês offline, mas não a consegue ler.</p></div>
        ${rows}
      </div>
      <div class="composer">${callPill()}
        <form id="composer">
          <button type="button" class="icon-btn" data-act="attach" aria-label="Anexar ficheiro" title="Anexar ficheiro">${I.clip}</button>
          <label class="sr" for="msg-input">Mensagem</label>
          <textarea id="msg-input" rows="1" placeholder="Mensagem para ${esc(p.name)}"></textarea>
          <button type="button" class="icon-btn" data-act="emoji" aria-label="Emoji" title="Emoji">${I.smile}</button>
          <button type="submit" class="icon-btn send" aria-label="Enviar">${I.send}</button>
        </form>
        <div class="typing">${S.typing && S.typing.channelId === c.id ? `<b>${esc(S.typing.who)}</b> está a escrever…` : ""}</div>
      </div>`;
    }
    const offline = !g.host.online ? `<div class="banner ${S.bannerOpen ? "open" : ""}" data-banner-toggle role="button" tabindex="0" aria-expanded="${!!S.bannerOpen}">${I.warn}<span class="banner-short"><b>Anfitrião offline</b> · a ver a tua cópia local</span><div class="banner-long"><b>${g.owner === "me" ? "O teu PC" : esc(who(g.owner).name)}</b>, o anfitrião, está offline. Estás a ver a tua cópia local. O que escreveres fica guardado e é enviado quando ele voltar, ou quando outro membro assumir o grupo.</div></div>` : "";
    const typing = S.typing && S.typing.channelId === c.id ? `<b>${esc(S.typing.who)}</b> está a escrever…` : "";
    return `
      <div class="main-head">${menuBtn()}
        <span class="title">${I.hash}${esc(c.name)}</span>
        <span class="topic">${esc(c.topic || "")}</span>
        <button class="icon-btn ${S.showMembers ? "on" : ""}" data-act="toggle-members" aria-label="Mostrar membros" title="Membros">${I.users}</button>
      </div>
      ${offline}
      <div class="messages ${S.settings.msgStyle === "list" ? "" : "bubbles"}" role="log" aria-live="polite">
        <div class="spacer"></div>
        <div class="welcome"><div class="tile"><span>#</span></div><h2>Bem-vindo a #${esc(c.name)}</h2><p>Este é o início do canal #${esc(c.name)} em ${esc(g.name)}. As mensagens ficam guardadas no PC do anfitrião e numa cópia em cada membro.</p></div>
        ${rows}
      </div>
      <div class="composer">${callPill()}
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
    if (who === "me" && kind === "screen" && media.screen) return `<video class="live-video" data-stream="screen" autoplay muted playsinline></video>`;
    if (who === "me" && kind === "cam" && media.cam) return `<video class="live-video mirror" data-stream="cam" autoplay muted playsinline></video>`;
    if (who !== "me" && !api.isMock) return `<div class="live-video" data-remote="${esc(who)}:${kind}"></div>`;
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
    if (!api.isMock) for (const p of ppl) if (p && p.id !== "me" && vs(p.id).screen) items.push({ id: p.id + ":screen", who: p.id, kind: "screen", label: "Ecrã de " + p.name, meta: "" });
    for (const p of ppl) {
      const cam = (p.id === "me" ? S.camera : api.isMock ? p.id === "ines" : !!vs(p.id).camera) && !(S.settings.userNoVideo || {})[p.id];
      items.push({ id: p.id + ":cam", who: p.id, kind: cam ? "cam" : "avatar", label: p.name + (p.id === "me" ? " (tu)" : "") });
    }
    // "Mostrar quem está sem vídeo" desligado: só ficam vídeos e ecrãs (se houver algum)
    if (S.settings.callShowNoVideo === false && items.some((x) => x.kind !== "avatar")) {
      for (let i = items.length - 1; i >= 0; i--) if (items[i].kind === "avatar") items.splice(i, 1);
    }
    if (S.focus && !items.some((x) => x.id === S.focus)) S.focus = null;
    // Sozinho na chamada não há nada para destacar: o quadrado fica sempre igual.
    if (items.length < 2) S.focus = null;

    const tile = (x, extra = "") => {
      const p = S.people[x.who];
      const muted = !!(vs(x.who).muted || vs(x.who).deaf);
      const speaking = x.kind !== "screen" && S.speaking.includes(x.who) && !muted;
      const inner = x.kind === "avatar" ? `<span class="tile-bg" style="--c:${p.color}"></span>${avatar(p)}` : fakeScreen(x.kind === "screen" ? "screen" : "cam", x.who);
      const badge = x.kind === "screen" ? `<span class="live">${x.meta ? `<b>${esc(x.meta)}</b>` : ""}AO VIVO</span>` : "";
      const host = x.kind !== "screen" && g.owner === x.who ? '<span class="host-tag">ANFITRIÃO</span>' : "";
      return `<button ${x.kind !== "screen" ? `data-spk="${x.who}"` : `data-who="${x.who}"`} class="vtile ${speaking ? "speaking" : ""} ${x.kind !== "avatar" ? "has-video" : ""} ${extra}" ${items.length > 1 ? `data-focus="${x.id}" aria-label="${S.focus === x.id ? "Voltar à grelha" : "Pôr em destaque: " + esc(x.label)}"` : `aria-label="${esc(x.label)}"`} ${extra.includes("grid-cell") ? 'style="width:var(--tw);"' : ""}>${inner}${badge}${host}
        <span class="lbl">${x.kind === "screen" ? I.screen.replace("<svg", '<svg style="color:inherit"') : voiceIc(x.who)}${esc(x.label)}</span></button>`;
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
        ${narrow() ? `<div class="call-top m">
          <button class="icon-btn" data-act="call-min" aria-label="Minimizar a chamada" title="Minimizar">${I.chevron}</button>
          <span class="title">${I.speaker}<span><b>${esc(c.name)}</b><small>${esc(g.name)} · ${S.voice.connected === false ? "a ligar outra vez…" : S.voice.ping + " ms"}</small></span></span>
          <button class="icon-btn" data-act="invite-call" aria-label="Convidar pessoas">${I.invite}</button>
          <button class="icon-btn" data-act="voice-quick" aria-label="Definições de voz">${I.gear}</button>
        </div>` : `<div class="call-top">
          <button class="icon-btn ${S.callSidebar ? "on" : ""}" data-act="call-sidebar" aria-label="${S.callSidebar ? "Esconder grupos e canais" : "Mostrar grupos e canais"}" title="${S.callSidebar ? "Esconder grupos e canais" : "Mostrar grupos e canais"}">${I.menu}</button>
          <span class="title">${I.speaker}${esc(c.name)}<small>${esc(g.name)}</small></span>
          <span class="call-chips">
            <span class="chip" title="Ligação ao anfitrião: ${esc(S.voice.route)}"><i class="dot"></i>Ping ${S.voice.ping} ms</span>
            <span class="chip ${up > 5 ? "warn" : ""}" title="Quanto estás a enviar"><i class="dot"></i>${up.toFixed(1).replace(".", ",")} Mbps</span>
          </span>
          <button class="icon-btn" data-chan-open="${esc(c.id)}" data-act="call-chat" aria-label="Abrir o chat do grupo" title="Abrir o chat (a chamada continua)">${I.hash}</button>
          ${fsBtn()}
        </div>`}
        ${body}
        <div class="call-controls">
          <div class="ctl-group">
            <button class="ctl ${S.muted ? "off" : ""}" data-act="mute" aria-pressed="${S.muted}" title="${S.muted ? "Ligar microfone" : "Desligar microfone"}" aria-label="${S.muted ? "Ligar microfone" : "Desligar microfone"}">${S.muted ? I.micOff : I.mic}</button>
            <button class="ctl ${S.deaf ? "off" : ""}" data-act="deaf" aria-pressed="${S.deaf}" title="${S.deaf ? "Ligar som" : "Desligar som"}" aria-label="${S.deaf ? "Ligar som" : "Desligar som"}">${S.deaf ? I.headOff : I.head}</button>
            <button class="ctl ${S.camera ? "on-media" : ""}" data-act="camera" aria-pressed="${S.camera}" title="${S.camera ? "Desligar câmara" : "Ligar câmara"}" aria-label="${S.camera ? "Desligar câmara" : "Ligar câmara"}">${S.camera ? I.cam : I.camOff}</button>
          </div>
          <div class="ctl-group">
            <button class="ctl ${S.sharing ? "on-media" : ""}" data-act="share" aria-pressed="${!!S.sharing}" title="${S.sharing ? "Parar partilha" : "Partilhar ecrã"}" aria-label="${S.sharing ? "Parar partilha" : "Partilhar ecrã"}">${S.sharing ? I.screenOff : I.screen}</button>
            ${narrow() ? `<button class="ctl" data-act="call-chat" title="Abrir o chat" aria-label="Abrir o chat do grupo">${I.hash}</button>` : `<button class="ctl" data-act="settings-voz" title="Definições de voz e vídeo" aria-label="Definições de voz e vídeo">${I.gear}</button>`}
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

  // O Windows mostra a sua própria janela para escolher o ecrã ou a janela.
  function openSharePicker() {
    const quals = ["720p · 30 fps", "1080p · 30 fps", "1080p · 60 fps"];
    const def = quals.includes(S.settings.shareQuality) ? S.settings.shareQuality : quals[1];
    modal(`<h2>Partilhar ecrã</h2>
      <p>A seguir o Windows pergunta se queres mostrar o ecrã inteiro ou só uma janela.</p>
      <div class="field"><label>Qualidade</label><div class="seg" role="radiogroup" aria-label="Qualidade">${quals.map((q) => `<button role="radio" aria-checked="${q === def}" class="${q === def ? "on" : ""}" data-q="${q}">${q}</button>`).join("")}</div>
        <small style="color:var(--texto-3)">Mais qualidade gasta mais upload e mais do teu PC.</small></div>
      <div class="row" style="padding:0"><div class="txt"><b>Partilhar também o som</b><small>O som do jogo ou do vídeo vai junto.</small></div><label class="switch"><input type="checkbox" id="share-audio" checked><span></span><span class="sr">Partilhar som</span></label></div>
      <div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn primary" id="do-share">${I.screen}Escolher o que partilhar</button></div>`,
    (el, close) => {
      let q = def;
      el.addEventListener("click", (e) => {
        const b = e.target.closest("[data-q]"); if (!b) return;
        el.querySelectorAll("[data-q]").forEach((x) => { x.classList.toggle("on", x === b); x.setAttribute("aria-checked", x === b); });
        q = b.dataset.q;
      });
      $("#do-share", el).onclick = async () => {
        const audio = $("#share-audio", el).checked;
        if (!navigator.mediaDevices || !navigator.mediaDevices.getDisplayMedia) { close(); return toast("Este sistema não deixa partilhar o ecrã."); }
        const [h, fps] = q.startsWith("720") ? [720, 30] : q.includes("60") ? [1080, 60] : [1080, 30];
        let st;
        try { st = await navigator.mediaDevices.getDisplayMedia({ video: { height: { ideal: h }, frameRate: { ideal: fps } }, audio }); }
        catch (x) { if (x && x.name === "NotAllowedError") return; close(); return toast(mediaError(x, "o ecrã")); } // cancelado na janela do Windows
        close();
        stopShare(true);
        const track = st.getVideoTracks()[0];
        const surf = (track.getSettings && track.getSettings().displaySurface) || "";
        const src = surf === "monitor" ? "Ecrã inteiro" : track.label && !/^(screen|window|web-contents)/i.test(track.label) ? track.label : surf === "window" ? "Uma janela" : "Ecrã";
        track.addEventListener("ended", () => { if (media.screen === st) { stopShare(); render(); } }); // "Parar partilha" do Windows
        media.screen = st;
        api.start_screen_share({ source: src, quality: q, audio }).catch(() => {});
        S.sharing = { source: src, quality: q, audio: audio && st.getAudioTracks().length > 0 }; sfx.shareStart();
        syncVideo();
        render(); toast(`Estás a partilhar «${src}».`);
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
        <p class="lede">Junta funcionalidades à tua Tasca. Os plugins com o selo <span class="verified">${I.check}Verificado</span> foram revistos pela equipa e a assinatura é confirmada antes de instalar.</p>
        <div class="tabs" role="tablist">
          <button class="tab ${S.pluginTab === "loja" ? "on" : ""}" data-plugin-tab="loja">Explorar<span class="n">${S.plugins.length}</span></button>
          <button class="tab ${S.pluginTab === "instalados" ? "on" : ""}" data-plugin-tab="instalados">Instalados<span class="n">${nInst}</span></button>
          <button class="tab ${S.pluginTab === "carregar" ? "on" : ""}" data-plugin-tab="carregar">Carregar ficheiro</button>
        </div>
        ${S.pluginTab !== "carregar" ? `<div class="toolbar"><div class="search">${I.search}<label class="sr" for="plugin-q">Procurar plugins</label><input id="plugin-q" placeholder="Procurar plugins" value="${esc(S.pluginQuery)}"></div></div>` : ""}
        ${body}`;
  }

  // ---------------- definições ----------------
  const SET_TABS = [
    ["Utilizador", [["conta", "A minha conta"], ["privacidade", "Privacidade e permissões"], ["aparencia", "Aparência"], ["voz", "Voz e vídeo"]]],
    ["Tasca", [["plugins", "Plugins"], ["rede", "Rede e relay"], ["backups", "Cópias de segurança"], ["grupos", "Grupos que aloja"], ["sobre", "Sobre"]]],
  ];
  function renderSettings() {
    const nav = SET_TABS.map(([cat, items]) => `<div class="cat"><span>${cat}</span></div>${items.map(([k, l]) => `<button class="${S.settingsTab === k ? "on" : ""}" data-set-tab="${k}">${l}</button>`).join("")}`).join("");
    const T = { plugins: renderPlugins, conta: setConta, privacidade: setPrivacidade, aparencia: setAparencia, voz: setVoz, rede: setRede, backups: setBackups, grupos: setGrupos, sobre: setSobre }[S.settingsTab];
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
      <div class="row">${avatar(me).replace('class="av ', 'class="av xl ')}
        <div class="txt"><b style="font-size:18px">${esc(me.name)}</b><small>${me.username ? "@" + esc(me.username) : "Ainda sem nome de utilizador"}</small></div><button class="btn" data-act="edit-profile">Editar perfil</button></div>
      <div class="row"><div class="txt"><b>O teu ID</b><small>${me.username ? "É único e é com ele que te adicionam como amigo. O nome que aparece pode repetir-se; este não." : "Ainda não tens. Escolhe um em Editar perfil para te poderem adicionar."}</small>${me.username ? `<div style="margin-top:6px;font-weight:700;font-size:16px">@${esc(me.username)}</div>` : ""}</div>${me.username ? `<button class="btn" data-act="copy-key">Copiar</button>` : `<button class="btn primary" data-act="edit-profile">Escolher</button>`}</div>
      <details class="row adv"><summary class="txt"><b>Chave de segurança</b><small>Só para quem quer confirmar a identidade à mão. A privada fica cifrada neste PC.</small></summary><div class="mono" style="color:var(--texto-2);word-break:break-all;font-size:12px">${esc(me.key)}</div><button class="btn ghost" data-act="copy-pubkey">Copiar chave</button></details>
      <div class="row"><div class="txt"><b>Cópia no Google Drive</b><small>A tua conta cifrada fica numa pasta privada da Tasca no teu Drive, para a recuperares noutro PC só com a palavra-passe.</small></div><button class="btn" data-act="google-backup">Ligar Google</button></div>
      <div class="row"><div class="txt"><b>Bloquear a Tasca</b><small>Fecha a sessão neste PC. Para voltar a entrar precisas da palavra-passe.</small></div><button class="btn danger" data-act="lock">Bloquear</button></div>
      <div class="row"><div class="txt"><b>Exportar identidade</b><small>Guarda a chave num ficheiro cifrado para usar a mesma conta noutro PC. Sem ela, perdes o acesso aos grupos.</small></div><button class="btn">Exportar…</button></div>
    </div></div>`;
  }
  // ---------------- privacidade e permissões ----------------
  const PERMS = [
    ["mic", "Microfone", "Para falares nas chamadas. Desligado, entras nas chamadas só a ouvir."],
    ["camera", "Câmara", "Para videochamadas. Desligada, o botão da câmara fica bloqueado."],
    ["notifications", "Notificações", "Avisos de mensagens e menções quando a Tasca está em segundo plano."],
    ["autostart", "Abrir com o Windows", "A Tasca abre minimizada quando ligas o PC, para os grupos que alojas estarem sempre disponíveis."],
  ].filter(([k]) => !(window.TASCA_MOBILE && k === "autostart"));
  function setPrivacidade() {
    const ok = S.settings.termsAccepted === termsVersion();
    return `<p class="lede">O que a Tasca pode usar neste PC. Desligar aqui desliga mesmo: a funcionalidade deixa de funcionar até a voltares a ligar.</p>
      <div class="section"><h2>Permissões</h2><div class="box">${PERMS.map(([k, l, d]) => `<div class="row"><div class="txt"><b>${l}</b><small>${d}</small></div>${sw("perms." + k, perm(k), l)}</div>`).join("")}</div>
      <p class="lede" style="font-size:13px">O Windows também tem as suas permissões. Se o microfone ou a câmara não funcionarem com isto ligado, vê Definições do Windows > Privacidade e segurança.</p></div>
      <div class="section"><h2>Termos e regras</h2><div class="box">
        <div class="row"><div class="txt"><b>${ok ? "Aceitaste os termos e regras da Tasca" : "Ainda não aceitaste os termos"}</b><small>Versão de ${esc(termsVersion())}.</small></div><button class="btn" data-act="read-terms">Ler</button></div>
      </div></div>`;
  }
  const termsVersion = () => (window.TASCA_TERMOS && window.TASCA_TERMOS.versao) || "";
  async function applyPerm(key, on) {
    if (key === "mic") {
      if (!on) { if (api.isMock) stopVad(); else if (S.voice) startVad(); stopMicMeter(); if (S.voice && !S.muted) { S.muted = true; api.set_voice_state({ muted: true, deaf: S.deaf }); } toast("Microfone desligado. Nas chamadas ficas só a ouvir."); }
      else { if (S.voice) startVad(); toast("Microfone ligado."); }
    }
    if (key === "camera") {
      if (!on && S.camera) { stopCamera(); api.set_camera({ on: false }).catch(() => {}); }
      toast(on ? "Câmara ligada." : "Câmara desligada.");
    }
    if (key === "notifications") {
      if (on && window.Notification && Notification.permission === "default") { try { await Notification.requestPermission(); } catch {} }
      toast(on ? "Notificações ligadas." : "Notificações desligadas.");
    }
    if (key === "autostart") {
      try { await api.set_autostart({ enabled: on }); toast(on ? "A Tasca abre quando o Windows arranca." : "A Tasca já não abre sozinha."); }
      catch (x) { await patch({ perms: { autostart: !on } }); toast("Não consegui mudar o arranque: " + (x.message || x)); }
    }
    render();
  }

  // Termos: sem os aceitar, a app abre mas fica atrás deste aviso.
  function termsHtml() { return (window.TASCA_TERMOS && window.TASCA_TERMOS.html) || "<p>Os termos não carregaram.</p>"; }
  function openTerms() {
    modal(`<h2>Termos e regras da Tasca</h2><div class="terms-box">${termsHtml()}</div>
      <div class="actions"><button class="btn primary" data-close>Fechar</button></div>`);
  }
  function termsGate() {
    if (!termsVersion() || S.settings.termsAccepted === termsVersion()) return Promise.resolve();
    return new Promise((resolve) => {
      const scrim = document.createElement("div");
      scrim.className = "scrim locked";
      const novo = !S.settings.termsAccepted;
      scrim.innerHTML = `<div class="modal terms-modal" role="dialog" aria-modal="true" aria-labelledby="terms-h">
        <h2 id="terms-h">${novo ? "Antes de continuar" : "Os termos mudaram"}</h2>
        <p>${novo ? "Para usares a Tasca tens de aceitar os termos e as regras da casa." : "Atualizámos os termos e as regras. Lê e aceita para continuares."}</p>
        <div class="terms-box">${termsHtml()}</div>
        <label class="check"><input type="checkbox" id="terms-ok"><span>Li e aceito os termos e as regras da Tasca</span></label>
        <div class="actions"><button class="btn ghost" data-act="sign-out">Sair da conta</button><button class="btn primary" id="terms-accept" disabled>Aceitar e continuar</button></div></div>`;
      document.body.appendChild(scrim);
      const cb = scrim.querySelector("#terms-ok"), btn = scrim.querySelector("#terms-accept");
      cb.onchange = () => { btn.disabled = !cb.checked; };
      btn.onclick = async () => { await patch({ termsAccepted: termsVersion() }); scrim.remove(); resolve(); };
    });
  }

  // ---------------- perfil ----------------
  const COLORS = ["#9B2C3F", "#C0563A", "#B8791A", "#5E8C4A", "#2E7D7A", "#2A5CAA", "#4A6FA5", "#6B4C9A", "#7A3B2E", "#4A4542"];
  function openProfile() {
    const me = S.me;
    let color = me.color, photo = me.photo || null, banner = me.banner || null;
    modal(`<h2>Editar perfil</h2>
      <div class="pf-banner" id="pf-banner" style="${bannerStyle(banner, color)}"></div>
      <div class="profile-prev with-banner"><button type="button" class="pf-photo" id="pf-photo" title="Mudar a foto" aria-label="Mudar a foto">${avatar({ ...me, photo }).replace('class="av ', 'id="pf-av" class="av xl ')}<span class="pf-cam">${I.cam}</span></button>
        <div><b id="pf-name-prev">${esc(me.name)}</b><small>${me.username ? "@" + esc(me.username) : ""}</small>
          <span class="pf-photo-btns"><button type="button" class="linkish" id="pf-pick">${photo ? "Mudar a foto" : "Escolher foto"}</button><button type="button" class="linkish" id="pf-rm" ${photo ? "" : "hidden"}>Tirar</button></span></div>
        <input type="file" id="pf-file" accept="image/png,image/jpeg,image/webp,image/gif" hidden></div>
      <div class="field"><label for="pf-name">Nome</label><input class="input" id="pf-name" maxlength="32" value="${esc(me.name)}"><small style="color:var(--texto-3)">É o nome que os outros veem. Podes usar espaços e acentos.</small></div>
      <div class="field"><label>Cor</label><div class="swatches" role="radiogroup" aria-label="Cor">${COLORS.map((c) => `<button type="button" role="radio" aria-checked="${c === color}" class="${c === color ? "on" : ""}" data-color="${c}" style="background:${c}" aria-label="Cor ${c}"></button>`).join("")}</div></div>
      <div class="field"><label>Banner</label><div class="swatches" role="radiogroup" aria-label="Banner">
          <button type="button" role="radio" class="${!banner ? "on" : ""} sw-auto" data-banner="" aria-label="Azulejo da tua cor" title="Azulejo da tua cor"></button>
          ${COLORS.map((c) => `<button type="button" role="radio" class="${c === banner ? "on" : ""}" data-banner="${c}" style="background:${c}" aria-label="Banner ${c}"></button>`).join("")}</div>
        <span class="pf-photo-btns"><button type="button" class="linkish" id="pf-bpick">Usar uma imagem</button><button type="button" class="linkish" id="pf-brm" ${banner && banner.startsWith("data:") ? "" : "hidden"}>Tirar imagem</button></span>
        <input type="file" id="pf-bfile" accept="image/png,image/jpeg,image/webp,image/gif" hidden></div>
      <div class="field"><label for="pf-bio">Sobre mim</label><textarea class="input" id="pf-bio" rows="3" maxlength="190" placeholder="Conta qualquer coisa sobre ti">${esc(me.bio || "")}</textarea><small class="hint" id="pf-bio-n">${(me.bio || "").length}/190</small></div>
      <div class="field"><label for="pf-user">ID (nome de utilizador)</label>
        <div class="user-in"><span>@</span><input class="input" id="pf-user" maxlength="24" autocomplete="off" spellcheck="false" value="${esc(me.username || "")}" placeholder="o.teu.nome"></div>
        <small class="hint" id="pf-user-st">${me.username ? "Único, só minúsculas. É com ele que te adicionam e que entras noutro PC." : "Ainda não tens. Escolhe um nome único (minúsculas, números, _ e .) para te adicionarem."}</small></div>
      <div class="field" id="pf-pw-f" hidden><label for="pf-pw">Palavra-passe</label><input class="input" id="pf-pw" type="password" autocomplete="current-password"><small style="color:var(--texto-3)">Para mudar o nome de utilizador precisamos da tua palavra-passe.</small></div>
      <p id="pf-err" style="color:var(--perigo)" hidden></p>
      <div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn primary" id="pf-save">Guardar</button></div>`,
    (el, close) => {
      const nameIn = $("#pf-name", el), userIn = $("#pf-user", el), st = $("#pf-user-st", el), err = $("#pf-err", el);
      const norm = (u) => u.trim().replace(/^@/, "").toLowerCase();
      let seq = 0, timer = 0, free = true;
      const drawAv = () => {
        const av = $("#pf-av", el);
        av.style.background = color; av.classList.toggle("has-photo", !!photo);
        av.innerHTML = photo ? `<img src="${photo}" alt="">` : esc(initials(nameIn.value || "?"));
        $("#pf-rm", el).hidden = !photo; $("#pf-pick", el).textContent = photo ? "Mudar a foto" : "Escolher foto";
      };
      nameIn.oninput = () => { $("#pf-name-prev", el).textContent = nameIn.value || " "; drawAv(); };
      const drawBanner = () => {
        $("#pf-banner", el).setAttribute("style", bannerStyle(banner, color));
        $("#pf-brm", el).hidden = !(banner && banner.startsWith("data:"));
        el.querySelectorAll("[data-banner]").forEach((x) => x.classList.toggle("on", (x.dataset.banner || null) === banner));
      };
      $("#pf-bio", el).oninput = (e) => { $("#pf-bio-n", el).textContent = e.target.value.length + "/190"; };
      $("#pf-bpick", el).onclick = () => $("#pf-bfile", el).click();
      $("#pf-brm", el).onclick = () => { banner = null; drawBanner(); };
      $("#pf-bfile", el).onchange = async (e) => {
        const f = e.target.files[0]; e.target.value = ""; if (!f) return;
        try { banner = await bannerPhoto(f); drawBanner(); }
        catch { err.textContent = "Não consegui abrir essa imagem. Tenta um PNG ou JPG."; err.hidden = false; }
      };
      const pick = () => $("#pf-file", el).click();
      $("#pf-photo", el).onclick = pick; $("#pf-pick", el).onclick = pick;
      $("#pf-rm", el).onclick = () => { photo = null; drawAv(); };
      $("#pf-file", el).onchange = async (e) => {
        const f = e.target.files[0]; e.target.value = ""; if (!f) return;
        try { photo = await squarePhoto(f); drawAv(); }
        catch { err.textContent = "Não consegui abrir essa imagem. Tenta um PNG ou JPG."; err.hidden = false; }
      };
      el.addEventListener("click", (e) => {
        const bb = e.target.closest("[data-banner]");
        if (bb) { banner = bb.dataset.banner || null; drawBanner(); return; }
        const b = e.target.closest("[data-color]"); if (!b) return;
        color = b.dataset.color; drawAv(); drawBanner();
        el.querySelectorAll("[data-color]").forEach((x) => { x.classList.toggle("on", x === b); x.setAttribute("aria-checked", x === b); });
      });
      userIn.oninput = () => {
        const u = norm(userIn.value); const changed = u !== (me.username || "");
        $("#pf-pw-f", el).hidden = !changed || !u;
        clearTimeout(timer); free = true;
        if (!changed) { st.className = "hint"; st.textContent = "É com ele que te encontram e que entras noutro PC."; return; }
        if (u.length < 3) { st.className = "hint"; st.textContent = "Pelo menos 3 caracteres."; return; }
        st.className = "hint"; st.textContent = "A verificar…"; const n = ++seq;
        timer = setTimeout(async () => {
          try { free = await api.username_available({ username: u }); if (n !== seq) return;
            st.className = "hint " + (free ? "ok" : "bad"); st.textContent = "@" + u + (free ? " está livre." : " já está ocupado."); }
          catch (x) { if (n === seq) { free = false; st.className = "hint bad"; st.textContent = String(x.message || x); } }
        }, 400);
      };
      $("#pf-save", el).onclick = async (e) => {
        const btn = e.currentTarget; err.hidden = true;
        const name = nameIn.value.trim(), u = norm(userIn.value);
        if (!name) { err.textContent = "Escreve um nome."; err.hidden = false; return nameIn.focus(); }
        const userChanged = u && u !== (me.username || "");
        if (userChanged && !free) { err.textContent = "Escolhe outro nome de utilizador."; err.hidden = false; return userIn.focus(); }
        if (userChanged && !$("#pf-pw", el).value) { err.textContent = "Escreve a tua palavra-passe para mudar o nome de utilizador."; err.hidden = false; return $("#pf-pw", el).focus(); }
        btn.disabled = true; btn.textContent = "A guardar…";
        try {
          if (name !== me.name) me.name = await api.set_display_name({ name });
          const bio = $("#pf-bio", el).value.trim().slice(0, 190);
          if (color !== me.color || photo !== (me.photo || null) || bio !== (me.bio || "") || banner !== (me.banner || null)) {
            await api.set_profile({ color, photo, bio, banner }); Object.assign(me, { color, photo, bio, banner });
          }
          if (userChanged) {
            const pw = $("#pf-pw", el).value;
            me.username = me.username ? await api.change_username({ newUsername: u, password: pw }) : await api.server_register({ username: u, password: pw });
          }
          Object.assign(S.people.me || {}, { name: me.name, color: me.color, username: me.username, photo: me.photo, bio: me.bio, banner: me.banner });
          for (const list of Object.values(S.allMembers || {})) for (const m of list) if (m.id === "me") Object.assign(m, { name: me.name, color: me.color, photo: me.photo, bio: me.bio, banner: me.banner, username: me.username });
          close(); render(); toast("Perfil guardado.");
        } catch (x) { err.textContent = String(x.message || x); err.hidden = false; btn.disabled = false; btn.textContent = "Guardar"; }
      };
    });
  }

  // Banner: cor (#RRGGBB), imagem (data:) ou, sem nada, o azulejo na cor do perfil.
  function bannerStyle(banner, color) {
    if (banner && banner.startsWith("data:")) return `background:#000 url("${banner.replace(/"/g, "")}") center / cover`;
    if (banner && /^#[0-9a-f]{6}$/i.test(banner)) return `background:${banner}`;
    return `--c:${/^#[0-9a-f]{6}$/i.test(color || "") ? color : "#9B2C3F"}`;
  }
  // Corta ao centro em 5:2 e reduz até caber (o banner vai nos eventos do grupo, ~60 KB no máximo).
  function bannerPhoto(file) {
    return new Promise((resolve, reject) => {
      const url = URL.createObjectURL(file);
      const img = new Image();
      img.onload = () => {
        const W = img.naturalWidth, H = img.naturalHeight, ratio = 5 / 2;
        let w = W, h = W / ratio; if (h > H) { h = H; w = H * ratio; }
        let out = 600, q = 0.82, d = "";
        for (let i = 0; i < 6; i++) {
          const c = document.createElement("canvas"); c.width = out; c.height = Math.round(out / ratio);
          c.getContext("2d").drawImage(img, (W - w) / 2, (H - h) / 2, w, h, 0, 0, c.width, c.height);
          d = c.toDataURL("image/webp", q);
          if (!d.startsWith("data:image/webp")) d = c.toDataURL("image/jpeg", q);
          if (d.length <= 58000) break;
          q -= 0.12; if (i >= 2) out = Math.round(out * 0.8);
        }
        URL.revokeObjectURL(url);
        d.length <= 60000 ? resolve(d) : reject(new Error("imagem pesada"));
      };
      img.onerror = () => { URL.revokeObjectURL(url); reject(new Error("imagem inválida")); };
      img.src = url;
    });
  }
  // Corta ao centro em quadrado e reduz para 256 px (fica leve para guardar e enviar).
  function squarePhoto(file) {
    return new Promise((resolve, reject) => {
      const url = URL.createObjectURL(file);
      const img = new Image();
      img.onload = () => {
        const n = Math.min(img.naturalWidth, img.naturalHeight), out = 256;
        const c = document.createElement("canvas"); c.width = c.height = out;
        c.getContext("2d").drawImage(img, (img.naturalWidth - n) / 2, (img.naturalHeight - n) / 2, n, n, 0, 0, out, out);
        URL.revokeObjectURL(url);
        let d = c.toDataURL("image/webp", 0.85);
        if (!d.startsWith("data:image/webp")) d = c.toDataURL("image/jpeg", 0.85);
        resolve(d);
      };
      img.onerror = () => { URL.revokeObjectURL(url); reject(new Error("imagem inválida")); };
      img.src = url;
    });
  }

  // ---------------- canais ----------------
  function openChannelEditor(opts) {
    const g = group(); if (!g || g.owner !== "me") return;
    const c = opts.id ? S.channels.find((x) => x.id === opts.id) : null;
    let kind = c ? c.kind : opts.kind || "text";
    // o canal novo fica na categoria do + em que carregaste
    const catDef = c ? c.cat : opts.cat || (S.cats[g.id] || [])[0] || "Canais";
    modal(`<h2>${c ? "Editar canal" : "Criar canal"}</h2>${c ? "" : `<p>Vai ficar na categoria <b>${esc(catDef)}</b>.</p>`}
      ${c ? "" : `<div class="field"><label>Tipo</label><div class="kind-pick" role="radiogroup" aria-label="Tipo de canal">
        <button type="button" role="radio" data-kind="text" class="${kind === "text" ? "on" : ""}" aria-checked="${kind === "text"}">${I.hash}<span><b>Texto</b><small>Mensagens, imagens e ficheiros.</small></span></button>
        <button type="button" role="radio" data-kind="voice" class="${kind === "voice" ? "on" : ""}" aria-checked="${kind === "voice"}">${I.speaker}<span><b>Voz</b><small>Falar, câmara e partilhar ecrã.</small></span></button></div></div>`}
      <div class="field"><label for="ch-name">Nome</label><div class="user-in"><span id="ch-ic">${kind === "voice" ? I.speaker : I.hash}</span><input class="input" id="ch-name" maxlength="40" value="${c ? esc(c.name) : ""}" placeholder="${kind === "voice" ? "Sala de estar" : "novo-canal"}"></div></div>
      <div class="field" id="ch-topic-f" ${kind === "voice" ? "hidden" : ""}><label for="ch-topic">Tema</label><input class="input" id="ch-topic" maxlength="120" value="${c ? esc(c.topic || "") : ""}" placeholder="Sobre o que se fala aqui"></div>
      ${c ? `<small style="color:var(--texto-3)">Para mudar a ordem ou a categoria, arrasta o canal na lista.</small>` : ""}
      <p id="ch-err" style="color:var(--perigo)" hidden></p>
      <div class="actions">${c ? `<button class="btn danger" id="ch-del" style="margin-right:auto">Apagar canal</button>` : ""}<button class="btn ghost" data-close>Cancelar</button><button class="btn primary" id="ch-save">${c ? "Guardar" : "Criar canal"}</button></div>`,
    (el, close) => {
      const err = (m) => { const p = $("#ch-err", el); p.textContent = m; p.hidden = !m; };
      const refresh = () => refreshChannels(g.id);
      el.addEventListener("click", async (e) => {
        const k = e.target.closest("[data-kind]");
        if (k) {
          kind = k.dataset.kind;
          el.querySelectorAll("[data-kind]").forEach((x) => { x.classList.toggle("on", x === k); x.setAttribute("aria-checked", x === k); });
          $("#ch-ic", el).innerHTML = kind === "voice" ? I.speaker : I.hash;
          $("#ch-topic-f", el).hidden = kind === "voice";
          $("#ch-name", el).placeholder = kind === "voice" ? "Sala de estar" : "novo-canal";
          return;
        }
      });
      $("#ch-save", el).onclick = async () => {
        err("");
        const name = $("#ch-name", el).value, topic = $("#ch-topic", el).value, cat = catDef;
        try {
          if (c) {
            await api.update_channel({ groupId: g.id, channelId: c.id, name, topic });
            await refresh(); close(); render(); toast("Canal guardado.");
          } else {
            const n = await api.create_channel({ groupId: g.id, name, kind, topic, cat });
            await refresh(); close();
            if (n.kind === "text") await openChannel(n.id); else render();
            toast(n.kind === "voice" ? `Canal de voz «${n.name}» criado.` : `Canal #${n.name} criado.`);
          }
        } catch (x) { err(String(x.message || x)); }
      };
      const del = $("#ch-del", el);
      if (del) del.onclick = async () => {
        if (del.dataset.sure !== "1") { del.dataset.sure = "1"; del.textContent = "Carrega outra vez para apagar"; err(c.kind === "text" ? "As mensagens deste canal são apagadas para toda a gente." : ""); return; }
        try {
          if (S.voice && S.voice.channelId === c.id) await leaveVoice();
          await api.delete_channel({ groupId: g.id, channelId: c.id });
          await refresh(); close();
          if (S.channelId === c.id) { const t = S.channels.find((x) => x.kind === "text"); if (t) return openChannel(t.id).then(() => toast("Canal apagado.")); }
          render(); toast("Canal apagado.");
        } catch (x) { err(String(x.message || x)); del.dataset.sure = ""; del.textContent = "Apagar canal"; }
      };
    });
  }

  async function refreshChannels(gid) {
    const ch = await api.list_channels({ groupId: gid });
    S.allChannels[gid] = ch; S.cats[gid] = await api.list_categories({ groupId: gid });
    if (S.groupId === gid) S.channels = ch;
  }

  // Categorias: criar, mudar o nome, apagar (os canais passam para a de cima).
  function openCategoryEditor(name) {
    const g = group(); if (!g || g.owner !== "me") return;
    document.querySelectorAll(".scrim:not(.locked)").forEach((x) => x.remove());
    modal(`<h2>${name ? "Editar categoria" : "Criar categoria"}</h2>
      <div class="field"><label for="cat-name">Nome</label><input class="input" id="cat-name" maxlength="24" value="${name ? esc(name) : ""}" placeholder="Ex.: Jogos"></div>
      ${name ? `<small style="color:var(--texto-3)">Para a mudar de lugar, arrasta o título da categoria na lista.</small>` : ""}
      <p id="cat-err" style="color:var(--perigo)" hidden></p>
      <div class="actions">${name ? `<button class="btn danger" id="cat-del" style="margin-right:auto">Apagar categoria</button>` : ""}<button class="btn ghost" data-close>Cancelar</button><button class="btn primary" id="cat-save">${name ? "Guardar" : "Criar categoria"}</button></div>`,
    (el, close) => {
      const err = (m) => { const p = $("#cat-err", el); p.textContent = m; p.hidden = !m; };
      const inp = $("#cat-name", el);
      inp.addEventListener("keydown", (e) => { if (e.key === "Enter") $("#cat-save", el).click(); });
      $("#cat-save", el).onclick = async () => {
        try {
          if (name) await api.rename_category({ groupId: g.id, from: name, to: inp.value });
          else await api.create_category({ groupId: g.id, name: inp.value });
          await refreshChannels(g.id); close(); render(); toast(name ? "Categoria guardada." : "Categoria criada. Carrega no + dela para lhe juntar canais.");
        } catch (x) { err(String(x.message || x)); }
      };
      const del = $("#cat-del", el);
      if (del) del.onclick = async () => {
        if (del.dataset.sure !== "1") { const n = S.channels.filter((c) => c.cat === name).length; del.dataset.sure = "1"; del.textContent = "Carrega outra vez para apagar"; if (n) err(`Os canais dela (${n}) passam para a categoria de cima.`); return; }
        try { await api.delete_category({ groupId: g.id, name }); await refreshChannels(g.id); close(); render(); toast("Categoria apagada."); }
        catch (x) { err(String(x.message || x)); del.dataset.sure = ""; del.textContent = "Apagar categoria"; }
      };
    });
  }

  // ---------------- arrastar canais e categorias (anfitrião) ----------------
  let dragging = null; // { kind: "chan" | "cat", id }
  function clearDrop() { document.querySelectorAll(".drop-before, .drop-after, .drop-into, .dragging").forEach((x) => x.classList.remove("drop-before", "drop-after", "drop-into", "dragging")); }
  function catHeaderOf(wrap) {
    const c = S.channels.find((x) => x.id === wrap.dataset.dragChan);
    return c ? [...document.querySelectorAll("[data-drag-cat]")].find((x) => x.dataset.dragCat === c.cat) : null;
  }
  function dropTarget(e) {
    if (!dragging || !e.target.closest) return null;
    const wrap = e.target.closest("[data-drag-chan]"), cat = e.target.closest("[data-drag-cat]");
    if (dragging.kind === "chan") {
      if (wrap) { const r = wrap.getBoundingClientRect(); return { el: wrap, pos: e.clientY < r.top + r.height / 2 ? "before" : "after" }; }
      if (cat) return { el: cat, pos: "into" };
    } else {
      const el = cat || (wrap && catHeaderOf(wrap));
      if (el) { const r = el.getBoundingClientRect(); return { el, pos: cat && e.clientY >= r.top + r.height / 2 ? "after" : "before" }; }
    }
    return null;
  }
  document.addEventListener("dragstart", (e) => {
    const w = e.target.closest && e.target.closest("[data-drag-chan], [data-drag-cat]");
    if (!w || !w.closest(".channels")) return;
    dragging = w.dataset.dragChan ? { kind: "chan", id: w.dataset.dragChan } : { kind: "cat", id: w.dataset.dragCat };
    e.dataTransfer.effectAllowed = "move"; e.dataTransfer.setData("text/plain", dragging.id);
    requestAnimationFrame(() => w.classList.add("dragging"));
  });
  document.addEventListener("dragover", (e) => {
    if (!dragging) return;
    const t = dropTarget(e);
    document.querySelectorAll(".drop-before, .drop-after, .drop-into").forEach((x) => x.classList.remove("drop-before", "drop-after", "drop-into"));
    if (!t) return;
    e.preventDefault(); e.dataTransfer.dropEffect = "move";
    t.el.classList.add("drop-" + t.pos);
  });
  document.addEventListener("dragend", () => { dragging = null; clearDrop(); });
  document.addEventListener("drop", async (e) => {
    if (!dragging) return;
    const t = dropTarget(e), d = dragging; dragging = null; clearDrop();
    if (!t) return;
    e.preventDefault();
    const g = group(); if (!g) return;
    if (d.kind === "chan") {
      const list = S.channels.map((c) => ({ id: c.id, cat: c.cat }));
      const me = list.find((x) => x.id === d.id); if (!me) return;
      list.splice(list.indexOf(me), 1);
      if (t.pos === "into") {
        me.cat = t.el.dataset.dragCat;
        const first = list.findIndex((x) => x.cat === me.cat);
        list.splice(first < 0 ? list.length : first, 0, me);
      } else {
        const other = list.find((x) => x.id === t.el.dataset.dragChan); if (!other) return;
        me.cat = other.cat;
        list.splice(list.indexOf(other) + (t.pos === "after" ? 1 : 0), 0, me);
      }
      await api.reorder_channels({ groupId: g.id, order: list });
    } else {
      if (t.el.dataset.dragCat === d.id) return;
      const order = (S.cats[g.id] || []).filter((x) => x !== d.id);
      const at = order.indexOf(t.el.dataset.dragCat); if (at < 0) return;
      order.splice(at + (t.pos === "after" ? 1 : 0), 0, d.id);
      await api.reorder_categories({ groupId: g.id, order });
    }
    await refreshChannels(g.id); render();
  });

  function setAparencia() {
    const s = S.settings;
    return `<div class="section"><h2>Tema</h2><div class="box"><div class="row"><div class="txt"><b>Modo</b><small>Madeira escura é o tema da casa.</small></div>
      <div class="seg" role="radiogroup" aria-label="Tema">${[["dark", "Madeira (escuro)"], ["light", "Cal (claro)"], ["system", "Sistema"]].map(([k, l]) => `<button role="radio" aria-checked="${s.theme === k}" class="${s.theme === k ? "on" : ""}" data-theme-set="${k}">${l}</button>`).join("")}</div></div>
      <div class="row"><div class="txt"><b>Estilo das mensagens</b><small>Balões ou lista corrida.</small></div>
      <div class="seg" role="radiogroup" aria-label="Estilo das mensagens">${[["bubbles", "Balões"], ["list", "Lista"]].map(([k, l]) => `<button role="radio" aria-checked="${(s.msgStyle || "bubbles") === k}" class="${(s.msgStyle || "bubbles") === k ? "on" : ""}" data-msg-style="${k}">${l}</button>`).join("")}</div></div>
      <div class="row"><div class="txt"><b>Tamanho do texto</b><small>${s.fontSize} px</small></div><input class="range" style="max-width:240px" type="range" min="13" max="18" id="fontSize" data-setting="fontSize" value="${s.fontSize}"></div>
      <div class="row"><div class="txt"><b>Mensagens compactas</b><small>Menos espaço entre mensagens.</small></div>${sw("compact", s.compact, "Mensagens compactas")}</div>
    </div></div>`;
  }
  // ---------------- dispositivos reais (microfone, colunas, câmara) ----------------
  // A lista vem do sistema (enumerateDevices). Os nomes só aparecem depois de a app ter
  // autorização para o microfone, por isso pede-a uma vez.
  const DEV = { loaded: false, loading: false, in: [], out: [], cam: [], err: null };
  async function loadDevices(ask = true) {
    if (!navigator.mediaDevices || !navigator.mediaDevices.enumerateDevices) { DEV.err = "Este sistema não deixa ver os dispositivos."; DEV.loaded = true; return; }
    DEV.loading = true;
    try {
      let list = await navigator.mediaDevices.enumerateDevices();
      if (ask && !list.some((d) => d.kind === "audioinput" && d.label)) {
        try {
          const st = await navigator.mediaDevices.getUserMedia({ audio: true });
          st.getTracks().forEach((t) => t.stop());
          list = await navigator.mediaDevices.enumerateDevices();
        } catch { DEV.err = "A Tasca não tem autorização para usar o microfone. Dá-lha nas definições de privacidade do Windows."; }
      }
      const pick = (kind) => list.filter((d) => d.kind === kind && d.deviceId !== "default" && d.deviceId !== "communications")
        .map((d, i) => ({ id: d.deviceId, label: d.label || `${kind === "videoinput" ? "Câmara" : kind === "audioinput" ? "Microfone" : "Saída"} ${i + 1}` }));
      DEV.in = pick("audioinput"); DEV.out = pick("audiooutput"); DEV.cam = pick("videoinput");
    } catch (x) { DEV.err = String(x.message || x); }
    DEV.loaded = true; DEV.loading = false;
  }
  if (navigator.mediaDevices && navigator.mediaDevices.addEventListener) {
    navigator.mediaDevices.addEventListener("devicechange", async () => {
      await loadDevices(false);
      if (S.view === "settings" && S.settingsTab === "voz") { render(); startMicMeter(); }
    });
  }
  const devSelect = (id, key, list, fallback) => {
    const cur = S.settings[key] || "default";
    const opts = [["default", fallback], ...list.map((d) => [d.id, d.label])];
    return `<select class="input" id="${id}" data-setting="${key}">${opts.map(([v, l]) => `<option value="${esc(v)}" ${v === cur ? "selected" : ""}>${esc(l)}</option>`).join("")}</select>`;
  };

  // Medidor do microfone nas definições: mostra o nível real da entrada escolhida.
  let meter = null;
  // Nível de 0 a 100 de um pico (0 a 1): -60 dBFS ou menos é 0, 0 dBFS é 100 (igual ao Rust).
  const micLevel = (pk) => (pk > 0 ? Math.max(0, Math.min(100, Math.round(((20 * Math.log10(pk) + 60) / 60) * 100))) : 0);
  function stopMicMeter() {
    if (!meter) return;
    cancelAnimationFrame(meter.raf); meter.stream.getTracks().forEach((t) => t.stop()); meter.ctx.close(); meter = null;
  }
  async function startMicMeter() {
    stopMicMeter();
    if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) return;
    const dev = S.settings.inputDevice;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: dev && dev !== "default" ? { deviceId: { exact: dev } } : true });
      if (S.view !== "settings" || S.settingsTab !== "voz") { stream.getTracks().forEach((t) => t.stop()); return; }
      const ctx = new AudioContext(); const an = ctx.createAnalyser(); an.fftSize = 512;
      ctx.createMediaStreamSource(stream).connect(an);
      const buf = new Uint8Array(an.fftSize);
      meter = { stream, ctx, raf: 0 };
      const tick = () => {
        an.getByteTimeDomainData(buf);
        let peak = 0; for (const v of buf) peak = Math.max(peak, Math.abs(v - 128));
        const el = document.getElementById("mic-meter");
        if (!el) return stopMicMeter();
        const pct = micLevel(peak / 128);
        el.style.width = pct + "%";
        el.classList.toggle("on", pct >= (S.settings.sensitivity ?? 40));
        meter.raf = requestAnimationFrame(tick);
      };
      tick();
    } catch { /* sem autorização: o medidor fica parado */ }
  }

  function setVoz() {
    const s = S.settings;
    if (!DEV.loaded && !DEV.loading) loadDevices().then(() => { if (S.view === "settings" && S.settingsTab === "voz") { render(); startMicMeter(); } });
    return `<div class="section"><h2>Dispositivos</h2><div class="box">
      <div class="row"><div class="field" style="flex:1;min-width:200px"><label for="inputDevice">Entrada</label>${devSelect("inputDevice", "inputDevice", DEV.in, "Predefinição do sistema")}</div>
        <div class="field" style="flex:1;min-width:200px"><label for="outputDevice">Saída</label>${devSelect("outputDevice", "outputDevice", DEV.out, "Predefinição do sistema")}</div></div>
      ${DEV.err ? `<div class="row"><small class="dev-err">${esc(DEV.err)}</small></div>` : !DEV.loaded ? `<div class="row"><small>A procurar dispositivos…</small></div>` : ""}
      <div class="row"><div class="txt"><b>Sensibilidade do microfone</b><small>Fala para ver o nível. Abaixo da sensibilidade escolhida o microfone não transmite.</small><div class="meter mic" style="margin-top:8px"><i id="mic-meter" style="width:0%"></i></div></div><input class="range" style="max-width:200px" type="range" min="0" max="100" id="sensitivity" data-setting="sensitivity" value="${s.sensitivity}"></div>
      <div class="row"><div class="txt"><b>Carregar para falar</b><small>Só transmite enquanto carregas na tecla (V).</small></div>${sw("pushToTalk", s.pushToTalk, "Carregar para falar")}</div>
      <div class="row"><div class="txt"><b>Supressão de ruído</b><small>Remove ventoinhas e teclado. Corre no teu PC.</small></div>${sw("noiseSuppression", s.noiseSuppression, "Supressão de ruído")}</div>
      ${s.noiseSuppression !== false ? `<div class="row"><div class="txt"><b>Modo da supressão</b><small>${s.noiseMode === "light" ? "Leve (RNNoise): gasta muito pouco, bom para PCs fracos." : "Forte (DeepFilterNet): tira mais ruído e soa mais natural."}</small></div>
        <div class="seg" role="radiogroup" aria-label="Modo da supressão de ruído">${[["deep", "Forte"], ["light", "Leve"]].map(([k, l]) => `<button role="radio" aria-checked="${(s.noiseMode || "deep") === k}" class="${(s.noiseMode || "deep") === k ? "on" : ""}" data-noise-mode="${k}">${l}</button>`).join("")}</div></div>` : ""}
      <div class="row"><div class="txt"><b>Cancelamento de eco</b></div>${sw("echoCancel", s.echoCancel, "Cancelamento de eco")}</div>
      <div class="row"><div class="txt"><b>Sons da chamada</b><small>Entrar e sair, microfone, som e partilha de ecrã.</small></div>
        <button class="btn ghost" data-act="test-sound">Ouvir</button>${sw("callSounds", S.settings.callSounds !== false, "Sons da chamada")}</div>
      <div class="row"><div class="txt"><b>Volume dos sons</b><small>${Math.round((S.settings.soundVolume ?? 0.6) * 100)}%</small></div><input class="range" style="max-width:200px" type="range" min="0" max="100" id="soundVolume" data-setting="soundVolume" value="${Math.round((S.settings.soundVolume ?? 0.6) * 100)}"></div>
    </div><h2>Vídeo</h2><div class="box">
      <div class="row"><div class="field" style="flex:1;min-width:200px"><label for="camDevice">Câmara</label>${devSelect("camDevice", "camDevice", DEV.cam, DEV.cam.length ? "Predefinição do sistema" : "Nenhuma câmara encontrada")}</div>
        <div class="field" style="flex:1;min-width:200px"><label for="shareQuality">Qualidade da partilha de ecrã</label><select class="input" id="shareQuality" data-setting="shareQuality">${["720p · 30 fps", "1080p · 30 fps", "1080p · 60 fps"].map((d) => `<option ${d === (S.settings.shareQuality || "1080p · 30 fps") ? "selected" : ""}>${d}</option>`).join("")}</select></div></div>
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
      <div class="section only-desktop"><h2>Arranque</h2><div class="box"><div class="row"><div class="txt"><b>Abrir a Tasca quando o Windows arranca</b><small>Minimizada na área de notificação, para os teus grupos estarem sempre disponíveis.</small></div>${sw("perms.autostart", perm("autostart"), "Abrir no arranque")}</div></div></div>`;
  }
  function setSobre() {
    const onde = window.api.isMock ? "dados de teste no browser" : window.TASCA_MOBILE ? "app Android" : "app para PC";
    return `<div class="section"><div class="box"><div class="row"><span style="width:56px;height:56px;display:block">${LOGO}</span><div class="txt"><b style="font-size:18px">Tasca</b><small>Versão ${esc(appVersion || "…")} · ${onde}</small></div>${window.TASCA_MOBILE && !window.api.isMock ? `<button class="btn primary" data-act="check-update">Procurar atualizações</button>` : ""}</div>
      <div class="row"><div class="txt"><small>Conversa, voz e plugins. As mensagens vão cifradas de ponta a ponta; ficheiros e chamadas vão direto entre PCs. Feito com Rust, iroh, SQLite e Tauri.</small></div></div></div></div>`;
  }

  // ---------------- modais ----------------
  // Fecha uma janela com a animação de saída (no telemóvel desce; no PC encolhe).
  function closeScrim(scrim) {
    if (!scrim || scrim.classList.contains("closing")) return;
    scrim.classList.add("closing");
    const m = scrim.querySelector(".modal"); if (m) m.style.transform = "";
    setTimeout(() => scrim.remove(), 200);
  }
  // No telemóvel, puxar a janela para baixo fecha-a.
  function sheetDrag(el, sheet, close, canStart = () => true) {
    let y0 = null, dy = 0, t0 = 0;
    el.addEventListener("touchstart", (e) => {
      if (window.innerWidth > 720 || e.touches.length > 1) return;
      const scroller = e.target.closest(".modal, .pcard, .pfull-body, .terms-box, .emoji-body");
      if (scroller && scroller.scrollTop > 0) return;
      if (!canStart(e)) return;
      y0 = e.touches[0].clientY; dy = 0; t0 = Date.now();
    }, { passive: true });
    el.addEventListener("touchmove", (e) => {
      if (y0 == null) return;
      dy = Math.max(0, e.touches[0].clientY - y0);
      if (dy > 4) { sheet.style.transition = "none"; sheet.style.transform = `translateY(${dy}px)`; }
    }, { passive: true });
    el.addEventListener("touchend", () => {
      if (y0 == null) return;
      const fast = dy > 40 && Date.now() - t0 < 250;
      y0 = null;
      sheet.style.transition = "transform .2s cubic-bezier(.2,.8,.2,1)";
      if (dy > 120 || fast) { sheet.style.transform = `translateY(110%)`; haptic(); setTimeout(close, 160); }
      else sheet.style.transform = "";
      setTimeout(() => { sheet.style.transition = ""; }, 220);
    });
  }
  function modal(html, onMount) {
    const scrim = document.createElement("div");
    scrim.className = "scrim";
    scrim.innerHTML = `<div class="modal" role="dialog" aria-modal="true">${html}</div>`;
    const close = () => closeScrim(scrim);
    scrim.addEventListener("click", (e) => { if (e.target === scrim || e.target.closest("[data-close]")) close(); });
    document.body.appendChild(scrim);
    sheetDrag(scrim, scrim.querySelector(".modal"), () => scrim.remove(), (e) => !e.target.closest("input, textarea, select, .seg, input[type=range]"));
    const first = scrim.querySelector("input, textarea, button.primary, button");
    // no telemóvel não abre o teclado sozinho
    if (first && !(window.TASCA_MOBILE && /INPUT|TEXTAREA/.test(first.tagName))) first.focus({ preventScroll: true });
    if (onMount) onMount(scrim, close);
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
        try {
          const g = await api.join_group({ invite: $("#invite-code", el).value });
          if (g && g.id) {
            if (!S.groups.some((x) => x.id === g.id)) S.groups.push(g);
            S.people = await api.list_people();
            S.allMembers[g.id] = await api.list_members({ groupId: g.id });
            close(); await openGroup(g.id); toast(`Entraste em «${g.name}».`);
          } else close();
        }
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
      <div class="field"><label for="inv">Convite${inv.expires ? " · expira em " + inv.expires : " · não expira"}</label><textarea class="input mono" id="inv" rows="3" readonly>${inv.code}</textarea></div>
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
        <div class="row"><div class="txt"><b>Anfitrião</b><small>${mine ? "Este PC" : esc(who(g.owner).name)}</small></div></div>
        ${mine ? `<div class="row"><div class="txt"><b>Canais</b><small>Texto e voz podem estar na mesma categoria. Arrasta canais e categorias na lista para os ordenar.</small></div><span style="display:flex;gap:6px"><button class="btn ghost" data-act="new-category">Criar categoria</button><button class="btn" data-act="new-channel">Criar canal</button></span></div>` : ""}
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
      <div class="field"><label for="to-who">Novo anfitrião</label><select class="input" id="to-who">${others.map((m) => `<option value="${m.id}">${esc(m.name)}${m.username ? " · @" + esc(m.username) : ""}</option>`).join("") || "<option disabled>Não há outros membros</option>"}</select></div>
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
  async function openDm(id) {
    S.home = true; S.view = "chat"; S.channelId = id;
    const d = S.dms.find((x) => x.id === id); if (d) d.unread = 0;
    S.messages = await api.list_messages({ channelId: id });
    S.navOpen = false;
    render();
    store.set("dm", id);
    const inp = $("#msg-input"); if (inp && window.innerWidth > 720) inp.focus();
  }
  async function openGroup(id) {
    S.home = false;
    S.groupId = id; S.view = "chat"; S.navOpen = S.navOpen && window.innerWidth <= 720;
    const g = group(); g.unread = 0;
    S.channels = await api.list_channels({ groupId: id });
    S.allChannels[id] = S.channels;
    S.cats[id] = await api.list_categories({ groupId: id });
    S.members = await api.list_members({ groupId: id });
    S.allMembers[id] = S.members;
    const firstText = S.channels.find((c) => c.kind === "text");
    await openChannel(firstText.id, true);
    store.set("group", id);
  }
  async function openChannel(id, keepNav) {
    S.home = false; S.channelId = id; S.view = "chat";
    const c = channel(); if (c) c.unread = 0;
    S.messages = await api.list_messages({ channelId: id });
    if (!keepNav) S.navOpen = false;
    render();
    const inp = $("#msg-input"); if (inp && window.innerWidth > 720) inp.focus();
  }
  // ---------------- telemóvel: painel antes de entrar na chamada ----------------
  // Tocar num canal de voz abre uma folha com quem está lá, o estado do anfitrião e
  // um botão grande para entrar. As definições de voz e o convite ficam à mão.
  function hostOffline(g) { return g && !g.host.online && !(g.owner === "me" && g.host.route === "este PC"); }
  function voicePeopleHtml(c) {
    const ppl = (c.inVoice || []).map((id) => who(id));
    if (!ppl.length) return `<div class="vs-empty"><span class="vs-empty-ic">${I.speaker}</span><b>Ainda não está ninguém aqui</b><small>Quando estiveres pronto para conversar, é só entrar.</small></div>`;
    return `<div class="vs-people">${ppl.map((p) => `<div class="vs-person ${S.speaking.includes(p.id) ? "speaking" : ""}" data-spk="${esc(p.id)}">${avatar(p, "xl")}<span>${esc(p.name)}</span></div>`).join("")}</div>`;
  }
  // Página de antes de entrar (telemóvel): entra a deslizar da direita, como uma página.
  function openVoiceSheet(id) {
    if (!findChannel(id)) return;
    S.prevView = S.view === "voicepre" ? S.prevView : S.view;
    S.preVoice = id; S.preVoiceErr = null; S.view = "voicepre"; S.navOpen = false; render();
  }
  function renderVoicePre() {
    const id = S.preVoice, c = findChannel(id);
    if (!c) { S.view = "chat"; return renderChat(); }
    const g = S.groups.find((x) => (S.allChannels[x.id] || []).some((y) => y.id === id)) || group();
    const off = hostOffline(g);
    const hostName = g.owner === "me" ? "O teu PC" : `O PC de ${esc(who(g.owner).name)}`;
    const n = (c.inVoice || []).length;
    return `<div class="vpre">
      <div class="call-top m">
        <button class="icon-btn" data-act="voicepre-back" aria-label="Voltar">${I.chevron}</button>
        <span class="title">${I.speaker}<span><b>${esc(c.name)}</b><small>${esc(g.name)}${n ? " · " + n + (n === 1 ? " pessoa" : " pessoas") : ""}</small></span></span>
        <button class="icon-btn" data-act="voicepre-invite" aria-label="Convidar pessoas">${I.invite}</button>
        <button class="icon-btn" data-act="voice-quick" aria-label="Definições de voz">${I.gear}</button>
      </div>
      <div class="vpre-body">${voicePeopleHtml(c)}</div>
      ${off && !n ? `<div class="vs-warn">${I.warn}<span><b>${hostName} está desligado.</b> Não faz mal: quem entra primeiro aloja a chamada no seu dispositivo.</span></div>` : ""}
      ${S.preVoiceErr ? `<p class="vs-err">${esc(S.preVoiceErr)}</p>` : ""}
      <div class="vs-bar">
        <button class="vs-round ${S.muted ? "off" : ""}" data-act="vp-mic" aria-pressed="${S.muted}" aria-label="${S.muted ? "Ligar microfone" : "Desligar microfone"}">${S.muted ? I.micOff : I.mic}</button>
        <button class="vs-join" data-act="vp-join" ${S.preVoiceBusy ? "disabled" : ""}>${S.preVoiceBusy ? "A ligar…" : S.preVoiceErr ? "Tentar outra vez" : "Entrar na chamada"}</button>
        <button class="vs-round ${S.deaf ? "off" : ""}" data-act="vp-deaf" aria-pressed="${S.deaf}" aria-label="${S.deaf ? "Ligar som" : "Desligar som"}">${S.deaf ? I.headOff : I.head}</button>
      </div>
    </div>`;
  }
  // Definições de voz rápidas (da folha da chamada e do ecrã da chamada).
  function openVoiceSettings() {
    const s = S.settings;
    const nm = noiseMode();
    modal(`<h2>Voz e vídeo</h2>
      <div class="field"><label>Supressão de ruído</label>
        <div class="seg vs-seg" role="radiogroup" aria-label="Supressão de ruído">${[["deep", "Forte"], ["light", "Leve"], ["off", "Nenhuma"]].map(([k, l]) => `<button role="radio" aria-checked="${nm === k}" class="${nm === k ? "on" : ""}" data-vs-noise="${k}">${l}</button>`).join("")}</div>
        <small class="hint" id="vs-noise-h">${nm === "deep" ? "Tira ventoinhas, teclado e barulho da rua. Gasta mais bateria." : nm === "light" ? "Gasta muito pouco. Tira o ruído mais constante." : "Sem filtro: usa só o do sistema."}</small></div>
      <div class="field"><label for="vs-sens">Sensibilidade do microfone</label>
        <input class="range" type="range" min="0" max="100" id="vs-sens" value="${s.sensitivity ?? 40}">
        <small class="hint">Abaixo deste nível o microfone não transmite. Mais para a esquerda apanha mais som.</small></div>
      <div class="box">
        <div class="row"><div class="txt"><b>Mostrar quem está sem vídeo</b><small>Desligado, a grelha só mostra câmaras e ecrãs.</small></div>${sw("callShowNoVideo", s.callShowNoVideo !== false, "Mostrar quem está sem vídeo")}</div>
        <div class="row"><div class="txt"><b>Cancelamento de eco</b><small>Ligado se usas o altifalante.</small></div>${sw("echoCancel", s.echoCancel !== false, "Cancelamento de eco")}</div>
        <div class="row"><div class="txt"><b>Sons da chamada</b><small>Entrar, sair, micro e som.</small></div>${sw("callSounds", s.callSounds !== false, "Sons da chamada")}</div>
      </div>
      <div class="actions"><button class="btn ghost" data-act="settings-voz" data-close>Todas as definições</button><button class="btn primary" data-close>Feito</button></div>`,
    (el) => {
      el.addEventListener("click", async (e) => {
        const b = e.target.closest("[data-vs-noise]"); if (!b) return;
        const k = b.dataset.vsNoise;
        await patch(k === "off" ? { noiseSuppression: false } : { noiseSuppression: true, noiseMode: k });
        el.querySelectorAll("[data-vs-noise]").forEach((x) => { x.classList.toggle("on", x === b); x.setAttribute("aria-checked", x === b); });
        $("#vs-noise-h", el).textContent = k === "deep" ? "Tira ventoinhas, teclado e barulho da rua. Gasta mais bateria." : k === "light" ? "Gasta muito pouco. Tira o ruído mais constante." : "Sem filtro: usa só o do sistema.";
        haptic();
      });
      $("#vs-sens", el).onchange = (e) => patch({ sensitivity: +e.target.value });
    });
  }

  async function joinVoice(id, opts = {}) {
    if (S.voice && S.voice.channelId === id) { S.view = "voice"; S.navOpen = false; render(); return; }
    if (S.voice) sfx.leave();
    let info;
    try { info = await api.join_voice({ channelId: id, noise: noiseMode(), sensitivity: S.settings.sensitivity ?? 40 }); }
    catch (e) {
      const g = S.groups.find((x) => (S.allChannels[x.id] || []).some((y) => y.id === id));
      let msg = String(e.message || e);
      if (hostOffline(g)) msg = (g.owner === "me" ? "O teu PC" : "O PC de " + who(g.owner).name) + " está desligado. Abre a Tasca nesse PC e tenta outra vez.";
      if (opts.quiet) return msg;
      return toast(msg);
    }
    sfx.join();
    S.voice = { channelId: id, groupId: S.groupId, ...info };
    S.voiceStates = {};
    if (S.muted || S.deaf) api.set_voice_state({ muted: S.muted, deaf: S.deaf });
    applyUserVolumes();
    syncVideo();
    if (!micAllowed()) { S.muted = true; api.set_voice_state({ muted: true, deaf: S.deaf }); toast("Entraste só a ouvir: o microfone está desligado nas permissões."); }
    startVad();
    S.channels = await api.list_channels({ groupId: S.groupId });
    S.allChannels[S.groupId] = S.channels;
    S.view = "voice"; S.navOpen = false; render();
    return null;
  }
  async function leaveVoice() {
    sfx.leave();
    await api.leave_voice();
    stopVad();
    stopCamera(); stopShare(true);
    const gid = S.voice.groupId; S.voice = null;
    S.allChannels[gid] = await api.list_channels({ groupId: gid });
    if (S.groupId === gid) S.channels = S.allChannels[gid];
    if (S.view === "voice") S.view = "chat";
    render(); toast("Saíste da chamada.");
  }

  function applyThemeOnly() {
    const t = S.settings.theme;
    if (t === "system") document.documentElement.removeAttribute("data-theme");
    else document.documentElement.setAttribute("data-theme", t);
  }
  function applyTheme() {
    const t = S.settings.theme;
    if (t === "system") document.documentElement.removeAttribute("data-theme");
    else document.documentElement.setAttribute("data-theme", t);
    document.body.style.fontSize = S.settings.fontSize + "px";
    sfx.setEnabled(S.settings.callSounds !== false);
    sfx.setVolume(S.settings.soundVolume ?? 0.6);
    if (sfx.setOutput) sfx.setOutput(S.settings.outputDevice);
  }
  async function patch(p) { S.settings = await api.set_settings({ patch: p }); voiceSettingsChanged(p); }
  // supressão de ruído: "deep" (DeepFilterNet), "light" (RNNoise) ou "off"
  const noiseMode = () => (S.settings.noiseSuppression === false ? "off" : S.settings.noiseMode === "light" ? "light" : "deep");
  // Volume e silêncio de cada pessoa só para ti (definidos no menu de cada membro).
  function applyUserVolumes() {
    if (!S.voice || api.isMock) return;
    const vol = S.settings.userVolume || {}, mut = S.settings.userMuted || {};
    for (const id of new Set([...Object.keys(vol), ...Object.keys(mut)])) {
      api.set_user_volume({ userId: id, volume: mut[id] ? 0 : (vol[id] ?? 100) / 100 }).catch(() => {});
    }
  }
  // Definições mudadas a meio de uma chamada.
  function voiceSettingsChanged(p) {
    if (!S.voice || api.isMock || !p) return;
    if ("inputDevice" in p || "outputDevice" in p || "echoCancel" in p) startVad();
    if ("noiseSuppression" in p || "noiseMode" in p || "sensitivity" in p) {
      api.set_voice_options({ noise: noiseMode(), sensitivity: S.settings.sensitivity ?? 40 }).catch(() => {});
      if ("noiseSuppression" in p) startVad(); // a do sistema liga-se quando a nossa está desligada
    }
    if ("userVolume" in p || "userMuted" in p) applyUserVolumes();
  }
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
    if (d.dm) return openDm(d.dm);
    if (d.chan) return openChannel(d.chan);
    if (d.chanName) {
      e.preventDefault();
      const c = S.channels.find((x) => x.name === d.chanName);
      if (c) openChannel(c.id);
      return;
    }
    if (d.voice) {
      // telemóvel: primeiro a folha da chamada (quem lá está, micro, entrar)
      if (window.innerWidth <= 720 && !(S.voice && S.voice.channelId === d.voice)) return openVoiceSheet(d.voice);
      return joinVoice(d.voice);
    }
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
    if (d.retry) {
      const m = S.messages.find((x) => String(x.id) === d.retry);
      if (m) { m.failed = false; m.pending = true; rerenderKeepInput(); trySend(S.channelId, m); }
      return;
    }
    if (d.react) {
      const [mid, i] = d.react.split(":");
      const m = S.messages.find((x) => String(x.id) === mid); const r = m.reactions[+i];
      r.mine = !r.mine; r.n += r.mine ? 1 : -1; return render();
    }
    if (d.editChan) return openChannelEditor({ id: d.editChan });
    if (d.editCat) return openCategoryEditor(d.editCat);
    if (d.setTab) { S.settingsTab = d.setTab; return render(); }
    if (d.themeSet) { await patch({ theme: d.themeSet }); applyTheme(); return render(); }
    if (d.status) { S.myStatus = d.status; S.autoAway = false; S.statusOpen = false; api.set_presence({ status: d.status, text: S.customStatus || "" }).catch(() => {}); render(); return toast("Estado: " + STATUS.find((x) => x[0] === d.status)[1] + "."); }
    if (d.msgStyle) { await patch({ msgStyle: d.msgStyle }); return render(); }
    if (d.relay) { await patch({ relay: d.relay }); return render(); }
    if (d.dest) { await patch({ backup: { dest: d.dest } }); return render(); }
    if (d.every) { await patch({ backup: { every: d.every } }); return render(); }
    switch (d.act) {
      case "home": {
        // como no balcão: começa pelos amigos; as conversas ficam na barra ao lado
        S.home = true; S.view = window.tascaAmigos ? "friends" : "chat"; S.channelId = null; S.messages = []; S.navOpen = false; return render();
      }
      case "dm-call": case "dm-video": return toast("Chamadas privadas: em breve. Vão diretamente entre os dois PCs.");
      case "new-dm": return toast("Escolhe alguém da lista de membros de um grupo para começar a conversar.");
      case "new-group": return openNewGroup();
      case "plugins":
        // os plugins vivem nas definições; carregar outra vez no puzzle fecha
        if (S.view === "settings" && S.settingsTab === "plugins") { S.view = S.prevView || "chat"; return render(); }
        S.prevView = S.view === "settings" ? S.prevView : S.view; S.settingsTab = "plugins"; S.view = "settings"; S.navOpen = false; return render();
      case "settings": S.prevView = S.view === "settings" ? S.prevView : S.view; S.view = "settings"; S.navOpen = false; return render();
      case "settings-voz": S.prevView = S.view; S.settingsTab = "voz"; S.view = "settings"; return render();
      case "close-settings": S.view = S.prevView || "chat"; return render();
      case "mute":
        if (S.muted && !micAllowed()) return toast("O microfone está desligado em Definições > Privacidade e permissões.");
        // ligar o micro com o som desligado liga também o som (como no Discord)
        S.muted = !S.muted; (S.muted ? sfx.mute : sfx.unmute)(); if (!S.muted && S.deaf) { S.deaf = false; S.mutedBeforeDeaf = false; }
        api.set_voice_state({ muted: S.muted, deaf: S.deaf }); store.set("voice-state", { muted: S.muted, deaf: S.deaf, before: !!S.mutedBeforeDeaf }); haptic(); return render();
      case "deaf":
        // tirar o som desliga o micro; voltar a ligar repõe o micro como estava antes
        if (!S.deaf) { S.mutedBeforeDeaf = S.muted; S.deaf = true; S.muted = true; sfx.deafen(); }
        else { S.deaf = false; S.muted = !!S.mutedBeforeDeaf || !micAllowed(); sfx.undeafen(); }
        api.set_voice_state({ muted: S.muted, deaf: S.deaf }); store.set("voice-state", { muted: S.muted, deaf: S.deaf, before: !!S.mutedBeforeDeaf }); haptic(); return render();
      case "leave-voice": return leaveVoice();
      case "status": S.statusOpen = !S.statusOpen; return render();
      case "noise": await patch({ noiseSuppression: !S.settings.noiseSuppression }); render(); return toast(S.settings.noiseSuppression ? "Supressão de ruído ligada." : "Supressão de ruído desligada.");
      case "camera":
        if (!S.camera && !camAllowed()) return toast("A câmara está desligada em Definições > Privacidade e permissões.");
        if (S.camera) stopCamera();
        else { if (!(await startCamera())) return; S.camera = true; }
        api.set_camera({ on: S.camera }).catch(() => {}); syncVideo(); render();
        return toast(S.camera ? "Câmara ligada." : "Câmara desligada.");
      case "share":
        if (S.sharing) { stopShare(); return render(); }
        return openSharePicker();
      case "call-sidebar": S.callSidebar = !S.callSidebar; return render();
      case "call-min": { S.view = "chat"; if (S.groupId !== S.voice.groupId) { await openGroup(S.voice.groupId); return; } if (!channel() || channel().kind !== "text") { const t = S.channels.find((x) => x.kind === "text"); if (t) return openChannel(t.id); } return render(); }
      case "invite-call": { if (S.groupId !== S.voice.groupId) await openGroup(S.voice.groupId); return openInvite(); }
      case "voice-quick": return openVoiceSettings();
      case "voicepre-back": S.view = S.prevView && S.prevView !== "voicepre" ? S.prevView : "chat"; return render();
      case "voicepre-invite": { const c = findChannel(S.preVoice); const g = S.groups.find((x) => (S.allChannels[x.id] || []).some((y) => y.id === c.id)); if (g && S.groupId !== g.id) await openGroup(g.id); S.view = "voicepre"; render(); return openInvite(); }
      case "vp-mic": haptic(); if (S.muted && !micAllowed()) return toast("O microfone está desligado em Definições > Privacidade e permissões."); S.muted = !S.muted; if (!S.muted && S.deaf) { S.deaf = false; S.mutedBeforeDeaf = false; } return render();
      case "vp-deaf": haptic(); if (!S.deaf) { S.mutedBeforeDeaf = S.muted; S.deaf = true; S.muted = true; } else { S.deaf = false; S.muted = !!S.mutedBeforeDeaf; } return render();
      case "vp-join": {
        haptic(12); S.preVoiceBusy = true; S.preVoiceErr = null; render();
        const fail = await joinVoice(S.preVoice, { quiet: true });
        S.preVoiceBusy = false;
        if (fail) { S.preVoiceErr = fail; if (S.view === "voicepre") render(); }
        return;
      }
      case "call-chat": { const gid = S.voice.groupId; if (S.groupId !== gid) await openGroup(gid); else await openChannel(S.channels.find((x) => x.kind === "text").id); return; }
      case "fullscreen": {
        try {
          if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
          else { const el = $(".callview") || $(".spotlight"); el.requestFullscreen().catch(() => {}); }
        } catch {}
        return;
      }
      case "invite": return openInvite();
      case "group-menu": return openGroupMenu();
      case "transfer": return openTransfer(d.groupId);
      case "toggle-members":
        if (window.innerWidth <= 1100) S.membersOpen = !S.membersOpen;
        else { S.showMembers = !S.showMembers; store.set("members", S.showMembers); }
        return render();
      case "nav": S.navOpen = !S.navOpen; return render();
      case "copy-key": return S.me.username ? copy("@" + S.me.username, null, "ID copiado.") : openProfile();
      case "my-card": S.statusOpen = false; render(); return window.tascaAmigos && window.tascaAmigos.openFullProfile("me");
      case "switch-account": S.statusOpen = false; if (S.voice) await leaveVoice(); await api.select_account({ id: null }); return location.reload();
      case "sign-out": S.statusOpen = false; if (S.voice) await leaveVoice(); await api.sign_out(); return location.reload();
      case "attach": return toast("No Tauri abre o seletor de ficheiros. O ficheiro vai direto para os PCs dos membros.");
      case "emoji": return window.tascaEmoji.open(t, $("#msg-input"));
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
      case "test-sound": sfx.join(); return;
      case "lock": await api.lock(); location.reload(); return;
      case "google-backup": t.disabled = true; t.textContent = "A ligar…"; await api.google_backup(); t.textContent = "Ligado"; return toast("Cópia cifrada guardada no Google Drive.");
      case "test-relay": return toast("Relay respondeu em 12 ms.");
      case "new-category": return openCategoryEditor(null);
      case "new-channel": document.querySelectorAll(".scrim:not(.locked)").forEach((x) => x.remove()); return openChannelEditor({ cat: d.cat });
      case "edit-profile": return openProfile();
      case "read-terms": return openTerms();
      case "check-update": return checkUpdate(true);
      case "copy-pubkey": return copy(S.me.key, null, "Chave copiada.");
    }
  });

  // ---------------- botão direito: canais e categorias ----------------
  const mutedChans = new Set(store.get("muted-chans", []));
  const isMuted = (id) => mutedChans.has(id);
  function toggleMute(id) {
    if (mutedChans.has(id)) mutedChans.delete(id); else mutedChans.add(id);
    store.set("muted-chans", [...mutedChans]); render();
  }
  const ID_IC = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="5" width="18" height="14" rx="3"/><path d="M8 9v6M12 9h2a3 3 0 0 1 0 6h-2z"/></svg>';
  async function duplicateChannel(g, c) {
    const n = await api.create_channel({ groupId: g.id, name: c.kind === "voice" ? c.name + " (cópia)" : c.name + "-copia", kind: c.kind, topic: c.topic, cat: c.cat });
    const order = S.channels.map((x) => ({ id: x.id, cat: x.cat })).filter((x) => x.id !== n.id);
    order.splice(order.findIndex((x) => x.id === c.id) + 1, 0, { id: n.id, cat: c.cat });
    await api.reorder_channels({ groupId: g.id, order });
    await refreshChannels(g.id); render(); toast("Canal duplicado.");
  }
  async function moveChannelTo(g, c, cat) {
    const order = S.channels.map((x) => ({ id: x.id, cat: x.cat })).filter((x) => x.id !== c.id);
    let at = -1; order.forEach((x, i) => { if (x.cat === cat) at = i; });
    order.splice(at < 0 ? order.length : at + 1, 0, { id: c.id, cat });
    await api.reorder_channels({ groupId: g.id, order });
    await refreshChannels(g.id); render(); toast(`Canal movido para ${cat}.`);
  }
  async function deleteChannelNow(g, c) {
    try {
      if (S.voice && S.voice.channelId === c.id) await leaveVoice();
      await api.delete_channel({ groupId: g.id, channelId: c.id });
      await refreshChannels(g.id);
      if (S.channelId === c.id) { const t = S.channels.find((x) => x.kind === "text"); if (t) await openChannel(t.id); }
      render(); toast("Canal apagado.");
    } catch (x) { toast(String(x.message || x)); }
  }
  function confirmDelete(g, c) {
    modal(`<h2>Apagar ${c.kind === "voice" ? "" : "#"}${esc(c.name)}?</h2>
      <p>${c.kind === "text" ? "As mensagens deste canal são apagadas para toda a gente. Não dá para desfazer." : "O canal de voz desaparece para toda a gente."}</p>
      <div class="actions"><button class="btn ghost" data-close>Cancelar</button><button class="btn danger" id="do-del">Apagar canal</button></div>`,
    (el, close) => { $("#do-del", el).onclick = () => { close(); deleteChannelNow(g, c); }; });
  }
  function channelMenu(x, y, c) {
    const g = group(); if (!g) return;
    const own = g.owner === "me";
    const cats = [...new Set([...(S.cats[g.id] || []), ...S.channels.map((z) => z.cat)])];
    const items = [
      { label: "Marcar como lida", disabled: !c.unread, onClick: () => { c.unread = 0; render(); } },
      "-",
      { label: "Convidar para o grupo", onClick: () => openInvite() },
      { label: "Copiar link", onClick: () => copy(`tasca://grupo/${g.id}/canal/${c.id}`, null, "Link do canal copiado.") },
      "-",
      { label: isMuted(c.id) ? "Deixar de silenciar" : "Silenciar canal", hint: isMuted(c.id) ? "Sem avisos deste canal" : "", onClick: () => { toggleMute(c.id); toast(isMuted(c.id) ? "Canal silenciado." : "Canal com avisos outra vez."); } },
    ];
    if (own) items.push("-",
      { label: "Mover para…", items: cats.map((k) => ({ label: k, checked: k === c.cat, disabled: k === c.cat, onClick: () => moveChannelTo(g, c, k) })) },
      "-",
      { label: "Editar canal", onClick: () => openChannelEditor({ id: c.id }) },
      { label: "Duplicar canal", onClick: () => duplicateChannel(g, c) },
      { label: "Criar canal de texto", onClick: () => openChannelEditor({ cat: c.cat, kind: "text" }) },
      { label: "Criar canal de voz", onClick: () => openChannelEditor({ cat: c.cat, kind: "voice" }) },
      { label: "Apagar canal", danger: true, onClick: () => confirmDelete(g, c) });
    items.push("-", { label: "Copiar ID do canal", icon: ID_IC, onClick: () => copy(c.id, null, "ID copiado.") });
    tascaMenu.open(x, y, items);
  }
  function categoryMenu(x, y, cat) {
    const g = group(); if (!g) return;
    const own = g.owner === "me";
    const list = S.channels.filter((c) => c.cat === cat);
    const items = [
      { label: "Marcar tudo como lido", disabled: !list.some((c) => c.unread), onClick: () => { list.forEach((c) => (c.unread = 0)); render(); } },
      { label: list.every((c) => isMuted(c.id)) && list.length ? "Deixar de silenciar a categoria" : "Silenciar categoria", disabled: !list.length, onClick: () => { const all = list.every((c) => isMuted(c.id)); list.forEach((c) => (all ? mutedChans.delete(c.id) : mutedChans.add(c.id))); store.set("muted-chans", [...mutedChans]); render(); } },
    ];
    if (own) items.push("-",
      { label: "Criar canal de texto", onClick: () => openChannelEditor({ cat, kind: "text" }) },
      { label: "Criar canal de voz", onClick: () => openChannelEditor({ cat, kind: "voice" }) },
      { label: "Criar categoria", onClick: () => openCategoryEditor(null) },
      "-",
      { label: "Editar categoria", onClick: () => openCategoryEditor(cat) },
      { label: "Apagar categoria", danger: true, onClick: () => openCategoryEditor(cat) });
    tascaMenu.open(x, y, items);
  }
  document.addEventListener("contextmenu", (e) => {
    if (!e.target.closest) return;
    const b = e.target.closest("[data-chan], [data-voice], [data-drag-chan]");
    const h = e.target.closest(".cat");
    const g = group();
    if (b && b.closest(".channels") && g) {
      const id = b.dataset.chan || b.dataset.voice || b.dataset.dragChan;
      const c = S.channels.find((x) => x.id === id); if (!c) return;
      e.preventDefault(); return channelMenu(e.clientX, e.clientY, c);
    }
    if (h && h.closest(".channels") && g) {
      const cat = h.dataset.dragCat || (h.querySelector("span") || {}).textContent;
      e.preventDefault(); return categoryMenu(e.clientX, e.clientY, h.dataset.dragCat || cat);
    }
  });

  // fechar painel de membros ao clicar fora (ecrãs estreitos)
  document.addEventListener("pointerdown", (e) => {
    if (S.statusOpen && !e.target.closest(".status-pop") && !e.target.closest('[data-act="status"]')) { S.statusOpen = false; render(); }
    if (S.membersOpen && !e.target.closest(".members") && !e.target.closest('[data-act="toggle-members"]')) { S.membersOpen = false; render(); }
  });

  document.addEventListener("submit", async (e) => {
    if (e.target.id !== "composer") return;
    e.preventDefault();
    const inp = $("#msg-input");
    const text = inp.value.trim();
    if (!text) return;
    inp.value = "";
    // aparece logo, mais escura, até o anfitrião confirmar
    const m = { id: "tmp-" + Date.now(), day: "Hoje", at: new Date().toTimeString().slice(0, 5), from: "me", text, pending: true };
    S.messages.push(m); render(); $("#msg-input").focus();
    trySend(S.channelId, m);
  });
  // Envia (ou volta a enviar) uma mensagem. Se falhar, ou se ninguém confirmar em 20 s,
  // fica marcada como não enviada, com o botão para tentar outra vez.
  async function trySend(channelId, m) {
    m.pending = true; m.failed = false;
    const localId = m.id;
    try {
      const r = await api.send_message({ channelId, text: m.text });
      Object.assign(m, r, { failed: false });
      seen.add(String(m.id));
      if (S.home) S.dms = await api.list_dms();
    } catch (x) {
      m.pending = false; m.failed = true;
      console.warn("mensagem não enviada:", x);
    }
    if (S.view === "chat") rerenderKeepInput();
    setTimeout(() => {
      if (m.pending && !m.failed) { m.pending = false; m.failed = true; if (S.view === "chat") rerenderKeepInput(); }
    }, 20000);
    return localId;
  }
  api.on("message_failed", ({ channelId, id }) => {
    const m = S.messages.find((x) => x.id === id);
    if (m && channelId === S.channelId) { m.pending = false; m.failed = true; if (S.view === "chat") rerenderKeepInput(); }
  });
  document.addEventListener("click", async (e) => {
    const b = e.target.closest("[data-noise-mode]");
    if (!b) return;
    await patch({ noiseMode: b.dataset.noiseMode }); render();
  });
  // Carregar para falar: só transmite enquanto a tecla V estiver carregada (com a Tasca à frente).
  let pttDown = false;
  const typing = (t) => t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable);
  window.addEventListener("keydown", (e) => { if ((e.key === "v" || e.key === "V") && !typing(e.target)) pttDown = true; });
  window.addEventListener("keyup", (e) => { if (e.key === "v" || e.key === "V") pttDown = false; });
  window.addEventListener("blur", () => { pttDown = false; });
  if (window.vozAudio) {
    vozAudio.transmit = () => !S.muted && !S.deaf && (!S.settings.pushToTalk || pttDown);
    vozAudio.onFrame = ({ speaking }) => { if (speaking !== meSpeaking) { meSpeaking = speaking; applySpeaking(); } };
  }
  document.addEventListener("keydown", (e) => {
    if (e.target.id === "msg-input" && e.key === "Enter" && !e.shiftKey) {
      e.preventDefault(); e.target.form.requestSubmit();
    }
    if (e.key === "Escape") {
      const sc = document.querySelector(".scrim:not(.locked)");
      if (document.querySelector(".scrim.locked") && !sc) return;
      if (sc) closeScrim(sc);
      else if (S.view === "settings") { S.view = S.prevView || "chat"; render(); }
    }
  });
  document.addEventListener("input", async (e) => {
    const t = e.target;
    if (t.id === "msg-input") { t.style.height = "auto"; t.style.height = Math.min(t.scrollHeight, 160) + "px"; return; }
    if (t.id === "custom-status") { S.customStatus = t.value; const w = $(".me .who small"); if (w) w.textContent = t.value || STATUS.find((x) => x[0] === S.myStatus)[1]; return; }
    if (t.id === "dm-q") {
      S.dmQuery = t.value; const pos = t.selectionStart;
      const ch = $(".channels"); $("#sidebar").innerHTML = renderSidebar(); attachMedia();
      const n = $("#dm-q"); n.focus(); n.setSelectionRange(pos, pos); return;
    }
    if (t.id === "plugin-q") {
      S.pluginQuery = t.value; const pos = t.selectionStart; render();
      const n = $("#plugin-q"); n.focus(); n.setSelectionRange(pos, pos); return;
    }
    if (t.dataset.setting && t.type === "range") {
      const v = +t.value;
      await patch(pathPatch(t.dataset.setting, v));
      if (t.dataset.setting === "fontSize") applyTheme();
      if (t.dataset.setting === "soundVolume") { await patch({ soundVolume: v / 100 }); applyTheme(); t.closest(".row").querySelector(".txt small").textContent = v + "%"; return; }
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
    if (t.id === "custom-status") {
      if (window.tascaSugg) tascaSugg.remember("estado", t.value);
      store.set("custom-status", t.value);
      api.set_presence({ status: myStatus(), text: t.value }).catch(() => {});
      return;
    }
    if (t.id === "sideload" && t.files[0]) { toast(`«${t.files[0].name}» carregado. Revê as permissões antes de ativar.`); return; }
    if (t.dataset.setting && t.type === "checkbox" && t.dataset.setting.startsWith("perms.")) {
      await patch(pathPatch(t.dataset.setting, t.checked)); return applyPerm(t.dataset.setting.slice(6), t.checked);
    }
    if (t.dataset.setting && t.type === "checkbox") { await patch(pathPatch(t.dataset.setting, t.checked)); if (t.dataset.setting === "callSounds") applyTheme(); return; }
    if (t.dataset.setting && t.tagName === "SELECT") {
      await patch(pathPatch(t.dataset.setting, t.value));
      if (t.dataset.setting === "inputDevice") startMicMeter();
      if (t.dataset.setting === "outputDevice" && window.sfx && sfx.setOutput) sfx.setOutput(t.value);
      return;
    }
    if (t.dataset.setting && t.type !== "range") return patch(pathPatch(t.dataset.setting, t.value));
  });

  // tooltips da barra de grupos
  let tipEl;
  document.addEventListener("pointerover", (e) => {
    // só com rato: no toque a dica ficava presa por cima de tudo
    if (e.pointerType && e.pointerType !== "mouse") { if (tipEl) { tipEl.remove(); tipEl = null; } return; }
    const b = e.target.closest("[data-tip]");
    if (!b) { if (tipEl) { tipEl.remove(); tipEl = null; } return; }
    if (!tipEl) { tipEl = document.createElement("div"); tipEl.className = "tip"; document.body.appendChild(tipEl); }
    const r = b.getBoundingClientRect();
    tipEl.textContent = b.dataset.tip;
    tipEl.style.left = r.right + 12 + "px"; tipEl.style.top = r.top + r.height / 2 - 15 + "px";
  });

  // ---------------- eventos do núcleo ----------------
  api.on("message", ({ channelId, groupId, message }) => {
    if (S.typing && S.typing.channelId === channelId) S.typing = null;
    if (channelId === S.channelId && S.view === "chat") {
      // a tua própria mensagem: se foi escrita aqui já está na lista
      const ja = S.messages.find((x) => x.id === message.id);
      if (ja) { if (ja.pending) { ja.pending = false; rerenderKeepInput(); } return; }
      S.messages.push(message); render(); return;
    }
    // as tuas (escritas noutro dispositivo) não contam como não lidas
    if (message && message.from === "me") return;
    // noutro canal: conta como não lida
    if (!S.groups || !S.allChannels) return;
    const c = groupId && (S.allChannels[groupId] || []).find((x) => x.id === channelId);
    if (c) c.unread = (c.unread || 0) + 1;
    const g = groupId && S.groups.find((x) => x.id === groupId);
    if (g && (S.home || S.groupId !== groupId)) g.unread = (g.unread || 0) + 1;
    if (c || g) rerenderKeepInput();
  });
  // o grupo mudou no núcleo (alguém entrou, canais novos, grupo acabado de chegar)
  api.on("group_changed", async ({ groupId }) => {
    if (!S.groups || !S.allChannels) return;
    try {
      S.people = await api.list_people();
      const fresh = await api.list_groups();
      for (const g of fresh) {
        const old = S.groups.find((x) => x.id === g.id);
        if (old) Object.assign(old, g, { unread: old.unread }); else S.groups.push(g);
      }
      if (!S.groups.some((x) => x.id === groupId)) return;
      await refreshChannels(groupId);
      S.allMembers[groupId] = await api.list_members({ groupId });
      if (S.groupId === groupId) S.members = S.allMembers[groupId];
      rerenderKeepInput();
    } catch (x) { console.warn("grupo não atualizado:", x); }
  });
  // o PC do anfitrião ligou-se ou desligou-se
  api.on("host_status", ({ groupId, online }) => {
    const g = S.groups && S.groups.find((x) => x.id === groupId);
    if (!g) return;
    g.host = { ...g.host, online, lastSeen: online ? g.host.lastSeen : "há pouco" };
    rerenderKeepInput();
  });
  // perfil mudado noutro dispositivo da conta
  api.on("profile_synced", () => setTimeout(async () => {
    try { S.people = await api.list_people(); rerenderKeepInput(); } catch (x) { console.warn(x); }
  }, 50));
  api.on("message_ack", ({ channelId, id }) => {
    const m = S.messages.find((x) => x.id === id);
    if (m && channelId === S.channelId) {
      m.pending = false;
      // só tira a classe: a mensagem passa de "a enviar" a enviada com uma transição
      const el = document.querySelector(`.msg[data-mid="${CSS.escape(String(id))}"]`);
      if (el) el.classList.remove("pending"); else if (S.view === "chat") rerenderKeepInput();
    }
  });
  api.on("typing", ({ channelId, who }) => { S.typing = { channelId, who }; if (S.view === "chat") rerenderKeepInput(); });
  // Quem está a falar: os outros chegam por evento; tu és detetado aqui, pelo microfone.
  let remoteSpeaking = [], meSpeaking = false;
  api.on("voice_activity", ({ speaking }) => {
    remoteSpeaking = api.isMock ? speaking : speaking.filter((x) => x !== "me");
    applySpeaking();
  });
  // Estado da ligação da chamada (ping, rota) e quem está sem microfone ou sem som.
  api.on("voice_status", (st) => {
    if (!S.voice) return;
    const antes = S.voice.connected;
    Object.assign(S.voice, { ping: st.ping, route: st.route, connected: st.connected });
    if (antes !== undefined && antes !== st.connected) {
      toast(st.connected ? "Voltaste a ligar à chamada." : "Perdeste a ligação à chamada. A tentar outra vez…");
      rerenderKeepInput();
    }
  });
  api.on("voice_members", ({ channelId, members }) => {
    if (!S.voice || S.voice.channelId !== channelId) return;
    S.voiceStates = Object.fromEntries(members.filter((m) => m.id !== "me").map((m) => [m.id, m]));
    if (S.view !== "chat") rerenderKeepInput();
  });
  const vad = { stream: null, ctx: null, timer: 0, lastLoud: 0 };
  async function startVad() {
    // Na app instalada, o áudio da chamada (microfone e colunas) corre em voz.js.
    if (!api.isMock && window.vozAudio) {
      if (!S.voice) return;
      vozAudio.video.subscribe();
      return vozAudio.start({
        inputDevice: S.settings.inputDevice, outputDevice: S.settings.outputDevice,
        echoCancel: S.settings.echoCancel, noise: noiseMode(), mic: micAllowed(),
      }).catch((e) => { console.warn(e); toast("Não consegui ligar o som da chamada."); });
    }
    stopVad();
    if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia || !micAllowed()) return;
    const dev = S.settings.inputDevice;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: {
        ...(dev && dev !== "default" ? { deviceId: { exact: dev } } : {}),
        echoCancellation: S.settings.echoCancel !== false, noiseSuppression: !!S.settings.noiseSuppression, autoGainControl: true,
      } });
      if (!S.voice) { stream.getTracks().forEach((t) => t.stop()); return; }
      const ctx = new AudioContext(); const an = ctx.createAnalyser(); an.fftSize = 512;
      ctx.createMediaStreamSource(stream).connect(an);
      const buf = new Uint8Array(an.fftSize);
      Object.assign(vad, { stream, ctx });
      vad.timer = setInterval(() => {
        an.getByteTimeDomainData(buf);
        let peak = 0; for (const v of buf) peak = Math.max(peak, Math.abs(v - 128));
        const level = micLevel(peak / 128);
        const now = Date.now();
        if (level >= (S.settings.sensitivity ?? 40)) vad.lastLoud = now;
        const on = !S.muted && !S.deaf && now - vad.lastLoud < 300; // fica aceso um pouco depois de parares
        if (on !== meSpeaking) { meSpeaking = on; applySpeaking(); }
      }, 60);
    } catch (x) { console.warn("microfone indisponível:", x); }
  }
  function stopVad() {
    if (!api.isMock && window.vozAudio) { vozAudio.stop(); vozAudio.video.clear(); }
    clearInterval(vad.timer);
    if (vad.stream) vad.stream.getTracks().forEach((t) => t.stop());
    if (vad.ctx) vad.ctx.close().catch(() => {});
    Object.assign(vad, { stream: null, ctx: null, timer: 0 });
    if (meSpeaking) { meSpeaking = false; applySpeaking(); }
  }
  // Só troca as classes: não volta a desenhar o ecrã (os vídeos não piscam).
  function applySpeaking() {
    const set = new Set(remoteSpeaking);
    if (meSpeaking) set.add("me");
    S.speaking = [...set];
    document.querySelectorAll("[data-spk]").forEach((el) => {
      const id = el.dataset.spk;
      el.classList.toggle(el.dataset.spkCls || "speaking", S.speaking.includes(id) && !(id === "me" && S.muted));
    });
  }
  api.on("backup_progress", ({ pct }) => { S.backupPct = pct; if (S.view === "settings" && S.settingsTab === "backups") rerenderKeepInput(); });

  function rerenderKeepInput() {
    const inp = $("#msg-input");
    const val = inp ? inp.value : "", focused = document.activeElement === inp;
    const pg = $(".page"); const st = pg ? pg.scrollTop : 0;
    render();
    const n = $("#msg-input"); if (n) { n.value = val; if (focused) n.focus(); }
    const pg2 = $(".page"); if (pg2) pg2.scrollTop = st;
  }

  // Para os outros ficheiros da interface (amigos.js) e, mais tarde, para os plugins.
  window.tascaApp = { bannerStyle, patch, S, I, FLOR, LOGO, $, esc, initials, avatar, toast, copy, modal, render, rerenderKeepInput, openDm, openGroup, openProfile, openTransfer, group, store, squarePhoto, myStatus, STATUS };
  window.tascaToast = toast;

  // ---------------- arranque ----------------
  // Se algo falhar ao arrancar, mostra o erro em vez de deixar a janela vazia.
  function startFailed(e) {
    console.error("arranque falhou", e);
    const box = document.createElement("div");
    box.id = "start-error";
    box.style.cssText = "position:fixed;inset:0;z-index:9999;display:grid;place-items:center;background:#1C1714;color:#F6EEE1;font-family:Inter,system-ui,sans-serif;padding:24px";
    const msg = String((e && (e.message || e)) || "erro desconhecido");
    box.innerHTML = '<div style="max-width:520px;text-align:center"><h2 style="margin:0 0 12px">A Tasca não conseguiu arrancar</h2>' +
      '<p style="opacity:.8;margin:0 0 16px">Manda este erro a quem te está a ajudar:</p>' +
      '<pre style="white-space:pre-wrap;text-align:left;background:#27201B;border-radius:10px;padding:12px;user-select:text"></pre>' +
      '<button style="margin-top:16px;padding:10px 18px;border:0;border-radius:10px;background:#9B2C3F;color:#F6EEE1;font:inherit;cursor:pointer">Tentar outra vez</button></div>';
    box.querySelector("pre").textContent = msg;
    box.querySelector("button").onclick = () => location.reload();
    document.body.appendChild(box);
  }
  window.addEventListener("error", (e) => { if (!S.groups) startFailed(e.error || e.message); });

  // ---------------- telemóvel: atualizações ----------------
  // No PC a app atualiza-se sozinha. No Android não dá para instalar sem a pessoa confirmar,
  // por isso a app vê se há versão nova e abre o APK para instalar por cima.
  let appVersion = null;
  if (window.__TAURI__ && window.__TAURI__.core) window.__TAURI__.core.invoke("app_info").then((i) => { appVersion = i.version; }).catch(() => {});
  const verNum = (v) => String(v || "").replace(/^v/, "").split(".").map((x) => parseInt(x, 10) || 0);
  function newer(a, b) {
    const x = verNum(a), y = verNum(b);
    for (let i = 0; i < 3; i++) if ((x[i] || 0) !== (y[i] || 0)) return (x[i] || 0) > (y[i] || 0);
    return false;
  }
  async function checkUpdate(manual) {
    if (!window.TASCA_MOBILE || window.api.isMock) return;
    try {
      if (!appVersion) appVersion = (await window.__TAURI__.core.invoke("app_info")).version;
      const r = await fetch("https://api.github.com/repos/RubenFernandes2004/Tasca/releases/latest", { headers: { Accept: "application/vnd.github+json" } });
      if (!r.ok) throw new Error("HTTP " + r.status);
      const rel = await r.json();
      const latest = String(rel.tag_name || "").replace(/^v/, "");
      const apk = (rel.assets || []).some((a) => a.name === "Tasca.apk");
      if (apk && newer(latest, appVersion)) offerUpdate(latest);
      else if (manual) toast(`Já tens a versão mais recente (${appVersion}).`);
    } catch (x) {
      console.warn("atualizações:", x);
      if (manual) toast("Não consegui ver se há versão nova. Tenta daqui a pouco.");
    }
  }
  function offerUpdate(latest) {
    if (document.getElementById("upd-go")) return;
    modal(`<h2>Há uma versão nova</h2>
      <p>Tens a ${esc(appVersion)} e já saiu a ${esc(latest)}. Carrega em Atualizar e depois em «Instalar» quando o Android perguntar. As contas e as conversas ficam.</p>
      <div class="actions"><button class="btn" data-close>Agora não</button><button class="btn primary" id="upd-go">Atualizar</button></div>`, (m, close) => {
      m.querySelector("#upd-go").onclick = () => {
        // a app descarrega e abre o instalador; sem isso (versões antigas), vai pelo browser
        if (window.TascaAndroid && window.TascaAndroid.atualizar) {
          const b = m.querySelector("#upd-go");
          b.disabled = true; b.textContent = "A transferir…";
          window.tascaAtualizacao = ({ estado, pct, erro }) => {
            if (estado === "a_transferir") b.textContent = `A transferir… ${pct}%`;
            else if (estado === "a_instalar") { b.textContent = "A abrir o instalador…"; setTimeout(close, 1500); }
            else if (estado === "permissao") {
              b.disabled = false; b.textContent = "Atualizar";
              toast("Autoriza a Tasca a instalar apps e carrega outra vez em Atualizar.");
            } else if (estado === "erro") {
              b.disabled = false; b.textContent = "Tentar outra vez";
              toast("A transferência falhou: " + (erro || "erro desconhecido"));
            }
          };
          window.TascaAndroid.atualizar();
          return;
        }
        window.__TAURI__.core.invoke("abrir_atualizacao").catch((e) => toast(String(e)));
        close();
      };
    });
  }
  if (window.TASCA_MOBILE) {
    setTimeout(() => checkUpdate(false), 4000);
    // ao voltar à app (depois de estar em segundo plano), vê outra vez, no máximo de hora a hora
    let ultimaVista = Date.now();
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "visible" && Date.now() - ultimaVista > 3600e3) { ultimaVista = Date.now(); checkUpdate(false); }
    });
  }

  // ---------------- telemóvel: botão de voltar e gestos ----------------
  if (window.TASCA_MOBILE) {
    const estreito = () => window.innerWidth <= 720;
    // Voltar: fecha o que estiver aberto, um passo de cada vez; no fim, duas vezes para sair.
    let ultimoVoltar = 0;
    const voltar = () => {
      if (document.querySelector(".ctx-menu")) return window.tascaMenu.close();
      const sc = document.querySelector(".scrim:not(.locked):not(.closing)");
      if (sc) return closeScrim(sc);
      if (document.querySelector(".scrim.locked")) return;
      if (document.querySelector(".emoji-pop")) return window.tascaEmoji.close();
      if (window.tascaAmigos && window.tascaAmigos.cardOpen && window.tascaAmigos.cardOpen()) return window.tascaAmigos.closeCard();
      if (S.msgMenu) return closeMsgSheet();
      if (S.reply || S.editing) { S.reply = null; S.editing = null; return rerenderKeepInput(); }
      if (S.search) { S.search = null; return render(); }
      if (S.statusOpen) { S.statusOpen = false; return render(); }
      if (S.membersOpen) { S.membersOpen = false; return render(); }
      if (S.view === "settings") { S.view = S.prevView || "chat"; return render(); }
      if (S.view === "voice" || S.view === "voicepre") { S.view = "chat"; return render(); }
      if (estreito() && !S.navOpen) { S.navOpen = true; return render(); }
      const agora = Date.now();
      if (agora - ultimoVoltar < 2000) { window.__TAURI__.core.invoke("sair_app").catch(() => {}); return; }
      ultimoVoltar = agora; toast("Carrega outra vez em voltar para sair.");
    };
    try { window.__TAURI__.app.onBackButtonPress(voltar); } catch (x) { console.warn("botão de voltar", x); }

    // Deslizar o dedo: a gaveta de grupos e canais acompanha o dedo (abre para a direita,
    // fecha para a esquerda); a lista de membros fecha a deslizar para a direita.
    let t0 = null;
    const appEl = () => $("#app");
    const navW = () => Math.min(372, window.innerWidth - 44) + 16;
    document.addEventListener("touchstart", (e) => {
      const t = e.touches[0];
      const alvo = e.target;
      const ignora = e.touches.length > 1 || S.view === "voice" || !estreito() ||
        (alvo.closest && alvo.closest("input[type=range], .call-strip, .seg, .scrim, .ctx-menu, textarea, .emoji-pop, .pcard, .fr-tabs, .reactions, pre, code"));
      t0 = ignora ? null : { x: t.clientX, y: t.clientY, at: Date.now(), drag: null, open: !!S.navOpen, members: !!S.membersOpen };
    }, { passive: true });
    document.addEventListener("touchmove", (e) => {
      if (!t0) return;
      const t = e.touches[0];
      const dx = t.clientX - t0.x, dy = t.clientY - t0.y;
      if (t0.drag == null) {
        if (Math.abs(dx) < 12 && Math.abs(dy) < 12) return;
        // só conta como gesto de lado se for mais horizontal do que vertical
        if (Math.abs(dy) > Math.abs(dx) * 0.7) { t0 = null; return; }
        if (t0.members) t0.drag = "members";
        else if (t0.open ? dx < 0 : dx > 0) t0.drag = "nav";
        else { t0 = null; return; }
        appEl().classList.add("dragging");
      }
      const a = appEl();
      if (t0.drag === "nav") {
        const w = navW();
        const x = t0.open ? Math.min(0, dx) : Math.min(0, -w + dx);
        a.style.setProperty("--navx", x + "px");
        a.style.setProperty("--scrim", String(Math.max(0, Math.min(1, 1 + x / w))));
        if (!t0.open && !a.classList.contains("show-nav-drag")) a.classList.add("show-nav-drag");
      } else if (t0.drag === "members") {
        const m = $("#members"); if (m) { m.style.transition = "none"; m.style.transform = `translateX(${Math.max(0, dx)}px)`; }
      }
    }, { passive: true });
    document.addEventListener("touchend", (e) => {
      if (!t0) return;
      const t = e.changedTouches[0];
      const dx = t.clientX - t0.x, dy = t.clientY - t0.y, ms = Date.now() - t0.at;
      const drag = t0.drag, was = t0;
      t0 = null;
      const a = appEl();
      a.classList.remove("dragging", "show-nav-drag");
      a.style.removeProperty("--navx"); a.style.removeProperty("--scrim");
      const m = $("#members"); if (m) { m.style.transition = ""; m.style.transform = ""; }
      if (!drag) return;
      const fast = ms < 300 && Math.abs(dx) > 40;
      if (drag === "nav") {
        const far = Math.abs(dx) > navW() * 0.35;
        if (!was.open && dx > 0 && (far || fast)) { S.navOpen = true; haptic(); render(); }
        else if (was.open && dx < 0 && (far || fast)) { S.navOpen = false; render(); }
      } else if (drag === "members" && dx > 0 && (dx > 90 || fast)) { S.membersOpen = false; render(); }
      void dy;
    }, { passive: true });
  }

  // Tocar fora da gaveta ou da lista de membros (no fundo escuro) fecha-as.
  document.addEventListener("click", (e) => {
    if (e.target && e.target.id === "app") {
      if (S.membersOpen) { S.membersOpen = false; render(); }
      else if (S.navOpen && window.innerWidth <= 720) { S.navOpen = false; render(); }
    }
    const b = e.target.closest && e.target.closest("[data-banner-toggle]");
    if (b) { S.bannerOpen = !S.bannerOpen; b.classList.toggle("open", S.bannerOpen); b.setAttribute("aria-expanded", S.bannerOpen); }
  });

  // Puxar a conversa para baixo (já no topo) volta a ler as mensagens e os grupos.
  if (window.TASCA_MOBILE) {
    let p0 = null, pull = 0, busy = false;
    const ptr = () => { let el = $("#main .ptr"); if (!el) { el = document.createElement("div"); el.className = "ptr"; el.innerHTML = I.retry; $("#main").appendChild(el); } return el; };
    document.addEventListener("touchstart", (e) => {
      const list = e.target.closest && e.target.closest(".messages, .fr-page");
      p0 = list && list.scrollTop <= 0 && !busy && e.touches.length === 1 ? { y: e.touches[0].clientY, x: e.touches[0].clientX } : null; pull = 0;
    }, { passive: true });
    document.addEventListener("touchmove", (e) => {
      if (!p0) return;
      const dy = e.touches[0].clientY - p0.y, dx = e.touches[0].clientX - p0.x;
      if (Math.abs(dx) > Math.abs(dy)) { p0 = null; return; }
      pull = Math.max(0, Math.min(110, dy * 0.5));
      const el = ptr();
      el.style.setProperty("--ptr", (pull - 40) + "px"); el.style.setProperty("--ptr-o", Math.min(1, pull / 60)); el.style.setProperty("--ptr-r", pull * 4 + "deg");
    }, { passive: true });
    document.addEventListener("touchend", async () => {
      if (!p0) return; p0 = null;
      const el = $("#main .ptr"); if (!el) return;
      if (pull < 70) { el.remove(); return; }
      busy = true; haptic(12); el.classList.add("spin"); el.style.setProperty("--ptr", "20px");
      try { await refreshAll(); } catch (x) { console.warn(x); }
      busy = false; const e2 = $("#main .ptr"); if (e2) e2.remove();
    });
  }
  async function refreshAll() {
    S.people = await api.list_people();
    const fresh = await api.list_groups();
    for (const g of fresh) { const old = S.groups.find((x) => x.id === g.id); if (old) Object.assign(old, g, { unread: old.unread }); else S.groups.push(g); }
    S.dms = await api.list_dms();
    if (S.home && S.view === "friends" && window.tascaAmigos && window.tascaAmigos.refresh) await window.tascaAmigos.refresh();
    if (!S.home && S.groupId) { await refreshChannels(S.groupId); S.members = await api.list_members({ groupId: S.groupId }); S.allMembers[S.groupId] = S.members; }
    if (S.channelId && S.view === "chat") S.messages = await api.list_messages({ channelId: S.channelId });
    rerenderKeepInput();
  }

  (async function start() {
    try { S.settings = await api.get_settings(); applyThemeOnly(); } catch {}
    await window.tascaAuth.gate();
    S.me = await api.get_me();
    S.people = await api.list_people();
    S.groups = await api.list_groups();
    S.plugins = await api.list_plugins();
    S.dms = await api.list_dms();
    S.customStatus = store.get("custom-status", "");
    if (window.tascaAmigos) await window.tascaAmigos.start();
    S.settings = await api.get_settings();
    S.allChannels = {}; S.allMembers = {}; S.cats = {};
    for (const g of S.groups) {
      S.allChannels[g.id] = await api.list_channels({ groupId: g.id });
      S.allMembers[g.id] = await api.list_members({ groupId: g.id });
    }
    applyTheme();
    const saved = store.get("group", null);
    if (!S.groups.length) { S.home = true; S.view = "chat"; S.channelId = null; S.messages = []; render(); }
    else await openGroup(S.groups.some((g) => g.id === saved) ? saved : S.groups[0].id);
    await termsGate(); // a app fica visível por trás, mas só se usa depois de aceitar
  })().catch(startFailed);
})();
