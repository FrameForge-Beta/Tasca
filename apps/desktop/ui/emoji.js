// Tasca — painel de emojis do campo de mensagem.
// Abre por cima do botão 🙂, escreve o emoji onde está o cursor e lembra-se dos últimos usados.
// Shift + clique escolhe vários sem fechar.
(function () {
  "use strict";
  const CATS = [
    ["Caras", "😀", "😀😃😄😁😆😅🤣😂🙂🙃😉😊😇🥰😍🤩😘😗😚😋😛😜🤪😝🤑🤗🤭🤫🤔🤐🤨😐😑😶😏😒🙄😬🤥😌😔😪🤤😴😷🤒🤕🤢🤮🥵🥶🥴😵🤯🤠🥳😎🤓🧐😕😟🙁😮😯😲😳🥺😦😧😨😰😥😢😭😱😖😣😞😓😩😫🥱😤😡😠🤬😈👿💀💩🤡👻👽🤖"],
    ["Gestos e amor", "👍", "👍👎👌🤌✌️🤞🤟🤘🤙👈👉👆👇☝️✋🤚🖐️🖖👋🤝🙏👏🙌👐🤲💪🫶❤️🧡💛💚💙💜🖤🤍💔❣️💕💞💓💗💖💘💝"],
    ["Comida e bebida", "🍷", "🍷🍺🍻🥂🍾🍸🍹☕🍵🧃🥤🍕🍔🍟🌭🥪🌮🌯🥗🍝🍜🍲🍣🍤🧀🥖🥐🍞🥚🍳🥓🍗🍖🐟🦐🍰🎂🧁🍩🍪🍫🍬🍭🍦🍨🍓🍒🍇🍉🍊🍋🍌🍍🥝🍑🥭🍎🍐🌶️🫒"],
    ["Atividades", "⚽", "⚽🏀🏈⚾🎾🏐🏉🎱🏓🏸🥅⛳🎣🎮🕹️🎲♟️🎯🎳🎤🎧🎸🎹🥁🎺🎷🎬🎨🏆🥇🥈🥉🏅🎉🎊🎈🎁"],
    ["Coisas e símbolos", "💡", "🔥✨⭐🌟💫💥💯✅❌❓❗⚠️🚀💡📌📎🔒🔑🔔🎵🎶💬💭⏰📅📷📹💻🖥️📱🐛🛠️⚙️🧩🏠🌍🌙☀️🌧️❄️🌈🇵🇹"],
  ];
  const seg = (s) => (Intl.Segmenter ? [...new Intl.Segmenter("pt", { granularity: "grapheme" }).segment(s)].map((x) => x.segment) : Array.from(s));
  const LISTS = CATS.map(([n, ic, s]) => [n, ic, seg(s)]);
  const KEY = "tasca:emoji-recentes";
  const recent = () => { try { return JSON.parse(localStorage.getItem(KEY)) || []; } catch { return []; } };
  const remember = (e) => { try { localStorage.setItem(KEY, JSON.stringify([e, ...recent().filter((x) => x !== e)].slice(0, 24))); } catch {} };

  let pop = null;
  // fecha com animação (no telemóvel desce como uma folha)
  function close() {
    if (!pop) return;
    const el = pop.el; el.classList.add("out"); setTimeout(() => el.remove(), 200);
    pop.btn.classList.remove("on"); pop.btn.setAttribute("aria-expanded", "false"); pop = null;
  }

  function insert(input, e) {
    if (!input.isConnected && input.id) input = document.getElementById(input.id) || input; // o ecrã pode ter sido redesenhado
    const a = input.selectionStart ?? input.value.length, b = input.selectionEnd ?? a;
    input.value = input.value.slice(0, a) + e + input.value.slice(b);
    const p = a + e.length; input.focus(); input.setSelectionRange(p, p);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }

  function open(btn, input) {
    if (pop && pop.btn === btn) return close();
    // no telemóvel o teclado sai da frente para o painel caber
    if (window.innerWidth <= 720 && typeof input !== "function" && document.activeElement === input) input.blur();
    close();
    const rec = recent();
    const el = document.createElement("div");
    el.className = "emoji-pop"; el.setAttribute("role", "dialog"); el.setAttribute("aria-label", "Emojis");
    const tabs = [...(rec.length ? [["Usados há pouco", "🕘", rec]] : []), ...LISTS];
    el.innerHTML = `<div class="emoji-tabs" role="tablist">${tabs.map(([n, ic], i) => `<button type="button" role="tab" data-jump="${i}" title="${n}" aria-label="${n}">${ic}</button>`).join("")}</div>
      <div class="emoji-body">${tabs.map(([n, , list], i) => `<section data-sec="${i}"><h4>${n}</h4><div class="emoji-grid">${list.map((e) => `<button type="button" data-emoji="${e}" aria-label="${e}">${e}</button>`).join("")}</div></section>`).join("")}</div>`;
    document.body.appendChild(el);
    const r = btn.getBoundingClientRect();
    const w = Math.min(352, window.innerWidth - 16);
    el.style.width = w + "px";
    el.style.left = Math.max(8, Math.min(r.right - w, window.innerWidth - w - 8)) + "px";
    el.style.bottom = window.innerHeight - r.top + 8 + "px";
    if (window.innerWidth <= 720) el.insertAdjacentHTML("afterbegin", '<span class="sheet-grip" aria-hidden="true"></span>');
    // no telemóvel, puxar o separador para baixo fecha
    let y0 = null;
    el.addEventListener("touchstart", (e) => { if (!e.target.closest(".emoji-body")) y0 = e.touches[0].clientY; }, { passive: true });
    el.addEventListener("touchmove", (e) => { if (y0 != null) { const d = Math.max(0, e.touches[0].clientY - y0); el.style.transform = `translateY(${d}px)`; } }, { passive: true });
    el.addEventListener("touchend", (e) => { if (y0 == null) return; const d = e.changedTouches[0].clientY - y0; y0 = null; if (d > 70) close(); else el.style.transform = ""; });
    btn.classList.add("on"); btn.setAttribute("aria-expanded", "true");
    pop = { el, btn, input };
    const body = el.querySelector(".emoji-body");
    const mark = () => {
      let cur = 0;
      el.querySelectorAll("[data-sec]").forEach((s, i) => { if (s.offsetTop - body.offsetTop <= body.scrollTop + 8) cur = i; });
      // no fundo, a última secção nunca chega ao topo: conta como a atual
      const secs = el.querySelectorAll("[data-sec]");
      if (body.scrollTop + body.clientHeight >= body.scrollHeight - 2) cur = secs.length - 1;
      el.querySelectorAll("[data-jump]").forEach((t, i) => t.classList.toggle("on", i === cur));
    };
    body.addEventListener("scroll", mark); mark();
    el.addEventListener("click", (ev) => {
      const j = ev.target.closest("[data-jump]");
      if (j) { const s = el.querySelector(`[data-sec="${j.dataset.jump}"]`); body.scrollTop = s.offsetTop - body.offsetTop; return; }
      const b = ev.target.closest("[data-emoji]"); if (!b) return;
      remember(b.dataset.emoji);
      if (typeof input === "function") { input(b.dataset.emoji); close(); return; }
      insert(input, b.dataset.emoji);
      if (!ev.shiftKey) close();
    });
  }

  document.addEventListener("pointerdown", (e) => { if (pop && !e.target.closest(".emoji-pop") && !pop.btn.contains(e.target)) close(); }, true);
  document.addEventListener("keydown", (e) => { if (pop && e.key === "Escape") { e.preventDefault(); e.stopPropagation(); const i = pop.input; close(); i.focus(); } }, true);
  window.addEventListener("resize", close);

  window.tascaEmoji = { open, close, isOpen: () => !!pop };
})();
