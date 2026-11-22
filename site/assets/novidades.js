// Novidades: as alterações aprovadas (pull requests) no GitHub, agrupadas por dia,
// com as versões publicadas nesse dia.
(function () {
  const REPO = "RubenFernandes2004/Tasca";
  const caixa = document.getElementById("linha-tempo");
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const dia = (iso) => iso.slice(0, 10);
  const bonito = (d) => new Date(d + "T12:00:00Z").toLocaleDateString("pt-PT", { weekday: "long", day: "numeric", month: "long", year: "numeric" });
  const api = (p, r) => window.tascaApi(p, r);

  Promise.all([
    api("pulls?state=closed&per_page=100&sort=updated&direction=desc", (l) => l.map((p) => ({ number: p.number, title: p.title, html_url: p.html_url, merged_at: p.merged_at, base: { ref: p.base && p.base.ref } }))),
    api("releases?per_page=100", (l) => l.map((r) => ({ tag_name: r.tag_name, published_at: r.published_at }))).catch(() => []),
  ])
    .then(([prs, rels]) => {
      const feitos = prs.filter((p) => p.merged_at && p.base && p.base.ref === "main");
      if (!feitos.length) throw new Error("vazio");
      const porDia = new Map();
      feitos.sort((a, b) => b.merged_at.localeCompare(a.merged_at)).forEach((p) => {
        const d = dia(p.merged_at);
        if (!porDia.has(d)) porDia.set(d, []);
        porDia.get(d).push(p);
      });
      const versoes = new Map();
      rels.forEach((r) => {
        if (!r.published_at) return;
        const d = dia(r.published_at), v = (r.tag_name || "").replace(/^v/, "");
        if (!versoes.has(d)) versoes.set(d, []);
        versoes.get(d).push(v);
      });
      const ordem = (a, b) => a.localeCompare(b, undefined, { numeric: true });
      caixa.innerHTML = Array.from(porDia, ([d, lista]) => {
        const vs = (versoes.get(d) || []).sort(ordem);
        const rotulo = vs.length ? (vs.length === 1 ? `Versão ${vs[0]}` : `Versões ${vs[0]} a ${vs[vs.length - 1]}`) : "";
        return `<div class="entrada revelar visto">
          <h3>${bonito(d).replace(/^./, (c) => c.toUpperCase())}</h3>
          <time datetime="${d}">${rotulo}${rotulo ? " · " : ""}${lista.length} ${lista.length === 1 ? "alteração" : "alterações"}</time>
          <ul>${lista.map((p) => `<li>${esc(p.title)} <a href="${p.html_url}" title="Ver no GitHub">#${p.number}</a></li>`).join("")}</ul>
        </div>`;
      }).join("");
    })
    .catch((e) => {
      console.warn("novidades", e);
      caixa.innerHTML = `<p class="carregar">Não foi possível carregar as novidades agora. Vê-as diretamente no <a href="https://github.com/${REPO}/releases">GitHub</a>.</p>`;
    });
})();
