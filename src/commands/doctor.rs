use crate::{config, node};
use anyhow::Result;
use std::process::Command;

pub fn run() -> Result<()> {
    let node_ok = node::capture(
        Command::new(node::node_bin())
            .arg("--version")
            .env_remove("GHOSTNET_SEED"),
        std::time::Duration::from_secs(10),
    )
    .map(|o| {
        o.status.success()
            && String::from_utf8_lossy(&o.stdout)
                .trim()
                .trim_start_matches('v')
                .split('.')
                .next()
                .and_then(|major| major.parse::<u32>().ok())
                .is_some_and(|major| major >= 18)
    })
    .unwrap_or(false);
    let npm_ok = node::capture(
        Command::new(node::npm_bin())
            .arg("--version")
            .env_remove("GHOSTNET_SEED"),
        std::time::Duration::from_secs(10),
    )
    .map(|o| o.status.success())
    .unwrap_or(false);
    println!("{} Node.js >=18", if node_ok { "✓" } else { "✗" });
    println!("{} npm", if npm_ok { "✓" } else { "✗" });
    println!(
        "{} SDK bridge",
        if node::bridge_installed() {
            "✓"
        } else {
            "✗ (run `ghostnet setup`)"
        }
    );
    let config_ok = config::load().is_ok();
    match config::load() {
        Ok(_) => println!("✓ configuration"),
        Err(error) => println!("✗ configuration: {error}"),
    }
    if !node_ok || !npm_ok || !node::bridge_installed() || !config_ok {
        anyhow::bail!("diagnostic checks failed; check Node.js/npm, repair configuration, and run `ghostnet setup`");
    }
    Ok(())
}
