//! Cargos, permissões e moderação de um grupo.
//!
//! O dono do grupo (o anfitrião) cria cargos com permissões e dá-os aos membros. Quem tem
//! a permissão certa modera: expulsar, banir, pôr de castigo, silenciar numa chamada, etc.
//! Quem aloja uma chamada não ganha poderes nenhuns por isso.
//!
//! Tudo isto são eventos assinados por quem os faz. O anfitrião só os aceita se o autor
//! tiver a permissão nesse momento (ver [`crate::store::Store::append_as_host`]), e os
//! membros recebem-nos pela ordem que o anfitrião lhes deu, por isso todos chegam ao mesmo
//! estado.
//!
//! As permissões calculam-se como no Discord: o cargo base `@todos` mais todos os cargos da
//! pessoa; depois, num canal, as exceções da categoria e do canal (primeiro `@todos`, depois
//! os cargos, por fim a própria pessoa). Administrador tem tudo. O dono tem sempre tudo.

use serde::{Deserialize, Serialize};

/// Ver os canais (sem isto o canal fica escondido).
pub const VER_CANAIS: u64 = 1 << 0;
/// Criar, editar e apagar canais e categorias, e as suas exceções de permissões.
pub const GERIR_CANAIS: u64 = 1 << 1;
/// Criar e editar cargos abaixo do seu, e dá-los a pessoas.
pub const GERIR_CARGOS: u64 = 1 << 2;
/// Emojis, autocolantes e sons do grupo.
pub const GERIR_EXPRESSOES: u64 = 1 << 3;
/// Ver o registo de auditoria (e as denúncias).
pub const VER_REGISTO: u64 = 1 << 4;
/// Nome, ícone, descrição, AutoMod e outras definições do grupo.
pub const GERIR_GRUPO: u64 = 1 << 5;
/// Webhooks e plugins do grupo.
pub const GERIR_INTEGRACOES: u64 = 1 << 6;
/// Criar convites.
pub const CRIAR_CONVITES: u64 = 1 << 7;
/// Mudar o próprio apelido neste grupo.
pub const MUDAR_APELIDO: u64 = 1 << 8;
/// Mudar o apelido dos outros.
pub const GERIR_APELIDOS: u64 = 1 << 9;
/// Tirar alguém do grupo (pode voltar com convite).
pub const EXPULSAR: u64 = 1 << 10;
/// Tirar alguém e impedir que volte.
pub const BANIR: u64 = 1 << 11;
/// Pôr de castigo: durante um tempo só lê.
pub const CASTIGAR: u64 = 1 << 12;
/// Escrever nos canais de texto.
pub const ENVIAR_MENSAGENS: u64 = 1 << 13;
/// Enviar ficheiros e imagens.
pub const ANEXAR: u64 = 1 << 14;
/// Mostrar o conteúdo dos links.
pub const PRE_VISUALIZAR: u64 = 1 << 15;
/// Reagir com emojis.
pub const REAGIR: u64 = 1 << 16;
/// Usar emojis de outros grupos.
pub const EMOJIS_DE_FORA: u64 = 1 << 17;
/// Mencionar @todos e cargos que não são mencionáveis.
pub const MENCIONAR_TODOS: u64 = 1 << 18;
/// Apagar e fixar mensagens dos outros.
pub const GERIR_MENSAGENS: u64 = 1 << 19;
/// Ver mensagens de antes de entrar.
pub const LER_HISTORICO: u64 = 1 << 20;
/// Criar tópicos e sondagens.
pub const CRIAR_TOPICOS: u64 = 1 << 21;
/// Entrar nas chamadas.
pub const LIGAR: u64 = 1 << 22;
/// Falar nas chamadas (sem isto entra silenciado).
pub const FALAR: u64 = 1 << 23;
/// Câmara e partilha de ecrã.
pub const VIDEO: u64 = 1 << 24;
/// Silenciar alguém para todos.
pub const SILENCIAR_MEMBROS: u64 = 1 << 25;
/// Cortar o som a alguém.
pub const ENSURDECER_MEMBROS: u64 = 1 << 26;
/// Mudar ou tirar alguém de uma chamada.
pub const MOVER_MEMBROS: u64 = 1 << 27;
/// A voz desta pessoa baixa o volume dos outros.
pub const PRIORIDADE: u64 = 1 << 28;
/// Poder ser o anfitrião de uma chamada (o dispositivo dela faz de servidor).
pub const ALOJAR_CHAMADAS: u64 = 1 << 29;
/// Comandos com / de plugins.
pub const USAR_COMANDOS: u64 = 1 << 30;
/// Todas as permissões.
pub const ADMINISTRADOR: u64 = 1 << 31;

