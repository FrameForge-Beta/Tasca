//! Mensagens cifradas no servidor: escrever, ler, esperar por novas e acesso negado.

use std::{net::SocketAddr, time::Duration};

use tasca_core::{ChaveCaixa, EventKind, Identity, SignedEvent, servidor::AccountServer};
use tasca_servidor::{AppState, router};

async fn start() -> (AccountServer, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(AppState::in_memory().unwrap()).into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let url = format!("http://127.0.0.1:{}/", addr.port());
    (AccountServer::new(&url).unwrap(), url)
}

fn msg(who: &Identity, text: &str) -> SignedEvent {
    SignedEvent::sign(who, None, EventKind::Message { channel: "geral".into(), text: text.into() })
}

#[tokio::test]
async fn conversa_privada_pelo_servidor() {
    let (srv, url) = start().await;
    let ana = Identity::generate();
    let rui = Identity::generate();
    let k_ana = ChaveCaixa::privada(&ana, &rui.public()).unwrap();
    let k_rui = ChaveCaixa::privada(&rui, &ana.public()).unwrap();

    // Caixa que ainda não existe: vazia.
    assert!(srv.caixa_ler(&k_rui, 0, 0).await.unwrap().is_empty());

    // A Ana escreve com o Rui desligado; o Rui lê depois.
    assert_eq!(srv.caixa_enviar(&k_ana, &msg(&ana, "olá Rui")).await.unwrap(), 1);
    assert_eq!(srv.caixa_enviar(&k_ana, &msg(&ana, "estás aí?")).await.unwrap(), 2);
    let lidas = srv.caixa_ler(&k_rui, 0, 0).await.unwrap();
    assert_eq!(lidas.len(), 2);
    assert_eq!(lidas[1].1.author().unwrap(), ana.public());
    assert!(matches!(&lidas[0].1.body.kind, EventKind::Message { text, .. } if text == "olá Rui"));
    assert_eq!(srv.caixa_ler(&k_rui, 2, 0).await.unwrap().len(), 0);

    // O Rui fica à espera; a resposta da Ana chega logo.
    let srv2 = AccountServer::new(&url).unwrap();
    let k2 = k_rui.clone();
    let espera = tokio::spawn(async move { srv2.caixa_ler(&k2, 2, 20).await.unwrap() });
    tokio::time::sleep(Duration::from_millis(300)).await;
    let t = std::time::Instant::now();
    srv.caixa_enviar(&k_ana, &msg(&ana, "agora sim")).await.unwrap();
    let novas = espera.await.unwrap();
    assert_eq!(novas.len(), 1);
    assert!(t.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn sem_a_chave_nao_ha_acesso() {
    let (srv, url) = start().await;
    let ana = Identity::generate();
    let k = ChaveCaixa::generate().unwrap();
    srv.caixa_enviar(&k, &msg(&ana, "segredo")).await.unwrap();

    // Mesmo id da caixa com token errado: recusado.
    let http = reqwest::Client::new();
    let r = http
        .get(format!("{url}v1/caixas/{}", k.id_hex()))
        .bearer_auth(hex::encode([7u8; 32]))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
    let r = http.post(format!("{url}v1/caixas/{}", k.id_hex())).body("x").send().await.unwrap();
    assert_eq!(r.status(), 401);

    // Outra chave vê outra caixa, vazia.
    assert!(srv.caixa_ler(&ChaveCaixa::generate().unwrap(), 0, 0).await.unwrap().is_empty());
}
