use anyhow::Result;
use colored::Colorize;

use crate::cli::PeerAction;
use crate::{config, node};

pub fn run(action: PeerAction) -> Result<()> {
    match action {
        PeerAction::Add {
            name,
            node_id,
            public_key,
        } => add(name, node_id, public_key),
        PeerAction::List => list(),
        PeerAction::Remove { name } => remove(name),
    }
}

fn add(name: String, expected_node_id: String, public_key: String) -> Result<()> {
    config::validate_label(&name)?;
    config::validate_node_id(&expected_node_id)?;
    config::validate_public_key(&public_key)?;

    let response = node::run_bridge_json(&["peer-from-key".to_string(), public_key.clone()], None)?;
    let derived_node_id = response
        .get("nodeId")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Atlas bridge response missing nodeId"))?;
    if derived_node_id != expected_node_id {
        anyhow::bail!(
            "public key belongs to node ID {derived_node_id}, not the expected {expected_node_id}; peer was not trusted"
        );
    }

    let mut current = config::load()?;
    if let Some(existing) = current.trusted_peers.get(&name) {
        if existing.node_id == expected_node_id && existing.public_key == public_key {
            println!("trusted peer {name} is already registered with this key");
            return Ok(());
        }
        anyhow::bail!(
            "peer label {name:?} already exists with different key material; remove it explicitly before replacing trust"
        );
    }
    if let Some((other_name, _)) = current
        .trusted_peers
        .iter()
        .find(|(other_name, peer)| **other_name != name && peer.node_id == expected_node_id)
    {
        anyhow::bail!("node ID is already trusted as peer label {other_name:?}");
    }
    current.trusted_peers.insert(
        name.clone(),
        config::TrustedPeer {
            node_id: expected_node_id.clone(),
            public_key,
            created_at: config::unix_time(),
        },
    );
    config::save(&current)?;
    println!(
        "{} trusted peer {} ({})",
        "✓".green(),
        name,
        expected_node_id
    );
    Ok(())
}

fn list() -> Result<()> {
    let current = config::load()?;
    if current.trusted_peers.is_empty() {
        println!("No trusted peers registered.");
        return Ok(());
    }
    for (name, peer) in current.trusted_peers {
        println!("  {name}: {} {}", peer.node_id, peer.public_key);
    }
    Ok(())
}

fn remove(name: String) -> Result<()> {
    let mut current = config::load()?;
    if current.trusted_peers.remove(&name).is_none() {
        anyhow::bail!("no trusted peer named {name:?}");
    }
    config::save(&current)?;
    println!("{} removed trusted peer {name}", "✓".green());
    Ok(())
}
