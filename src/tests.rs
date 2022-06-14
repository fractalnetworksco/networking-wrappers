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