/// Todas as permissões que existem.
pub const TODAS: u64 = (1 << 32) - 1;

/// O que um membro sem cargos pode fazer num grupo novo.
pub const BASE: u64 = VER_CANAIS
    | CRIAR_CONVITES
    | MUDAR_APELIDO
    | ENVIAR_MENSAGENS
    | ANEXAR
    | PRE_VISUALIZAR
    | REAGIR
    | EMOJIS_DE_FORA
    | LER_HISTORICO
    | CRIAR_TOPICOS
    | LIGAR
    | FALAR
    | VIDEO
    | ALOJAR_CHAMADAS
    | USAR_COMANDOS;

/// O que uma pessoa de castigo perde.
pub const TIRADAS_NO_CASTIGO: u64 =
    ENVIAR_MENSAGENS | ANEXAR | REAGIR | CRIAR_TOPICOS | FALAR | VIDEO | CRIAR_CONVITES | MUDAR_APELIDO;

/// Id do cargo base, que toda a gente tem.
pub const TODOS: &str = "todos";

/// Nome e explicação de cada permissão, por ordem, agrupadas como na interface.
pub const LISTA: &[(&str, u64, &str, &str)] = &[
    ("Gerais", VER_CANAIS, "Ver canais", "Ver os canais que não são privados."),
    ("Gerais", GERIR_CANAIS, "Gerir canais", "Criar, editar e apagar canais e categorias."),
    ("Gerais", GERIR_CARGOS, "Gerir cargos", "Criar e editar cargos abaixo do seu, e dá-los a pessoas."),
    ("Gerais", GERIR_EXPRESSOES, "Gerir expressões", "Emojis, autocolantes e sons do grupo."),
    ("Gerais", VER_REGISTO, "Ver o registo de auditoria", "Ver quem mudou o quê, e as denúncias."),
    ("Gerais", GERIR_GRUPO, "Gerir o grupo", "Nome, ícone, descrição, AutoMod e outras definições."),
    ("Gerais", GERIR_INTEGRACOES, "Gerir webhooks e plugins", "Ligar bots e integrações ao grupo."),
    ("Membros", CRIAR_CONVITES, "Criar convites", "Convidar pessoas novas."),
    ("Membros", MUDAR_APELIDO, "Mudar o próprio apelido", "Um apelido só neste grupo."),
    ("Membros", GERIR_APELIDOS, "Gerir apelidos", "Mudar o apelido dos outros."),
    ("Membros", EXPULSAR, "Expulsar", "Tirar alguém do grupo. Pode voltar com um convite novo."),
    ("Membros", BANIR, "Banir", "Tirar alguém do grupo e impedir que volte."),
    ("Membros", CASTIGAR, "Pôr de castigo", "Durante um tempo, a pessoa só lê."),
    ("Texto", ENVIAR_MENSAGENS, "Enviar mensagens", "Escrever nos canais de texto."),
    ("Texto", ANEXAR, "Anexar ficheiros", "Enviar ficheiros e imagens."),
    ("Texto", PRE_VISUALIZAR, "Pré-visualizar links", "Mostrar o conteúdo dos links."),
    ("Texto", REAGIR, "Adicionar reações", "Reagir com emojis."),
    ("Texto", EMOJIS_DE_FORA, "Usar emojis de outros grupos", "Emojis de outros grupos nas mensagens."),
    ("Texto", MENCIONAR_TODOS, "Mencionar @todos e cargos", "Avisar o grupo inteiro ou um cargo de uma vez."),
    ("Texto", GERIR_MENSAGENS, "Gerir mensagens", "Apagar e fixar mensagens dos outros."),
    ("Texto", LER_HISTORICO, "Ler o histórico", "Ver mensagens de antes de entrar."),
    ("Texto", CRIAR_TOPICOS, "Criar tópicos e sondagens", "Abrir tópicos a partir de uma mensagem."),
    ("Voz", LIGAR, "Ligar-se", "Entrar nas chamadas."),
    ("Voz", FALAR, "Falar", "Sem isto, entra silenciado."),
    ("Voz", VIDEO, "Vídeo e ecrã", "Câmara e partilha de ecrã."),
    ("Voz", SILENCIAR_MEMBROS, "Silenciar membros", "Silenciar alguém para todos, e desligar a câmara dele."),
    ("Voz", ENSURDECER_MEMBROS, "Ensurdecer membros", "Cortar o som a alguém."),
    ("Voz", MOVER_MEMBROS, "Mover membros", "Mudar ou tirar alguém de uma chamada."),
    ("Voz", PRIORIDADE, "Prioridade a falar", "A voz desta pessoa baixa o volume dos outros."),
    ("Voz", ALOJAR_CHAMADAS, "Alojar chamadas", "Poder ser o anfitrião da chamada (o dispositivo dela faz de servidor)."),
    ("Plugins", USAR_COMANDOS, "Usar comandos de plugins", "Comandos com /."),
    ("Avançadas", ADMINISTRADOR, "Administrador", "Todas as permissões. Cuidado."),
];

