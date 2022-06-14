use crate::*;
use std::error::Error;

#[ignore]
#[tokio::test]
async fn test_netns_creation() -> Result<(), Box<dyn Error>> {
    let netns_name = "test_network_namespace";
    assert!(!netns_exists(netns_name).await?);
    netns_add(netns_name).await?;
    assert!(netns_exists(netns_name).await?);
    netns_del(netns_name).await?;
    assert!(!netns_exists(netns_name).await?);
    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_netns_list() -> Result<(), Box<dyn Error>> {
    let netns_name = "test2_network_namespace";

    let netnss = netns_list().await?;
    assert!(!netnss.iter().any(|n| n.name == netns_name));

    netns_add(netns_name).await?;
    let netnss = netns_list().await?;
    assert!(netnss.iter().any(|n| n.name == netns_name));

    netns_del(netns_name).await?;
    let netnss = netns_list().await?;
    assert!(!netnss.iter().any(|n| n.name == netns_name));
    Ok(())
}
