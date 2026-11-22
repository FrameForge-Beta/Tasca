# Servidor de contas

O servidor de contas deixa entrar na Tasca noutro PC com **nome de utilizador e palavra-passe**.

Ele guarda, por cada utilizador:
- o cofre da conta, já cifrado no PC com a palavra-passe;
- a chave pública;
- um verificador da palavra-passe.

Nunca vê a palavra-passe e não consegue abrir os cofres.

Guarda também as **caixas** das conversas e dos grupos: blocos cifrados de ponta a ponta,
pela ordem em que chegam. Os grupos vivem aí, por isso funcionam com o PC do dono
desligado. O servidor não sabe de que grupo é cada caixa nem o que lá está. Diz também
quem está online em cada grupo (blocos cifrados que só guarda em memória durante 90 s).
Chamadas e ficheiros não passam por ele.

## 1. Arrancar no mini servidor (Docker)

É preciso Docker com o plugin `compose`.

```sh
git clone https://github.com/RubenFernandes2004/tasca
cd tasca/deploy/servidor
nano Caddyfile        # troca contas.exemplo.pt pelo teu domínio
docker compose up -d --build
```

O Caddy pede sozinho o certificado HTTPS ao Let's Encrypt. Para isso:
- o domínio tem de apontar para o IP público da tua casa;
- o router tem de reencaminhar as portas **80** e **443** para o mini servidor.

As contas ficam no volume Docker `contas`, no ficheiro `contas.db`. Faz cópias desse volume de vez em quando.

Para confirmar que está tudo a funcionar, abre `https://o-teu-dominio/saude` no browser. Deve aparecer `ok`.

### Atualizar

Quando sai uma versão nova do servidor:

```sh
cd tasca && git pull
cd deploy/servidor && docker compose up -d --build
```

Os dados ficam no volume, não se perdem.

### Não tens domínio?

- **DuckDNS** (grátis): cria por exemplo `tasca-tiago.duckdns.org` e põe-no no Caddyfile. Se o teu IP muda, instala o atualizador deles.
- **Cloudflare Tunnel**: dispensa abrir portas no router, mas precisa de um domínio na Cloudflare. Nesse caso tira o serviço `caddy` do `docker-compose.yml` e aponta o túnel para `http://contas:8080`.

## 2. Ligar a app ao servidor

No GitHub, vai a **Settings → Secrets and variables → Actions → Variables** e cria a variável:

- `TASCA_SERVIDOR_URL` = `https://o-teu-dominio`

A versão seguinte da app já mostra "Entrar com utilizador e palavra-passe" e pede o nome de utilizador ao criar conta. Sem esta variável, a app funciona como antes.

## Limites de segurança

- 10 tentativas erradas por utilizador bloqueiam o login durante 15 minutos.
- Cada IP pode criar no máximo 5 contas por hora.
- A app recusa ligar-se a um servidor sem `https://`.

## Sem Docker

Também podes compilar e correr o servidor diretamente:

```sh
cargo build --release -p tasca-servidor
TASCA_DADOS=/var/lib/tasca/contas.db TASCA_ENDERECO=127.0.0.1:8080 TASCA_PROXY=1 ./target/release/tasca-servidor
```

Mete à frente um proxy com HTTPS (Caddy, nginx). Usa `TASCA_PROXY=1` só quando o servidor estiver atrás desse proxy.
