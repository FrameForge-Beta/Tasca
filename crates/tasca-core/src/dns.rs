//! Resolver de nomes para o servidor de contas.
//!
//! Há redes de casa em que o DNS do router (ou de um Pi-hole / AdGuard no NAS) responde ao
//! nome do servidor com um IP da rede local, e nesse IP não há nada a ouvir na porta do
//! servidor. O browser às vezes funciona na mesma porque usa DNS seguro (DoH) e recebe o IP
//! público. Este resolver faz o mesmo quando é preciso: pergunta primeiro ao DNS do sistema;
//! se só vierem IPs locais e nenhum responder (ou o DNS falhar), pergunta por DoH à
//! Cloudflare (1.1.1.1) ou à Google (8.8.8.8) e usa o IP público.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

/// Quanto tempo fico com o IP público depois de ver que o local não responde.
const VALIDADE: Duration = Duration::from_secs(10 * 60);

pub(crate) struct Resolvedor {
    porta: u16,
    doh: reqwest::Client,
    publicos: Arc<Mutex<HashMap<String, (Instant, Vec<IpAddr>)>>>,
}

impl Resolvedor {
    pub(crate) fn new(porta: u16) -> Self {
        let doh = crate::http_builder()
            .timeout(Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        Self { porta, doh, publicos: Default::default() }
    }
}

/// IP que só existe dentro de uma rede (casa, empresa) ou na própria máquina.
pub(crate) fn e_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.is_private() || v.is_loopback() || v.is_link_local() || v.octets()[0] == 100 && (v.octets()[1] & 0xc0) == 64,
        IpAddr::V6(v) => v.is_loopback() || (v.segments()[0] & 0xfe00) == 0xfc00 || (v.segments()[0] & 0xffc0) == 0xfe80,
    }
}

async fn responde(addrs: &[SocketAddr], porta: u16) -> bool {
    for a in addrs.iter().take(2) {
        let a = SocketAddr::new(a.ip(), porta);
        if let Ok(Ok(_)) = tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(a)).await {
            return true;
        }
    }
    false
}

/// Pergunta o IP (A) de `host` por DNS sobre HTTPS.
pub(crate) async fn doh(cliente: &reqwest::Client, host: &str) -> Vec<IpAddr> {
    for url in ["https://1.1.1.1/dns-query", "https://8.8.8.8/resolve"] {
        let r = cliente
            .get(url)
            .query(&[("name", host), ("type", "A")])
            .header("accept", "application/dns-json")
            .send()
            .await;
        let Ok(r) = r else { continue };
        let Ok(v) = r.json::<serde_json::Value>().await else { continue };
        let ips: Vec<IpAddr> = v["Answer"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|x| x["type"] == 1)
                    .filter_map(|x| x["data"].as_str()?.parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        if !ips.is_empty() {
            return ips;
        }
    }
    Vec::new()
}

impl Resolve for Resolvedor {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let (porta, doh_cliente, publicos) = (self.porta, self.doh.clone(), self.publicos.clone());
        Box::pin(async move {
            if let Some((quando, ips)) = publicos.lock().unwrap().get(&host).cloned() {
                if quando.elapsed() < VALIDADE {
                    return Ok(Box::new(ips.into_iter().map(|ip| SocketAddr::new(ip, 0))) as Addrs);
                }
            }
            let sistema: std::io::Result<Vec<SocketAddr>> =
                tokio::net::lookup_host((host.as_str(), porta)).await.map(|i| i.collect());
            let precisa = match &sistema {
                Ok(v) if v.is_empty() => true,
                Ok(v) => v.iter().all(|a| e_local(a.ip())) && !responde(v, porta).await,
                Err(_) => true,
            };
            if precisa {
                let ips: Vec<IpAddr> = doh(&doh_cliente, &host).await.into_iter().filter(|ip| !e_local(*ip)).collect();
                if !ips.is_empty() {
                    tracing::warn!("{host}: o DNS da rede deu um IP local que não responde; uso o IP público {ips:?}");
                    publicos.lock().unwrap().insert(host.clone(), (Instant::now(), ips.clone()));
                    return Ok(Box::new(ips.into_iter().map(|ip| SocketAddr::new(ip, 0))) as Addrs);
                }
            }
            match sistema {
                Ok(v) if !v.is_empty() => Ok(Box::new(v.into_iter()) as Addrs),
                Ok(_) => Err(format!("não encontrei o endereço de {host}").into()),
                Err(e) => Err(e.into()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ips_locais() {
        for ip in ["192.168.1.191", "10.0.0.2", "172.16.5.1", "127.0.0.1", "169.254.1.1", "100.64.0.1", "fd00::1", "fe80::1", "::1"] {
            assert!(e_local(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["85.247.133.175", "1.1.1.1", "2a00:1450::1"] {
            assert!(!e_local(ip.parse().unwrap()), "{ip}");
        }
    }
}
