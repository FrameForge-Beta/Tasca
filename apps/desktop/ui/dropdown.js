// Tasca — menus de escolha com o desenho da casa.
// Cada <select class="input"> ganha por cima um botão e uma lista próprios. O <select>
// continua lá (escondido) e recebe o valor e o evento "change", por isso o resto do código
// não muda. Nomes como "Microfone (Realtek Audio)" mostram-se em duas linhas.
(function () {
  "use strict";
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  const CHEV = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 9l6 6 6-6"/></svg>';
  const TICK = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12l5 5 9-10"/></svg>';

  // "SteelSeries Sonar - Chat (SteelSeries Sonar Virtual Audio Device)" → ["SteelSeries Sonar - Chat", "SteelSeries Sonar Virtual Audio Device"]
  function split(label) {
    const m = /^(.*\S)\s*\(([^()]+)\)\s*$/.exec(label);
    return m ? [m[1], m[2]] : [label, ""];
  }

  let open = null; // { sel, btn, menu, idx }

  function close() {
    if (!open) return;
    open.menu.remove(); open.btn.setAttribute("aria-expanded", "false"); open.btn.classList.remove("on");
    open = null;
  }

  function choose(i) {
    const { sel } = open;
    const opt = sel.options[i];
    close();
    if (!opt || sel.value === opt.value) return;
    sel.value = opt.value;
    sel.dispatchEvent(new Event("change", { bubbles: true }));
    sync(sel);
  }

  function highlight(i) {
    const items = open.menu.querySelectorAll(".dd-opt");
    if (!items.length) return;
    open.idx = (i + items.length) % items.length;
    items.forEach((el, k) => el.classList.toggle("hl", k === open.idx));
    items[open.idx].scrollIntoView({ block: "nearest" });
  }

  function show(sel, btn) {
    close();
    const menu = document.createElement("div");
    menu.className = "dd-menu"; menu.setAttribute("role", "listbox");
    menu.innerHTML = [...sel.options].map((o, i) => {
      const [a, b] = split(o.textContent);
      const on = o.value === sel.value;
      return `<button type="button" class="dd-opt ${on ? "sel" : ""}" role="option" aria-selected="${on}" data-i="${i}">
        <span class="dd-txt"><b>${esc(a)}</b>${b ? `<small>${esc(b)}</small>` : ""}</span>${on ? TICK : ""}</button>`;
    }).join("");
    document.body.appendChild(menu);
    const r = btn.getBoundingClientRect();
    const h = Math.min(menu.scrollHeight, 320);
    const below = window.innerHeight - r.bottom - 12;
    menu.style.width = Math.max(r.width, 260) + "px";
    menu.style.left = Math.min(r.left, window.innerWidth - Math.max(r.width, 260) - 8) + "px";
    if (below >= h || below >= r.top) { menu.style.top = r.bottom + 6 + "px"; menu.style.maxHeight = Math.max(160, below) + "px"; }
    else { menu.style.bottom = window.innerHeight - r.top + 6 + "px"; menu.style.maxHeight = Math.max(160, r.top - 12) + "px"; }
    btn.setAttribute("aria-expanded", "true"); btn.classList.add("on");
    open = { sel, btn, menu, idx: sel.selectedIndex };
    highlight(Math.max(0, sel.selectedIndex));
    menu.addEventListener("click", (e) => { const o = e.target.closest(".dd-opt"); if (o) choose(+o.dataset.i); });
    menu.addEventListener("pointermove", (e) => { const o = e.target.closest(".dd-opt"); if (o && +o.dataset.i !== open.idx) highlight(+o.dataset.i); });
  }

  function sync(sel) {
    const btn = sel._dd; if (!btn) return;
    const o = sel.options[sel.selectedIndex];
    const [a, b] = split(o ? o.textContent : "");
    btn.querySelector(".dd-cur").innerHTML = `<b>${esc(a)}</b>${b ? `<small>${esc(b)}</small>` : ""}`;
    btn.disabled = sel.disabled;
  }

  function enhance(root) {
    (root || document).querySelectorAll("select.input:not([data-dd])").forEach((sel) => {
      sel.dataset.dd = "1";
      const btn = document.createElement("button");
      btn.type = "button"; btn.className = "dd-btn"; btn.setAttribute("aria-haspopup", "listbox"); btn.setAttribute("aria-expanded", "false");
      if (sel.id) btn.id = "dd-" + sel.id;
      const lab = sel.id && document.querySelector(`label[for="${sel.id}"]`);
      if (lab) { lab.htmlFor = btn.id; btn.setAttribute("aria-label", lab.textContent); }
      btn.innerHTML = `<span class="dd-cur"></span>${CHEV}`;
      sel.hidden = true; sel._dd = btn;
      sel.after(btn);
      sync(sel);
      btn.addEventListener("click", () => (open && open.sel === sel ? close() : show(sel, btn)));
      btn.addEventListener("keydown", (e) => {
        if (!open && (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Enter" || e.key === " ")) { e.preventDefault(); show(sel, btn); }
      });
    });
  }

  document.addEventListener("keydown", (e) => {
    if (!open) return;
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); const b = open.btn; close(); b.focus(); }
    else if (e.key === "ArrowDown") { e.preventDefault(); highlight(open.idx + 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); highlight(open.idx - 1); }
    else if (e.key === "Enter" || e.key === " ") { e.preventDefault(); choose(open.idx); }
    else if (e.key === "Tab") close();
  }, true);
  document.addEventListener("pointerdown", (e) => { if (open && !e.target.closest(".dd-menu") && !e.target.closest(".dd-btn")) close(); }, true);
  window.addEventListener("resize", close);
  document.addEventListener("scroll", (e) => { if (open && !(e.target.closest && e.target.closest(".dd-menu"))) close(); }, true);

  // Enquanto o ecrã muda, os <select> novos ganham o menu sozinhos.
  new MutationObserver(() => { if (document.querySelector("select.input:not([data-dd])")) enhance(); })
    .observe(document.documentElement, { childList: true, subtree: true });

  window.tascaDropdown = { enhance, close };
})();
