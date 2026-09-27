use crate::{config, node};
use anyhow::Result;
use std::process::Command;

pub fn run() -> Result<()> {
    let node_ok = Command::new(node::node_bin())
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let npm_ok = Command::new(node::npm_bin())
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    println!("{} Node.js", if node_ok { "✓" } else { "✗" });
    println!("{} npm", if npm_ok { "✓" } else { "✗" });
    println!(
        "{} SDK bridge",
        if node::bridge_installed() {
            "✓"
        } else {
            "✗ (run `hecate setup`)"
        }
    );
    match config::load() {
        Ok(_) => println!("✓ configuration"),
        Err(error) => println!("✗ configuration: {error}"),
    }
    Ok(())
}
