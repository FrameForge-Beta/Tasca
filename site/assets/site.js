// Tasca — código comum do site: topo, rodapé, tema, links de download, galeria e manual.
(function () {
  const REPO = "RubenFernandes2004/Tasca";
  const GH = "https://github.com/" + REPO;
  const $ = (s, el = document) => el.querySelector(s);
  const $$ = (s, el = document) => Array.from(el.querySelectorAll(s));

  // ---------- tema (claro/escuro), lembrado neste browser ----------
  const raiz = document.documentElement;
  const guardado = (() => { try { return localStorage.getItem("tasca:tema"); } catch { return null; } })();
  if (guardado) raiz.dataset.theme = guardado;
  else if (matchMedia("(prefers-color-scheme: light)").matches) raiz.dataset.theme = "light";

  const pagina = (location.pathname.split("/").pop() || "index.html").replace(/\.html$/, "") || "index";

  const NAV = [
    ["funcionalidades", "index.html#funcionalidades", "Funcionalidades"],
    ["manual", "manual.html", "Manual"],
    ["como-funciona", "como-funciona.html", "Como funciona"],
    ["plugins", "plugins.html", "Plugins"],
    ["novidades", "novidades.html", "Novidades"],
    ["faq", "faq.html", "Perguntas"],
  ];
  const ICON_GH = '<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .5a11.5 11.5 0 0 0-3.64 22.41c.58.1.79-.25.79-.56v-2c-3.2.7-3.88-1.37-3.88-1.37-.52-1.33-1.28-1.69-1.28-1.69-1.05-.71.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.77 2.7 1.26 3.36.96.1-.75.4-1.26.73-1.55-2.55-.29-5.24-1.28-5.24-5.68 0-1.26.45-2.28 1.19-3.09-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.77 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.83 1.19 3.09 0 4.41-2.69 5.38-5.26 5.67.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 12 .5Z"/></svg>';
  const ICON_LUA = '<svg class="tema-lua" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5Z"/></svg>';
  const ICON_SOL = '<svg class="tema-sol" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>';
  const ICON_MENU = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16"/></svg>';

  const topo = $("[data-topo]");
  if (topo) {
    topo.outerHTML = `
    <a class="saltar" href="#conteudo">Saltar para o conteúdo</a>
    <header class="topo" id="topo">
      <div class="contentor">
        <a href="index.html" class="marca" aria-label="Tasca, página inicial"><img src="tasca-icon.svg" alt="" width="34" height="34">Tasca</a>
        <nav class="principal" aria-label="Principal">
          ${NAV.map(([id, href, t]) => `<a href="${href}"${id === pagina ? ' aria-current="page"' : ""}>${t}</a>`).join("")}
        </nav>
        <div class="acoes">
          <a class="icone-btn so-largo" href="${GH}" aria-label="Código no GitHub" title="Código no GitHub">${ICON_GH}</a>
          <button class="icone-btn" id="tema" aria-label="Mudar entre tema claro e escuro" title="Tema claro / escuro">${ICON_LUA}${ICON_SOL}</button>
          <a class="btn btn-vinho" href="descarregar.html">Descarregar</a>
          <button class="icone-btn menu-btn" id="menu" aria-label="Menu" aria-expanded="false" aria-controls="gaveta">${ICON_MENU}</button>
        </div>
      </div>
    </header>
    <nav class="gaveta" id="gaveta" aria-label="Menu">
      ${NAV.map(([, href, t]) => `<a href="${href}">${t}</a>`).join("")}
      <a href="descarregar.html">Descarregar</a>
      <a href="${GH}">GitHub</a>
    </nav>`;
  }

  const rodape = $("[data-rodape]");
  if (rodape) {
    rodape.outerHTML = `
    <footer class="rodape">
      <div class="contentor">
        <div class="colunas">
          <div>
            <a href="index.html" class="marca"><img src="tasca-icon.svg" alt="" width="34" height="34">Tasca</a>
            <p style="margin:0;max-width:34ch">Conversa, voz e vídeo com os teus grupos, ponto a ponto e cifrado. Feito em Portugal, em Rust.</p>
          </div>
          <div><h4>Produto</h4><ul>
            <li><a href="descarregar.html">Descarregar</a></li>
            <li><a href="index.html#funcionalidades">Funcionalidades</a></li>
            <li><a href="novidades.html">Novidades</a></li>
            <li><a href="plugins.html">Plugins</a></li>
          </ul></div>
          <div><h4>Ajuda</h4><ul>
            <li><a href="manual.html">Manual</a></li>
            <li><a href="faq.html">Perguntas frequentes</a></li>
            <li><a href="manual.html#problemas">Resolver problemas</a></li>
            <li><a href="${GH}/issues/new">Reportar um problema</a></li>
          </ul></div>
          <div><h4>Projeto</h4><ul>
            <li><a href="como-funciona.html">Como funciona</a></li>
            <li><a href="como-funciona.html#seguranca">Segurança</a></li>
            <li><a href="termos.html">Termos e regras</a></li>
            <li><a href="${GH}">Código no GitHub</a></li>
          </ul></div>
        </div>
        <div class="fim">
          <span>© ${new Date().getFullYear()} Tasca. Código aberto, licença MIT ou Apache 2.0.</span>
          <span class="versao-rodape"></span>
        </div>
      </div>
    </footer>`;
  }

  // ---------- topo: sombra, tema, menu ----------
  const header = $("#topo");
  if (header) addEventListener("scroll", () => header.classList.toggle("rolou", scrollY > 8), { passive: true });
  const btnTema = $("#tema");
  if (btnTema) btnTema.addEventListener("click", () => {
    const novo = raiz.dataset.theme === "light" ? "dark" : "light";
    raiz.dataset.theme = novo;
    try { localStorage.setItem("tasca:tema", novo); } catch {}
  });
  const btnMenu = $("#menu"), gaveta = $("#gaveta");
  if (btnMenu && gaveta) {
    btnMenu.addEventListener("click", () => {
      const aberta = gaveta.classList.toggle("aberta");
      btnMenu.setAttribute("aria-expanded", aberta);
    });
    gaveta.addEventListener("click", (e) => { if (e.target.tagName === "A") gaveta.classList.remove("aberta"); });
  }

  // ---------- aparecer ao fazer scroll ----------
  if ("IntersectionObserver" in window) {
    const io = new IntersectionObserver((es) => es.forEach((e) => { if (e.isIntersecting) { e.target.classList.add("visto"); io.unobserve(e.target); } }), { rootMargin: "0px 0px -8% 0px" });
    $$(".revelar").forEach((el) => io.observe(el));
  } else $$(".revelar").forEach((el) => el.classList.add("visto"));

  // ---------- links de download: vão buscar a versão mais recente ----------
  // Sem API (limite, sem rede), os botões abrem a página da última versão no GitHub.
  const mb = (b) => (b / 1024 / 1024).toFixed(1).replace(".", ",") + " MB";
  const quando = (d) => new Date(d).toLocaleDateString("pt-PT", { day: "numeric", month: "long", year: "numeric" });
  // Pedidos à API do GitHub, guardados 15 min neste browser (a API só deixa 60 pedidos por hora).
  window.tascaApi = function (caminho, reduzir) {
    const chave = "tasca:api:" + caminho;
    try {
      const c = JSON.parse(localStorage.getItem(chave));
      if (c && Date.now() - c.t < 15 * 60 * 1000) return Promise.resolve(c.d);
    } catch {}
    return fetch(`https://api.github.com/repos/${REPO}/${caminho}`, { headers: { Accept: "application/vnd.github+json" } })
      .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
      .then((d) => (reduzir ? reduzir(d) : d))
      .then((d) => { try { localStorage.setItem(chave, JSON.stringify({ t: Date.now(), d })); } catch {} return d; });
  };
  window.tascaVersao = window.tascaApi("releases/latest")
    .then((v) => {
      const exe = (v.assets || []).find((a) => /-setup\.exe$/i.test(a.name));
      const apk = (v.assets || []).find((a) => a.name === "Tasca.apk") || (v.assets || []).find((a) => /\.apk$/i.test(a.name));
      const versao = (v.tag_name || "").replace(/^v/, "");
      if (exe) {
        $$("[data-link=windows]").forEach((a) => { a.href = exe.browser_download_url; a.setAttribute("download", ""); });
        $$("[data-info=windows]").forEach((s) => (s.textContent = `Versão ${versao} · ${mb(exe.size)} · Windows 10 e 11`));
      }
      if (apk) {
        $$("[data-link=android]").forEach((a) => (a.href = apk.browser_download_url));
        $$("[data-info=android]").forEach((s) => (s.textContent = `Versão ${versao} · ${mb(apk.size)} · Android 8 ou mais recente`));
      }
      $$("[data-versao]").forEach((s) => (s.textContent = versao));
      $$("[data-data]").forEach((s) => (s.textContent = quando(v.published_at)));
      $$(".versao-rodape").forEach((s) => (s.innerHTML = `Versão mais recente: <a href="novidades.html">${versao}</a>`));
      return v;
    })
    .catch(() => null);

  // No telemóvel Android, o botão do Android passa para primeiro.
  if (/Android/i.test(navigator.userAgent)) {
    $$("[data-link=android]").forEach((a) => { if (a.parentNode.classList.contains("descarregar")) { a.parentNode.prepend(a); a.classList.remove("secundario"); a.parentNode.querySelectorAll("[data-link=windows]").forEach((w) => w.classList.add("secundario")); } });
  }

  // ---------- galeria com separadores ----------
  $$("[data-abas]").forEach((abas) => {
    const figs = $$("figure", $(abas.dataset.abas));
    const botoes = $$("button", abas);
    botoes.forEach((b, i) => b.addEventListener("click", () => {
      botoes.forEach((x, j) => x.setAttribute("aria-selected", i === j));
      figs.forEach((f, j) => f.classList.toggle("on", i === j));
    }));
  });

  // ---------- manual: índice, pesquisa e secção atual ----------
  const doc = $(".doc-corpo");
  if (doc) {
    // âncoras nos títulos
    $$("h2[id], h3[id]", doc).forEach((h) => h.insertAdjacentHTML("beforeend", `<a class="ancora" href="#${h.id}" aria-label="Link para esta secção">#</a>`));

    const nav = $(".doc-nav");
    const links = $$(".grupos a", nav);
    const seccoes = links.map((a) => $(a.getAttribute("href"))).filter(Boolean);

    // índice da secção (lado direito)
    const toc = $(".doc-toc");
    const atualizarToc = (sec) => {
      if (!toc || !sec) return;
      const hs = $$("h3[id]", sec);
      toc.innerHTML = hs.length ? `<b>Nesta secção</b>${hs.map((h) => `<a href="#${h.id}">${h.firstChild.textContent}</a>`).join("")}` : "";
    };

    let atual = null;
    const marcar = () => {
      let s = seccoes[0];
      for (const x of seccoes) if (x.getBoundingClientRect().top < 140) s = x;
      if (s === atual) return;
      atual = s;
      links.forEach((a) => a.classList.toggle("ativo", a.getAttribute("href") === "#" + s.id));
      atualizarToc(s);
    };
    addEventListener("scroll", marcar, { passive: true });
    marcar();

    // abrir/fechar o índice no telemóvel
    const abrir = $(".abrir-indice", nav);
    if (abrir) abrir.addEventListener("click", () => nav.classList.toggle("aberta"));
    links.forEach((a) => a.addEventListener("click", () => nav.classList.remove("aberta")));

    // pesquisa: filtra o índice pelo texto de cada secção
    const caixa = $(".procura", nav);
    const semRes = $(".sem-resultados", nav);
    const norm = (t) => t.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
    const textos = new Map(seccoes.map((s) => [s.id, norm(s.textContent)]));
    if (caixa) {
      caixa.addEventListener("input", () => {
        const q = norm(caixa.value.trim());
        let algum = false;
        links.forEach((a) => {
          const id = a.getAttribute("href").slice(1);
          const ok = !q || q.split(/\s+/).every((p) => textos.get(id).includes(p) || norm(a.textContent).includes(p));
          a.style.display = ok ? "" : "none";
          if (ok) algum = true;
        });
        $$(".grupo", nav).forEach((g) => (g.style.display = $$("a", g).some((a) => a.style.display !== "none") ? "" : "none"));
        semRes.style.display = algum ? "none" : "block";
        if (q) nav.classList.add("aberta");
      });
      caixa.addEventListener("keydown", (e) => {
        if (e.key === "Enter") { const a = links.find((l) => l.style.display !== "none"); if (a) { location.hash = a.getAttribute("href"); caixa.blur(); } }
        if (e.key === "Escape") { caixa.value = ""; caixa.dispatchEvent(new Event("input")); }
      });
      addEventListener("keydown", (e) => {
        if (e.key === "/" && document.activeElement.tagName !== "INPUT") { e.preventDefault(); caixa.focus(); }
      });
    }
  }
})();