/// Um cargo do grupo. A ordem na lista é a hierarquia: o primeiro manda mais.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cargo {
    pub id: String,
    pub name: String,
    /// `#RRGGBB`, ou vazio para não mudar a cor do nome.
    pub color: String,
    /// Emoji, ou imagem pequena (`data:image/...`).
    pub icon: Option<String>,
    /// Mostrar à parte na lista de membros.
    pub hoist: bool,
    /// Qualquer pessoa pode mencionar com @.
    pub mentionable: bool,
    pub perms: u64,
}

impl Cargo {
    /// O cargo base de um grupo novo.
    pub fn todos() -> Self {
        Cargo {
            id: TODOS.into(),
            name: "@todos".into(),
            color: String::new(),
            icon: None,
            hoist: false,
            mentionable: false,
            perms: BASE,
        }
    }
}

/// A quem se aplica uma exceção num canal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alvo {
    Cargo(String),
    Membro([u8; 32]),
}

/// Exceção de permissões num canal ou categoria: tira (`deny`) e dá (`allow`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Excecao {
    pub alvo: Alvo,
    pub allow: u64,
    pub deny: u64,
}

/// Chave das exceções de uma categoria (os canais guardam-se pelo id).
pub fn chave_categoria(nome: &str) -> String {
    format!("cat:{nome}")
}

/// O que fazer a alguém numa chamada.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcaoVoz {
    /// Silenciar (ou tirar o silêncio) para todos.
    Silenciar(bool),
    /// Cortar (ou devolver) o som.
    Ensurdecer(bool),
    /// Tirar da chamada.
    Desligar,
    /// Mudar para outra sala de voz (id do canal).
    Mover(String),
    /// Desligar a câmara e a partilha de ecrã.
    PararVideo,
}

impl AcaoVoz {
    pub fn permissao(&self) -> u64 {
        match self {
            AcaoVoz::Silenciar(_) | AcaoVoz::PararVideo => SILENCIAR_MEMBROS,
            AcaoVoz::Ensurdecer(_) => ENSURDECER_MEMBROS,
            AcaoVoz::Desligar | AcaoVoz::Mover(_) => MOVER_MEMBROS,
        }
    }
}

/// Definições do grupo (a última publicada vale).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Definicoes {
    pub name: String,
    pub description: String,
    /// Ícone do grupo (`data:image/...`).
    pub icon: Option<String>,
    /// Banner (`#RRGGBB` ou `data:image/...`).
    pub banner: Option<String>,
    /// Cor de destaque (`#RRGGBB`), ou vazio.
    pub color: String,
    /// Etiqueta curta que os membros podem mostrar ao lado do nome.
    pub tag: String,
    /// Canal onde aparecem as mensagens do sistema.
    pub system_channel: Option<String>,
    /// Mensagem de boas-vindas quando alguém entra.
    pub welcome: bool,
    /// Avisar quando alguém sai, é expulso ou banido.
    pub leave_notices: bool,
    /// Canal de inatividade e o tempo (segundos) até lá ir parar.
    pub afk_channel: Option<String>,
    pub afk_secs: u32,
    /// Avisos para membros novos: 0 = todas as mensagens, 1 = só menções.
    pub default_notify: u8,
    /// Regras do grupo, mostradas a quem entra.
    pub rules: String,
    /// Membro há pelo menos isto (segundos) para poder escrever.
    pub min_member_secs: u32,
    /// Só deixa entrar contas com @nome de utilizador.
    pub need_username: bool,
    /// Convites em pausa: ninguém entra.
    pub invites_paused: bool,
    /// Proteção contra ataques: no máximo isto de entradas por minuto (0 = sem limite).
    pub raid_limit: u32,
    pub automod: Vec<Regra>,
    /// Para definições futuras, sem mudar o formato.
    pub extra: Vec<(String, String)>,
}

