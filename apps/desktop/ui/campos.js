// Tasca — campos de texto.
// 1) Desliga o autopreenchimento do Windows/WebView2 ("Informação guardada") em todos os campos.
// 2) Sugestões próprias, com o desenho da casa, só onde fazem sentido:
//      <input data-sugg="chave"> + tascaSugg.source("chave", () => [...valores])
//    tascaSugg.remember("chave", valor) guarda um valor usado (fica no topo da lista).
(function () {
  "use strict";
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

  function noAutofill(root) {
    (root || document).querySelectorAll("input:not([data-ac]), textarea:not([data-ac])").forEach((el) => {
      el.dataset.ac = "1";
      if (["checkbox", "radio", "range", "file", "hidden", "button", "submit"].includes(el.type)) return;
      el.setAttribute("autocomplete", "off");
      if (el.tagName === "INPUT" && el.hasAttribute("list")) el.removeAttribute("list"); // <datalist> abre o menu do sistema
    });
  }
  new MutationObserver(() => noAutofill()).observe(document.documentElement, { childList: true, subtree: true });
  document.addEventListener("DOMContentLoaded", () => noAutofill());

  // ---------- sugestões ----------
  const sources = {};
  const KEY = (k) => "tasca:sugg:" + k;
  const used = (k) => { try { return JSON.parse(localStorage.getItem(KEY(k))) || []; } catch { return []; } };
  function remember(k, v) {
    v = String(v || "").trim(); if (!v) return;
    try { localStorage.setItem(KEY(k), JSON.stringify([v, ...used(k).filter((x) => x.toLowerCase() !== v.toLowerCase())].slice(0, 8))); } catch {}
  }
  function forget(k, v) { try { localStorage.setItem(KEY(k), JSON.stringify(used(k).filter((x) => x !== v))); } catch {} }
  function list(k, q) {
    const seen = new Set(), out = [];
    for (const v of [...used(k), ...((sources[k] && sources[k]()) || [])]) {
      const s = String(v || "").trim(); if (!s || seen.has(s.toLowerCase())) continue;
      seen.add(s.toLowerCase()); out.push({ v: s, mine: used(k).includes(s) });
    }
    q = q.trim().toLowerCase();
    return out.filter((x) => !q || (x.v.toLowerCase().includes(q) && x.v.toLowerCase() !== q)).slice(0, 6);
  }

  let open = null; // { input, menu, idx, items }
  function close() { if (open) { open.menu.remove(); open.input.setAttribute("aria-expanded", "false"); open = null; } }
  function show(input) {
    const k = input.dataset.sugg, items = list(k, input.value);
    if (!items.length) return close();
    if (!open || open.input !== input) { close(); const m = document.createElement("div"); m.className = "sugg-menu dd-menu"; m.setAttribute("role", "listbox"); document.body.appendChild(m); open = { input, menu: m, idx: -1 }; input.setAttribute("aria-expanded", "true"); }
    open.items = items; open.idx = Math.min(open.idx, items.length - 1);
    open.menu.innerHTML = items.map((x, i) => `<div class="dd-opt sugg-opt ${i === open.idx ? "hl" : ""}" role="option" data-i="${i}"><span class="dd-txt"><b>${esc(x.v)}</b></span>${x.mine ? `<button type="button" class="sugg-x" data-x="${i}" aria-label="Esquecer ${esc(x.v)}" title="Esquecer">×</button>` : ""}</div>`).join("");
    const r = input.getBoundingClientRect();
    Object.assign(open.menu.style, { left: r.left + "px", top: r.bottom + 6 + "px", width: r.width + "px", maxHeight: "240px", bottom: "auto" });
  }
  function pick(i) {
    const x = open && open.items[i]; if (!x) return;
    const input = open.input; close();
    input.value = x.v; input.dispatchEvent(new Event("input", { bubbles: true })); input.focus();
  }

  document.addEventListener("focusin", (e) => { if (e.target.dataset && e.target.dataset.sugg) show(e.target); });
  document.addEventListener("input", (e) => { if (e.target.dataset && e.target.dataset.sugg) show(e.target); });
  document.addEventListener("focusout", (e) => { if (open && e.target === open.input) setTimeout(() => { if (open && document.activeElement !== open.input) close(); }, 120); });
  document.addEventListener("pointerdown", (e) => {
    if (!open) return;
    const x = e.target.closest(".sugg-x");
    if (x) { e.preventDefault(); e.stopPropagation(); forget(open.input.dataset.sugg, open.items[+x.dataset.x].v); show(open.input); return; }
    const o = e.target.closest(".sugg-opt");
    if (o) { e.preventDefault(); e.stopPropagation(); pick(+o.dataset.i); return; }
    if (e.target !== open.input) close();
  }, true);
  document.addEventListener("keydown", (e) => {
    if (!open || e.target !== open.input) return;
    const n = open.items.length;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); open.idx = e.key === "ArrowDown" ? (open.idx + 1 >= n ? -1 : open.idx + 1) : (open.idx <= -1 ? n - 1 : open.idx - 1); show(open.input); }
    else if (e.key === "Enter" && open.idx >= 0) { e.preventDefault(); e.stopPropagation(); pick(open.idx); }
    else if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(); }
    else if (e.key === "Tab") close();
  }, true);
  window.addEventListener("resize", close);

  window.tascaSugg = { source: (k, fn) => { sources[k] = fn; }, remember, close };
})();
