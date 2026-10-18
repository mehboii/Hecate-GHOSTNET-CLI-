//! Locating and invoking the bundled Node bridge that wraps @n11x/ghostnet-sdk.

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Bridge sources are embedded into the binary and materialized on disk at setup.
pub const BRIDGE_JS: &str = include_str!("../bridge/ghostnet-bridge.mjs");
pub const BRIDGE_PKG: &str = include_str!("../bridge/package.json");

/// `node` executable (override with GHOSTNET_NODE).
pub fn node_bin() -> String {
    std::env::var("GHOSTNET_NODE").unwrap_or_else(|_| "node".to_string())
}

/// `npm` executable. On Windows the launcher is `npm.cmd`.
pub fn npm_bin() -> String {
    if let Ok(custom) = std::env::var("GHOSTNET_NPM") {
        return custom;
    }
    if cfg!(windows) {
        "npm.cmd".to_string()
    } else {
        "npm".to_string()
    }
}

/// Where the bridge + its node_modules live: ~/.ghostnet-cli/bridge
pub fn bridge_dir() -> Result<PathBuf> {
    Ok(crate::config::data_dir()?.join("bridge"))
}

pub fn bridge_script() -> Result<PathBuf> {
    Ok(bridge_dir()?.join("ghostnet-bridge.mjs"))
}

/// True once `npm install` has pulled the SDK into the bridge.
pub fn bridge_installed() -> bool {
    bridge_dir()
        .map(|d| {
            d.join("ghostnet-bridge.mjs").is_file()
                && d.join("node_modules")
                    .join("@n11x")
                    .join("ghostnet-sdk")
                    .join("package.json")
                    .is_file()
        })
        .unwrap_or(false)
}

/// Write the embedded bridge files to disk, creating the directory if needed.
pub fn ensure_bridge_files() -> Result<PathBuf> {
    let dir = bridge_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating bridge directory {}", dir.display()))?;
    std::fs::write(dir.join("ghostnet-bridge.mjs"), BRIDGE_JS)?;
    std::fs::write(dir.join("package.json"), BRIDGE_PKG)?;
    Ok(dir)
}

fn require_ready() -> Result<PathBuf> {
    let script = bridge_script()?;
    if !script.exists() {
        return Err(anyhow!(
            "the GhostNet SDK bridge isn't set up yet — run `ghostnet setup` first"
        ));
    }
    if !bridge_installed() {
        return Err(anyhow!(
            "the GhostNet SDK isn't installed yet — run `ghostnet setup` first"
        ));
    }
    Ok(script)
}

