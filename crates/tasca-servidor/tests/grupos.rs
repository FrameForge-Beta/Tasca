//! Grupos que vivem na caixa do servidor: funcionam sem o PC do dono ligado. Entrar com
//! convite, mensagens, regras (permissões), expulsar com troca de chave, presença, e
//! registos falsos recusados.

use std::{net::SocketAddr, time::Duration};

use tasca_core::{
    EventKind, GroupId, Identity, Node, NodeOptions, SignedEvent, Store,
    caixa::ItemGrupo,
    cargos::{self, Cargo},
    invite::Convite,
    servidor::AccountServer,
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
    node_com(srv, Identity::generate()).await
}

async fn node_com(srv: &AccountServer, id: Identity) -> Node {
    Node::spawn(
        id,
        Store::open_in_memory().unwrap(),
        NodeOptions { local_only: true, servidor: Some(srv.clone()), ..Default::default() },
    )
    .await
    .unwrap()
}

fn textos(n: &Node, g: GroupId) -> Vec<String> {
    n.with_store(|s| s.events_since(g, 0).unwrap())
        .into_iter()
        .filter_map(|(_, e)| match e.body.kind {
            EventKind::Message { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

fn ids(n: &Node, g: GroupId) -> Vec<[u8; 32]> {
    n.with_store(|s| s.events_since(g, 0).unwrap()).into_iter().map(|(_, e)| e.id()).collect()
}

/// Escreve no #geral e espera que a mensagem chegue ao servidor e volte (o envio é em
/// segundo plano).
async fn escrever(n: &Node, g: GroupId, texto: &str) {
    n.open_offline(g).unwrap().send_message("geral", texto).await.unwrap();
    for _ in 0..100 {
        n.sync_group(g, 0).await.unwrap();
        if textos(n, g).iter().any(|t| t == texto) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("a mensagem «{texto}» não voltou do grupo");
}

fn membro(n: &Node, g: GroupId, quem: &Node) -> bool {
    n.with_store(|s| s.is_member(g, &quem.identity().public()).unwrap())
}

/// Dono cria o grupo, escreve, faz um convite e desliga.
async fn grupo_sem_dono(srv: &AccountServer) -> (Node, GroupId, Convite) {
    let dono = node(srv).await;
    let g = dono.create_group("Malta", "Tiago").unwrap();
    let h = dono.open_offline(g).unwrap();
    escrever(&dono, g, "primeira").await;
    let convite = h.criar_convite(0, 0).await.unwrap();
    dono.sync_group(g, 0).await.unwrap();
    assert_eq!(textos(&dono, g), ["primeira"]);
    (dono, g, Convite::decode(&convite.encode()).unwrap())
}

#[tokio::test]
async fn grupo_funciona_sem_o_dono() {
    let srv = servidor().await;
    let (dono, g, convite) = grupo_sem_dono(&srv).await;
    let dono_id = dono.identity().clone();
    dono.shutdown().await.unwrap(); // o PC do dono desliga

    // A Ana e o Bruno entram com o convite, sem o dono.
    let ana = node(&srv).await;
    ana.entrar_por_convite(&convite, "Ana").await.unwrap();
    let bruno = node(&srv).await;
    bruno.entrar_por_convite(&convite, "Bruno").await.unwrap();
    assert!(membro(&ana, g, &ana) && membro(&bruno, g, &ana) && membro(&bruno, g, &bruno));
    assert_eq!(textos(&ana, g), ["primeira"]);

    // Conversam: as mensagens ficam logo a valer (não ficam "à espera do anfitrião").
    escrever(&ana, g, "olá Bruno").await;
    escrever(&bruno, g, "olá Ana").await;
    ana.sync_group(g, 0).await.unwrap();
    bruno.sync_group(g, 0).await.unwrap();
    ana.sync_group(g, 0).await.unwrap();
    assert_eq!(textos(&ana, g), ["primeira", "olá Bruno", "olá Ana"]);
    assert_eq!(ids(&ana, g), ids(&bruno, g), "todos chegam ao mesmo histórico, pela mesma ordem");
    assert!(ana.with_store(|s| s.pending(g).unwrap()).is_empty());

    // As regras valem sem o dono: o Bruno não pode mexer nos canais.
    let antes = bruno.with_store(|s| s.layout(g).unwrap());
    let mut canais = antes.0.clone();
    canais.retain(|c| c.id != "sala");
    bruno.open_offline(g).unwrap().set_layout(canais, antes.1.clone()).await.unwrap();
    bruno.sync_group(g, 0).await.unwrap();
    ana.sync_group(g, 0).await.unwrap();
    assert_eq!(bruno.with_store(|s| s.layout(g).unwrap()), antes);
    assert_eq!(ana.with_store(|s| s.layout(g).unwrap()), antes);

    // Um membro malicioso tenta pôr um registo "do anfitrião" com número inventado.
    let (_, chave, _, _) = ana.with_store(|s| s.group_key(g).unwrap()).unwrap();
    let falso = SignedEvent::sign(ana.identity(), Some(g), EventKind::Message { channel: "geral".into(), text: "falso".into() });
    let item = ItemGrupo::registo(ana.identity(), &g, 99, falso);
    srv.caixa_enviar_bloco(&chave, chave.seal_item(&item).unwrap()).await.unwrap();

    // Entrar sem convite (a prova não bate) não vale.
    let intruso = node(&srv).await;
    let ev = SignedEvent::sign(intruso.identity(), Some(g), EventKind::MemberJoined { display_name: "Intruso".into() });
    let item = ItemGrupo::Entrada { prova: vec![0; 64], convite: convite.publica(), event: ev };
    srv.caixa_enviar_bloco(&chave, chave.seal_item(&item).unwrap()).await.unwrap();
    ana.sync_group(g, 0).await.unwrap();
    assert!(!membro(&ana, g, &intruso));
    assert!(!textos(&ana, g).contains(&"falso".to_string()));

    // O dono volta (noutro PC, só com a conta e a chave) e fica com o mesmo grupo.
    let dono2 = node_com(&srv, dono_id).await;
    dono2.with_store(|s| s.set_group_key(g, 0, &chave, 0).unwrap());
    dono2.sync_group(g, 0).await.unwrap();
    assert_eq!(ids(&dono2, g), ids(&ana, g));
}

#[tokio::test]
async fn banir_troca_a_chave_sem_o_dono() {
    let srv = servidor().await;
    let (dono, g, convite) = grupo_sem_dono(&srv).await;
    let ana = node(&srv).await;
    ana.entrar_por_convite(&convite, "Ana").await.unwrap();
    let bruno = node(&srv).await;
    bruno.entrar_por_convite(&convite, "Bruno").await.unwrap();

    // O dono dá à Ana o cargo de moderadora e desliga.
    dono.sync_group(g, 0).await.unwrap();
    let hd = dono.open_offline(g).unwrap();
    let modr = Cargo { id: "mod".into(), name: "Mod".into(), color: String::new(), icon: None, hoist: false, mentionable: false, perms: cargos::EXPULSAR | cargos::BANIR };
    hd.set_roles(vec![modr, Cargo::todos()]).await.unwrap();
    hd.set_member_roles(*ana.identity().public().as_bytes(), vec!["mod".into()]).await.unwrap();
    dono.sync_group(g, 0).await.unwrap();
    dono.shutdown().await.unwrap();

    // A Ana bane o Bruno: a chave muda e o Bruno deixa de ler o grupo.
    ana.sync_group(g, 0).await.unwrap();
    ana.open_offline(g).unwrap().ban(*bruno.identity().public().as_bytes(), String::new(), 0).await.unwrap();
    ana.sync_group(g, 0).await.unwrap(); // aplica a expulsão e roda a chave
    ana.sync_group(g, 0).await.unwrap(); // lê a chave nova
    let (epoca, _, _, _) = ana.with_store(|s| s.group_key(g).unwrap()).unwrap();
    assert_eq!(epoca, 1);
    bruno.sync_group(g, 0).await.unwrap();
    assert!(!membro(&bruno, g, &bruno));
    assert_eq!(bruno.with_store(|s| s.group_key(g).unwrap()).unwrap().0, 0, "o Bruno não recebe a chave nova");

    escrever(&ana, g, "já sem o Bruno").await;
    ana.sync_group(g, 0).await.unwrap();
    assert!(textos(&ana, g).contains(&"já sem o Bruno".to_string()));
    bruno.sync_group(g, 0).await.unwrap();
    assert!(!textos(&bruno, g).contains(&"já sem o Bruno".to_string()));

    // Depois de banir, os convites antigos deixam de servir (o Bruno podia ter um).
    let carla = node(&srv).await;
    assert!(carla.entrar_por_convite(&convite, "Carla").await.is_err());

    // Um convite feito depois da troca (só com a chave nova) traz o histórico todo.
    let novo = ana.open_offline(g).unwrap().criar_convite(1, 0).await.unwrap();
    assert_eq!(novo.epoch, 1);
    let rui = node(&srv).await;
    rui.entrar_por_convite(&novo, "Rui").await.unwrap();
    assert_eq!(textos(&rui, g), ["primeira", "já sem o Bruno"]);

    // O convite era de um uso só.
    let zé = node(&srv).await;
    assert!(zé.entrar_por_convite(&novo, "Zé").await.is_err());
}

#[tokio::test]
async fn quem_esta_online_vem_do_servidor() {
    let srv = servidor().await;
    let (dono, g, convite) = grupo_sem_dono(&srv).await;
    dono.shutdown().await.unwrap();
    let ana = node(&srv).await;
    ana.entrar_por_convite(&convite, "Ana").await.unwrap();
    let bruno = node(&srv).await;
    bruno.entrar_por_convite(&convite, "Bruno").await.unwrap();
    ana.sync_group(g, 0).await.unwrap();

    let a = *ana.identity().public().as_bytes();
    let b = *bruno.identity().public().as_bytes();
    bruno.open_offline(g).unwrap().set_voice(Some("sala".into())).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    bruno.presenca(g).await.unwrap();
    ana.presenca(g).await.unwrap();
    let p = ana.presence(g).borrow().clone();
    assert_eq!(p.get(&b), Some(&Some("sala".to_string())));
    assert!(p.contains_key(&a));

    // O Bruno fecha a app: deixa de aparecer.
    bruno.shutdown().await.unwrap();
    ana.presenca(g).await.unwrap();
    assert!(!ana.presence(g).borrow().contains_key(&b));
}

#[tokio::test]
async fn depois_de_expulsar_o_convite_antigo_continua_a_servir() {
    let srv = servidor().await;
    let (dono, g, convite) = grupo_sem_dono(&srv).await;
    let bruno = node(&srv).await;
    bruno.entrar_por_convite(&convite, "Bruno").await.unwrap();
    dono.sync_group(g, 0).await.unwrap();
    dono.open_offline(g).unwrap().kick(*bruno.identity().public().as_bytes(), String::new()).await.unwrap();
    dono.sync_group(g, 0).await.unwrap();
    dono.sync_group(g, 0).await.unwrap();
    assert_eq!(dono.with_store(|s| s.group_key(g).unwrap()).unwrap().0, 1);
    escrever(&dono, g, "segunda").await;
    dono.sync_group(g, 0).await.unwrap();
    dono.shutdown().await.unwrap();

    // Quem entra com o convite antigo recebe a chave nova e lê tudo, sem o dono.
    let carla = node(&srv).await;
    carla.entrar_por_convite(&convite, "Carla").await.unwrap();
    assert!(membro(&carla, g, &carla));
    assert_eq!(textos(&carla, g), ["primeira", "segunda"]);
}
