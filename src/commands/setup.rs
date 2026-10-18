use anyhow::{Context, Result};
use colored::Colorize;
use std::process::Command;

use crate::node;

pub fn run() -> Result<()> {
    println!("{}", "Setting up the GhostNet SDK bridge…".bold());
    println!();

    let dir = node::ensure_bridge_files()?;
    println!(
        "  bridge files → {}",
        dir.display().to_string().bright_black()
    );

    println!(
        "  {} running {}",
        "→".cyan(),
        "npm install @n11x/ghostnet-sdk".bold()
    );
    println!();

    let output = node::capture(
        Command::new(node::npm_bin())
            .args(["install", "--no-audit", "--no-fund"])
            .env_remove("GHOSTNET_SEED")
            .current_dir(&dir),
        std::time::Duration::from_secs(300),
    )
    .context("failed to launch npm — is Node.js / npm installed and on your PATH?")?;

    if !output.status.success() {
        anyhow::bail!(
            "`npm install` failed ({}); check npm registry access and permissions in {}",
            output.status,
            dir.display()
        );
    }
    if !node::bridge_installed() {
        anyhow::bail!("npm completed without installing the SDK; check registry configuration");
    }

    println!();
    println!(
        "{}",
        "GhostNet SDK installed. You're ready to go.".green().bold()
    );
    println!("  Next: {}", "hecate identity create".bold());
    Ok(())
}
