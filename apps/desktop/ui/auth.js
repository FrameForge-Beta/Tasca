// Tasca — ecrãs de conta: boas-vindas, criar conta, desbloquear, recuperar e Google.
// A identidade (par de chaves) fica cifrada com a palavra-passe neste PC. Se a versão tiver
// servidor de contas (st.server), a mesma cópia cifrada fica lá com um nome de utilizador,
// para entrar noutro PC com utilizador e palavra-passe. O servidor nunca a consegue abrir.
// Opcionalmente, outra cópia cifrada vai para a pasta privada da app no teu Google Drive.
(function () {
  "use strict";
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  const LOGO = '<svg viewBox="0 0 512 512" aria-hidden="true"><rect width="512" height="512" rx="116" fill="#9B2C3F"/><path fill="#F6EEE1" d="M168,96 H344 A72,72 0 0 1 416,168 V300 A72,72 0 0 1 344,372 H226 L138,430 L152,364 A72,72 0 0 1 96,300 V168 A72,72 0 0 1 168,96Z"/><g transform="translate(256,234)"><path fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(90)" fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(180)" fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(270)" fill="#2A5CAA" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><circle r="16" fill="#E3A72F"/><circle r="7" fill="#F6EEE1"/></g></svg>';
  const G = '<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="#4285F4" d="M22.6 12.3c0-.8-.1-1.5-.2-2.3H12v4.3h5.9a5 5 0 0 1-2.2 3.3v2.8h3.6c2.1-1.9 3.3-4.8 3.3-8.1z"/><path fill="#34A853" d="M12 23c3 0 5.5-1 7.3-2.7l-3.6-2.8c-1 .7-2.2 1.1-3.7 1.1-2.9 0-5.3-1.9-6.2-4.5H2.1v2.9A11 11 0 0 0 12 23z"/><path fill="#FBBC05" d="M5.8 14.1a6.6 6.6 0 0 1 0-4.2V7H2.1a11 11 0 0 0 0 10l3.7-2.9z"/><path fill="#EA4335" d="M12 5.4c1.6 0 3.1.6 4.2 1.7l3.2-3.2A11 11 0 0 0 2.1 7l3.7 2.9C6.7 7.3 9.1 5.4 12 5.4z"/></svg>';

  let root, done, st = {};

  function strength(pw) {
    let s = 0;
    if (pw.length >= 8) s++;
    if (pw.length >= 12) s++;
    if (/[A-Z]/.test(pw) && /[a-z]/.test(pw)) s++;
    if (/\d/.test(pw) || /[^\w]/.test(pw)) s++;
    return s; // 0..4
  }
  const LV = ["Muito fraca", "Fraca", "Razoável", "Boa", "Forte"];

  function shell(inner, { back, cls } = {}) {
    root.innerHTML = `
      <div class="auth${cls ? " " + cls : ""}">
        <aside class="auth-brand" aria-hidden="true">
          <div class="auth-brand-in">
            <span class="auth-logo">${LOGO}</span>
            <p class="auth-tag">Conversa, voz e plugins.<br>Com os teus amigos.</p>
          </div>
        </aside>
        <main class="auth-main">
          <div class="auth-card">
            ${back ? `<button class="auth-back" data-go="${back}">← Voltar</button>` : ""}
            ${inner}
          </div>
        </main>
      </div>`;
    const f = root.querySelector("input, textarea");
    if (f) f.focus();
  }

  function err(msg) {
    const e = root.querySelector(".auth-err");
    if (e) { e.textContent = msg; e.hidden = !msg; }
  }
  // Quando não liga ao servidor: botão que testa DNS, TCP e HTTPS e mostra o resultado.
  function diagnostico() {
    const e = root.querySelector(".auth-err");
    if (!e || root.querySelector(".auth-diag")) return;
    const box = document.createElement("div");
    box.className = "auth-diag";
    box.innerHTML = `<button type="button" class="linkish">Ver diagnóstico da ligação</button><pre hidden></pre>`;
    e.after(box);
    const pre = box.querySelector("pre");
    box.querySelector("button").onclick = async (ev) => {
      ev.target.disabled = true; pre.hidden = false; pre.textContent = "A testar…";
      try { pre.textContent = (await api.server_diagnostics()).join("\n"); }
      catch (x) { pre.textContent = String(x.message || x); }
      ev.target.disabled = false;
    };
  }
  function busy(btn, on, label) {
    btn.disabled = on;
    if (on) { btn.dataset.l = btn.textContent; btn.textContent = label; } else if (btn.dataset.l) btn.textContent = btn.dataset.l;
  }
  const pwField = (id, label, auto) => `<div class="field"><label for="${id}">${label}</label>
      <div class="pw"><input class="input" type="password" id="${id}" autocomplete="${auto}"><button type="button" class="pw-eye" data-eye="${id}" aria-label="Mostrar palavra-passe">ver</button></div></div>`;

  const rememberField = (id) => `<label class="check remember"><input type="checkbox" id="${id}" checked><span>Lembrar esta conta neste PC<small>Da próxima vez abre sem pedir a palavra-passe.</small></span></label>`;
  const AV = ["#9B2C3F", "#2A5CAA", "#5E8C4A", "#B8791A", "#6B4C9A", "#2E7D7A"];
  const avColor = (s) => AV[[...String(s)].reduce((a, c) => a + c.charCodeAt(0), 0) % AV.length];
  const accountRow = (a) => `<button class="acct-row" data-acct="${esc(a.id)}">
      <span class="av" style="background:${avColor(a.id)}">${esc((a.displayName || "?")[0].toUpperCase())}</span>
      <span class="acct-txt"><b>${esc(a.displayName)}</b><small>${a.username ? "@" + esc(a.username) : "Só neste PC"}${a.remembered ? " · abre sozinha" : ""}</small></span>
      <span class="acct-go" aria-hidden="true">›</span></button>`;

  // ícones do ecrã de boas-vindas
  const sv = (d) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${d}</svg>`;
  const WL = {
    chat: sv('<path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z"/>'),
    mic: sv('<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/>'),
    lock: sv('<rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>'),
    puzzle: sv('<path d="M19.4 14.6a2.4 2.4 0 1 0 0-4.8H18V6a2 2 0 0 0-2-2h-3.8V2.6a2.4 2.4 0 1 0-4.8 0V4H4a2 2 0 0 0-2 2v3.8h1.4a2.4 2.4 0 1 1 0 4.8H2V18a2 2 0 0 0 2 2h3.6v-1.4a2.4 2.4 0 1 1 4.8 0V20H16a2 2 0 0 0 2-2v-3.4z"/>'),
  };

  // ---------------- ecrãs ----------------
  const screens = {
    welcome() {
      const list = st.accounts || [];
      shell(list.length ? `
        <h1>Escolhe a conta</h1>
        <p class="auth-lede">Contas guardadas neste PC.</p>
        <div class="acct-list">${list.map(accountRow).join("")}</div>
        <div class="auth-actions">
          <button class="btn outline big" data-go="login">Entrar com outra conta</button>
        </div>
        <div class="auth-links auth-links-center"><button data-go="create">Criar conta nova</button></div>` : `
        <div class="wl-hero">
          <span class="wl-logo">${LOGO}</span>
          <b class="wl-name">Tasca</b>
          <p class="wl-tag">O teu grupo, na <em>tua</em> tasca.</p>
        </div>
        <h1 class="wl-title">Bem-vindo à Tasca</h1>
        <p class="auth-lede wl-lede">Fala, joga e partilha com os teus amigos.</p>
        <ul class="wl-feats">
          <li><i style="--c:#9B2C3F">${WL.chat}</i><span><b>Grupos e mensagens</b><small>Canais de texto, amigos e mensagens privadas.</small></span></li>
          <li><i style="--c:#5E8C4A">${WL.mic}</i><span><b>Voz e vídeo</b><small>Chamadas com supressão de ruído.</small></span></li>
          <li><i style="--c:#2A5CAA">${WL.lock}</i><span><b>Privado</b><small>Só tu e o teu grupo leem as vossas mensagens.</small></span></li>
          <li><i style="--c:#B8791A">${WL.puzzle}</i><span><b>Plugins</b><small>Junta funcionalidades novas à tua medida.</small></span></li>
        </ul>
        <div class="wl-actions">
          <button class="btn primary big" data-go="login">Entrar</button>
          <div class="auth-sep"><span>Ainda não tens conta?</span></div>
          <button class="btn outline big" data-go="create">Criar conta</button>
        </div>`, { cls: "is-welcome" });
    },

    create() {
      shell(`
        <h1>Criar conta</h1>
        <p class="auth-lede">${st.server ? "Não precisas de email. Com o nome de utilizador entras noutro PC." : "Só fica neste PC. Não precisas de email."}</p>
        <form id="f-create" class="auth-form" novalidate>
          <div class="field"><label for="c-name">Nome</label><input class="input" id="c-name" maxlength="32" autocomplete="nickname" placeholder="Como te vão ver nos grupos"></div>
          ${st.server ? `<div class="field"><label for="c-user">O teu ID (nome de utilizador)</label><div class="user-in"><span aria-hidden="true">@</span><input class="input" id="c-user" maxlength="24" autocomplete="username" autocapitalize="off" spellcheck="false" placeholder="letras, números, _ e ."></div><small class="hint" id="c-user-st">Único e em minúsculas, como @tiago.f. É com ele que te adicionam como amigo e que entras noutro PC. O nome acima pode repetir-se; o ID não.</small><div class="sugg" id="c-sugg" hidden></div></div>` : ""}
          ${pwField("c-pw", "Palavra-passe", "new-password")}
          <div class="meter-row"><div class="meter"><i id="c-meter"></i></div><small id="c-lv">Pelo menos 8 caracteres</small></div>
          ${pwField("c-pw2", "Repete a palavra-passe", "new-password")}
          ${rememberField("c-rem")}
          <p class="auth-note">Se te esqueceres da palavra-passe, só as 12 palavras que vais ver a seguir te devolvem a conta.</p>
          <p class="auth-err" hidden></p>
          <button class="btn primary big" type="submit">Continuar</button>
        </form>`, { back: "welcome" });
    },

    words() {
      shell(`
        <h1>As tuas 12 palavras</h1>
        <p class="auth-lede">Escreve-as num papel e guarda-o num sítio seguro. Servem para recuperar a conta noutro PC ou se te esqueceres da palavra-passe. Quem as tiver, tem a tua conta.</p>
        ${st.serverErr ? `<p class="auth-note">A conta foi criada, mas não ficou guardada online: ${esc(st.serverErr)}</p>` : ""}
        <ol class="words">${st.phrase.split(" ").map((w) => `<li><span>${esc(w)}</span></li>`).join("")}</ol>
        <div class="auth-row"><button class="btn ghost only-desktop" id="w-txt">Guardar .txt no ambiente de trabalho</button><button class="btn ghost" id="w-copy">Copiar</button></div>
        <p class="auth-note" id="w-txt-ok" hidden></p>
        <label class="check"><input type="checkbox" id="w-ok"><span>Já as guardei num sítio seguro</span></label>
        <button class="btn primary big" id="w-next" disabled>Continuar</button>`);
    },

    confirm() {
      const words = st.phrase.split(" ");
      st.ask = st.ask || [...new Set(Array.from(crypto.getRandomValues(new Uint8Array(8))).map((b) => b % 12))].slice(0, 3).sort((a, b) => a - b);
      shell(`
        <h1>Confirma as palavras</h1>
        <p class="auth-lede">Para teres a certeza de que as guardaste bem.</p>
        <form id="f-confirm" class="auth-form" novalidate>
          ${st.ask.map((i) => `<div class="field"><label for="k-${i}">Palavra n.º ${i + 1}</label><input class="input mono" id="k-${i}" autocomplete="off" autocapitalize="off" spellcheck="false"></div>`).join("")}
          <p class="auth-err" hidden></p>
          <button class="btn primary big" type="submit">Concluir</button>
        </form>`, { back: "words" });
      root._expected = st.ask.map((i) => [i, words[i]]);
    },

    // Depois de criar a conta: o que a Tasca pode usar e os termos (obrigatórios).
    perms() {
      const P = [
        ["mic", "Microfone", "Para falares nas chamadas.", true],
        ["camera", "Câmara", "Para videochamadas.", true],
        ["notifications", "Notificações", "Avisos de mensagens e menções.", true],
        ["autostart", "Abrir com o Windows", "Para os grupos que alojas estarem sempre disponíveis.", false],
      ].filter(([k]) => !(window.TASCA_MOBILE && k === "autostart"));
      shell(`
        <h1>Antes de começar</h1>
        <p class="auth-lede">Escolhe o que a Tasca pode usar neste PC. Podes mudar tudo depois em Definições > Privacidade e permissões.</p>
        <div class="perm-list">${P.map(([k, l, d, on]) => `<div class="perm-row"><div class="txt"><b>${l}</b><small>${d}</small></div><label class="switch"><input type="checkbox" id="p-${k}" ${on ? "checked" : ""}><span></span><span class="sr">${l}</span></label></div>`).join("")}</div>
        <div class="auth-terms">
          <label class="check"><input type="checkbox" id="p-terms"><span>Li e aceito os <button type="button" class="linkish" id="p-read">termos e regras da Tasca</button></span></label>
          <div class="terms-box" id="p-terms-box" hidden>${(window.TASCA_TERMOS && window.TASCA_TERMOS.html) || ""}</div>
        </div>
        <p class="auth-err" hidden></p>
        <button class="btn primary big" id="p-next" disabled>Entrar na Tasca</button>`);
    },

    unlock() {
      shell(`
        <div class="auth-who"><span class="av big" style="background:#9B2C3F">${esc((st.displayName || "?")[0].toUpperCase())}</span><div><b>${esc(st.displayName || "")}</b><small>${st.username ? "@" + esc(st.username) + " · " : ""}Conta guardada neste PC${st.google ? " · cópia no Google Drive" : ""}</small></div></div>
        <form id="f-unlock" class="auth-form" novalidate>
          ${pwField("u-pw", "Palavra-passe", "current-password")}
          ${rememberField("u-rem")}
          <p class="auth-err" hidden></p>
          <button class="btn primary big" type="submit">Entrar</button>
        </form>
        <div class="auth-links"><button data-go="recover">Esqueci-me da palavra-passe</button><button id="u-switch">Não és tu? Trocar de conta</button></div>`);
    },

    login() {
      shell(`
        <h1>Entrar</h1>
        ${st.server ? `
        <p class="auth-lede">Entra com a tua palavra-passe.</p>
        <form id="f-login" class="auth-form" novalidate>
          <div class="field"><label for="l-user">Nome de utilizador</label><input class="input" id="l-user" data-sugg="utilizador" maxlength="25" autocomplete="username" autocapitalize="off" spellcheck="false" value="${esc(st.username || "")}"></div>
          ${pwField("l-pw", "Palavra-passe", "current-password")}
          ${rememberField("l-rem")}
          <p class="auth-err" hidden></p>
          <button class="btn primary big" type="submit">Entrar</button>
        </form>
        <div class="auth-links"><button data-go="recover">Esqueci-me da palavra-passe</button></div>
        <div class="auth-sep"><span>ou</span></div>` : `
        <p class="auth-note">Entrar com nome de utilizador ainda não está ligado nesta versão. Por agora, entra com o Google ou com as tuas 12 palavras.</p>`}
        <div class="auth-actions">
          <button class="btn google big" data-go="google">${G}Entrar com Google</button>
          <button class="btn outline big" data-go="recover">Usar as 12 palavras de recuperação</button>
        </div>
        <div class="auth-links auth-links-center"><button data-go="create">Ainda não tens conta? Criar conta</button></div>`, { back: "welcome" });
    },

    recover() {
      shell(`
        <h1>Recuperar conta</h1>
        <p class="auth-lede">Escreve as 12 palavras pela ordem, separadas por espaços. Depois escolhe uma palavra-passe nova.</p>
        <form id="f-recover" class="auth-form" novalidate>
          <div class="field"><label for="r-name">Nome</label><input class="input" id="r-name" maxlength="32" autocomplete="nickname" placeholder="Como te vão ver nos grupos" value="${esc(st.displayName || "")}"></div>
          <div class="field"><label for="r-phrase">12 palavras</label><textarea class="input mono" id="r-phrase" rows="3" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder="palavra1 palavra2 …"></textarea><small id="r-count" class="hint">0 de 12</small></div>
          ${pwField("r-pw", "Palavra-passe nova", "new-password")}
          ${pwField("r-pw2", "Repete a palavra-passe", "new-password")}
          ${rememberField("r-rem")}
          <p class="auth-err" hidden></p>
          <button class="btn primary big" type="submit">Recuperar</button>
        </form>`, { back: "login" });
    },

    google() {
      shell(`
        <h1>Entrar com Google</h1>
        <p class="auth-lede">Vamos abrir o browser para escolheres a conta Google. A Tasca só pede acesso a uma pasta privada dela no teu Google Drive, onde guarda a tua conta. Não vê os teus emails nem os teus ficheiros.</p>
        <p class="auth-err" hidden></p>
        <button class="btn google big" id="g-go">${G}Abrir o browser</button>`, { back: "login" });
    },

    googleWait() {
      shell(`
        <div class="auth-wait"><span class="spin" aria-hidden="true"></span><div><b>À espera do browser…</b><small>Escolhe a conta Google e volta para aqui.</small></div></div>
        <button class="btn ghost" data-go="google">Cancelar</button>`);
    },

    googleRestore() {
      shell(`
        <div class="auth-who">${G.replace("<svg", '<svg class="g-big"')}<div><b>${esc(st.gmail || "Conta Google")}</b><small>Encontrámos a tua conta da Tasca no Google Drive</small></div></div>
        <form id="f-grestore" class="auth-form" novalidate>
          ${pwField("g-pw", "Palavra-passe da Tasca", "current-password")}
          <p class="auth-note">É a palavra-passe da Tasca, não a do Google.</p>
          ${rememberField("g-rem")}
          <p class="auth-err" hidden></p>
          <button class="btn primary big" type="submit">Entrar</button>
        </form>`, { back: "welcome" });
    },

    googleNone() {
      shell(`
        <h1>Ainda não tens conta guardada</h1>
        <p class="auth-lede">A conta Google ${esc(st.gmail || "")} ficou ligada, mas não há nenhuma conta da Tasca guardada nela. Cria uma agora e guardamos uma cópia no teu Google Drive.</p>
        <button class="btn primary big" data-go="create">Criar conta</button>`, { back: "welcome" });
    },
  };

  function go(name) { st.screen = name; screens[name](); }
  async function finish() {
    if (st.wantGoogleBackup) { try { await api.google_backup(); } catch {} }
    root.classList.add("out");
    setTimeout(() => { root.remove(); done(); }, 260);
  }

  // Nome de utilizador: diz logo se está livre e, se não estiver, sugere parecidos.
  let userTimer = 0, userSeq = 0;
  function userState(text, cls) {
    const h = root.querySelector("#c-user-st"); if (!h) return;
    h.textContent = text; h.className = "hint " + (cls || "");
  }
  function checkUser(val) {
    clearTimeout(userTimer);
    const sg = root.querySelector("#c-sugg"); if (sg) sg.hidden = true;
    const u = val.trim().replace(/^@/, "");
    if (!u) return userState("É assim que te encontram, e com ele entras noutro PC.");
    if (u.length < 3) return userState("Pelo menos 3 caracteres.");
    userState("A verificar…");
    const seq = ++userSeq;
    userTimer = setTimeout(async () => {
      try {
        const free = await api.username_available({ username: u });
        if (seq !== userSeq) return;
        if (free) userState("@" + u.toLowerCase() + " está livre.", "ok");
        else { userState("@" + u.toLowerCase() + " já está ocupado.", "bad"); showSuggestions(u); }
      } catch (x) { if (seq === userSeq) userState(String(x.message || x), "bad"); }
    }, 400);
  }
  async function showSuggestions(u) {
    const sg = root.querySelector("#c-sugg"); if (!sg) return;
    try {
      const list = await api.suggest_usernames({ username: u });
      if (!list.length) return;
      sg.innerHTML = `<small>Livres:</small>${list.map((x) => `<button type="button" class="chip" data-sugg="${esc(x)}">@${esc(x)}</button>`).join("")}`;
      sg.hidden = false;
    } catch {}
  }

  // ---------------- eventos ----------------
  function wire() {
    root.addEventListener("click", async (e) => {
      const b = e.target.closest("button");
      if (!b) return;
      if (b.dataset.go) { if (b.dataset.go === "create" && st.gmail) st.wantGoogleBackup = true; return go(b.dataset.go); }
      if (b.dataset.eye) {
        const i = root.querySelector("#" + b.dataset.eye);
        i.type = i.type === "password" ? "text" : "password";
        b.textContent = i.type === "password" ? "ver" : "esconder";
        return;
      }
      if (b.id === "w-copy") {
        try { await navigator.clipboard.writeText(st.phrase); b.textContent = "Copiado"; }
        catch { b.textContent = "Seleciona e copia"; }
        return;
      }
      if (b.id === "w-txt") {
        const ok = root.querySelector("#w-txt-ok");
        try {
          const path = await api.save_phrase_desktop({ phrase: st.phrase, displayName: st.displayName || "" });
          b.textContent = "Guardado";
          ok.textContent = `Ficou em ${path}. Passa as palavras para papel e depois apaga o ficheiro: quem o abrir fica com a tua conta.`;
        } catch (x) { ok.textContent = String(x.message || x); }
        ok.hidden = false;
        return;
      }
      if (b.id === "w-next") return go("confirm");
      if (b.id === "p-read") { e.preventDefault(); const bx = root.querySelector("#p-terms-box"); bx.hidden = !bx.hidden; return; }
      if (b.id === "p-next") {
        const on = (k) => root.querySelector("#p-" + k).checked;
        const perms = { mic: on("mic"), camera: on("camera"), notifications: on("notifications"), autostart: !window.TASCA_MOBILE && on("autostart") };
        b.disabled = true; b.textContent = "A preparar…";
        // Pede já ao Windows o que foi ligado, para a pergunta aparecer agora e não a meio de uma chamada.
        if (perms.mic && navigator.mediaDevices && navigator.mediaDevices.getUserMedia) {
          try { (await navigator.mediaDevices.getUserMedia({ audio: true })).getTracks().forEach((t) => t.stop()); } catch {}
        }
        if (perms.notifications && window.Notification && Notification.permission === "default") { try { await Notification.requestPermission(); } catch {} }
        if (perms.autostart) { try { await api.set_autostart({ enabled: true }); } catch { perms.autostart = false; } }
        try { await api.set_settings({ patch: { perms, termsAccepted: (window.TASCA_TERMOS && window.TASCA_TERMOS.versao) || "" } }); } catch {}
        return finish();
      }
      if (b.dataset.acct) {
        b.disabled = true;
        try {
          await api.select_account({ id: b.dataset.acct });
          Object.assign(st, await api.account_status());
          return st.unlocked ? finish() : go("unlock");
        } catch (x) { b.disabled = false; return err(String(x.message || x)); }
      }
      if (b.id === "u-switch") {
        st.accounts = await api.list_accounts();
        return go("welcome");
      }
      if (b.dataset.sugg) {
        const i = root.querySelector("#c-user"); i.value = b.dataset.sugg; i.dispatchEvent(new Event("input", { bubbles: true }));
        return;
      }
      if (b.id === "g-go") {
        go("googleWait");
        try {
          const r = await api.google_sign_in();
          st.gmail = r.email;
          go(r.hasBackup ? "googleRestore" : "googleNone");
        } catch (x) { go("google"); err(String(x.message || x)); }
      }
    });
    root.addEventListener("change", (e) => {
      if (e.target.id === "w-ok") root.querySelector("#w-next").disabled = !e.target.checked;
      if (e.target.id === "p-terms") root.querySelector("#p-next").disabled = !e.target.checked;
    });
    root.addEventListener("input", (e) => {
      const t = e.target;
      if (t.id === "c-pw") {
        const s = strength(t.value);
        const m = root.querySelector("#c-meter");
        m.style.width = (t.value ? 10 + s * 22.5 : 0) + "%";
        m.dataset.s = s;
        root.querySelector("#c-lv").textContent = t.value ? LV[s] : "Pelo menos 8 caracteres";
      }
      if (t.id === "c-user") checkUser(t.value);
      if (t.id === "r-phrase") {
        const n = t.value.trim() ? t.value.trim().split(/\s+/).length : 0;
        const c = root.querySelector("#r-count"); c.textContent = `${n} de 12`; c.classList.toggle("ok", n === 12);
      }
    });
    root.addEventListener("submit", async (e) => {
      e.preventDefault();
      const f = e.target, btn = f.querySelector("button[type=submit]");
      const v = (id) => root.querySelector("#" + id).value;
      const chk = (id) => { const c = root.querySelector("#" + id); return !!(c && c.checked); };
      err("");
      try {
        if (f.id === "f-create") {
          const name = v("c-name").trim();
          if (!name) return err("Escreve o teu nome.");
          if (v("c-pw").length < 8) return err("A palavra-passe tem de ter pelo menos 8 caracteres.");
          if (v("c-pw") !== v("c-pw2")) return err("As palavras-passe não são iguais.");
          const user = st.server ? v("c-user").trim() : "";
          if (st.server && !user) return err("Escolhe um nome de utilizador.");
          if (user) {
            busy(btn, true, "A verificar o nome…");
            if (!(await api.username_available({ username: user }))) { busy(btn, false); showSuggestions(user); return err("Esse nome de utilizador já está ocupado. Escolhe um dos sugeridos ou outro."); }
            busy(btn, false);
          }
          busy(btn, true, "A criar a tua chave…");
          const r = await api.create_account({ displayName: name, password: v("c-pw"), remember: chk("c-rem") });
          st.phrase = r.recoveryPhrase; st.displayName = name; st.ask = null; st.serverErr = null;
          if (user) {
            try { st.username = await api.server_register({ username: user, password: v("c-pw") }); }
            catch (x) { st.serverErr = String(x.message || x); }
          }
          return go("words");
        }
        if (f.id === "f-confirm") {
          const wrong = root._expected.find(([i, w]) => v("k-" + i).trim().toLowerCase() !== w);
          if (wrong) return err(`A palavra n.º ${wrong[0] + 1} não está certa. Confirma no papel.`);
          return go("perms");
        }
        if (f.id === "f-unlock") {
          busy(btn, true, "A abrir…");
          await api.unlock({ password: v("u-pw"), remember: chk("u-rem") });
          return finish();
        }
        if (f.id === "f-login") {
          if (!v("l-user").trim()) return err("Escreve o teu nome de utilizador.");
          busy(btn, true, "A entrar…");
          await api.server_login({ username: v("l-user").trim(), password: v("l-pw"), remember: chk("l-rem") });
          if (window.tascaSugg) tascaSugg.remember("utilizador", v("l-user").trim().replace(/^@/, "").toLowerCase());
          return finish();
        }
        if (f.id === "f-recover") {
          const name = v("r-name").trim();
          if (!name) return err("Escreve o teu nome.");
          if (v("r-pw").length < 8) return err("A palavra-passe tem de ter pelo menos 8 caracteres.");
          if (v("r-pw") !== v("r-pw2")) return err("As palavras-passe não são iguais.");
          busy(btn, true, "A recuperar…");
          await api.recover({ phrase: v("r-phrase").trim().toLowerCase(), displayName: name, newPassword: v("r-pw"), remember: chk("r-rem") });
          st.displayName = name;
          return finish();
        }
        if (f.id === "f-grestore") {
          busy(btn, true, "A abrir…");
          await api.google_restore({ password: v("g-pw"), remember: chk("g-rem") });
          return finish();
        }
      } catch (x) {
        busy(btn, false);
        const msg = String(x.message || x);
        err(msg);
        if (/ligar ao servidor/i.test(msg)) diagnostico();
      }
    });
  }

  // Mostra os ecrãs de conta e resolve quando a conta estiver desbloqueada.
  // Sugestões no campo de utilizador: as contas que já entraram neste PC.
  if (window.tascaSugg) tascaSugg.source("utilizador", () => (st.accounts || []).map((a) => a.username).filter(Boolean));
  window.tascaAuth = {
    async gate() {
      st = await api.account_status();
      if (st.unlocked) return st;
      try { st.accounts = await api.list_accounts(); } catch { st.accounts = []; }
      root = document.createElement("div");
      root.id = "auth-root";
      document.body.appendChild(root);
      wire();
      go(st.exists ? "unlock" : "welcome");
      await new Promise((r) => (done = r));
      return api.account_status();
    },
  };
})();