/// Uma regra do AutoMod.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Regra {
    pub id: String,
    pub name: String,
    pub on: bool,
    pub tipo: TipoRegra,
    /// Bloquear a mensagem (com aviso a quem a escreveu).
    pub block: bool,
    /// Mandar um alerta para este canal.
    pub alert_channel: Option<String>,
    /// Pôr de castigo durante isto (segundos; 0 = não).
    pub timeout_secs: u32,
    pub exempt_roles: Vec<String>,
    pub exempt_channels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoRegra {
    /// Palavras ou expressões proibidas (`*` no início ou fim apanha partes de palavras).
    Palavras(Vec<String>),
    /// Links; os domínios da lista são permitidos.
    Links { permitidos: Vec<String> },
    /// Mais do que isto de menções numa mensagem.
    Mencoes(u32),
    /// A mesma mensagem repetida isto de vezes seguidas.
    Repetidas(u32),
    /// Convites de outros grupos da Tasca.
    Convites,
    /// Mais do que isto de emojis.
    Emojis(u32),
    /// Mais do que esta percentagem de maiúsculas (em mensagens com 10 letras ou mais).
    Maiusculas(u32),
}

/// O que o AutoMod encontrou numa mensagem.
#[derive(Debug, Clone)]
pub struct Bloqueio {
    pub regra: String,
    pub block: bool,
    pub alert_channel: Option<String>,
    pub timeout_secs: u32,
}

impl std::fmt::Display for Bloqueio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mensagem bloqueada pelo AutoMod ({})", self.regra)
    }
}

impl std::error::Error for Bloqueio {}

/// Permissões finais de uma pessoa. `cargos` são os cargos do grupo por ordem, `meus` os
/// ids dos cargos da pessoa, `excecoes` as da categoria e do canal (por esta ordem).
pub fn calcular(
    dono: bool,
    cargos: &[Cargo],
    meus: &[String],
    quem: &[u8; 32],
    excecoes: &[&[Excecao]],
    castigo: bool,
) -> u64 {
    if dono {
        return TODAS;
    }
    let base = cargos.iter().find(|c| c.id == TODOS).map(|c| c.perms).unwrap_or(BASE);
    let mut p = base;
    for c in cargos {
        if meus.contains(&c.id) {
            p |= c.perms;
        }
    }
    if p & ADMINISTRADOR != 0 {
        return TODAS;
    }
    for lista in excecoes {
        // 1. @todos
        for e in lista.iter().filter(|e| e.alvo == Alvo::Cargo(TODOS.into())) {
            p = (p & !e.deny) | e.allow;
        }
        // 2. cargos da pessoa, todos juntos
        let (mut allow, mut deny) = (0, 0);
        for e in lista.iter() {
            if let Alvo::Cargo(id) = &e.alvo {
                if id != TODOS && meus.contains(id) {
                    allow |= e.allow;
                    deny |= e.deny;
                }
            }
        }
        p = (p & !deny) | allow;
        // 3. a própria pessoa
        for e in lista.iter().filter(|e| e.alvo == Alvo::Membro(*quem)) {
            p = (p & !e.deny) | e.allow;
        }
    }
    if p & VER_CANAIS == 0 && !excecoes.is_empty() {
        // sem ver o canal, não faz nada nele
        p &= !(ENVIAR_MENSAGENS | ANEXAR | REAGIR | LIGAR | FALAR | VIDEO | CRIAR_TOPICOS);
    }
    if castigo {
        p &= !TIRADAS_NO_CASTIGO;
    }
    p
}

