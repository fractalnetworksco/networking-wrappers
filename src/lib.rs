mod types;

use anyhow::{anyhow, Context, Result};
use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use log::*;
use serde::Deserialize;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use types::*;
use wireguard_keys::{Privkey, Pubkey};

pub const IPTABLES_SAVE_PATH: &'static str = "/usr/sbin/iptables-save";
pub const IPTABLES_RESTORE_PATH: &'static str = "/usr/sbin/iptables-restore";
pub const IP_PATH: &'static str = "/usr/sbin/ip";

/// Adds a network namespace. This creates a new, isolated network namespace
/// with nothing but the loopback interface in it.
pub async fn netns_add(name: &str) -> Result<()> {
    info!("netns add {}", name);
    let success = Command::new(IP_PATH)
        .arg("netns")
        .arg("add")
        .arg(name)
        .status()
        .await?
        .success();
    match success {
        true => Ok(()),
        false => Err(anyhow!("Error creating netns")),
    }
}

/// Checks if a network namespaces exists.
/// TODO: use `ip --json netns list` here.
pub async fn netns_exists(name: &str) -> Result<bool> {
    let success = Command::new(IP_PATH)
        .arg("netns")
        .arg("exec")
        .arg(name)
        .arg("/bin/true")
        .status()
        .await?
        .success();
    Ok(success)
}

/// Delete a network namespace. This will also delete any network interfaces contained therein.
pub async fn netns_del(name: &str) -> Result<()> {
    info!("netns del {}", name);
    let success = Command::new(IP_PATH)
        .arg("netns")
        .arg("del")
        .arg(name)
        .status()
        .await?
        .success();
    match success {
        true => Ok(()),
        false => Err(anyhow!("Error deleting netns")),
    }
}

/// Write file into network namespace config folder.
pub async fn netns_write_file(netns: &str, filename: &Path, data: &str) -> Result<()> {
    let mut path = PathBuf::from("/etc/netns");
    path.push(netns);
    if let Some(parent) = filename.parent() {
        path.push(parent);
    }
    tokio::fs::create_dir_all(&path).await?;
    path.push(filename.file_name().unwrap());
    tokio::fs::write(path, data.as_bytes()).await?;
    Ok(())
}

/// List all network namespaces.
pub async fn netns_list() -> Result<Vec<NetnsItem>> {
    let output = Command::new(IP_PATH)
        .arg("--json")
        .arg("netns")
        .arg("list")
        .output()
        .await?;
    if !output.status.success() {
        return Err(anyhow!("Error fetching wireguard stats"));
    }
    let output = String::from_utf8(output.stdout).context("Parsing command output as string")?;
    let mut items: Vec<NetnsItem> = vec![];
    if output.len() > 0 {
        items = serde_json::from_str(&output).context("Pasing netns list output as JSON")?;
    }
    Ok(items)
}

/// Add an address to an interface
pub async fn addr_add(netns: Option<&str>, interface: &str, addr: IpNet) -> Result<()> {
    info!("addr add {:?}, {}, {}", netns, interface, addr);
    let mut command = Command::new(IP_PATH);
    if let Some(netns) = netns {
        command.arg("-n").arg(netns);
    }
    let success = command
        .arg("addr")
        .arg("add")
        .arg(addr.to_string())
        .arg("dev")
        .arg(interface)
        .status()
        .await?
        .success();
    if !success {
        return Err(anyhow!("Error setting address"));
    }
    Ok(())
}

/// Create bridge interface.
pub async fn bridge_add(netns: Option<&str>, interface: &str) -> Result<()> {
    info!("bridge_add({:?}, {})", netns, interface);
    let mut command = Command::new(IP_PATH);
    if let Some(netns) = netns {
        command.arg("-n").arg(netns);
    }
    let success = command
        .arg("link")
        .arg("add")
        .arg(interface)
        .arg("type")
        .arg("bridge")
        .status()
        .await?
        .success();
    if !success {
        return Err(anyhow!(
            "Error creating bridge {} in {:?}",
            interface,
            netns
        ));
    }
    Ok(())
}

