# Fazer plugins para a Tasca

Um plugin é um ficheiro JavaScript. Corre dentro da Tasca, num espaço isolado, e fala com a
app através do objeto `tasca`. Não tem acesso à rede, aos teus ficheiros nem ao resto da
interface. Só pode fazer o que as permissões dele deixam.

## O primeiro plugin

```js
// ola.js
tasca.comando("ola", "Diz olá ao canal", ({ args, channelId }) => {
  tasca.enviarMensagem(channelId, "Olá" + (args ? ", " + args : "") + "! 👋");
});
```

Para o experimentar:

1. Abre **Definições > Plugins > Carregar ficheiro** e escolhe `ola.js`.
2. Num canal, escreve `/ola Tasca`.

Os plugins carregados à mão aparecem como **Não verificado** e têm todas as permissões.

## API

Todas as funções devolvem uma `Promise`. Se o plugin não tiver a permissão precisa, a
`Promise` falha com o erro `sem permissão: …`.

| Função | Permissão | O que faz |
| --- | --- | --- |
| `tasca.comando(nome, descricao, fn)` | `comandos` | Regista `/nome`. Quando alguém o escreve, chama `fn({ args, channelId })`. O texto não é enviado para o chat. |
| `tasca.enviarMensagem(canal, texto)` | `enviar mensagens` | Envia uma mensagem em teu nome. `canal` vem de `channelId`. Se for `null`, vai para o canal aberto. Aceita `**negrito**` e `` `código` ``. |
| `tasca.aoReceberMensagem(fn)` | `ler mensagens` | Chama `fn({ channelId, from, text })` sempre que chega uma mensagem. |
| `tasca.guardar(chave, valor)` | `guardar dados` | Guarda um valor que se possa passar para JSON, até 200 KB. Fica só neste dispositivo. |
| `tasca.ler(chave)` | `guardar dados` | Lê o que guardaste, ou `null` se não houver nada. |
| `tasca.toast(texto)` | nenhuma | Mostra um aviso curto com o nome do plugin. |
| `tasca.canalAtual()` | nenhuma | Devolve o id do canal aberto, ou `null`. |

## Limites

- Um ficheiro só, até 2 MB.
- Não há `fetch`, `XMLHttpRequest`, `WebSocket`, `localStorage` nem acesso ao `document` da
  app. A política de segurança do iframe bloqueia tudo isto.
- `setTimeout`, `crypto.getRandomValues`, `Date`, `Intl` e o resto do JavaScript normal
  funcionam.
- Só os comandos de texto (`/nome`) chegam ao plugin. Por enquanto, nenhum plugin desenha
  na interface.

## Pôr o plugin na loja

A loja é a pasta [`plugins/`](../plugins) deste repositório.

1. Cria `plugins/<id>/plugin.js`. O `id` usa só letras, números e `-`.
2. Junta a entrada a `plugins/catalogo.base.json`:
   ```json
   {
     "id": "ola",
     "name": "Olá",
     "author": "o teu nome",
     "version": "1.0.0",
     "color": "#2A5CAA",
     "icon": "puzzle",
     "desc": "Diz olá com /ola.",
     "perms": ["comandos", "enviar mensagens"],
     "file": "ola/plugin.js"
   }
   ```
   Pede só as permissões que o plugin usa: a app recusa as outras.
3. Abre um pull request.

A equipa revê o código. Depois corre:

```sh
TASCA_CHAVE_LOJA=<chave privada> python3 plugins/assinar.py
```

O script escreve `catalogo.json`, com o SHA-256 de cada ficheiro, e `catalogo.json.sig`,
a assinatura Ed25519 da equipa.

A app faz estas verificações:

- Só aceita o catálogo se a assinatura bater com a chave pública que traz embutida (em
  `apps/desktop/src/plugins.rs`).
- Antes de instalar, confirma o SHA-256 de cada plugin.

Por isso, um plugin alterado depois da revisão não se instala. A chave privada nunca vai
para o repositório.

## Ícones e cores

- `icon` aceita um nome dos ícones da app: `bell`, `music`, `grid`, `globe`, `puzzle`,
  `palette`, `wave`, `cam` e outros. Se não reconhecer o nome, mostra `puzzle`.
- `color` é a cor do quadrado do ícone, em `#RRGGBB`.

## Exemplos

- [`plugins/lembretes`](../plugins/lembretes/plugin.js): `/lembrar 10m texto` guarda
  lembretes e envia-os à hora certa.
- [`plugins/dados`](../plugins/dados/plugin.js): `/dados 2d6`, `/moeda` e
  `/escolher a, b`.
