use crate::{config, node};
use anyhow::Result;

pub fn run() -> Result<()> {
    let config = config::load()?;
    println!(
        "SDK bridge: {}",
        if node::bridge_installed() {
            "installed"
        } else {
            "not installed — run `hecate setup`"
        }
    );
    println!(
        "Endpoint: {}",
        config.endpoint.as_deref().unwrap_or("SDK default")
    );
    println!("Public identity labels: {}", config.identities.len());
    println!(
        "Identity loaded: {}",
        if std::env::var("GHOSTNET_SEED")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .is_some()
        {
            "yes (environment)"
        } else {
            "no"
        }
    );
    Ok(())
}
