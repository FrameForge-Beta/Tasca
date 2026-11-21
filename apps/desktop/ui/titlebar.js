// Tasca — barra de cima da janela (a janela não usa a barra do Windows).
// Arrasta-se para mover a janela; duplo clique maximiza. À direita: plugins e os botões
// de minimizar, maximizar e fechar. No browser (pré-visualização) só mostra o título.
(function () {
  "use strict";
  const win = window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow
    ? window.__TAURI__.window.getCurrentWindow() : null;
  const LOGO = '<svg viewBox="0 0 512 512" aria-hidden="true"><rect width="512" height="512" rx="116" fill="#9B2C3F"/><path fill="#F6EEE1" d="M168,96 H344 A72,72 0 0 1 416,168 V300 A72,72 0 0 1 344,372 H226 L138,430 L152,364 A72,72 0 0 1 96,300 V168 A72,72 0 0 1 168,96Z"/><g transform="translate(256,234)" fill="#2A5CAA"><path d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(90)" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(180)" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><path transform="rotate(270)" d="M0,0 C40,-29.4 40,-65.52 0,-84 C-40,-65.52 -40,-29.4 0,0Z"/><circle r="16" fill="#E3A72F"/></g></svg>';
  const PUZZLE = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linejoin="round" aria-hidden="true"><path d="M19.4 14.6a2.4 2.4 0 1 0 0-4.8H18V6a2 2 0 0 0-2-2h-3.8V2.6a2.4 2.4 0 1 0-4.8 0V4H4a2 2 0 0 0-2 2v3.8h1.4a2.4 2.4 0 1 1 0 4.8H2V18a2 2 0 0 0 2 2h3.6v-1.4a2.4 2.4 0 1 1 4.8 0V20H16a2 2 0 0 0 2-2v-3.4z"/></svg>';
  const MIN = '<svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 6h8" stroke="currentColor" stroke-width="1.2"/></svg>';
  const MAX = '<svg viewBox="0 0 12 12" aria-hidden="true"><rect x="2" y="2" width="8" height="8" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.2"/></svg>';
  const RESTORE = '<svg viewBox="0 0 12 12" aria-hidden="true"><rect x="2" y="3.5" width="6.5" height="6.5" rx="1.3" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M4 3.5V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-.5" fill="none" stroke="currentColor" stroke-width="1.2"/></svg>';
  const CLOSE = '<svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.2"/></svg>';

  const bar = document.createElement("header");
  bar.id = "titlebar"; bar.className = "titlebar"; bar.setAttribute("data-tauri-drag-region", "");
  bar.innerHTML = `
    <div class="tb-brand" data-tauri-drag-region>${LOGO}<span data-tauri-drag-region>Tasca</span></div>
    <div class="tb-fill" data-tauri-drag-region></div>
    <div class="tb-actions">
      <button class="tb-btn" id="tb-plugins" data-act="plugins" title="Plugins" aria-label="Plugins">${PUZZLE}</button>
    </div>
    ${win ? `<div class="tb-win">
      <button class="tb-ctl" id="tb-min" aria-label="Minimizar" title="Minimizar">${MIN}</button>
      <button class="tb-ctl" id="tb-max" aria-label="Maximizar" title="Maximizar">${MAX}</button>
      <button class="tb-ctl close" id="tb-close" aria-label="Fechar" title="Fechar">${CLOSE}</button>
    </div>` : ""}`;
  document.body.prepend(bar);
  document.body.classList.add("has-titlebar");

  if (win) {
    const maxBtn = bar.querySelector("#tb-max");
    const sync = async () => {
      try {
        const on = await win.isMaximized();
        maxBtn.innerHTML = on ? RESTORE : MAX;
        maxBtn.title = maxBtn.ariaLabel = on ? "Restaurar" : "Maximizar";
        document.body.classList.toggle("maximized", on);
      } catch {}
    };
    bar.querySelector("#tb-min").onclick = () => win.minimize();
    maxBtn.onclick = () => win.toggleMaximize();
    bar.querySelector("#tb-close").onclick = () => win.close();
    try { win.onResized(sync); } catch {}
    sync();
  }
})();
