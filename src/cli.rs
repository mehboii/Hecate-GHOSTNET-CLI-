use clap::{Parser, Subcommand};

/// GhostNet CLI — encrypted mesh network client by the N11X Collective.
#[derive(Parser)]
#[command(
    name = "ghostnet",
    version,
    about = "GhostNet CLI — encrypted mesh client · N11X Collective",
    long_about = None,
    propagate_version = true,
    after_help = "Examples:\n  ghostnet setup\n  ghostnet identity create --name laptop\n  ghostnet config show\n  ghostnet history --limit 10\n  ghostnet send PEER \"hello\"\n\nSet GHOSTNET_SEED for send/listen. Run without a command in a terminal for the shell."
)]
pub struct Cli {
    /// Show version (alias for --version).
    #[arg(short = 'v', action = clap::ArgAction::Version)]
    pub version_alias: (),

    /// Show the error context chain (never dumps environment variables).
    #[arg(long, global = true)]
    pub debug: bool,
    /// Disable colored output.
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install the GhostNet SDK bridge (runs `npm install @n11x/ghostnet-sdk`).
    Setup,

    /// Show CLI / SDK status and N11X Collective info.
    Info,

    /// Show local installation and configuration diagnostics.
    Doctor,

    /// Show the configured endpoint, identities, and SDK readiness.
    Status,

    /// Manage non-secret local CLI configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Print locally recorded message metadata (never message bodies).
    History {
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Emit a JSON array of message metadata.
        #[arg(long)]
        json: bool,
    },

    /// Create or restore a GhostNet identity.
    Identity {
        #[command(subcommand)]
        action: IdentityAction,
    },

    /// Send an end-to-end encrypted message to a peer.
    Send {
        /// Peer node ID (e.g. 0x...).
        peer: String,

        /// Message text (maximum 64 KiB UTF-8).
        message: String,

        /// Seed phrase to restore an identity. Prefer GHOSTNET_SEED to avoid argv/history exposure.
        #[arg(long)]
        seed: Option<String>,

        /// Override the relay endpoint (wss:// only).
        #[arg(long)]
        endpoint: Option<String>,
    },

    /// Connect to the mesh and stream incoming messages.
    Listen {
        /// Seed phrase to restore an identity. Prefer GHOSTNET_SEED to avoid argv/history exposure.
        #[arg(long)]
        seed: Option<String>,

        /// Override the relay endpoint (wss:// only).
        #[arg(long)]
        endpoint: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum IdentityAction {
    /// Generate a brand-new BIP-39 identity.
    Create {
        /// Save the public node ID under a local label. The seed is never stored.
        #[arg(long)]
        name: Option<String>,
    },

    /// Restore an identity from a 12-word seed phrase.
    ///
    /// Tip: set the GHOSTNET_SEED environment variable instead of passing the
    /// phrase as an argument, to keep it out of shell history and process lists.
    Load {
        /// The 12-word seed phrase (quote it). Omit to read from GHOSTNET_SEED.
        seed: Option<String>,
        /// Save the restored public node ID under a local label. The seed is never stored.
        #[arg(long)]
        name: Option<String>,
    },

    /// List locally saved public identity labels.
    List,

    /// Remove a local public identity label (not the network identity).
    Remove { name: String },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Print the current non-secret configuration.
    Show {
        /// Emit the non-secret configuration as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Save the default relay endpoint (must be wss://).
    SetEndpoint { endpoint: String },
    /// Clear the default endpoint and use the SDK default.
    ClearEndpoint,
}
