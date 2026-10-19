use anyhow::{Context, Result};
use colored::Colorize;
use std::process::Command;

use crate::node;

pub fn run(sdk_version: String) -> Result<()> {
    validate_sdk_version(&sdk_version)?;
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
        format!("npm install @n11x/atlas@{sdk_version}").bold()
    );
    println!();

    let install_args = npm_install_args(&sdk_version);
    let output = node::capture(
        Command::new(node::npm_bin())
            .args(&install_args)
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

fn npm_install_args(version: &str) -> [String; 6] {
    [
        "install".into(),
        "--save-exact".into(),
        "--ignore-scripts".into(),
        "--no-audit".into(),
        "--no-fund".into(),
        format!("@n11x/atlas@{version}"),
    ]
}

fn validate_sdk_version(version: &str) -> Result<()> {
    semver::Version::parse(version)
        .map(|_| ())
        .map_err(|_| anyhow::anyhow!("SDK version must be an exact semantic version such as 1.2.3"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn sdk_version_must_be_exact() {
        for valid in ["1.2.3", "0.1.0-beta.1", "1.2.3+build.7"] {
            assert!(super::validate_sdk_version(valid).is_ok(), "{valid}");
        }
        for invalid in [
            "latest",
            "^1.2.3",
            "1.2",
            "1.2.3-",
            "01.2.3",
            "1.2.3-alpha..1",
            "1.2.3 || evil",
            "file:../sdk",
        ] {
            assert!(super::validate_sdk_version(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn npm_install_is_exact_and_disables_lifecycle_scripts() {
        let args = super::npm_install_args("1.2.3");
        assert!(args.iter().any(|arg| arg == "--save-exact"));
        assert!(args.iter().any(|arg| arg == "--ignore-scripts"));
        assert_eq!(args.last().unwrap(), "@n11x/atlas@1.2.3");
        assert!(!args.iter().any(|arg| arg == "latest"));
    }
}
