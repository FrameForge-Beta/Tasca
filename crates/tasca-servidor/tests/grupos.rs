//! Grupos com caixa no servidor: chave entregue ao entrar, pedidos com o anfitrião
//! desligado, réplica reconstruída só a partir do servidor e registos falsos recusados.

use std::{net::SocketAddr, time::Duration};

use tasca_core::{
    EventKind, Identity, Invite, Node, NodeOptions, SignedEvent, Store,
    caixa::ItemGrupo, servidor::AccountServer,
};
use tasca_servidor::{AppState, router};

async fn servidor() -> AccountServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(AppState::in_memory().unwrap()).into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    AccountServer::new(&format!("http://127.0.0.1:{}/", addr.port())).unwrap()
}

async fn node(srv: &AccountServer) -> Node {
    Node::spawn(
        Identity::generate(),
        Store::open_in_memory().unwrap(),
        NodeOptions { local_only: true, servidor: Some(srv.clone()), ..Default::default() },
    )
    .await
    .unwrap()
}

fn textos(n: &Node, g: tasca_core::GroupId) -> Vec<String> {
    n.with_store(|s| s.events_since(g, 0).unwrap())
        .into_iter()
        .filter_map(|(_, e)| match e.body.kind {
            EventKind::Message { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn grupo_funciona_pelo_servidor() {
    let srv = servidor().await;
    let host = node(&srv).await;
    let g = host.create_group("Malta", "Tiago").unwrap();
    host.open_hosted(g).unwrap().send_message("geral", "primeira").await.unwrap();
    host.sync_group(g, 0).await.unwrap();

    // A Ana entra por P2P e recebe a chave da caixa.
    let ana = node(&srv).await;
    let invite = Invite::decode(&host.create_invite(g).await.unwrap().encode()).unwrap();
    ana.join(&invite, "Ana").await.unwrap();
    for _ in 0..100 {
        if ana.with_store(|s| s.group_key(g).unwrap()).is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let (epoch, chave, _, _) = ana.with_store(|s| s.group_key(g).unwrap()).expect("chave do grupo");

    // Com o anfitrião "desligado", a Ana deixa uma mensagem na caixa.
    ana.open_offline(g).unwrap().send_message("geral", "deixei isto no servidor").await.unwrap();

    // Um membro malicioso tenta pôr um registo com número de ordem inventado.
    let falso = SignedEvent::sign(
        ana.identity(),
        Some(g),
        EventKind::Message { channel: "geral".into(), text: "falso".into() },
    );
    let item = ItemGrupo::registo(ana.identity(), &g, 99, falso);
    srv.caixa_enviar_bloco(&chave, chave.seal_item(&item).unwrap()).await.unwrap();

    // O anfitrião volta, aceita o pedido e copia-o para a caixa.
    assert_eq!(host.sync_group(g, 0).await.unwrap(), 1);
    assert_eq!(textos(&host, g), ["primeira", "deixei isto no servidor"]);

    // Um PC que só tem a chave reconstrói o grupo inteiro a partir do servidor.
    let novo = node(&srv).await;
    novo.with_store(|s| s.set_group_key(g, epoch, &chave, 0).unwrap());
    novo.sync_group(g, 0).await.unwrap();
    assert_eq!(textos(&novo, g), ["primeira", "deixei isto no servidor"]);
    assert_eq!(
        novo.with_store(|s| s.events_since(g, 0).unwrap()),
        host.with_store(|s| s.events_since(g, 0).unwrap())
    );

    // Trocar a chave: o que vem a seguir vai para uma caixa nova.
    let nova_epoca = host.rotate_group_key(g).unwrap();
    assert_eq!(nova_epoca, epoch + 1);
    host.open_hosted(g).unwrap().send_message("geral", "depois da troca").await.unwrap();
    host.sync_group(g, 0).await.unwrap();
    assert_eq!(novo.sync_group(g, 0).await.unwrap(), 0, "a chave antiga já não recebe nada novo");
    let (_, chave2, _, _) = host.with_store(|s| s.group_key(g).unwrap()).unwrap();
    assert_eq!(srv.caixa_ler_blocos(&chave2, 0, 0).await.unwrap().len(), 1);
}
