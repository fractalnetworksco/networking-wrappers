use crate::*;
use std::error::Error;

#[ignore]
#[tokio::test]
async fn test_netns_creation() -> Result<(), Box<dyn Error>>{
    let netns_name = "test_network_namespace";
    assert!(!netns_exists(netns_name).await?);
    netns_add(netns_name).await?;
    assert!(netns_exists(netns_name).await?);
    netns_del(netns_name).await?;
    assert!(!netns_exists(netns_name).await?);
    Ok(())
}
