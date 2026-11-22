//! Chamada de voz entre o anfitrião e dois membros, no mesmo PC.

use std::time::Duration;

use tasca_core::{Identity, Invite, Node, NodeOptions, Store};
use tokio::time::timeout;

async fn node() -> Node {
    Node::spawn(
        Identity::generate(),
        Store::open_in_memory().unwrap(),
        NodeOptions { local_only: true, ..Default::default() },
    )
    .await
    .unwrap()
}

async fn sala_com(c: &tasca_core::Chamada, n: usize) -> Vec<tasca_core::Participante> {
    let mut rx = c.sala();
    timeout(Duration::from_secs(10), rx.wait_for(|l| l.len() == n)).await.expect("timeout").unwrap().clone()
}

#[tokio::test]
async fn audio_passa_pelo_anfitriao() {
    let host = node().await;
    let group = host.create_group("Malta", "Tiago").unwrap();
    let code = host.create_invite(group).await.unwrap().encode();
    let ze = node().await;
    let _g1 = ze.join(&Invite::decode(&code).unwrap(), "Zé").await.unwrap();
    let ana = node().await;
    let code = host.create_invite(group).await.unwrap().encode();
    let _g2 = ana.join(&Invite::decode(&code).unwrap(), "Ana").await.unwrap();
    // Espera até o anfitrião ter os dois como membros.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let mut h = host.entrar_voz(group, "sala").await.unwrap();
    let mut z = ze.entrar_voz(group, "sala").await.unwrap();
    let mut a = ana.entrar_voz(group, "sala").await.unwrap();
    let lista = sala_com(&z, 3).await;
    sala_com(&a, 3).await;
    sala_com(&h, 3).await;
    assert!(lista.iter().any(|p| p.key == *ze.identity().public().as_bytes()));
    assert_eq!(host.salas_voz(group).len(), 1);
    // A sala aparece na presença, que chega aos membros.
    let zk = *ze.identity().public().as_bytes();
    assert_eq!(host.presence(group).borrow().get(&zk), Some(&Some("sala".to_string())));
    let mut pa = ana.presence(group);
    timeout(Duration::from_secs(5), pa.wait_for(|p| p.get(&zk) == Some(&Some("sala".to_string())))).await.unwrap().unwrap();

    let mut hp = h.pacotes().unwrap();
    let mut zp = z.pacotes().unwrap();
    let mut ap = a.pacotes().unwrap();

    // O Zé fala: chega ao anfitrião e à Ana, com o seq certo.
    z.enviar(b"ola");
    for rx in [&mut hp, &mut ap] {
        let p = timeout(Duration::from_secs(5), rx.recv()).await.expect("timeout").unwrap();
        assert_eq!(&p.data[..], b"ola");
        assert_eq!(p.seq, 0);
        assert_eq!(p.from, *ze.identity().public().as_bytes());
    }
    // O anfitrião fala: chega aos dois.
    h.enviar(b"anfitriao");
    for rx in [&mut zp, &mut ap] {
        let p = timeout(Duration::from_secs(5), rx.recv()).await.expect("timeout").unwrap();
        assert_eq!(&p.data[..], b"anfitriao");
    }

    // A Ana fica sem som: deixa de receber, e os outros veem o estado.
    a.set_estado(true, true).await.unwrap();
    let mut rx = z.sala();
    timeout(Duration::from_secs(5), rx.wait_for(|l| l.iter().any(|p| p.deaf))).await.unwrap().unwrap();
    z.enviar(b"ninguem");
    let p = timeout(Duration::from_secs(5), hp.recv()).await.unwrap().unwrap();
    assert_eq!(&p.data[..], b"ninguem");
    assert!(timeout(Duration::from_millis(300), ap.recv()).await.is_err());
    // E sem microfone, o que envia não passa.
    a.enviar(b"calada");
    assert!(timeout(Duration::from_millis(300), hp.recv()).await.is_err());

    // O Zé sai: a sala fica com dois.
    drop(z);
    sala_com(&h, 2).await;
    timeout(Duration::from_secs(5), pa.wait_for(|p| p.get(&zk) == Some(&None))).await.unwrap().unwrap();
    sala_com(&a, 2).await;
    drop((a, h));
    timeout(Duration::from_secs(5), async {
        while !host.salas_voz(group).is_empty() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn quem_nao_e_membro_nao_entra() {
    let host = node().await;
    let group = host.create_group("Malta", "Tiago").unwrap();
    let code = host.create_invite(group).await.unwrap().encode();
    // Junta-se para ficar com o grupo e o endereço do anfitrião, mas o anfitrião esquece-o.
    let intruso = node().await;
    let _g = intruso.join(&Invite::decode(&code).unwrap(), "X").await;
    let outro = host.create_group("Outro", "Tiago").unwrap();
    assert!(intruso.entrar_voz(outro, "sala").await.is_err());
}

#[tokio::test]
async fn video_passa_pelo_anfitriao() {
    let host = node().await;
    let group = host.create_group("Malta", "Tiago").unwrap();
    let code = host.create_invite(group).await.unwrap().encode();
    let ze = node().await;
    let _g1 = ze.join(&Invite::decode(&code).unwrap(), "Zé").await.unwrap();
    let ana = node().await;
    let code = host.create_invite(group).await.unwrap().encode();
    let _g2 = ana.join(&Invite::decode(&code).unwrap(), "Ana").await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let mut h = host.entrar_voz(group, "sala").await.unwrap();
    let z = ze.entrar_voz(group, "sala").await.unwrap();
    let mut a = ana.entrar_voz(group, "sala").await.unwrap();
    sala_com(&z, 3).await;
    sala_com(&a, 3).await;

    // O Zé liga a câmara: todos veem o estado.
    z.set_video(true, false).await.unwrap();
    let mut rx = a.sala();
    timeout(Duration::from_secs(5), rx.wait_for(|l| l.iter().any(|p| p.camera && !p.screen))).await.unwrap().unwrap();

    // Uma imagem grande (chave) chega inteira ao anfitrião e à Ana.
    let img: Vec<u8> = std::iter::once(1u8).chain((0..300_000).map(|i| (i % 251) as u8)).collect();
    z.enviar_video(bytes::Bytes::from(img.clone()));
    let mut hv = h.imagens().unwrap();
    let mut av = a.imagens().unwrap();
    for rx in [&mut hv, &mut av] {
        let i = timeout(Duration::from_secs(10), rx.recv()).await.expect("timeout").unwrap();
        assert_eq!(i.from, *ze.identity().public().as_bytes());
        assert_eq!(&i.data[..], &img[..]);
    }
    // O anfitrião também envia.
    h.enviar_video(bytes::Bytes::from_static(b"\x00delta"));
    let i = timeout(Duration::from_secs(5), av.recv()).await.expect("timeout").unwrap();
    assert_eq!(&i.data[..], b"\x00delta");
}

/// Três pessoas num grupo cujo dono está ligado mas fora da chamada.
async fn grupo_de_tres() -> (Node, Node, Node, tasca_core::GroupId) {
    let host = node().await;
    let group = host.create_group("Malta", "Tiago").unwrap();
    let ze = node().await;
    let code = host.create_invite(group).await.unwrap().encode();
    std::mem::forget(ze.join(&Invite::decode(&code).unwrap(), "Zé").await.unwrap());
    let ana = node().await;
    let code = host.create_invite(group).await.unwrap().encode();
    std::mem::forget(ana.join(&Invite::decode(&code).unwrap(), "Ana").await.unwrap());
    tokio::time::sleep(Duration::from_millis(300)).await;
    (host, ze, ana, group)
}

#[tokio::test]
async fn quem_entra_primeiro_aloja_e_pode_passar() {
    let (host, ze, ana, group) = grupo_de_tres().await;
    // O Zé entra primeiro: aloja ele, não o dono do grupo.
    let z = ze.entrar_voz(group, "sala").await.unwrap();
    assert!(z.e_anfitriao());
    assert!(host.salas_voz(group).is_empty());
    // O dono fica a saber pelo anúncio.
    timeout(Duration::from_secs(5), async {
        while !host.salas_conhecidas(group).iter().any(|a| a.room == "sala") {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("o dono não soube da chamada");
    // A Ana pergunta e liga-se ao Zé.
    let a = ana.entrar_voz(group, "sala").await.unwrap();
    assert!(!a.e_anfitriao());
    let l = sala_com(&a, 2).await;
    assert!(l.iter().any(|p| p.anfitriao && p.key == *ze.identity().public().as_bytes()));

    // O Zé passa a chamada à Ana.
    let ana_id = l.iter().find(|p| !p.anfitriao).unwrap().id;
    z.passar_host(ana_id).await.unwrap();
    let mut na = a.novo_host();
    let n = timeout(Duration::from_secs(5), na.wait_for(|n| n.is_some())).await.unwrap().unwrap().clone().unwrap();
    assert_eq!(n.conta, *ana.identity().public().as_bytes());
    let mut nz = z.novo_host();
    timeout(Duration::from_secs(5), nz.wait_for(|n| n.is_some())).await.unwrap().unwrap();
    // Cada um segue para o novo anfitrião (o que a app faz sozinha).
    let a2 = ana.entrar_voz_em(group, "sala", n.addr().unwrap(), Duration::from_secs(10)).await.unwrap();
    assert!(a2.e_anfitriao());
    drop(a);
    let z2 = ze.entrar_voz_em(group, "sala", n.addr().unwrap(), Duration::from_secs(10)).await.unwrap();
    drop(z);
    assert!(!z2.e_anfitriao());
    sala_com(&a2, 2).await;
    assert!(ze.salas_voz(group).is_empty());

    // A Ana (anfitriã) sai: a chamada passa sozinha para o Zé.
    let mut nz = z2.novo_host();
    drop(a2);
    let n = timeout(Duration::from_secs(5), nz.wait_for(|n| n.is_some())).await.unwrap().unwrap().clone().unwrap();
    assert_eq!(n.conta, *ze.identity().public().as_bytes());
}

#[tokio::test]
async fn dono_do_grupo_pode_mudar_quem_aloja() {
    let (host, ze, ana, group) = grupo_de_tres().await;
    let z = ze.entrar_voz(group, "sala").await.unwrap();
    assert!(z.e_anfitriao());
    timeout(Duration::from_secs(5), async {
        while host.salas_conhecidas(group).is_empty() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let a = ana.entrar_voz(group, "sala").await.unwrap();
    let h = host.entrar_voz(group, "sala").await.unwrap();
    assert!(!h.e_anfitriao());
    let l = sala_com(&h, 3).await;
    // A Ana não é dona: o pedido dela é ignorado.
    let h_id = l.iter().find(|p| p.key == *host.identity().public().as_bytes()).unwrap().id;
    a.passar_host(h_id).await.unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(h.novo_host().borrow().is_none());
    // O dono pede para ficar com ela.
    h.passar_host(h_id).await.unwrap();
    let mut nh = h.novo_host();
    let n = timeout(Duration::from_secs(5), nh.wait_for(|n| n.is_some())).await.unwrap().unwrap().clone().unwrap();
    assert_eq!(n.conta, *host.identity().public().as_bytes());
}

#[tokio::test]
async fn chamada_privada_entre_dois() {
    let ana = node().await;
    let ze = node().await;
    let estranho = node().await;
    let mut toques = ze.toques();
    // A Ana liga ao Zé: aloja ela e o PC do Zé recebe um toque. (Na rede local de teste,
    // o Zé só é encontrado pelo endereço; na rede a sério basta a chave.)
    let zk = ze.identity().public();
    let _ = ana.amigos();
    // dá à Ana o endereço do Zé, como a descoberta do iroh faria
    let c = ana.ligar_privado(zk, Some(ze.addr())).await.unwrap();
    assert!(c.e_anfitriao());
    let t = timeout(Duration::from_secs(5), toques.recv()).await.expect("sem toque").unwrap();
    assert_eq!(t.de, *ana.identity().public().as_bytes());
    // O Zé atende, ligando-se a quem tocou.
    let z = ze.ligar_privado(ana.identity().public(), Some(t.endpoint)).await.unwrap();
    assert!(!z.e_anfitriao());
    sala_com(&c, 2).await;
    // Um estranho não entra, mesmo sabendo onde está a chamada.
    let g = tasca_core::voz::grupo_privado(&ana.identity().public(), &zk);
    assert!(estranho.entrar_voz_em(g, "privada", ana.addr(), Duration::from_millis(500)).await.is_err());
}
