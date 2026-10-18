use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
use std::process::Stdio;

use crate::{config, history, node};

pub fn run(seed: Option<String>, endpoint: Option<String>) -> Result<()> {
    let seed = node::resolve_seed(seed);
    if seed.is_none() {
        anyhow::bail!("an identity is required: set GHOSTNET_SEED or pass --seed; refusing to listen as an unreachable temporary identity");
    }
    let endpoint = config::endpoint(endpoint)?;
    let mut args = vec!["listen".to_string()];
    if let Some(endpoint) = endpoint {
        args.push("--endpoint".to_string());
        args.push(endpoint);
    }

    let mut cmd = node::build_command(&args, seed.as_deref())?;
    cmd.stdout(Stdio::piped());

    let mut child = node::ChildGuard(
        cmd.spawn()
            .context("failed to launch Node.js — is it installed and on your PATH?")?,
    );

    let stdout = child
        .0
        .stdout
        .take()
        .ok_or_else(|| anyhow!("could not capture bridge output"))?;

    println!("{}", "Listening for messages…  (Ctrl-C to stop)".bold());
    println!();

    let mut failed = false;
    let mut reader = BufReader::new(stdout);
    loop {
        let mut line = String::new();
        if (&mut reader).take(1024 * 1024 + 1).read_line(&mut line)? == 0 {
            break;
        }
        if line.len() > 1024 * 1024 {
            anyhow::bail!("SDK listener event exceeds the 1 MiB limit");
        }
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&line) {
            Ok(value) => {
                if value.get("ok").and_then(Value::as_bool) != Some(true) {
                    failed = true;
                    eprintln!("error: SDK listener reported a failure");
                    break;
                }
                render_event(&value)
            }
            Err(_) => anyhow::bail!("invalid JSON from SDK listener"),
        }
    }

    if failed {
        anyhow::bail!("SDK listener failed");
    }
    let status = child.0.wait()?;
    if !status.success() {
        anyhow::bail!("SDK listener failed ({status})");
    }
    Ok(())
}

fn render_event(value: &Value) {
    let event = value.get("event").and_then(Value::as_str).unwrap_or("");
    match event {
        "identity" => {
            let id = value
                .get("nodeId")
                .and_then(Value::as_str)
                .unwrap_or("(unknown)");
            println!("  {} {}", "your node id:".bright_black(), id.bright_cyan());
        }
        "connect" => println!("  {}", "● connected to the mesh".green()),
        "disconnect" => {
            let reason = value.get("reason").and_then(Value::as_str).unwrap_or("");
            println!("  {} {}", "○ disconnected".yellow(), reason.bright_black());
        }
        "message" => {
            let from = value.get("from").and_then(Value::as_str).unwrap_or("?");
            let data = value.get("data").and_then(Value::as_str).unwrap_or("");
            println!(
                "  {} {}",
                format!("{}:", node::redact(from)).bright_magenta().bold(),
                node::redact(data)
            );
            if let Err(error) = history::record("received", from, data.len(), "received") {
                eprintln!("warning: could not record message metadata: {error}");
            }
        }
        "error" => {
            let err = value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            eprintln!("  {} {}", "error:".red().bold(), err);
        }
        _ => println!("  {value}"),
    }
}