/// Posição da pessoa na hierarquia: 0 é o cargo mais alto; o dono é -1; sem cargos, a
/// posição do `@todos` (o fim da lista).
pub fn posicao(dono: bool, cargos: &[Cargo], meus: &[String]) -> i64 {
    if dono {
        return -1;
    }
    cargos
        .iter()
        .position(|c| c.id != TODOS && meus.contains(&c.id))
        .map(|i| i as i64)
        .unwrap_or(cargos.len() as i64)
}

/// Vê se uma mensagem passa nas regras do AutoMod. `cargos_autor` são os cargos de quem
/// escreveu; `anteriores` as últimas mensagens dele no canal (as mais recentes no fim).
pub fn automod(
    regras: &[Regra],
    canal: &str,
    texto: &str,
    cargos_autor: &[String],
    anteriores: &[String],
) -> Option<Bloqueio> {
    let baixo = texto.to_lowercase();
    for r in regras.iter().filter(|r| r.on) {
        if r.exempt_channels.iter().any(|c| c == canal) || r.exempt_roles.iter().any(|c| cargos_autor.contains(c)) {
            continue;
        }
        let apanhou = match &r.tipo {
            TipoRegra::Palavras(lista) => lista.iter().any(|p| palavra(&baixo, p)),
            TipoRegra::Links { permitidos } => links(&baixo).iter().any(|d| {
                !permitidos.iter().any(|ok| {
                    let ok = ok.trim().trim_start_matches("*.").to_lowercase();
                    !ok.is_empty() && (*d == ok || d.ends_with(&format!(".{ok}")))
                })
            }),
            TipoRegra::Mencoes(n) => {
                let m = texto.split_whitespace().filter(|w| w.starts_with('@') && w.len() > 1).count();
                m as u32 > *n
            }
            TipoRegra::Repetidas(n) => {
                let n = (*n).max(2) as usize;
                let t = texto.trim();
                !t.is_empty()
                    && anteriores.len() + 1 >= n
                    && anteriores.iter().rev().take(n - 1).all(|a| a.trim() == t)
            }
            TipoRegra::Convites => baixo.contains("tasca1"),
            TipoRegra::Emojis(n) => texto.chars().filter(|c| e_emoji(*c)).count() as u32 > *n,
            TipoRegra::Maiusculas(pct) => {
                let letras: Vec<char> = texto.chars().filter(|c| c.is_alphabetic()).collect();
                letras.len() >= 10
                    && (letras.iter().filter(|c| c.is_uppercase()).count() * 100 / letras.len()) as u32 > *pct
            }
        };
        if apanhou {
            return Some(Bloqueio {
                regra: r.name.clone(),
                block: r.block,
                alert_channel: r.alert_channel.clone(),
                timeout_secs: r.timeout_secs,
            });
        }
    }
    None
}

fn e_emoji(c: char) -> bool {
    matches!(c as u32, 0x1F300..=0x1FAFF | 0x2600..=0x27BF | 0x1F000..=0x1F2FF)
}

/// Palavra proibida: `*` no início ou no fim deixa apanhar partes de palavras; sem `*`,
/// só a palavra inteira.
fn palavra(texto: &str, p: &str) -> bool {
    let p = p.trim().to_lowercase();
    let (ini, fim) = (p.starts_with('*'), p.ends_with('*') && p.len() > 1);
    let nucleo = p.trim_matches('*');
    if nucleo.is_empty() {
        return false;
    }
    let mut desde = 0;
    while let Some(i) = texto[desde..].find(nucleo) {
        let a = desde + i;
        let b = a + nucleo.len();
        let antes_ok = ini || a == 0 || !texto[..a].chars().next_back().is_some_and(|c| c.is_alphanumeric());
        let depois_ok = fim || b == texto.len() || !texto[b..].chars().next().is_some_and(|c| c.is_alphanumeric());
        if antes_ok && depois_ok {
            return true;
        }
        desde = a + nucleo.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }
    false
}

