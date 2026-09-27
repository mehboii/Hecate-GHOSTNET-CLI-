use anyhow::Result;
use colored::Colorize;

use crate::node;

pub fn run() -> Result<()> {
    println!("{}", "About".bold().underline());
    row("CLI version", env!("CARGO_PKG_VERSION"));
    row("SDK package", "@n11x/ghostnet-sdk");
    row("Install", "npm install @n11x/ghostnet-sdk");
    row("Maintainer", "N11X Collective");
    println!();

    let bridge_status = if node::bridge_installed() {
        "installed".green().to_string()
    } else {
        "not installed — run `hecate setup`".yellow().to_string()
    };
    row("SDK bridge", &bridge_status);

    Ok(())
}

fn row(label: &str, value: &str) {
    println!("  {:<14}{}", format!("{label}:").bright_black(), value);
}
