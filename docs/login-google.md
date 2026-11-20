# Configurar o login com Google

A Tasca usa o login da Google só para guardar uma cópia **encriptada** da conta no teu Google Drive, numa pasta escondida que só a app vê. Para isso a Google exige um "cliente OAuth", que é grátis.

## 1. Criar o projeto

1. Vai a <https://console.cloud.google.com/> e entra com a tua conta Google.
2. No topo, carrega no seletor de projetos e depois em **Novo projeto**. Nome: `Tasca`. Clica em **Criar**.

## 2. Ativar a API do Google Drive

1. Menu ☰ → **APIs e serviços** → **Biblioteca**.
2. Procura **Google Drive API** e carrega em **Ativar**.

## 3. Ecrã de consentimento

1. Menu ☰ → **APIs e serviços** → **Ecrã de consentimento OAuth** (ou "Google Auth Platform").
2. Tipo de utilizador: **Externo**.
3. Nome da app: `Tasca`, email de suporte: o teu.
4. Em **Acesso a dados / Âmbitos**, adiciona:
   - `openid`
   - `.../auth/userinfo.email`
   - `.../auth/drive.appdata`
5. Em **Público / Utilizadores de teste**, adiciona o teu email e o dos amigos que vão testar.

Enquanto a app estiver em modo de teste, só esses emails conseguem entrar (até 100). Mais tarde, para abrir a toda a gente, carregas em **Publicar app**. A Google pode pedir uma verificação.

## 4. Criar o cliente OAuth

1. Menu ☰ → **APIs e serviços** → **Credenciais** → **Criar credenciais** → **ID do cliente OAuth**.
2. Tipo de aplicação: **App para computador** (Desktop app). Nome: `Tasca Desktop`.
3. Copia o **ID do cliente** e o **Segredo do cliente**.

## 5. Pôr as credenciais na Tasca

No GitHub, vai a **Settings → Secrets and variables → Actions** e cria dois secrets:

- `TASCA_GOOGLE_CLIENT_ID`
- `TASCA_GOOGLE_CLIENT_SECRET`

A compilação da app junta-os automaticamente. Para testar no teu PC, define as mesmas variáveis de ambiente antes de correr a app:

```
set TASCA_GOOGLE_CLIENT_ID=...
set TASCA_GOOGLE_CLIENT_SECRET=...
```

Em apps de computador, a Google não considera o "segredo do cliente" verdadeiramente secreto, porque vai dentro da app. A segurança vem da palavra-passe: o ficheiro guardado no Drive está encriptado, e nem a Google nem quem tiver estas credenciais o consegue abrir.
