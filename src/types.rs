use anyhow::{anyhow, Context};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use wireguard_keys::{Privkey, Pubkey, Secret};

#[derive(Deserialize, Clone, Debug)]
pub struct NetnsItem {
    pub name: String,
    pub id: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct NetworkStats {
    private_key: Privkey,
    pub public_key: Pubkey,
    pub listen_port: u16,
    fwmark: Option<u16>,
    peers: Vec<PeerStats>,
}

impl FromStr for NetworkStats {
    type Err = anyhow::Error;
    fn from_str(output: &str) -> Result<Self, Self::Err> {
        let mut lines = output.lines();
        let network_stats = lines.next().ok_or(anyhow!("Missing network line"))?;
        let components: Vec<&str> = network_stats.split('\t').collect();
        if components.len() != 4 {
            println!("{:?}", components);
            return Err(anyhow!("Wrong network stats line len"));
        }
        Ok(NetworkStats {
            private_key: Privkey::from_str(components[0])?,
            public_key: Pubkey::from_str(components[1])?,
            listen_port: components[2].parse()?,
            fwmark: if components[3] == "off" {
                None
            } else {
                Some(components[3].parse()?)
            },
            peers: lines
                .map(|line| PeerStats::from_str(line))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl NetworkStats {
    pub fn peers(&self) -> &[PeerStats] {
        &self.peers
    }

    pub fn listen_port(&self) -> u16 {
        self.listen_port
    }
}

#[derive(Clone, Debug)]
pub struct PeerStats {
    pub public_key: Pubkey,
    pub preshared_key: Option<Secret>,
    pub endpoint: Option<SocketAddr>,
    pub allowed_ips: Vec<IpNet>,
    pub latest_handshake: Option<SystemTime>,
    pub transfer_rx: usize,
    pub transfer_tx: usize,
    pub persistent_keepalive: Option<usize>,
}

impl FromStr for PeerStats {
    type Err = anyhow::Error;
    fn from_str(output: &str) -> Result<Self, Self::Err> {
        let components: Vec<&str> = output.split('\t').collect();
        if components.len() != 8 {
            return Err(anyhow!("Wrong network stats line len"));
        }
        Ok(PeerStats {
            public_key: Pubkey::from_str(components[0])?,
            preshared_key: if components[1] == "(none)" {
                None
            } else {
                Some(Secret::from_str(components[1])?)
            },
            endpoint: if components[2] == "(none)" {
                None
            } else {
                Some(components[2].parse().context("Parsing endpoint")?)
            },
            allowed_ips: if components[3] == "(none)" {
                vec![]
            } else {
                components[3]
                    .split(',')
                    .map(|ipnet| ipnet.parse())
                    .collect::<Result<Vec<_>, _>>()
                    .context("Parsing IpNet")?
            },
            latest_handshake: {
                let timestamp: u64 = components[4].parse()?;
                if timestamp > 0 {
                    Some(
                        UNIX_EPOCH
                            .checked_add(Duration::from_secs(timestamp))
                            .ok_or(anyhow!("Error parsing latest handshake time"))?,
                    )
                } else {
                    None
                }
            },
            transfer_rx: components[5].parse()?,
            transfer_tx: components[6].parse()?,
            persistent_keepalive: if components[7] == "off" {
                None
            } else {
                Some(components[4].parse()?)
            },
        })
    }
}

impl PeerStats {
    pub fn transfer(&self) -> (usize, usize) {
        (self.transfer_rx, self.transfer_tx)
    }
}
