# Interface da Tasca

HTML, CSS e JavaScript simples, sem framework nem passo de build. Abre `index.html` no browser para ver com dados de teste, ou corre `cargo run -p tasca-app` para a abrir na app.

- `api.js`: camada de dados. Na app chama os comandos Rust com `invoke`; um comando que o Rust ainda não tenha cai para os dados de teste. A lista de comandos e eventos está no topo do ficheiro.
- `app.js`: ecrãs (grupos, canais, chat, membros, chamadas com câmara e partilha de ecrã, plugins, definições, cópias de segurança).
- `styles.css`: estilos com as cores da marca. Tema Madeira (escuro) por defeito, Cal (claro) como alternativa.

As letras (Inter, JetBrains Mono) vêm do Google Fonts; para a app funcionar sem internet devem passar a ficheiros locais.