/// Resolve a seed phrase from an explicit value, falling back to the
/// `GHOSTNET_SEED` environment variable. Lets callers keep the secret out of
/// the command line (and shell history) entirely.
pub fn resolve_seed(explicit: Option<String>) -> Option<String> {
    explicit
        .or_else(|| {
            std::env::var("GHOSTNET_SEED")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
        .filter(|s| !s.trim().is_empty())
}

/// Build the `node <bridge> <args…>` command. A seed, if present, is passed via
/// the child's environment — never as a process argument — so it is not exposed
/// in `ps` / `/proc/<pid>/cmdline` / Task Manager to other local processes.
pub fn build_command(args: &[String], seed: Option<&str>) -> Result<Command> {
    let script = require_ready()?;
    let dir = bridge_dir()?;
    let mut cmd = Command::new(node_bin());
    cmd.arg(&script).args(args).current_dir(&dir);
    cmd.env_remove("GHOSTNET_SEED");
    if let Some(seed) = seed {
        cmd.env("GHOSTNET_SEED", seed);
    }
    Ok(cmd)
}

/// Run a one-shot bridge command and return its raw process output.
pub fn run_bridge(args: &[String], seed: Option<&str>) -> Result<Output> {
    let mut command = build_command(args, seed)?;
    capture(&mut command, timeout()?)
}

pub fn timeout() -> Result<Duration> {
    let seconds = std::env::var("GHOSTNET_TIMEOUT_SECS").unwrap_or_else(|_| "60".into());
    let seconds: u64 = seconds
        .parse()
        .context("GHOSTNET_TIMEOUT_SECS must be a positive integer")?;
    if seconds == 0 || seconds > 86400 {
        anyhow::bail!("GHOSTNET_TIMEOUT_SECS must be between 1 and 86400");
    }
    Ok(Duration::from_secs(seconds))
}

/// Always reap children, including early protocol/read failures.
pub struct ChildGuard(pub Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn capture(command: &mut Command, timeout: Duration) -> Result<Output> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = ChildGuard(
        command
            .spawn()
            .context("failed to launch subprocess; check the configured executable and PATH")?,
    );
    let stdout = child.0.stdout.take().unwrap();
    let stderr = child.0.stderr.take().unwrap();
    let out = std::thread::spawn(move || read_bounded(stdout));
    let err = std::thread::spawn(move || read_bounded(stderr));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        if start.elapsed() >= timeout {
            child.0.kill()?;
            child.0.wait()?;
            anyhow::bail!("subprocess timed out after {} seconds; check connectivity or GHOSTNET_TIMEOUT_SECS", timeout.as_secs());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Ok(Output {
        status,
        stdout: out.join().map_err(|_| anyhow!("stdout reader failed"))??,
        stderr: err.join().map_err(|_| anyhow!("stderr reader failed"))??,
    })
}

fn read_bounded(mut reader: impl Read) -> Result<Vec<u8>> {
    const LIMIT: usize = 4 * 1024 * 1024;
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    let mut exceeded = false;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        if bytes.len() + count <= LIMIT && !exceeded {
            bytes.extend_from_slice(&buffer[..count]);
        } else {
            exceeded = true;
        }
    }
    if exceeded {
        anyhow::bail!("subprocess output exceeded the 4 MiB capture limit");
    }
    Ok(bytes)
}

/// Run a one-shot bridge command and parse the last NDJSON line it emitted.
pub fn run_bridge_json(args: &[String], seed: Option<&str>) -> Result<Value> {
    let output = run_bridge(args, seed)?;
    parse_last_json(&output).map_err(|error| {
        let mut message = error.to_string();
        if let Some(seed) = seed.filter(|s| !s.is_empty()) {
            message = message.replace(seed, "[redacted]");
        }
        anyhow!(message)
    })
}

/// Extract and parse the final non-empty JSON line from a process output.
pub fn parse_last_json(output: &Output) -> Result<Value> {
    if !output.status.success() {
        if let Some(value) = String::from_utf8_lossy(&output.stdout)
            .lines()
            .rev()
            .find_map(|line| serde_json::from_str::<Value>(line).ok())
        {
            if value.get("ok").and_then(Value::as_bool) == Some(false) {
                if let Some(message) = value.get("error").and_then(Value::as_str) {
                    anyhow::bail!("SDK bridge failed: {}", redact(message));
                }
            }
        }
        anyhow::bail!(
            "SDK bridge failed ({}); check setup and Node.js compatibility",
            output.status
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);

    let line = stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .ok_or_else(|| anyhow!("the SDK bridge produced no output"))?;

    let value: Value = serde_json::from_str(line).context("invalid JSON from SDK bridge")?;

    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        let msg = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("bridge reported an error");
        return Err(anyhow!(redact(msg)));
    }

    if value.get("ok").and_then(Value::as_bool) != Some(true) {
        anyhow::bail!("SDK bridge response missing ok: true");
    }

    Ok(value)
}

pub fn redact(message: &str) -> String {
    let mut message = message.to_string();
    if let Ok(seed) = std::env::var("GHOSTNET_SEED") {
        if !seed.is_empty() {
            message = message.replace(&seed, "[redacted]");
        }
    }
    message.chars().filter(|c| !c.is_control()).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn subprocess_capture_is_bounded() {
        assert!(super::read_bounded(&vec![b'x'; 4 * 1024 * 1024 + 1][..]).is_err());
        assert_eq!(super::read_bounded(&b"hello"[..]).unwrap(), b"hello");
    }
}