/// Domínios dos links de um texto (em minúsculas).
fn links(texto: &str) -> Vec<String> {
    let mut out = Vec::new();
    for w in texto.split_whitespace() {
        let w = w.trim_matches(|c: char| "<>()[]\"'".contains(c));
        let resto = if let Some(r) = w.strip_prefix("https://").or_else(|| w.strip_prefix("http://")) {
            r
        } else if let Some(r) = w.strip_prefix("www.") {
            r
        } else {
            continue;
        };
        let dom = resto.split(['/', '?', '#', ':']).next().unwrap_or("").trim_start_matches("www.");
        if dom.contains('.') {
            out.push(dom.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cargo(id: &str, perms: u64) -> Cargo {
        Cargo { id: id.into(), name: id.into(), color: String::new(), icon: None, hoist: false, mentionable: false, perms }
    }

    #[test]
    fn calculo_como_no_discord() {
        let eu = [1; 32];
        let cargos = vec![cargo("mod", EXPULSAR | GERIR_MENSAGENS), cargo(TODOS, BASE)];
        let meus = vec!["mod".to_string()];
        let p = calcular(false, &cargos, &meus, &eu, &[], false);
        assert!(p & EXPULSAR != 0 && p & ENVIAR_MENSAGENS != 0 && p & BANIR == 0);
        // canal só de leitura para @todos, mas os mods escrevem
        let canal = vec![
            Excecao { alvo: Alvo::Cargo(TODOS.into()), allow: 0, deny: ENVIAR_MENSAGENS },
            Excecao { alvo: Alvo::Cargo("mod".into()), allow: ENVIAR_MENSAGENS, deny: 0 },
        ];
        assert!(calcular(false, &cargos, &meus, &eu, &[&canal], false) & ENVIAR_MENSAGENS != 0);
        assert!(calcular(false, &cargos, &[], &eu, &[&canal], false) & ENVIAR_MENSAGENS == 0);
        // a pessoa ganha a quem a tira
        let pessoal = vec![Excecao { alvo: Alvo::Membro(eu), allow: 0, deny: ENVIAR_MENSAGENS }];
        assert!(calcular(false, &cargos, &meus, &eu, &[&canal, &pessoal], false) & ENVIAR_MENSAGENS == 0);
        // castigo tira o escrever; administrador tem tudo; dono tem tudo
        assert!(calcular(false, &cargos, &meus, &eu, &[], true) & ENVIAR_MENSAGENS == 0);
        let adm = vec![cargo("adm", ADMINISTRADOR), cargo(TODOS, 0)];
        assert_eq!(calcular(false, &adm, &["adm".into()], &eu, &[&canal], false), TODAS);
        assert_eq!(calcular(true, &[], &[], &eu, &[], false), TODAS);
        // posição
        assert_eq!(posicao(false, &cargos, &meus), 0);
        assert_eq!(posicao(false, &cargos, &[]), 2);
        assert_eq!(posicao(true, &cargos, &[]), -1);
    }

    #[test]
    fn automod_basico() {
        let r = |tipo| Regra {
            id: "r".into(),
            name: "teste".into(),
            on: true,
            tipo,
            block: true,
            alert_channel: None,
            timeout_secs: 0,
            exempt_roles: vec!["mod".into()],
            exempt_channels: vec![],
        };
        let pal = vec![r(TipoRegra::Palavras(vec!["burro".into(), "*merd*".into()]))];
        assert!(automod(&pal, "geral", "és um BURRO!", &[], &[]).is_some());
        assert!(automod(&pal, "geral", "burros não", &[], &[]).is_none());
        assert!(automod(&pal, "geral", "que merda", &[], &[]).is_some());
        assert!(automod(&pal, "geral", "és um burro", &["mod".into()], &[]).is_none());
        let lk = vec![r(TipoRegra::Links { permitidos: vec!["youtube.com".into()] })];
        assert!(automod(&lk, "g", "vê https://www.youtube.com/watch?v=1", &[], &[]).is_none());
        assert!(automod(&lk, "g", "vê https://m.youtube.com/x", &[], &[]).is_none());
        assert!(automod(&lk, "g", "vê http://golpe.xyz/premio", &[], &[]).is_some());
        let rep = vec![r(TipoRegra::Repetidas(3))];
        assert!(automod(&rep, "g", "olá", &[], &["olá".into(), "olá".into()]).is_some());
        assert!(automod(&rep, "g", "olá", &[], &["xau".into(), "olá".into()]).is_none());
        let men = vec![r(TipoRegra::Mencoes(2))];
        assert!(automod(&men, "g", "@a @b @c", &[], &[]).is_some());
        let mai = vec![r(TipoRegra::Maiusculas(70))];
        assert!(automod(&mai, "g", "OLHA AQUI MALTA", &[], &[]).is_some());
        assert!(automod(&mai, "g", "Olha aqui malta", &[], &[]).is_none());
    }
}
