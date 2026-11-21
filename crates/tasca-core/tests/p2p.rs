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

#[tokio::test]
async fn canais_so_o_dono_muda_e_chegam_aos_membros() {
    use tasca_core::{ChannelDef, ChannelKind};
    let host = node().await;
    let group = host.create_group("Malta", "Tiago").unwrap();
    let invite = host.create_invite(group).await.unwrap();
    let member = node().await;
    let mut member_rx = member.subscribe(group);
    let chat = member.join(&invite, "Zé").await.unwrap();
    assert!(chat.is_connected());

    let canal = |id: &str| ChannelDef {
        id: id.into(),
        kind: ChannelKind::Text,
        name: id.into(),
        topic: String::new(),
        category: "Conversa".into(),
    };
    host.open_hosted(group)
        .unwrap()
        .set_layout(vec![canal("geral"), canal("memes")], vec!["Conversa".into(), "Vazia".into()])
        .await
        .unwrap();
    loop {
        let (_, ev) = timeout(Duration::from_secs(10), member_rx.recv()).await.unwrap().unwrap();
        if matches!(ev.body.kind, EventKind::Layout { .. }) {
            break;
        }
    }
    let (canais, cats) = member.with_store(|s| s.layout(group).unwrap());
    assert_eq!(canais.len(), 2);
    assert_eq!(cats, vec!["Conversa".to_string(), "Vazia".to_string()]);

    // Um membro não consegue mudar os canais: o anfitrião recusa.
    chat.set_layout(vec![canal("meu")], vec![]).await.unwrap();
    chat.send_message("geral", "depois").await.unwrap();
    assert_eq!(next_text(&mut member_rx).await, "depois");
    assert_eq!(host.with_store(|s| s.layout(group).unwrap()).0.len(), 2);
    // Sem nenhum canal de texto, nem chega a sair.
    let voz = ChannelDef { kind: ChannelKind::Voice, ..canal("sala") };
    assert!(host.open_hosted(group).unwrap().set_layout(vec![voz], vec![]).await.is_err());
}

/// Pela internet a sério (relays e descoberta públicos do iroh): entra com convite, a
/// ligação cai, e o membro volta a ligar só com a chave do dono. `cargo test -- --ignored`.
#[tokio::test]
#[ignore]
async fn volta_a_ligar_pela_internet() {
    let net = || async { Node::spawn(Identity::generate(), Store::open_in_memory().unwrap(), NodeOptions::default()).await.unwrap() };
    let host = net().await;
    let group = host.create_group("Malta", "Tiago").unwrap();
    let invite = host.create_invite(group).await.unwrap();
    let member = net().await;
    let chat = timeout(Duration::from_secs(60), member.join(&invite, "Zé")).await.unwrap().unwrap();
    assert!(chat.is_connected());
    while member.with_store(|s| s.group(group).unwrap()).is_none() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let mut rx = member.subscribe(group);
    let again = timeout(Duration::from_secs(60), member.reconnect(group)).await.unwrap().unwrap();
    host.open_hosted(group).unwrap().send_message("geral", "outra vez").await.unwrap();
    assert_eq!(next_text(&mut rx).await, "outra vez");
    drop((chat, again));
}

