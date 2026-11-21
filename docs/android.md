# Tasca para Android

A app Android é a mesma app Tauri do PC (`apps/desktop`), compilada como biblioteca
(`tasca_app_lib`) e embrulhada pelo projeto Android em `apps/desktop/gen/android`.
A interface (`apps/desktop/ui`) deteta o telemóvel e usa o modo de ecrã estreito.

## Descarregar

O GitHub Actions cria o APK a cada push para o `main` e junta-o à versão:

    https://github.com/RubenFernandes2004/Tasca/releases/latest/download/Tasca.apk

No telemóvel, ao abrir o APK, o Android pede para permitir instalar apps desta fonte.
Para atualizar, instala o APK novo por cima (não há atualização silenciosa no Android).

## Assinatura

Segredos do repositório (Settings > Secrets and variables > Actions):
`ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`.
Sem eles, o APK é assinado com uma chave temporária, diferente em cada versão, e é
preciso desinstalar antes de instalar a seguinte.

## Compilar à mão

Precisa do Android SDK, NDK, Java 17 e do alvo Rust `aarch64-linux-android`:

    cd apps/desktop
    export NDK_HOME=$ANDROID_HOME/ndk/<versão>
    npx @tauri-apps/cli@2 android build --apk --target aarch64
    # ou, com o telemóvel ligado por USB:
    npx @tauri-apps/cli@2 android dev

## O que muda no telemóvel

- Sem barra de janela; grupos e canais numa gaveta (botão ☰ ou deslizar da esquerda).
- Botão de voltar do Android fecha menus, janelas e definições; no ecrã principal, duas vezes para sair.
- Microfone e câmara: o Android pergunta ao entrar na chamada / ligar a câmara.
- "Lembrar a conta" guarda as palavras na pasta privada da app (no PC usa o cofre do sistema).

## Ainda não funciona no Android

- Partilha de ecrã (o WebView do Android não a suporta; o botão está escondido).
- Notificações de mensagens com a app em segundo plano, e a app continuar ligada em
  segundo plano: o Android suspende-a pouco depois de sair dela (falta um serviço em primeiro plano).
- Abrir com o sistema e atualização automática (só no PC).
- Guardar as 12 palavras num `.txt` (botão escondido; usa "Copiar").
- Login com Google: usa o fluxo de desktop (redireciona para 127.0.0.1); não foi testado no telemóvel.
- Só é gerado APK para ARM 64 bits (quase todos os telemóveis desde 2017).
