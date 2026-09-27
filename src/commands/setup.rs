use anyhow::{Context, Result};
use colored::Colorize;
use std::process::Command;

use crate::node;

pub fn run() -> Result<()> {
    println!("{}", "Setting up the GhostNet SDK bridge…".bold());
    println!();

    let dir = node::ensure_bridge_files()?;
    println!("  bridge files → {}", dir.display().to_string().bright_black());

    println!(
        "  {} running {}",
        "→".cyan(),
        "npm install @n11x/ghostnet-sdk".bold()
    );
    println!();

    let status = Command::new(node::npm_bin())
        .arg("install")
        .current_dir(&dir)
        .status()
        .context("failed to launch npm — is Node.js / npm installed and on your PATH?")?;

    if !status.success() {
        anyhow::bail!("`npm install` failed (exit status {status})");
    }

    println!();
    println!("{}", "GhostNet SDK installed. You're ready to go.".green().bold());
    println!("  Next: {}", "hecate identity create".bold());
    Ok(())
}