#[tokio::test]
async fn presenca_sala_de_voz_e_perfil() {
    let host = node().await;
    let group = host.create_group("Malta", "Tiago").unwrap();
    let hosted = host.open_hosted(group).unwrap();
    let invite = host.create_invite(group).await.unwrap();
    let member = node().await;
    let chat = member.join(&invite, "Zé").await.unwrap();
    let eu = *member.identity().public().as_bytes();
    let dono = *host.identity().public().as_bytes();

    // O membro fica a ver o anfitrião e a si próprio online.
    let mut pres = member.presence(group);
    timeout(Duration::from_secs(10), pres.wait_for(|p| p.contains_key(&eu) && p.contains_key(&dono)))
        .await
        .unwrap()
        .unwrap();

    // Sala de voz: o membro avisa, o anfitrião e o próprio membro veem.
    chat.set_voice(Some("sala".into())).await.unwrap();
    let mut host_pres = host.presence(group);
    timeout(Duration::from_secs(10), host_pres.wait_for(|p| p.get(&eu) == Some(&Some("sala".into()))))
        .await
        .unwrap()
        .unwrap();
    hosted.set_voice(Some("sala".into())).await.unwrap();
    timeout(Duration::from_secs(10), pres.wait_for(|p| p.get(&dono) == Some(&Some("sala".into()))))
        .await
        .unwrap()
        .unwrap();

    // Perfil: nome novo, cor e foto chegam ao anfitrião.
    chat.set_profile("Zé Manel", "#2A5CAA", Some("data:image/webp;base64,AAAA".into())).await.unwrap();
    let mut rx = host.subscribe(group);
    loop {
        let (_, ev) = timeout(Duration::from_secs(10), rx.recv()).await.unwrap().unwrap();
        if matches!(ev.body.kind, EventKind::Profile { .. }) {
            break;
        }
    }
    let k = member.identity().public();
    assert_eq!(host.with_store(|s| s.display_name(group, &k).unwrap()).as_deref(), Some("Zé Manel"));
    let perfil = host.with_store(|s| s.profiles(group).unwrap()).remove(&eu).unwrap();
    assert_eq!(perfil.color, "#2A5CAA");

    // Ao sair, deixa de estar online para o anfitrião.
    member.shutdown().await.unwrap();
    timeout(Duration::from_secs(10), host_pres.wait_for(|p| !p.contains_key(&eu)))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn amigos_trocam_estado_e_perfil() {
    use tasca_core::amigos::{Estado, Perfil};
    let ana = node().await;
    let rui = node().await;
    let ka = *ana.identity().public().as_bytes();
    let kr = *rui.identity().public().as_bytes();
    let perfil = |n: &str| Perfil { nome: n.into(), cor: "#9B2C3F".into(), bio: format!("sou {n}"), ..Default::default() };
    ana.amigos().definir_eu(Estado { estado: "online".into(), mensagem: "a programar".into() }, perfil("Ana"));
    rui.amigos().definir_eu(Estado { estado: "dnd".into(), mensagem: String::new() }, perfil("Rui"));

    // Ainda não são amigos: o Rui não responde.
    assert_eq!(ana.perguntar_amigo(rui.addr()).await.unwrap(), None);

    ana.amigos().definir_lista([kr]);
    rui.amigos().definir_lista([ka]);
    let mut novidades_rui = rui.amigos().subscrever();
    let mut novidades_ana = ana.amigos().subscrever();
    let e = ana.perguntar_amigo(rui.addr()).await.unwrap().expect("online");
    assert_eq!(e.estado, "dnd");
    let n = timeout(Duration::from_secs(5), novidades_ana.recv()).await.unwrap().unwrap();
    assert_eq!(n.perfil.unwrap().bio, "sou Rui");
    // O Rui fica a saber que a Ana está online e recebe o perfil dela.
    let n = timeout(Duration::from_secs(5), novidades_rui.recv()).await.unwrap().unwrap();
    assert_eq!(n.quem, ka);
    assert_eq!(n.estado.unwrap().mensagem, "a programar");
    assert_eq!(n.perfil.unwrap().nome, "Ana");

    // Segunda vez: o perfil não mudou, não volta a ir.
    ana.perguntar_amigo(rui.addr()).await.unwrap();
    let n = timeout(Duration::from_secs(5), novidades_ana.recv()).await.unwrap().unwrap();
    assert!(n.perfil.is_none());

    // Invisível: parece offline.
    rui.amigos().definir_eu(Estado { estado: "offline".into(), mensagem: String::new() }, perfil("Rui"));
    assert_eq!(ana.perguntar_amigo(rui.addr()).await.unwrap(), None);
}
