//! Non-secret Hecate configuration. Seed phrases deliberately never live here.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "config_version")]
    pub version: u8,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub identities: BTreeMap<String, IdentityLabel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityLabel {
    pub node_id: String,
    pub created_at: u64,
}
fn config_version() -> u8 {
    1
}

pub fn data_dir() -> Result<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .map(|p| p.join(".ghostnet-cli"))
        .ok_or_else(|| anyhow!("could not determine your home directory"))
}
pub fn path() -> Result<PathBuf> {
    Ok(data_dir()?.join("config.json"))
}
pub fn load() -> Result<Config> {
    let path = path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}
pub fn save(config: &Config) -> Result<()> {
    let path = path()?;
    let dir = data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let temporary = dir.join("config.json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(config)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}
pub fn endpoint(override_endpoint: Option<String>) -> Result<Option<String>> {
    let endpoint = override_endpoint.or(load()?.endpoint);
    if let Some(ref value) = endpoint {
        validate_endpoint(value)?;
    }
    Ok(endpoint)
}
pub fn validate_endpoint(value: &str) -> Result<()> {
    let host = value
        .strip_prefix("wss://")
        .ok_or_else(|| anyhow!("endpoint must start with wss://"))?;
    if host.is_empty()
        || host.starts_with('/')
        || host.contains('#')
        || host.chars().any(char::is_whitespace)
    {
        anyhow::bail!("endpoint must be an absolute wss:// URL without a fragment");
    }
    Ok(())
}
pub fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn label(name: String, node_id: String) -> Result<()> {
    if name.trim().is_empty() {
        anyhow::bail!("identity label cannot be empty");
    }
    let mut config = load()?;
    config.identities.insert(
        name,
        IdentityLabel {
            node_id,
            created_at: unix_time(),
        },
    );
    save(&config)
}
#[cfg(test)]
mod tests {
    use super::validate_endpoint;
    #[test]
    fn only_secure_endpoints_are_accepted() {
        assert!(validate_endpoint("wss://relay.example/path").is_ok());
        assert!(validate_endpoint("ws://relay.example").is_err());
        assert!(validate_endpoint("wss:///missing-host").is_err());
    }
}
