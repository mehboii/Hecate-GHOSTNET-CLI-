use anyhow::Result;
use colored::Colorize;

use crate::{config, history, node};

pub fn run(peer: String, message: String, endpoint: Option<String>) -> Result<()> {
    validate_message(&message)?;
    config::validate_label(&peer)?;
    let trusted_peer = config::trusted_peer(&peer)?;
    let seed = node::resolve_seed();
    if seed.is_none() {
        anyhow::bail!("an identity is required: set GHOSTNET_SEED; refusing to create an unreachable temporary identity");
    }
    let endpoint = config::endpoint(endpoint)?;
    let mut args = vec![
        "send".to_string(),
        trusted_peer.node_id.clone(),
        message,
        trusted_peer.public_key,
    ];
    if let Some(endpoint) = endpoint {
        args.push("--endpoint".to_string());
        args.push(endpoint);
    }

    println!(
        "{} {}",
        "→ Connecting to the mesh and sending to".cyan(),
        trusted_peer.node_id.bright_cyan()
    );

    let response = node::run_bridge_json(&args, seed.as_deref())?;
    if response.get("sent").and_then(serde_json::Value::as_bool) != Some(true) {
        anyhow::bail!("SDK bridge did not confirm the send operation");
    }
    if let Err(error) = history::record("sent", &trusted_peer.node_id, args[2].len(), "sent") {
        eprintln!("warning: message sent, but could not record metadata: {error}");
    }

    println!("{}", "Message sent.".green().bold());
    Ok(())
}

fn validate_message(message: &str) -> Result<()> {
    if message.is_empty() {
        anyhow::bail!("message cannot be empty");
    }
    if message.len() > 64 * 1024 {
        anyhow::bail!("message exceeds the 64 KiB UTF-8 limit");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn utf8_message_limit_is_in_bytes() {
        assert!(super::validate_message("").is_err());
        assert!(super::validate_message(&"é".repeat(32768)).is_ok());
        assert!(super::validate_message(&"é".repeat(32769)).is_err());
    }
}
