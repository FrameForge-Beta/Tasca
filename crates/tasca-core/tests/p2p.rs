//! Teste de ponta a ponta: dois nós no mesmo PC a conversar por P2P.

use std::time::Duration;

use tasca_core::{EventKind, Identity, Invite, Node, NodeOptions, Store};
use tokio::time::timeout;

async fn node() -> Node {
    Node::spawn(
        Identity::generate(),
        Store::open_in_memory().unwrap(),
        NodeOptions {
            local_only: true,
            ..Default::default()
        },
    )
    .await
    .unwrap()
}

async fn next_text(rx: &mut tokio::sync::broadcast::Receiver<tasca_core::node::Update>) -> String {
    loop {
        let (_, ev) = timeout(Duration::from_secs(10), rx.recv())
            .await
            .expect("timeout")
            .unwrap();
        if let EventKind::Message { text, .. } = ev.body.kind {
            return text;
        }
    }
}

#[tokio::test]
async fn conversa_entre_anfitriao_e_membro() {
    let host = node().await;
    let group = host.create_group("Malta", "Tiago").unwrap();
    let host_chat = host.open_hosted(group).unwrap();
    host_chat
        .send_message("geral", "antes de entrares")
        .await
        .unwrap();
    let mut host_rx = host.subscribe(group);

    // O convite passa por texto, como seria copiado e colado.
    let code = host.create_invite(group).await.unwrap().encode();
    let invite = Invite::decode(&code).unwrap();

    let member = node().await;
    let mut member_rx = member.subscribe(group);
    let chat = member.join(&invite, "Zé").await.unwrap();

    // O membro recebe o histórico antigo.
    assert_eq!(next_text(&mut member_rx).await, "antes de entrares");

    chat.send_message("geral", "olá a todos").await.unwrap();
    assert_eq!(next_text(&mut host_rx).await, "olá a todos");
    assert_eq!(next_text(&mut member_rx).await, "olá a todos");

    host_chat.send_message("geral", "bem-vindo").await.unwrap();
    assert_eq!(next_text(&mut member_rx).await, "bem-vindo");

    // Ambos têm o mesmo histórico e o nome do membro.
    let host_log = host.with_store(|s| s.events_since(group, 0).unwrap());
    let member_log = member.with_store(|s| s.events_since(group, 0).unwrap());
    assert_eq!(host_log, member_log);
    let zé = member.identity().public();
    assert_eq!(
        host.with_store(|s| s.display_name(group, &zé).unwrap())
            .as_deref(),
        Some("Zé")
    );
}

#[tokio::test]
async fn convite_falso_e_recusado() {
    let host = node().await;
    let group = host.create_group("Malta", "Tiago").unwrap();
    let mut invite = host.create_invite(group).await.unwrap();
    invite.token = [0; 16];
    let intruso = node().await;
    let err = intruso
        .join(&invite, "Intruso")
        .await
        .err()
        .expect("devia ser recusado");
    assert!(err.to_string().contains("recusou"), "{err:#}");
}
