use crate::{cli::ConfigAction, config};
use anyhow::Result;
use colored::Colorize;

pub fn run(action: ConfigAction) -> Result<()> {
    let mut current = config::load()?;
    match action {
        ConfigAction::Show { json: true } => {
            println!("{}", serde_json::to_string_pretty(&current)?)
        }
        ConfigAction::Show { json: false } => {
            println!("Config file: {}", config::path()?.display());
            println!(
                "Endpoint: {}",
                current.endpoint.as_deref().unwrap_or("SDK default")
            );
            println!("Public identity labels: {}", current.identities.len());
            println!("Trusted peers: {}", current.trusted_peers.len());
        }
        ConfigAction::SetEndpoint { endpoint } => {
            config::validate_endpoint(&endpoint)?;
            current.endpoint = Some(endpoint.clone());
            config::save(&current)?;
            println!("{} default endpoint set to {}", "✓".green(), endpoint);
        }
        ConfigAction::ClearEndpoint => {
            current.endpoint = None;
            config::save(&current)?;
            println!("{} default endpoint cleared", "✓".green());
        }
    }
    Ok(())
}
