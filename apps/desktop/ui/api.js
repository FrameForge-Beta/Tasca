// Tasca — camada de dados da interface.
//
// Todas as chamadas da UI passam por `api`. Dentro da app Tauri, cada função chama o
// comando Rust com o mesmo nome (snake_case) via `invoke`. No browser (pré-visualização),
// usa os dados de teste abaixo. Os eventos (mensagem nova, alguém a falar, anfitrião
// offline) chegam por `api.on(evento, fn)`, que no Tauri liga a `listen()`.
//
// Comandos Rust esperados (tasca-app):
//   get_me, list_groups, list_channels, list_messages, send_message, list_members,
//   account_status, create_account, unlock, recover, google_sign_in, google_restore,
//   google_backup, lock, username_available, server_register, server_login,
//   list_accounts, select_account, sign_out, suggest_usernames, save_phrase_desktop,
//   list_dms, create_group, join_group, create_invite, join_voice, leave_voice, set_voice_state,
//   set_voice_options, set_user_volume, voice_frame (áudio em bytes, ver voz.js),
//   set_camera, list_capture_sources, start_screen_share, stop_screen_share,
//   list_plugins, set_plugin_enabled, install_plugin, get_settings, set_settings,
//   run_backup, list_backups, transfer_ownership,
//   list_friends, send_friend_request, accept_friend, decline_friend, cancel_friend, remove_friend,
//   block_user, unblock_user, set_presence, list_dm_messages, send_dm, close_dm (amigos.rs, dms.rs)
// Eventos: "message", "message_ack", "group_changed", "host_status", "typing", "voice_activity",
//   "voice_status" (ping, rota, ligação), "voice_members" (quem está sem microfone/som),
//   "video_track", "backup_progress", "friends_changed", "friend_request", "friend_presence", "dms_changed"
//
// Na app instalada, grupos, canais, membros e mensagens vêm do núcleo (apps/desktop/src/grupos.rs);
// ver `real` mais abaixo. Amigos e conversas privadas: amigos.rs e dms.rs. Plugins e cópias ainda são simulados.

