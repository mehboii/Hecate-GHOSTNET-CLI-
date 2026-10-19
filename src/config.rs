//! Non-secret Hecate configuration. Seed phrases deliberately never live here.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "config_version")]
    pub version: u8,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub identities: BTreeMap<String, IdentityLabel>,
    #[serde(default)]
    pub trusted_peers: BTreeMap<String, TrustedPeer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityLabel {
    pub node_id: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub node_id: String,
    pub public_key: String,
    pub created_at: u64,
}
fn config_version() -> u8 {
    1
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            endpoint: None,
            identities: BTreeMap::new(),
            trusted_peers: BTreeMap::new(),
        }
    }
}

impl Config {
    fn validate(&self) -> Result<()> {
        // Accept schema 0 written by older CLI versions, upgrade on next save.
        if self.version > 1 {
            anyhow::bail!(
                "unsupported configuration version {}; update the CLI",
                self.version
            );
        }
        if let Some(endpoint) = &self.endpoint {
            validate_endpoint(endpoint)?;
        }
        for name in self.identities.keys() {
            validate_label(name)?;
        }
        for (name, peer) in &self.trusted_peers {
            validate_label(name)?;
            validate_node_id(&peer.node_id)?;
            validate_public_key(&peer.public_key)?;
        }
        Ok(())
    }
}

pub fn data_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("GHOSTNET_HOME") {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            anyhow::bail!("GHOSTNET_HOME must be an absolute directory path");
        }
        return Ok(path);
    }
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
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let config: Config = serde_json::from_str(&raw)
        .with_context(|| format!("parsing {}; repair or restore this file", path.display()))?;
    config
        .validate()
        .with_context(|| format!("invalid configuration in {}", path.display()))?;
    Ok(config)
}
pub fn save(config: &Config) -> Result<()> {
    use std::io::Write;
    config.validate()?;
    let path = path()?;
    let dir = data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let temporary = dir.join(format!(
        "config.{}.{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    let result = (|| -> Result<()> {
        let mut value = serde_json::to_value(config)?;
        value["version"] = 1.into();
        file.write_all(&serde_json::to_vec_pretty(&value)?)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result?;
    Ok(())
}
pub fn endpoint(override_endpoint: Option<String>) -> Result<Option<String>> {
    let saved = load()?;
    let endpoint = override_endpoint.or(saved.endpoint);
    if let Some(ref value) = endpoint {
        validate_endpoint(value)?;
    }
    Ok(endpoint)
}
pub fn validate_endpoint(value: &str) -> Result<()> {
    let host = value
        .strip_prefix("wss://")
        .ok_or_else(|| anyhow!("endpoint must start with wss://"))?;
    let authority = host.split(['/', '?']).next().unwrap_or("");
    let valid_authority = if let Some(ipv6) = authority.strip_prefix('[') {
        ipv6.split_once(']').is_some_and(|(ip, suffix)| {
            ip.parse::<std::net::Ipv6Addr>().is_ok() && valid_port_suffix(suffix)
        })
    } else {
        let (hostname, suffix) = authority
            .find(':')
            .map_or((authority, ""), |index| authority.split_at(index));
        !hostname.is_empty()
            && hostname.split('.').all(|part| {
                !part.is_empty()
                    && !part.starts_with('-')
                    && !part.ends_with('-')
                    && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            })
            && valid_port_suffix(suffix)
    };
    if !valid_authority
        || host.is_empty()
        || host.starts_with('/')
        || host.contains('#')
        || host.contains('\\')
        || authority.contains('@')
        || host.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        anyhow::bail!("endpoint must be an absolute wss:// URL without a fragment");
    }
    Ok(())
}
fn valid_port_suffix(value: &str) -> bool {
    value.is_empty()
        || value.strip_prefix(':').is_some_and(|port| {
            !port.is_empty()
                && port.bytes().all(|c| c.is_ascii_digit())
                && port.parse::<u16>().is_ok_and(|p| p > 0)
        })
}

pub fn validate_label(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.chars().any(char::is_control) {
        anyhow::bail!("label must be nonempty and contain no control characters");
    }
    Ok(())
}

pub fn validate_node_id(node_id: &str) -> Result<()> {
    if node_id.len() != 66
        || !node_id.starts_with("0x")
        || !node_id[2..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        anyhow::bail!("peer node ID must be 0x followed by 64 lowercase hex characters");
    }
    Ok(())
}

pub fn validate_public_key(public_key: &str) -> Result<()> {
    if public_key.len() != 64
        || !public_key
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        anyhow::bail!("peer public key must be 64 lowercase hex characters");
    }
    Ok(())
}

pub fn trusted_peer(recipient: &str) -> Result<TrustedPeer> {
    let config = load()?;
    if validate_node_id(recipient).is_ok() {
        let mut matches = config
            .trusted_peers
            .values()
            .filter(|peer| peer.node_id == recipient);
        let peer = matches.next().cloned().ok_or_else(|| {
            anyhow!(
                "peer {recipient:?} is not trusted; register its verified node ID and public key with `hecate peer add`"
            )
        })?;
        if matches.next().is_some() {
            anyhow::bail!("multiple trusted peer records use node ID {recipient}");
        }
        return Ok(peer);
    }
    config.trusted_peers.get(recipient).cloned().ok_or_else(|| {
        anyhow!(
            "peer {recipient:?} is not trusted; register its verified node ID and public key with `hecate peer add`"
        )
    })
}
pub fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn label(name: String, node_id: String) -> Result<()> {
    validate_label(&name)?;
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
    use super::{validate_endpoint, validate_node_id, validate_public_key};
    #[test]
    fn only_secure_endpoints_are_accepted() {
        assert!(validate_endpoint("wss://relay.example/path").is_ok());
        assert!(validate_endpoint("ws://relay.example").is_err());
        assert!(validate_endpoint("wss:///missing-host").is_err());
        for endpoint in [
            "wss://[::1]:443/path",
            "wss://relay.example:65535/path?x=y",
            "wss://localhost",
        ] {
            assert!(validate_endpoint(endpoint).is_ok(), "{endpoint}");
        }
        for endpoint in [
            "wss://[bad]:443",
            "wss://host:",
            "wss://host:65536",
            "wss://host:0",
            "wss://host:abc",
            "wss://user@host",
            "wss://host\n",
            "wss://?foo",
        ] {
            assert!(validate_endpoint(endpoint).is_err(), "{endpoint}");
        }
    }

    #[test]
    fn trusted_peer_identifiers_are_strict_lowercase_hex() {
        let node_id = format!("0x{}", "ab".repeat(32));
        let public_key = "cd".repeat(32);
        assert!(validate_node_id(&node_id).is_ok());
        assert!(validate_public_key(&public_key).is_ok());
        assert!(validate_node_id(&node_id.to_uppercase()).is_err());
        assert!(validate_node_id("0x1234").is_err());
        assert!(validate_public_key(&public_key.to_uppercase()).is_err());
        assert!(validate_public_key("not-a-key").is_err());
    }
}
