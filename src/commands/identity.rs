use anyhow::Result;
use colored::Colorize;
use serde_json::Value;

use crate::cli::IdentityAction;
use crate::{config, node};

pub fn run(action: IdentityAction) -> Result<()> {
    // For `load`, the seed is passed to the child via env (not argv).
    let (args, seed): (Vec<String>, Option<String>) = match &action {
        IdentityAction::Create { .. } => (vec!["identity-create".to_string()], None),
        IdentityAction::Load { seed, .. } => (
            vec!["identity-load".to_string()],
            node::resolve_seed(seed.clone()),
        ),
        IdentityAction::List | IdentityAction::Remove { .. } => {
            unreachable!("dispatched separately")
        }
    };

    if matches!(action, IdentityAction::Load { .. }) && seed.is_none() {
        return Err(anyhow::anyhow!(
            "a 12-word seed phrase is required (pass it as an argument or set GHOSTNET_SEED)"
        ));
    }

    let resp = node::run_bridge_json(&args, seed.as_deref())?;
    let node_id = field(&resp, "nodeId");

    match action {
        IdentityAction::Create { name } => {
            println!("{}", "New identity created".green().bold());
            kv("Node ID", &node_id.bright_cyan().to_string());
            kv(
                "Seed phrase",
                &field(&resp, "seedPhrase").yellow().to_string(),
            );
            println!();
            println!(
                "{}",
                "  Back up your seed phrase. Anyone who has it controls this identity.".yellow()
            );
            if let Some(name) = name {
                config::label(name.clone(), node_id.clone())?;
                println!("  {} saved public label {}", "✓".green(), name);
            }
        }
        IdentityAction::Load { name, .. } => {
            println!("{}", "Identity restored ✓".green().bold());
            kv("Node ID", &node_id.bright_cyan().to_string());
            if let Some(name) = name {
                config::label(name.clone(), node_id.clone())?;
                println!("  {} saved public label {}", "✓".green(), name);
            }
        }
        IdentityAction::List | IdentityAction::Remove { .. } => unreachable!(),
    }

    Ok(())
}

pub fn list() -> Result<()> {
    let config = config::load()?;
    if config.identities.is_empty() {
        println!("No public identity labels saved. Create or load with --name.");
        return Ok(());
    }
    for (name, identity) in config.identities {
        println!("  {:<16}{}", format!("{name}:"), identity.node_id);
    }
    Ok(())
}

pub fn remove(name: String) -> Result<()> {
    let mut config = config::load()?;
    if config.identities.remove(&name).is_none() {
        anyhow::bail!("no identity label named {name:?}");
    }
    config::save(&config)?;
    println!("{} removed local public identity label {name}", "✓".green());
    Ok(())
}

fn field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("(unknown)")
        .to_string()
}

fn kv(label: &str, value: &str) {
    println!("  {:<13}{}", format!("{label}:").bright_black(), value);
}