(function () {
  const tauri = window.__TAURI__ && window.__TAURI__.core;

  // ---------------- dados de teste ----------------
  const people = {
    me:   { id: "me",   name: "Tiago",    color: "#9B2C3F", key: "ed25519:7Hk2…Qp9x", status: "online" },
    rui:  { id: "rui",  name: "Rui",      color: "#2A5CAA", key: "ed25519:Rm4c…1aZe", status: "online", username: "rui.dev", bio: "Anfitrião da Malta. Rust e cafés." },
    ines: { id: "ines", name: "Inês",     color: "#5E8C4A", key: "ed25519:In8s…v0Lk", status: "online", username: "ines_m", bio: "🎧 sempre em chamada" },
    marta:{ id: "marta",name: "Marta",    color: "#B8791A", key: "ed25519:Mt2a…9cWw", status: "away", username: "marta_cx", bio: "Xadrez às quintas." },
    joao: { id: "joao", name: "João",     color: "#6B4C9A", key: "ed25519:Jo5a…oR3t", status: "online", username: "joaozinho", bio: "" },
    bia:  { id: "bia",  name: "Beatriz",  color: "#2E7D7A", key: "ed25519:Bz7t…ee1Q", status: "offline", username: "bia.t", bio: "" },
    diogo:{ id: "diogo",name: "Diogo",    color: "#7A3B2E", key: "ed25519:Dg0o…Hh2s", status: "offline", username: "diogo", bio: "" },
    carol:{ id: "carol",name: "Carolina", color: "#4A6FA5", key: "ed25519:Cr1l…xx8p", status: "online", username: "carol", bio: "Fotografia e viagens." },
    bot:  { id: "bot",  name: "Lembretes",color: "#2A5CAA", key: "plugin:lembretes", status: "online", bot: true },
  };

  const groups = [
    { id: "g-malta", name: "Malta do Secundário", short: "MS", owner: "rui", color: "#9B2C3F",
      host: { online: true, route: "direto", ping: 18 }, unread: 3 },
    { id: "g-tasca", name: "Projeto Tasca", short: "PT", owner: "me", color: "#2A5CAA",
      host: { online: true, route: "este PC", ping: 0 }, unread: 0 },
    { id: "g-xadrez", name: "Clube de Xadrez", short: "CX", owner: "marta", color: "#5E8C4A",
      host: { online: true, route: "via relay", ping: 64 }, unread: 0 },
    { id: "g-sexta", name: "Jogos de Sexta", short: "JS", owner: "diogo", color: "#B8791A",
      host: { online: false, lastSeen: "ontem às 23:40" }, unread: 0 },
  ];

  const cats = {}; // ordem das categorias por grupo (as vazias também ficam)
  const channels = {
    "g-malta": [
      { id: "c-geral", kind: "text", cat: "Conversa", name: "geral", topic: "Tudo e mais alguma coisa. Sejam simpáticos." },
      { id: "c-memes", kind: "text", cat: "Conversa", name: "memes", topic: "Só memes.", unread: 2 },
      { id: "c-jantar", kind: "text", cat: "Conversa", name: "jantar-de-natal", topic: "Dia 19, no sítio do costume. Quem vem?", unread: 1 },
      { id: "v-sala", kind: "voice", cat: "Voz", name: "Sala de estar", inVoice: ["rui", "ines", "joao"] },
      { id: "v-jogos", kind: "voice", cat: "Voz", name: "Jogos", inVoice: [] },
    ],
    "g-tasca": [
      { id: "c-dev", kind: "text", cat: "Desenvolvimento", name: "núcleo-rust", topic: "iroh, SQLite, protocolo de eventos." },
      { id: "c-ui", kind: "text", cat: "Desenvolvimento", name: "interface", topic: "Tauri + HTML. Capturas de ecrã aqui." },
      { id: "c-plugins", kind: "text", cat: "Desenvolvimento", name: "plugins", topic: "Revisão de plugins para a loja." },
      { id: "v-reuniao", kind: "voice", cat: "Voz", name: "Reunião", inVoice: [] },
    ],
    "g-xadrez": [
      { id: "c-partidas", kind: "text", cat: "Clube", name: "partidas", topic: "Partilhem as vossas partidas (PGN)." },
      { id: "c-torneio", kind: "text", cat: "Clube", name: "torneio-dezembro", topic: "Inscrições até dia 30." },
      { id: "v-analise", kind: "voice", cat: "Voz", name: "Análise", inVoice: ["marta"] },
    ],
    "g-sexta": [
      { id: "c-planos", kind: "text", cat: "Geral", name: "planos", topic: "O que jogamos esta sexta?" },
      { id: "v-lobby", kind: "voice", cat: "Voz", name: "Lobby", inVoice: [] },
    ],
  };

  const members = {
    "g-malta": ["rui", "me", "ines", "joao", "carol", "marta", "bia", "diogo", "bot"],
    "g-tasca": ["me", "rui"],
    "g-xadrez": ["marta", "me", "joao", "bia"],
    "g-sexta": ["diogo", "me", "rui", "joao", "carol"],
  };

  const today = "Hoje";
  const messages = {
    "c-geral": [
      { id: 1, day: "Ontem", at: "22:14", from: "joao", text: "alguém viu o jogo de ontem? que segunda parte" },
      { id: 2, day: "Ontem", at: "22:15", from: "joao", text: "o golo aos 89 foi ridículo" },
      { id: 3, day: "Ontem", at: "22:31", from: "carol", text: "vi! o estádio veio abaixo", reactions: [{ e: "⚽", n: 3, mine: true }] },
      { id: 4, day: today, at: "18:02", from: "rui", text: "Malta, passámos esta tasca para a Tasca 🎉 Está alojada no meu PC, por isso se eu desligar ficam a ver a cópia local até eu voltar." },
      { id: 5, day: today, at: "18:03", from: "rui", text: "O convite para quem faltar está no canal #jantar-de-natal." },
      { id: 6, day: today, at: "18:05", from: "ines", text: "Finalmente! Já instalei o plugin de lembretes, funciona muito bem.", reactions: [{ e: "👍", n: 4, mine: false }, { e: "🍷", n: 2, mine: true }] },
      { id: 7, day: today, at: "18:06", from: "bot", text: "Lembrete criado: **Jantar de Natal** a 19 de dezembro, 20:30. Aviso 1 dia antes." },
      { id: 8, day: today, at: "18:11", from: "marta", text: "@Tiago consegues trazer as colunas para o jantar?", mentionsMe: true },
      { id: 9, day: today, at: "18:12", from: "me", text: "Consigo, levo as pequenas que cabem no carro." },
      { id: 10, day: today, at: "18:20", from: "joao", text: "deixei aqui a lista de quem já confirmou", file: { name: "confirmados.xlsx", size: "12 KB", ext: "XLS" } },
      { id: 11, day: today, at: "18:24", from: "ines", text: "Estamos na Sala de estar se alguém quiser aparecer 🎧" },
    ],
    "c-memes": [
      { id: 1, day: today, at: "17:40", from: "joao", text: "quando o host desliga o PC a meio da call" },
      { id: 2, day: today, at: "17:41", from: "carol", text: "😂😂" },
    ],
    "c-jantar": [
      { id: 1, day: today, at: "16:10", from: "rui", text: "Convite para quem ainda não entrou:" },
      { id: 2, day: today, at: "16:10", from: "rui", text: "`tasca1qx7f3k9w2m…v8hd0` (válido 7 dias)" },
    ],
    "c-dev": [
      { id: 1, day: today, at: "19:30", from: "me", text: "Núcleo em Rust: identidade ed25519, grupos com log de eventos assinado, SQLite para o histórico." },
      { id: 2, day: today, at: "19:41", from: "rui", text: "E o iroh trata do NAT? Tenho a fibra com CGNAT." },
      { id: 3, day: today, at: "19:42", from: "me", text: "Trata. Se não der ligação direta passa pelo relay; o meu mini servidor pode ser um relay próprio." },
    ],
    "c-partidas": [
      { id: 1, day: today, at: "15:00", from: "marta", text: "Partida de ontem: 1. e4 e5 2. Cf3 Cc6 3. Bb5 a6. Quem quer analisar logo à noite?" },
    ],
    "c-planos": [
      { id: 1, day: "Ontem", at: "21:00", from: "diogo", text: "Esta sexta: torneio de Mario Kart às 22h." },
      { id: 2, day: "Ontem", at: "21:04", from: "rui", text: "Conta comigo." },
    ],
  };

  // mensagens privadas: ligação direta entre os dois PCs, sem anfitrião
  const dms = [
    { id: "dm-ines", with: "ines", unread: 2 },
    { id: "dm-rui", with: "rui", unread: 0 },
    { id: "dm-marta", with: "marta", unread: 0 },
    { id: "dm-joao", with: "joao", unread: 0 },
    { id: "dm-bia", with: "bia", unread: 0 },
  ];
  Object.assign(messages, {
    "dm-ines": [
      { id: 1, day: "Ontem", at: "23:02", from: "ines", text: "Viste o vídeo que te mandei?" },
      { id: 2, day: "Ontem", at: "23:05", from: "me", text: "Vi! Muito bom 😂" },
      { id: 3, day: today, at: "18:30", from: "ines", text: "Vens à chamada logo? Vamos jogar às 22h" },
      { id: 4, day: today, at: "18:31", from: "ines", text: "E traz o comando extra se puderes" },
    ],
    "dm-rui": [
      { id: 1, day: today, at: "17:50", from: "rui", text: "Já passei o grupo para a Tasca. Se der algum erro diz-me." },
      { id: 2, day: today, at: "17:52", from: "me", text: "Combinado. Até agora está tudo a funcionar." },
    ],
    "dm-marta": [
      { id: 1, day: "Ontem", at: "15:12", from: "marta", text: "Obrigada pela partida de ontem, foi renhida!" },
    ],
    "dm-joao": [
      { id: 1, day: "Segunda", at: "21:40", from: "joao", text: "manda-me a lista do jantar quando tiveres" },
      { id: 2, day: "Segunda", at: "21:45", from: "me", text: "Já está no #jantar-de-natal" },
    ],
    "dm-bia": [
      { id: 1, day: "Domingo", at: "12:00", from: "bia", text: "Parabéns pelo projeto! 🎉" },
    ],
  });

  const plugins = [
    { id: "lembretes", name: "Lembretes", author: "equipa Tasca", version: "1.2.0", verified: true, installed: true, enabled: true,
      runtime: "WASM", color: "#2A5CAA", icon: "bell",
      desc: "Cria lembretes no chat com /lembrar. Avisa todo o grupo à hora marcada.",
      perms: ["ler mensagens", "enviar mensagens"], installs: "2,1 mil" },
    { id: "sons-tasca", name: "Sons de Tasca", author: "rui.dev", version: "0.4.1", verified: true, installed: true, enabled: false,
      runtime: "JS", color: "#9B2C3F", icon: "music",
      desc: "Painel de sons para as chamadas de voz. Inclui o clássico «saúde!».",
      perms: ["áudio da chamada"], installs: "830" },
    { id: "xadrez", name: "Tabuleiro de Xadrez", author: "marta_cx", version: "2.0.0", verified: true, installed: false, enabled: false,
      runtime: "WASM", color: "#5E8C4A", icon: "grid",
      desc: "Mostra partidas em PGN como tabuleiro interativo e deixa jogar dentro do canal.",
      perms: ["ler mensagens", "painel próprio"], installs: "412" },
    { id: "traduz", name: "Tradutor", author: "equipa Tasca", version: "1.0.3", verified: true, installed: false, enabled: false,
      runtime: "WASM", color: "#E3A72F", icon: "globe",
      desc: "Traduz mensagens com um clique, no teu PC, sem enviar texto para fora.",
      perms: ["ler mensagens"], installs: "1,4 mil" },
    { id: "spotify-status", name: "A ouvir agora", author: "joaozinho", version: "0.2.0", verified: false, installed: false, enabled: false,
      runtime: "JS", color: "#6B4C9A", icon: "music",
      desc: "Mostra no teu perfil a música que estás a ouvir. Ainda em revisão pela equipa.",
      perms: ["rede: api.spotify.com", "editar perfil"], risky: ["rede: api.spotify.com"], installs: "—" },
    { id: "temas", name: "Temas de Azulejo", author: "equipa Tasca", version: "1.1.0", verified: true, installed: false, enabled: false,
      runtime: "CSS", color: "#2A5CAA", icon: "palette",
      desc: "Seis temas extra inspirados em azulejos de Lisboa, Porto e Coimbra.",
      perms: ["aparência"], installs: "3,0 mil" },
  ];

  // amigos (teste): rel = amigo | enviado | recebido | bloqueado
  const DAY = 86400000;
  const friends = [
    { id: "rui", rel: "amigo", since: Date.now() - 400 * DAY },
    { id: "ines", rel: "amigo", since: Date.now() - 120 * DAY },
    { id: "marta", rel: "amigo", since: Date.now() - 30 * DAY },
    { id: "bia", rel: "amigo", since: Date.now() - 9 * DAY },
    { id: "carol", rel: "recebido", since: Date.now() - 3600000 },
    { id: "diogo", rel: "enviado", since: Date.now() - 2 * DAY },
  ];
  people.ines.statusText = "a jogar Mario Kart";

  const settings = {
    theme: "dark",
    fontSize: 15,
    compact: false,
    inputDevice: "default",
    outputDevice: "default",
    sensitivity: 62,
    noiseSuppression: true,
    noiseMode: "deep",
    echoCancel: true,
    pushToTalk: false,
    relay: "public",
    relayUrl: "https://relay.omeuservidor.pt",
    backup: {
      dest: "gdrive",
      connected: { gdrive: "conta Google ligada", onedrive: null, local: "D:\\Backups\\Tasca" },
      auto: true,
      every: "diario",
      zstdLevel: 19,
      avif: true,
      groups: ["g-tasca"],
      history: [
        { at: "20 nov, 04:00", size: "3,8 MB", note: "incremental", ok: true },
        { at: "19 nov, 04:00", size: "1,1 MB", note: "incremental", ok: true },
        { at: "18 nov, 04:00", size: "42,6 MB", note: "completo", ok: true },
      ],
    },
  };

  // ---------------- eventos ----------------
  // Na app instalada não há dados de teste: só a tua conta, até o núcleo P2P estar ligado.
  if (tauri) {
    for (const k of Object.keys(people)) if (k !== "me") delete people[k];
    Object.assign(people.me, { name: "Eu", key: "" });
    groups.length = 0; dms.length = 0; friends.length = 0;
    for (const o of [channels, cats, members, messages]) for (const k of Object.keys(o)) delete o[k];
  }

  // Na app instalada, os grupos, canais e mensagens vivem no núcleo (Rust). Aqui no
  // browser só fica o perfil (cor e foto), por conta. Versões antigas guardavam também
  // grupos de teste aqui: na primeira vez passam para o núcleo (só nome e canais).
  let localKey = null;
  function persist() {
    if (!tauri || !localKey) return;
    try { localStorage.setItem("tasca:local:" + localKey, JSON.stringify({ v: 2, profile: { color: people.me.color, photo: people.me.photo || null, bio: people.me.bio || "", banner: people.me.banner || null } })); } catch {}
  }
  async function loadLocal(key) {
    if (!tauri || !key || localKey === key) return;
    localKey = key;
    let d = null; try { d = JSON.parse(localStorage.getItem("tasca:local:" + key)); } catch {}
    if (!d) return;
    if (d.profile && d.profile.color) people.me.color = d.profile.color;
    if (d.profile && d.profile.photo) people.me.photo = d.profile.photo;
    if (d.profile && d.profile.bio) people.me.bio = d.profile.bio;
    if (d.profile && d.profile.banner) people.me.banner = d.profile.banner;
    if (d.v !== 2) {
      for (const g of d.groups || []) {
        if (g.owner !== "me") continue;
        try {
          await inv("import_local_group", { name: g.name, channels: ((d.channels || {})[g.id] || []).map((c) => ({ id: c.id, kind: c.kind, name: c.name, topic: c.topic || "", cat: c.cat })), categories: (d.cats || {})[g.id] || null });
        } catch (e) { console.warn("grupo antigo não importado:", e); }
      }
      persist();
    }
  }
  const mockProfile = {
    get() { try { return JSON.parse(localStorage.getItem("tasca:mock-profile")) || {}; } catch { return {}; } },
    set(v) { try { localStorage.setItem("tasca:mock-profile", JSON.stringify(v)); } catch {} },
  };

  const listeners = {};
  function emit(ev, data) { (listeners[ev] || []).forEach((fn) => fn(data)); }

  let nextId = 1000;
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const nowHHMM = () => new Date().toTimeString().slice(0, 5);
  const clone = (x) => JSON.parse(JSON.stringify(x));

  // conta (teste): guardada no browser para a pré-visualização lembrar-se de ti
  const acct = {
    get() { try { return JSON.parse(localStorage.getItem("tasca:mock-account")); } catch { return null; } },
    set(v) { try { localStorage.setItem("tasca:mock-account", JSON.stringify(v)); } catch {} },
  };
  let unlocked = false;
  let noneSel = (() => { try { return sessionStorage.getItem("tasca:mock-none") === "1"; } catch { return false; } })(); // "Trocar de conta"
  const remember = (on) => { const a = acct.get(); if (a) acct.set({ ...a, remember: !!on }); };
  const WORDS = "abandon ability able about above absent absorb abstract absurd abuse access accident acoustic acquire across action actor actress actual adapt address adjust admit adult advance advice aerobic afford afraid again agent agree ahead aim air airport aisle alarm album alcohol alert alien".split(" ");
  const phrase = () => Array.from(crypto.getRandomValues(new Uint8Array(12))).map((b) => WORDS[b % WORDS.length]).join(" ");

  const mock = {
    async account_status() {
      const a = noneSel ? null : acct.get();
      if (a && a.remember && !unlocked) { unlocked = true; people.me.name = a.displayName; }
      return { exists: !!a, unlocked: unlocked && !!a, displayName: a ? a.displayName : null, google: a ? a.google || null : null, server: true, username: a ? a.username || null : null, remembered: !!(a && a.remember) };
    },
    async list_accounts() {
      const a = acct.get();
      return a ? [{ id: "teste", displayName: a.displayName, username: a.username || null, remembered: !!a.remember, active: !noneSel }] : [];
    },
    async idle_seconds() { return null; },
    async set_presence() { return true; },
    async select_account({ id }) { noneSel = !id; try { sessionStorage.setItem("tasca:mock-none", noneSel ? "1" : "0"); } catch {} unlocked = false; return true; },
    async sign_out() { remember(false); unlocked = false; return true; },
    async suggest_usernames({ username }) {
      await wait(300);
      const b = username.replace(/^@/, "").toLowerCase().replace(/[^a-z0-9_.]/g, "").slice(0, 19) || "tasca";
      return [b + "_", b + ".pt", b + (10 + Math.floor(Math.random() * 90)), "o." + b];
    },
    async username_available({ username }) {
      await wait(300);
      if (!/^@?[a-z0-9_.]{3,24}$/i.test(username)) throw new Error("O nome de utilizador tem de ter entre 3 e 24 caracteres: letras, números, _ e .");
      return username.replace(/^@/, "").toLowerCase() !== "tiago";
    },
    async server_register({ username }) { await wait(400); const u = username.replace(/^@/, "").toLowerCase(); const a = acct.get(); if (a) acct.set({ ...a, username: u }); return u; },
    async server_login({ username, password, remember: r }) {
      await wait(900);
      if (password.length < 8) throw new Error("Nome de utilizador ou palavra-passe errados.");
      const u = username.replace(/^@/, "").toLowerCase();
      acct.set({ displayName: u, pw: password.length, google: null, username: u, remember: !!r }); noneSel = false; unlocked = true; people.me.name = u; return true;
    },
    async create_account({ displayName, password, remember: r }) {
      await wait(700); // Argon2id demora de propósito
      acct.set({ displayName, pw: password.length, google: null, remember: !!r }); noneSel = false;
      unlocked = true; people.me.name = displayName;
      return { recoveryPhrase: phrase() };
    },
    async unlock({ password, remember: r }) {
      await wait(500);
      const a = acct.get();
      if (!a || password.length !== a.pw) throw new Error("Palavra-passe errada.");
      remember(r); unlocked = true; people.me.name = a.displayName; return true;
    },
    async recover({ phrase: p, displayName, newPassword, remember: r }) {
      await wait(700);
      if (p.trim().split(/\s+/).length !== 12) throw new Error("A frase de recuperação tem de ter 12 palavras.");
      const a = { ...(acct.get() || {}), displayName: displayName || "Eu" };
      acct.set({ ...a, pw: newPassword.length, remember: !!r }); noneSel = false; unlocked = true; people.me.name = a.displayName; return true;
    },
    async google_sign_in() { await wait(1500); return { email: "o.teu.email@gmail.com", hasBackup: true }; },
    async google_restore({ password, remember: r }) {
      await wait(700);
      if (password.length < 8) throw new Error("Palavra-passe errada para a cópia guardada no Google Drive.");
      acct.set({ displayName: "Eu", pw: password.length, google: "conta Google ligada", remember: !!r }); noneSel = false; unlocked = true; people.me.name = "Eu"; return true;
    },
    async google_backup() { await wait(1200); const a = acct.get(); if (a) acct.set({ ...a, google: "conta Google ligada" }); return { email: "conta Google ligada" }; },
    async save_phrase_desktop({ phrase }) {
      // No browser não há ambiente de trabalho: descarrega o ficheiro.
      const a = document.createElement("a");
      a.href = URL.createObjectURL(new Blob([phrase.split(" ").map((w, i) => `${i + 1}. ${w}`).join("\r\n")], { type: "text/plain" }));
      a.download = "Tasca - 12 palavras.txt"; a.click();
      return "Transferências/Tasca - 12 palavras.txt";
    },
    async lock() { unlocked = false; return true; },
    async get_me() {
      if (tauri) {
        try {
          const s = await tauri.invoke("account_status");
          if (s.displayName) people.me.name = s.displayName;
          if (s.publicKey) people.me.key = "ed25519:" + s.publicKey;
          people.me.username = s.username || null;
          await loadLocal(s.publicKey);
          if (s.unlocked) publishProfile();
        } catch {}
      }
      else {
        const a = acct.get(); if (a && a.username) people.me.username = a.username;
        const pr = mockProfile.get(); if (pr.color) people.me.color = pr.color; people.me.photo = pr.photo || null;
        people.me.bio = pr.bio || ""; people.me.banner = pr.banner || null;
        if (a && a.displayName && unlocked) people.me.name = a.displayName;
      }
      return clone(people.me);
    },
    async list_people() { return clone(people); },
    async list_groups() { return clone(groups); },
    async list_channels({ groupId }) { return clone(channels[groupId] || []); },
    async list_members({ groupId }) {
      const g = groups.find((x) => x.id === groupId);
      return (members[groupId] || []).map((id) => ({ ...clone(people[id]), owner: g && g.owner === id }));
    },
    async list_dms() {
      return dms.map((d) => {
        const list = messages[d.id] || []; const last = list[list.length - 1];
        return { ...clone(d), last: last ? { text: last.text, from: last.from, at: last.at, day: last.day } : null };
      });
    },
    async list_messages({ channelId }) { return clone(messages[channelId] || []); },
    async send_message({ channelId, text }) {
      const m = { id: nextId++, day: today, at: nowHHMM(), from: "me", text, pending: true };
      (messages[channelId] = messages[channelId] || []).push(m);
      if (tauri) { m.pending = false; persist(); setTimeout(() => emit("message_ack", { channelId, id: m.id }), 0); return clone(m); }
      // o anfitrião confirma (assina e guarda) pouco depois
      setTimeout(() => { m.pending = false; emit("message_ack", { channelId, id: m.id }); }, 600);
      if (/tasca|plugin|voz/i.test(text)) {
        setTimeout(() => emit("typing", { channelId, who: "Inês" }), 900);
        setTimeout(() => {
          const r = { id: nextId++, day: today, at: nowHHMM(), from: "ines", text: "Boa! 👌" };
          messages[channelId].push(r);
          emit("message", { channelId, message: clone(r) });
        }, 2600);
      }
      return clone(m);
    },
    // ---- perfil ----
    async set_display_name({ name }) {
      const n = String(name || "").trim();
      if (!n || n.length > 32) throw new Error("O nome tem de ter entre 1 e 32 caracteres.");
      const a = acct.get(); if (a) acct.set({ ...a, displayName: n });
      people.me.name = n; return n;
    },
    async set_profile({ color, photo, bio, banner }) {
      if (color) people.me.color = color;
      if (photo !== undefined) people.me.photo = photo || null;
      if (bio !== undefined) people.me.bio = String(bio || "").slice(0, 190);
      if (banner !== undefined) people.me.banner = banner || null;
      if (tauri) persist(); else mockProfile.set({ ...mockProfile.get(), color: people.me.color, photo: people.me.photo, bio: people.me.bio, banner: people.me.banner });
      return clone(people.me);
    },
    async change_username({ newUsername, password }) {
      await wait(500);
      if (!password) throw new Error("Escreve a tua palavra-passe.");
      const u = String(newUsername).trim().replace(/^@/, "").toLowerCase();
      if (!(await mock.username_available({ username: u }))) throw new Error("Esse nome de utilizador já está ocupado.");
      const a = acct.get(); if (a) acct.set({ ...a, username: u });
      people.me.username = u; return u;
    },
    async set_autostart({ enabled }) { return !!enabled; },
    async get_autostart() { return !!settings.perms && !!settings.perms.autostart; },
    // ---- canais (só o anfitrião) ----
    async create_channel({ groupId, name, kind, topic, cat }) {
      const g = groups.find((x) => x.id === groupId);
      if (!g || g.owner !== "me") throw new Error("Só o anfitrião pode criar canais.");
      const list = (channels[groupId] = channels[groupId] || []);
      const c = { id: (kind === "voice" ? "v-" : "c-") + nextId++, kind, name: cleanName(name, kind), topic: topic || "", cat: cat || (kind === "voice" ? "Voz" : "Conversa"), ...(kind === "voice" ? { inVoice: [] } : {}) };
      if (!c.name) throw new Error("Dá um nome ao canal.");
      // fica no fim da sua categoria
      let at = -1; list.forEach((x, i) => { if (x.cat === c.cat) at = i; });
      list.splice(at < 0 ? list.length : at + 1, 0, c);
      persist(); return clone(c);
    },
    async update_channel({ groupId, channelId, name, topic, cat }) {
      const c = (channels[groupId] || []).find((x) => x.id === channelId);
      if (!c) throw new Error("Esse canal já não existe.");
      if (name != null) { const n = cleanName(name, c.kind); if (!n) throw new Error("Dá um nome ao canal."); c.name = n; }
      if (topic != null) c.topic = topic;
      if (cat != null && cat.trim()) c.cat = cat.trim();
      persist(); return clone(c);
    },
    async delete_channel({ groupId, channelId }) {
      const list = channels[groupId] || [];
      const c = list.find((x) => x.id === channelId);
      if (!c) return true;
      if (c.kind === "text" && list.filter((x) => x.kind === "text").length <= 1) throw new Error("O grupo precisa de pelo menos um canal de texto.");
      list.splice(list.indexOf(c), 1); delete messages[channelId];
      persist(); return true;
    },
    async list_categories({ groupId }) {
      const out = [...(cats[groupId] || [])];
      for (const c of channels[groupId] || []) if (!out.includes(c.cat)) out.push(c.cat);
      return cats[groupId] = out, clone(out);
    },
    async create_category({ groupId, name }) {
      const n = String(name || "").trim().slice(0, 24);
      if (!n) throw new Error("Dá um nome à categoria.");
      const list = await mock.list_categories({ groupId });
      if (list.some((x) => x.toLowerCase() === n.toLowerCase())) throw new Error("Já há uma categoria com esse nome.");
      cats[groupId].push(n); persist(); return n;
    },
    async rename_category({ groupId, from, to }) {
      const n = String(to || "").trim().slice(0, 24);
      if (!n) throw new Error("Dá um nome à categoria.");
      await mock.list_categories({ groupId });
      if (n !== from && cats[groupId].some((x) => x.toLowerCase() === n.toLowerCase())) throw new Error("Já há uma categoria com esse nome.");
      cats[groupId] = cats[groupId].map((x) => (x === from ? n : x));
      for (const c of channels[groupId] || []) if (c.cat === from) c.cat = n;
      persist(); return n;
    },
    // Apaga a categoria; os canais dela passam para a categoria de cima (ou a de baixo).
    async delete_category({ groupId, name }) {
      const list = await mock.list_categories({ groupId });
      if (list.length <= 1) throw new Error("O grupo precisa de pelo menos uma categoria.");
      const i = list.indexOf(name); const to = list[i - 1] || list[i + 1];
      for (const c of channels[groupId] || []) if (c.cat === name) c.cat = to;
      cats[groupId] = list.filter((x) => x !== name); persist(); return to;
    },
    async reorder_categories({ groupId, order }) {
      const list = await mock.list_categories({ groupId });
      cats[groupId] = [...order.filter((x) => list.includes(x)), ...list.filter((x) => !order.includes(x))];
      persist(); return true;
    },
    // Nova ordem dos canais, cada um com a sua categoria (texto e voz podem misturar-se).
    async reorder_channels({ groupId, order }) {
      const list = channels[groupId] || [];
      const byId = Object.fromEntries(list.map((c) => [c.id, c]));
      const next = order.map((o) => byId[o.id] && Object.assign(byId[o.id], { cat: o.cat || byId[o.id].cat })).filter(Boolean);
      for (const c of list) if (!next.includes(c)) next.push(c);
      list.length = 0; list.push(...next);
      persist(); return true;
    },
    async move_channel({ groupId, channelId, dir }) {
      const list = channels[groupId] || [];
      const i = list.findIndex((x) => x.id === channelId); if (i < 0) return false;
      // troca com o vizinho da mesma categoria
      const c = list[i]; let j = i + dir;
      while (j >= 0 && j < list.length && list[j].cat !== c.cat) j += dir;
      if (j < 0 || j >= list.length) return false;
      [list[i], list[j]] = [list[j], list[i]]; persist(); return true;
    },
    async create_group({ name }) {
      const g = { id: "g-" + nextId++, name, short: name.split(/\s+/).map((w) => w[0]).join("").slice(0, 2).toUpperCase(),
        owner: "me", host: { online: true, route: "este PC", ping: 0 }, unread: 0 };
      groups.push(g);
      channels[g.id] = [
        { id: "c-" + nextId++, kind: "text", cat: "Conversa", name: "geral", topic: "Bem-vindos à " + name + "." },
        { id: "v-" + nextId++, kind: "voice", cat: "Voz", name: "Sala", inVoice: [] },
      ];
      members[g.id] = ["me"];
      persist();
      return clone(g);
    },
    async join_group({ invite }) {
      if (!/^tasca1[0-9a-z]{8,}/i.test(invite.trim())) throw new Error("Este código não parece um convite da Tasca. Os convites começam por «tasca1».");
      await wait(900);
      throw new Error("Não foi possível chegar ao anfitrião. Ele pode estar offline; tenta mais tarde.");
    },
    async create_invite({ groupId }) {
      const rnd = Array.from(crypto.getRandomValues(new Uint8Array(24))).map((b) => "0123456789abcdefghjkmnpqrstvwxyz"[b % 32]).join("");
      return { code: "tasca1" + rnd, expires: "7 dias", uses: "ilimitado" };
    },
    async join_voice({ channelId }) {
      for (const list of Object.values(channels)) {
        for (const c of list) {
          c.inVoice = (c.inVoice || []).filter((x) => x !== "me");
          if (c.id === channelId) c.inVoice.push("me");
        }
      }
      return { route: "direto P2P", ping: 18, codec: "Opus 64 kbps" };
    },
    async leave_voice() {
      for (const list of Object.values(channels)) for (const c of list) c.inVoice = (c.inVoice || []).filter((x) => x !== "me");
      return true;
    },
    async set_voice_state() { return true; },
    async set_camera() { return true; },
    async list_capture_sources() { return []; },
    async start_screen_share() { await wait(300); return true; },
    async stop_screen_share() { return true; },
    async list_plugins() { return clone(plugins); },
    async set_plugin_enabled({ id, enabled }) { const p = plugins.find((x) => x.id === id); p.enabled = enabled; return clone(p); },
    async install_plugin({ id }) { await wait(700); const p = plugins.find((x) => x.id === id); p.installed = true; p.enabled = true; return clone(p); },
    async uninstall_plugin({ id }) { const p = plugins.find((x) => x.id === id); p.installed = false; p.enabled = false; return clone(p); },
    async get_settings() {
      try { const saved = JSON.parse(localStorage.getItem("tasca:settings")); if (saved) deepMerge(settings, saved); } catch {}
      return clone(settings);
    },
    async set_settings({ patch }) {
      deepMerge(settings, patch);
      try { localStorage.setItem("tasca:settings", JSON.stringify(settings)); } catch {}
      return clone(settings);
    },
    async run_backup() {
      for (let i = 1; i <= 10; i++) { await wait(180); emit("backup_progress", { pct: i * 10 }); }
      const entry = { at: "20 nov, " + nowHHMM(), size: "0,4 MB", note: "incremental", ok: true };
      settings.backup.history.unshift(entry);
      return clone(entry);
    },
    async transfer_ownership() { await wait(500); return true; },
    // ---- amigos (teste) ----
    async list_friends() { return clone(friends.map((f) => ({ ...people[f.id], ...f, statusText: people[f.id].statusText || "" }))); },
    async send_friend_request({ username, userId, name }) {
      await wait(400);
      let id = userId;
      if (username) {
        const u = String(username).trim().replace(/^@/, "").toLowerCase();
        id = Object.keys(people).find((k) => people[k].username === u);
        if (!id) throw new Error(`Não há ninguém com o nome @${u}.`);
      }
      if (id === "me") throw new Error("Não te podes adicionar a ti próprio.");
      const f = friends.find((x) => x.id === id);
      if (f && f.rel === "amigo") throw new Error("Já são amigos.");
      if (f && f.rel === "enviado") throw new Error("Já pediste. Agora é esperar que aceite.");
      if (f && f.rel === "recebido") { f.rel = "amigo"; f.since = Date.now(); }
      else friends.push({ id, rel: "enviado", since: Date.now(), name: name || (people[id] && people[id].name) });
      emit("friends_changed", {}); return clone(friends.find((x) => x.id === id));
    },
    async accept_friend({ userId }) { const f = friends.find((x) => x.id === userId); f.rel = "amigo"; f.since = Date.now(); emit("friends_changed", {}); return true; },
    async decline_friend({ userId }) { friends.splice(friends.findIndex((x) => x.id === userId), 1); emit("friends_changed", {}); return true; },
    async cancel_friend({ userId }) { friends.splice(friends.findIndex((x) => x.id === userId), 1); emit("friends_changed", {}); return true; },
    async remove_friend({ userId }) { friends.splice(friends.findIndex((x) => x.id === userId), 1); emit("friends_changed", {}); return true; },
    async block_user({ userId }) {
      const f = friends.find((x) => x.id === userId);
      if (f) { f.rel = "bloqueado"; f.since = Date.now(); } else friends.push({ id: userId, rel: "bloqueado", since: Date.now() });
      emit("friends_changed", {}); return true;
    },
    async unblock_user({ userId }) { const i = friends.findIndex((x) => x.id === userId && x.rel === "bloqueado"); if (i >= 0) friends.splice(i, 1); emit("friends_changed", {}); return true; },
  };

  // ---------------- app instalada: comandos do núcleo ----------------
  const inv = (n, a) => tauri.invoke(n, a).catch((e) => { throw e instanceof Error ? e : new Error(String(e)); });
  const DIAS = ["Domingo", "Segunda", "Terça", "Quarta", "Quinta", "Sexta", "Sábado"];
  // mensagens do núcleo trazem `ts` (ms); a interface mostra `day` e `at`
  function fmt(m) {
    if (!m || m.ts == null) return m;
    const d = new Date(m.ts), now = new Date();
    const dia = (x) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
    const diff = Math.round((dia(now) - dia(d)) / 86400000);
    const day = diff === 0 ? today : diff === 1 ? "Ontem" : diff > 1 && diff < 7 ? DIAS[d.getDay()] : d.toLocaleDateString("pt-PT");
    return { ...m, day, at: d.toTimeString().slice(0, 5) };
  }
  // o teu perfil (cor e foto, guardados neste PC) vai para os grupos onde ainda está diferente
  // `sync`: mudaste-o agora, por isso vai também para os teus outros dispositivos
  const publishProfile = (sync) => inv("publish_profile", { color: people.me.color, photo: people.me.photo || null, bio: people.me.bio || "", banner: people.me.banner || null, sync: !!sync }).catch((e) => console.warn("perfil não publicado:", e));
  if (tauri && window.__TAURI__.event) window.__TAURI__.event.listen("host_status", (e) => { if (e.payload && e.payload.online) publishProfile(); });
  // perfil mudado noutro dispositivo da conta (PC ou telemóvel)
  if (tauri && window.__TAURI__.event) window.__TAURI__.event.listen("profile_synced", (e) => {
    const p = e.payload || {};
    if (p.color) people.me.color = p.color;
    if ("bio" in p) people.me.bio = p.bio || "";
    if ("photo" in p) people.me.photo = p.photo || null;
    if ("banner" in p) people.me.banner = p.banner || null;
    persist();
  });
  const real = {
    async list_people() { const p = clone(people); try { Object.assign(p, await inv("list_group_people")); } catch (e) { console.warn(e); } return p; },
    async list_members(a) { return (await inv("list_members", a)).map((m) => (m.id === "me" ? { ...clone(people.me), owner: m.owner } : m)); },
    async list_channels(a) { return inv("list_channels", a); },
    // conversas privadas (dm-<chave do amigo>) vão cifradas pelo servidor; ver apps/desktop/src/dms.rs
    async list_dms() { return (await inv("list_dms")).map((d) => ({ ...d, last: d.last ? fmt(d.last) : null })); },
    async list_messages(a) {
      if (String(a.channelId).startsWith("dm-")) return (await inv("list_dm_messages", { userId: a.channelId.slice(3) })).map(fmt);
      inv("close_dm").catch(() => {});
      return (await inv("list_messages", a)).map(fmt);
    },
    async send_message(a) {
      if (String(a.channelId).startsWith("dm-")) return fmt(await inv("send_dm", { userId: a.channelId.slice(3), text: a.text }));
      return fmt(await inv("send_message", a));
    },
    async create_group(a) { return inv("create_group", a); },
    async join_group(a) { const g = await inv("join_group", a); publishProfile(); return g; },
    async set_profile(a) { const r = await mock.set_profile(a); publishProfile(true); return r; },
    async set_display_name(a) { const r = await inv("set_display_name", a); publishProfile(); return r; },
    async create_invite(a) { return inv("create_invite", a); },
    // voz a sério: o áudio corre em voz.js e no Rust (voz.rs)
    async join_voice(a) { return inv("join_voice", a); },
    async leave_voice() { return inv("leave_voice"); },
    async set_voice_state(a) { return inv("set_voice_state", a); },
  };

  // nome de canal: texto em minúsculas com hífens; voz fica como está
  function cleanName(n, kind) {
    n = String(n || "").trim().slice(0, 40);
    return kind === "voice" ? n : n.toLowerCase().replace(/\s+/g, "-").replace(/[^\p{L}\p{N}_-]/gu, "").replace(/-+/g, "-").replace(/^-|-$/g, "");
  }
  function deepMerge(t, s) {
    for (const k of Object.keys(s)) {
      if (s[k] && typeof s[k] === "object" && !Array.isArray(s[k])) deepMerge((t[k] = t[k] || {}), s[k]);
      else t[k] = s[k];
    }
  }

  // ---------------- voz simulada: quem está a falar ----------------
  if (!tauri) setInterval(() => {
    const speakers = ["rui", "ines", "joao", "me"].filter(() => Math.random() < 0.35);
    emit("voice_activity", { speaking: speakers });
  }, 900);

  const NICKED = ["list_people", "list_members", "list_friends"];
  function nicks(r) {
    const n = settings.userNicks || {};
    const one = (p) => { if (p && p.id && n[p.id]) { p.realName = p.realName || p.name; p.name = n[p.id]; } return p; };
    if (Array.isArray(r)) r.forEach(one); else if (r && typeof r === "object") Object.values(r).forEach(one);
    return r;
  }
  window.api = new Proxy({}, {
    get(_, name) {
      if (name === "on") {
        return (ev, fn) => {
          if (tauri && window.__TAURI__.event) window.__TAURI__.event.listen(ev, (e) => fn(ev === "message" ? { ...e.payload, message: fmt(e.payload.message) } : e.payload));
          (listeners[ev] = listeners[ev] || []).push(fn);
        };
      }
      if (name === "isMock") return !tauri;
      // apelidos (só teus) por cima do nome que a pessoa escolheu
      if (NICKED.includes(name)) return (args = {}) => call(name, args).then(nicks);
      // Comandos que o Rust ainda não tem caem para os dados de teste.
      return (args = {}) => call(name, args);
    },
  });
  function call(name, args) {
      if (tauri && real[name]) return real[name](args);
      return (tauri
        ? tauri.invoke(String(name), args).catch((e) => {
            if (/not found|unknown command/i.test(String(e)) && mock[name]) return mock[name](args);
            throw e;
          })
        : mock[name](args));
  }
})();
