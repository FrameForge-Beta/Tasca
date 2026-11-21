// Tasca — menu de botão direito.
// tascaMenu.open(x, y, items): items = [{ label, hint, icon, danger, disabled, checked, onClick, items: [...] } | "-"]
// Um item com `items` abre um submenu ao lado.
(function () {
  "use strict";
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  const ARROW = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 6 6 6-6 6"/></svg>';
  const TICK = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12l5 5 9-10"/></svg>';
  let stack = []; // menus abertos (0 = principal, 1 = submenu)

  function closeFrom(level) { while (stack.length > level) stack.pop().el.remove(); }
  function close() { closeFrom(0); }

  function build(items, level, x, y, anchor) {
    closeFrom(level);
    const el = document.createElement("div");
    el.className = "ctx-menu"; el.setAttribute("role", "menu");
    el.innerHTML = items.map((it, i) => it === "-" ? '<div class="ctx-sep" role="separator"></div>' :
      `<button type="button" role="menuitem" class="ctx-item ${it.danger ? "danger" : ""}" data-i="${i}" ${it.disabled ? "disabled" : ""}>
        ${it.icon ? `<span class="ctx-ic">${it.icon}</span>` : ""}<span class="ctx-txt">${esc(it.label)}${it.hint ? `<small>${esc(it.hint)}</small>` : ""}</span>
        ${it.items ? `<span class="ctx-arrow">${ARROW}</span>` : it.checked ? `<span class="ctx-arrow">${TICK}</span>` : ""}</button>`).join("");
    document.body.appendChild(el);
    const w = el.offsetWidth, h = el.offsetHeight;
    let left = x, top = y;
    if (anchor) { const r = anchor.getBoundingClientRect(); left = r.right + 4; top = r.top - 6; if (left + w > innerWidth - 8) left = r.left - w - 4; }
    if (left + w > innerWidth - 8) left = innerWidth - w - 8;
    if (top + h > innerHeight - 8) top = Math.max(8, innerHeight - h - 8);
    el.style.left = Math.max(8, left) + "px"; el.style.top = Math.max(8, top) + "px";
    const menu = { el, items }; stack.push(menu);
    el.addEventListener("pointerover", (e) => {
      const b = e.target.closest(".ctx-item"); if (!b) return;
      const it = items[+b.dataset.i];
      if (it.items && !it.disabled) { if (!(stack[level + 1] && stack[level + 1].from === b)) { build(it.items, level + 1, 0, 0, b); stack[level + 1].from = b; } }
      else closeFrom(level + 1);
    });
    el.addEventListener("click", (e) => {
      const b = e.target.closest(".ctx-item"); if (!b || b.disabled) return;
      const it = items[+b.dataset.i];
      if (it.items) { build(it.items, level + 1, 0, 0, b); stack[level + 1].from = b; return; }
      close(); if (it.onClick) it.onClick();
    });
    const first = el.querySelector(".ctx-item:not([disabled])"); if (first && level === 0) first.focus({ preventScroll: true });
    return menu;
  }

  function open(x, y, items) { build(items, 0, x, y); }

  document.addEventListener("pointerdown", (e) => { if (stack.length && !e.target.closest(".ctx-menu")) close(); }, true);
  document.addEventListener("keydown", (e) => {
    if (!stack.length) return;
    const menu = stack[stack.length - 1];
    const btns = [...menu.el.querySelectorAll(".ctx-item:not([disabled])")];
    const i = btns.indexOf(document.activeElement);
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); stack.length > 1 ? closeFrom(stack.length - 1) : close(); }
    else if (e.key === "ArrowDown") { e.preventDefault(); (btns[i + 1] || btns[0]).focus(); }
    else if (e.key === "ArrowUp") { e.preventDefault(); (btns[i - 1] || btns[btns.length - 1]).focus(); }
    else if (e.key === "ArrowRight" && document.activeElement.querySelector(".ctx-arrow svg path[d^='m9']")) { e.preventDefault(); document.activeElement.click(); const s = stack[stack.length - 1].el.querySelector(".ctx-item"); if (s) s.focus(); }
    else if (e.key === "ArrowLeft" && stack.length > 1) { e.preventDefault(); const from = stack[stack.length - 1].from; closeFrom(stack.length - 1); if (from) from.focus(); }
  }, true);
  window.addEventListener("resize", close);
  window.addEventListener("blur", close);
  document.addEventListener("scroll", (e) => { if (stack.length && !(e.target.closest && e.target.closest(".ctx-menu"))) close(); }, true);

  window.tascaMenu = { open, close };
})();
