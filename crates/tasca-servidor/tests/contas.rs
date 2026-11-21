//! Teste de ponta a ponta: servidor real numa porta local + cliente do tasca-core.

use std::net::SocketAddr;

use tasca_core::{Account, servidor::AccountServer};
use tasca_servidor::{AppState, router};

async fn start() -> AccountServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(AppState::in_memory().unwrap())
        .into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    AccountServer::new(&format!("http://127.0.0.1:{}/", addr.port())).unwrap()
}

#[tokio::test]
async fn registar_entrar_mudar_palavra_passe() {
    let srv = start().await;
    let pw = "uma palavra-passe longa";
    let (conta, _frase) = Account::create("Tiago", pw).unwrap();
    let cofre = conta.seal(pw).unwrap();

    srv.register("Tiago", pw, &cofre).await.unwrap();

    // Nome repetido (maiúsculas não contam).
    let (outra, _) = Account::create("Outro", pw).unwrap();
    let err = srv.register("tiago", pw, &outra.seal(pw).unwrap()).await.unwrap_err();
    assert!(err.to_string().contains("ocupado"), "{err}");

    // Entrar noutro PC.
    let (v, a) = srv.login("tiago", pw).await.unwrap();
    assert_eq!(v.public_key().unwrap(), cofre.public_key().unwrap());
    assert_eq!(a.identity.public(), conta.identity.public());

    // Palavra-passe errada.
    assert!(srv.login("tiago", "palavra-passe errada!!").await.is_err());

    // Procurar pelo nome.
    let info = srv.lookup("Tiago").await.unwrap().unwrap();
    assert_eq!(info.public_key, conta.identity.public().to_string());
    assert!(srv.lookup("ninguem").await.unwrap().is_none());

    // Mudar a palavra-passe: o cofre novo vai para o servidor.
    let novo = "outra palavra-passe longa";
    srv.update("tiago", pw, Some(novo), &conta.seal(novo).unwrap()).await.unwrap();
    assert!(srv.login("tiago", pw).await.is_err());
    srv.login("tiago", novo).await.unwrap();

    // Não se pode trocar o cofre por um de outra conta.
    assert!(srv.update("tiago", novo, None, &outra.seal(novo).unwrap()).await.is_err());

    // Mudar o nome: o antigo fica livre, entra-se com o novo e a mesma palavra-passe.
    srv.register("outro", pw, &outra.seal(pw).unwrap()).await.unwrap();
    let err = srv.rename("tiago", "outro", novo).await.unwrap_err();
    assert!(err.to_string().contains("ocupado"), "{err}");
    assert!(srv.rename("tiago", "tiagof", "palavra-passe errada!!").await.is_err());
    assert_eq!(srv.rename("tiago", "TiagoF", novo).await.unwrap(), "tiagof");
    assert!(srv.login("tiago", novo).await.is_err());
    assert!(srv.lookup("tiago").await.unwrap().is_none());
    let (_, a) = srv.login("tiagof", novo).await.unwrap();
    assert_eq!(a.identity.public(), conta.identity.public());
}

#[tokio::test]
async fn bloqueia_depois_de_muitas_tentativas() {
    let srv = start().await;
    let pw = "uma palavra-passe longa";
    let (conta, _) = Account::create("Ana", pw).unwrap();
    srv.register("ana", pw, &conta.seal(pw).unwrap()).await.unwrap();
    for _ in 0..10 {
        let _ = srv.login("ana", "palavra-passe errada!!").await;
    }
    let err = srv.login("ana", pw).await.unwrap_err();
    assert!(err.to_string().to_lowercase().contains("tentativas"), "{err}");
}
