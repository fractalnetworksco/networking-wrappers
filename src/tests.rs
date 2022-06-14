use crate::*;
use std::error::Error;

#[ignore]
#[tokio::test]
async fn test_netns_creation() -> Result<(), Box<dyn Error>> {
    let netns_name = "test_network_namespace";

    // netns does not exist yet
    assert!(!netns_exists(netns_name).await?);

    // add it, now it exists
    netns_add(netns_name).await?;
    assert!(netns_exists(netns_name).await?);

    // adding it again shouldn't work
    assert!(netns_add(netns_name).await.is_err());

    // delete it, now it doesn't
    netns_del(netns_name).await?;
    assert!(!netns_exists(netns_name).await?);

    // can't delete if it doesn't exist
    assert!(netns_del(netns_name).await.is_err());

    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_netns_list() -> Result<(), Box<dyn Error>> {
    let netns_name = "test2_network_namespace";

    // netns name does not appear yet
    let netnss = netns_list().await?;
    assert!(!netnss.iter().any(|n| n.name == netns_name));

    // add it, now it should appear
    netns_add(netns_name).await?;
    let netnss = netns_list().await?;
    assert!(netnss.iter().any(|n| n.name == netns_name));

    // remove it, now it should not be there anymore
    netns_del(netns_name).await?;
    let netnss = netns_list().await?;
    assert!(!netnss.iter().any(|n| n.name == netns_name));

    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_wireguard_mtu() -> Result<(), Box<dyn Error>> {
    let wireguard_interface = "wg83932849";

    // create some interface
    wireguard_create(None, wireguard_interface).await?;
    let show = interface_show(None, wireguard_interface).await?;
    assert_eq!(show.ifname, wireguard_interface);

    // set MTU and verify
    interface_mtu(None, wireguard_interface, 1300).await?;
    let show = interface_show(None, wireguard_interface).await?;
    assert_eq!(show.mtu, Some(1300));

    // clean up
    interface_del(None, wireguard_interface).await?;

    // same, but in network namespace
    let netns = "asadsasd";
    netns_add(netns).await?;
    wireguard_create(Some(netns), wireguard_interface).await?;
    let show = interface_show(Some(netns), wireguard_interface).await?;
    assert_eq!(show.ifname, wireguard_interface);

    // set MTU and verify
    interface_mtu(Some(netns), wireguard_interface, 1300).await?;
    let show = interface_show(Some(netns), wireguard_interface).await?;
    assert_eq!(show.mtu, Some(1300));

    // clean up
    interface_del(Some(netns), wireguard_interface).await?;
    netns_del(netns).await?;

    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_wireguard_up() -> Result<(), Box<dyn Error>> {
    let wireguard_interface = "wg42839432";

    // create some interface
    wireguard_create(None, wireguard_interface).await?;
    let show = interface_show(None, wireguard_interface).await?;
    assert_eq!(show.ifname, wireguard_interface);
    assert!(show.is_down());

    // set MTU and verify
    interface_up(None, wireguard_interface).await?;
    let show = interface_show(None, wireguard_interface).await?;
    assert!(!show.is_down());

    // clean up
    interface_del(None, wireguard_interface).await?;

    // same, but in network namespace
    let netns = "akjhaskd";
    netns_add(netns).await?;
    wireguard_create(Some(netns), wireguard_interface).await?;
    let show = interface_show(Some(netns), wireguard_interface).await?;
    assert_eq!(show.ifname, wireguard_interface);
    assert!(show.is_down());

    // set MTU and verify
    interface_up(Some(netns), wireguard_interface).await?;
    let show = interface_show(Some(netns), wireguard_interface).await?;
    assert!(!show.is_down());

    // clean up
    interface_del(Some(netns), wireguard_interface).await?;
    netns_del(netns).await?;

    Ok(())
}