/// Check if bridge interface exists.
pub async fn bridge_exists(netns: Option<&str>, name: &str) -> Result<bool> {
    let mut command = Command::new(IP_PATH);
    if let Some(netns) = netns {
        command.arg("-n").arg(netns);
    }
    let output = command
        .arg("link")
        .arg("show")
        .arg(name)
        .arg("type")
        .arg("bridge")
        .output()
        .await?;
    if output.status.success() && output.stdout.len() > 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct InterfaceShow {
    pub ifindex: usize,
    pub ifname: String,
    pub mtu: Option<usize>,
    pub operstate: String,
}

/// Get details of an interface.
pub async fn interface_show(netns: Option<&str>, interface: &str) -> Result<InterfaceShow> {
    let mut command = Command::new(IP_PATH);
    command.arg("--json");
    if let Some(netns) = netns {
        command.arg("-n").arg(netns);
    }
    command.arg("link").arg("show").arg("dev").arg(interface);
    let output = command.output().await?;
    if !output.status.success() {
        return Err(anyhow!("Error checking interface state"));
    }
    let output = String::from_utf8(output.stdout)?;
    let items: Vec<InterfaceShow> = serde_json::from_str(&output)?;
    if items.len() == 1 {
        Ok(items[0].clone())
    } else {
        Err(anyhow!("Did not return any interfaces"))
    }
}

/// Check if an interface is down.
pub async fn interface_down(netns: Option<&str>, interface: &str) -> Result<bool> {
    let show = interface_show(netns, interface).await?;
    Ok(show.operstate == "DOWN")
}

/// Set an interface to be up.
pub async fn interface_set_up(netns: Option<&str>, interface: &str) -> Result<()> {
    info!("interface_up({:?}, {})", netns, interface);
    let mut command = Command::new(IP_PATH);
    if let Some(netns) = netns {
        command.arg("-n").arg(netns);
    }
    command.arg("link").arg("set").arg(interface).arg("up");
    if !command.status().await?.success() {
        return Err(anyhow!("Error setting interface up"));
    }
    Ok(())
}

/// Sets an interface's MTU.
pub async fn interface_mtu(netns: Option<&str>, interface: &str, mtu: usize) -> Result<()> {
    info!("interface_mtu({:?}, {}, {})", netns, interface, mtu);
    let mut command = Command::new(IP_PATH);
    if let Some(netns) = netns {
        command.arg("-n").arg(netns);
    }
    command
        .arg("link")
        .arg("set")
        .arg(interface)
        .arg("mtu")
        .arg(mtu.to_string());
    if !command.status().await?.success() {
        return Err(anyhow!("Error setting interface up"));
    }
    Ok(())
}

#[derive(Deserialize, PartialEq, Debug)]
struct IpInterfaceAddr {
    addr_info: Vec<IpInterfaceAddrInfo>,
}

#[derive(Deserialize, PartialEq, Debug)]
struct IpInterfaceAddrInfo {
    local: IpAddr,
    prefixlen: u8,
}

#[test]
fn test_ip_addr() {
    use std::net::Ipv4Addr;
    let test = r#"[{"ifindex":58,"ifname":"wg0","flags":["POINTOPOINT","NOARP","UP","LOWER_UP"],"mtu":1420,"qdisc":"noqueue","operstate":"UNKNOWN","group":"default","txqlen":1000,"link_type":"none","addr_info":[{"family":"inet","local":"10.80.69.7","prefixlen":24,"scope":"global","label":"wg0","valid_life_time":4294967295,"preferred_life_time":4294967295}]}]"#;
    let output: Vec<IpInterfaceAddr> = serde_json::from_str(test).unwrap();
    assert_eq!(
        output,
        vec![IpInterfaceAddr {
            addr_info: vec![IpInterfaceAddrInfo {
                local: IpAddr::V4(Ipv4Addr::new(10, 80, 69, 7)),
                prefixlen: 24
            }],
        }]
    );
}

/// Given an interface, list addresses.
pub async fn addr_list(netns: Option<&str>, interface: &str) -> Result<Vec<IpNet>> {
    let mut command = Command::new(IP_PATH);
    command.arg("--json");
    if let Some(netns) = netns {
        command.arg("-n").arg(netns);
    }
    let output = command
        .arg("addr")
        .arg("show")
        .arg("dev")
        .arg(interface)
        .output()
        .await?;
    if !output.status.success() {
        return Err(anyhow!(
            "Error fetching addr for {} in {:?}",
            interface,
            netns
        ));
    }
    let output = String::from_utf8(output.stdout)?;
    let items: Vec<IpInterfaceAddr> = serde_json::from_str(&output)?;
    Ok(items
        .iter()
        .map(|addr| {
            addr.addr_info.iter().map(|info| match info.local {
                IpAddr::V4(addr) => IpNet::V4(Ipv4Net::new(addr, info.prefixlen).unwrap()),
                IpAddr::V6(addr) => IpNet::V6(Ipv6Net::new(addr, info.prefixlen).unwrap()),
            })
        })
        .flatten()
        .collect())
}

#[derive(Deserialize)]
struct LinkInfo {
    master: Option<String>,
}

pub async fn link_get_master(netns: Option<&str>, interface: &str) -> Result<Option<String>> {
    let mut command = Command::new(IP_PATH);
    command.arg("--json");
    if let Some(netns) = netns {
        command.arg("-n").arg(netns);
    }
    let output = command
        .arg("link")
        .arg("show")
        .arg("dev")
        .arg(interface)
        .output()
        .await?;
    if !output.status.success() {
        return Err(anyhow!(
            "Error checking interface {} master in {:?}",
            interface,
            netns
        ));
    }
    let output = String::from_utf8(output.stdout)?;
    if output.len() == 0 {
        return Ok(None);
    }
    let output: Vec<LinkInfo> = serde_json::from_str(&output)?;
    if output.len() == 0 {
        return Ok(None);
    }
    Ok(output[0].master.clone())
}

pub async fn link_set_master(netns: Option<&str>, interface: &str, master: &str) -> Result<()> {
    let mut command = Command::new(IP_PATH);
    command.arg("--json");
    if let Some(netns) = netns {
        command.arg("-n").arg(netns);
    }
    let status = command
        .arg("link")
        .arg("set")
        .arg("dev")
        .arg(interface)
        .arg("master")
        .arg(master)
        .status()
        .await?;
    if !status.success() {
        return Err(anyhow!(
            "Error setting interface {} master in {:?} to {}",
            interface,
            netns,
            master
        ));
    }
    Ok(())
}

/// Create veth interface.
pub async fn veth_add(netns: &str, outer: &str, inner: &str) -> Result<()> {
    info!("veth add {}, {}, {}", netns, outer, inner);
    if !Command::new(IP_PATH)
        .arg("link")
        .arg("add")
        .arg("dev")
        .arg(outer)
        .arg("type")
        .arg("veth")
        .arg("peer")
        .arg(inner)
        .arg("netns")
        .arg(netns)
        .status()
        .await?
        .success()
    {
        return Err(anyhow!(
            "Error creating veth interfaces {} and {} in {}",
            outer,
            inner,
            netns
        ));
    }
    Ok(())
}

/// Check if a veth interface exists.
pub async fn veth_exists(netns: &str, name: &str) -> Result<bool> {
    let output = Command::new(IP_PATH)
        .arg("-n")
        .arg(netns)
        .arg("link")
        .arg("show")
        .arg(name)
        .arg("type")
        .arg("veth")
        .output()
        .await?;
    if output.status.success() && output.stdout.len() > 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Create a wireguard interface.
pub async fn wireguard_create(netns: &str, name: &str) -> Result<()> {
    info!("wireguard create {}, {}", netns, name);
    if !Command::new(IP_PATH)
        .arg("link")
        .arg("add")
        .arg("dev")
        .arg(name)
        .arg("type")
        .arg("wireguard")
        .status()
        .await?
        .success()
    {
        return Err(anyhow!("Error creating wireguard interface"));
    }
    if !Command::new(IP_PATH)
        .arg("link")
        .arg("set")
        .arg(name)
        .arg("netns")
        .arg(netns)
        .status()
        .await?
        .success()
    {
        return Err(anyhow!("Error moving wireguard interface"));
    }
    Ok(())
}

/// Check if wireguard interface exists.
pub async fn wireguard_exists(netns: &str, name: &str) -> Result<bool> {
    let output = Command::new(IP_PATH)
        .arg("-n")
        .arg(netns)
        .arg("link")
        .arg("show")
        .arg(name)
        .arg("type")
        .arg("wireguard")
        .output()
        .await?;
    if output.status.success() && output.stdout.len() > 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Sync the configuration state of a WireGuard interface with a given one.
pub async fn wireguard_syncconf(netns: &str, name: &str) -> Result<()> {
    info!("wireguard syncconf {}, {}", netns, name);
    if !Command::new(IP_PATH)
        .arg("netns")
        .arg("exec")
        .arg(netns)
        .arg("wg")
        .arg("syncconf")
        .arg(name)
        .arg(format!("/etc/wireguard/{}.conf", name))
        .status()
        .await?
        .success()
    {
        return Err(anyhow!("Error syncronizing wireguard config"));
    }
    Ok(())
}

pub async fn wireguard_stats(netns: &str, name: &str) -> Result<NetworkStats> {
    let result = Command::new(IP_PATH)
        .arg("netns")
        .arg("exec")
        .arg(netns)
        .arg("wg")
        .arg("show")
        .arg(name)
        .arg("dump")
        .output()
        .await?;
    if !result.status.success() {
        return Err(anyhow!("Error fetching wireguard stats"));
    }
    let result = String::from_utf8(result.stdout)?;
    let stats = NetworkStats::from_str(&result)?;
    Ok(stats)
}

pub async fn iptables_save(netns: Option<&str>) -> Result<String> {
    let mut command = if let Some(netns) = netns {
        let mut command = Command::new(IP_PATH);
        command
            .arg("netns")
            .arg("exec")
            .arg(netns)
            .arg(IPTABLES_SAVE_PATH);
        command
    } else {
        Command::new(IPTABLES_SAVE_PATH)
    };
    let output = command.output().await?;
    if !output.status.success() {
        return Err(anyhow!("Error saving iptables state"));
    }
    let state = String::from_utf8(output.stdout)?;
    Ok(state)
}

pub async fn iptables_restore(netns: Option<&str>, state: &str) -> Result<()> {
    info!("iptables_restore({:?}, {})", netns, state.len());
    let mut command = if let Some(netns) = netns {
        let mut command = Command::new(IP_PATH);
        command
            .arg("netns")
            .arg("exec")
            .arg(netns)
            .arg(IPTABLES_RESTORE_PATH)
            .arg("-w");
        command
    } else {
        Command::new(IPTABLES_RESTORE_PATH)
    };
    let mut handle = command.stdin(std::process::Stdio::piped()).spawn()?;
    let mut stdin = handle.stdin.take().unwrap();
    stdin.write_all(state.as_bytes()).await?;
    drop(stdin);
    let result = handle.wait().await?;
    if !result.success() {
        return Err(anyhow!("Error restoring iptables state"));
    }
    Ok(())
}

pub async fn nginx_reload() -> Result<()> {
    let status = Command::new("nginx")
        .arg("-s")
        .arg("reload")
        .status()
        .await?;
    if !status.success() {
        return Err(anyhow!("Error reloading nginx"));
    }
    Ok(())
}
