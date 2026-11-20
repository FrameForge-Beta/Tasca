//! `tasca`: programa de teste em linha de comandos para o núcleo P2P.
//!
//!   tasca criar "Malta" --nome Tiago     cria um grupo, mostra o convite e abre o chat
//!   tasca hospedar                       volta a hospedar os teus grupos
//!   tasca entrar <convite> --nome Zé     entra num grupo de outra pessoa

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use tasca_core::{EventKind, GroupHandle, GroupId, Identity, Invite, Node, NodeOptions, Store};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Parser)]
#[command(
    name = "tasca",
    version,
    about = "Tasca: chat P2P, os teus grupos no teu PC"
)]
struct Cli {
    /// Pasta de dados (por omissão, a pasta de dados do sistema, ex. %APPDATA%\Tasca).
    #[arg(long, global = true)]
    dados: Option<PathBuf>,
    /// Só ligações diretas na rede local, sem relays.
    #[arg(long, global = true)]
    local: bool,
    /// Relay próprio (ex. https://relay.o-teu-servidor.pt). Pode repetir-se.
    #[arg(long, global = true)]
    relay: Vec<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Cria um grupo novo hospedado neste PC.
    Criar {
        nome_do_grupo: String,
        /// O teu nome no grupo.
        #[arg(long)]
        nome: String,
    },
    /// Volta a hospedar um grupo teu e mostra um convite novo.
    Hospedar {
        /// Nome do grupo (se tiveres mais do que um).
        grupo: Option<String>,
    },
    /// Entra num grupo de outra pessoa com um convite.
    Entrar {
        convite: String,
        /// O teu nome no grupo.
        #[arg(long)]
        nome: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tasca_core=warn".into()),
        )
        .init();
    let cli = Cli::parse();
    let dir = match cli.dados {
        Some(d) => d,
        None => dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Tasca"),
    };
    let identity = Identity::load_or_create(&dir.join("identidade.chave"))?;
    let store = Store::open(&dir.join("tasca.db"))?;
    let node = Node::spawn(
        identity,
        store,
        NodeOptions {
            local_only: cli.local,
            relay_urls: cli.relay,
            servidor: tasca_core::servidor::AccountServer::from_env(),
        },
    )
    .await?;
    println!("A tua identidade: {}", node.identity().public());

    let handle = match cli.cmd {
        Cmd::Criar {
            nome_do_grupo,
            nome,
        } => {
            let group = node.create_group(&nome_do_grupo, &nome)?;
            announce_invite(&node, group).await?;
            node.open_hosted(group)?
        }
        Cmd::Hospedar { grupo } => {
            let mine: Vec<_> = node
                .with_store(|s| s.groups())?
                .into_iter()
                .filter(|g| g.hosted)
                .collect();
            let chosen = match (grupo, mine.as_slice()) {
                (_, []) => bail!("ainda não tens grupos. Usa `tasca criar`."),
                (None, [one]) => one.clone(),
                (None, _) => bail!(
                    "tens vários grupos, indica qual: {}",
                    mine.iter()
                        .map(|g| g.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                (Some(n), _) => match mine.iter().find(|g| g.name.eq_ignore_ascii_case(&n)) {
                    Some(g) => g.clone(),
                    None => bail!("não encontrei o grupo \"{n}\""),
                },
            };
            announce_invite(&node, chosen.id).await?;
            node.open_hosted(chosen.id)?
        }
        Cmd::Entrar { convite, nome } => {
            let invite = Invite::decode(&convite)?;
            println!("A ligar ao anfitrião...");
            let rx = node.subscribe(invite.group);
            let handle = node.join(&invite, &nome).await?;
            println!("Ligado! Escreve e carrega Enter para enviar. Ctrl+C para sair.\n");
            return chat(&node, handle, Some(rx)).await;
        }
    };
    println!("Escreve e carrega Enter para enviar. Ctrl+C para sair.\n");
    print_history(&node, handle.group);
    chat(&node, handle, None).await
}

async fn announce_invite(node: &Node, group: GroupId) -> Result<()> {
    println!("A preparar o convite...");
    let invite = node.create_invite(group).await?;
    println!(
        "\nConvite (envia isto a quem quiseres convidar):\n\n{}\n",
        invite.encode()
    );
    Ok(())
}

fn print_history(node: &Node, group: GroupId) {
    let events = node
        .with_store(|s| s.events_since(group, 0))
        .unwrap_or_default();
    for (_, ev) in events {
        print_event(node, group, &ev);
    }
}

fn print_event(node: &Node, group: GroupId, ev: &tasca_core::SignedEvent) {
    let who = ev
        .author()
        .ok()
        .and_then(|a| {
            node.with_store(|s| s.display_name(group, &a))
                .ok()
                .flatten()
        })
        .unwrap_or_else(|| "?".into());
    match &ev.body.kind {
        EventKind::GroupCreated { name, .. } => println!("* {who} criou o grupo \"{name}\""),
        EventKind::MemberJoined { .. } => println!("* {who} entrou no grupo"),
        EventKind::Message { channel, text } => println!("[#{channel}] {who}: {text}"),
    }
}

async fn chat(
    node: &Node,
    handle: GroupHandle,
    rx: Option<tokio::sync::broadcast::Receiver<tasca_core::node::Update>>,
) -> Result<()> {
    let group = handle.group;
    let mut rx = rx.unwrap_or_else(|| node.subscribe(group));
    let me = *node.identity().public().as_bytes();
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        tokio::select! {
            update = rx.recv() => match update {
                Ok((_, ev)) => {
                    // As nossas mensagens já aparecem no ecrã quando as escrevemos.
                    let is_own_message = ev.body.author == me && matches!(ev.body.kind, EventKind::Message { .. });
                    if !is_own_message {
                        print_event(node, group, &ev);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(_) => break,
            },
            line = lines.next_line() => match line? {
                Some(text) if !text.trim().is_empty() => {
                    if let Err(e) = handle.send_message("geral", text.trim()).await {
                        eprintln!("não foi possível enviar: {e:#}");
                    }
                }
                Some(_) => {}
                None => break,
            },
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    node.clone().shutdown().await
}
