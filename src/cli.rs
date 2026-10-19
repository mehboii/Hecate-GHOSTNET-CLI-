use clap::{Parser, Subcommand};

/// Hecate — encrypted mesh network client by the N11X Collective.
#[derive(Parser)]
#[command(
    name = "hecate",
    version,
    about = "Hecate — encrypted mesh client · N11X Collective",
    long_about = None,
    propagate_version = true,
    after_help = "Examples:\n  hecate setup --sdk-version 0.2.0\n  hecate identity create --name laptop\n  hecate peer add friend 0xNODE_ID PUBLIC_KEY\n  hecate send friend \"hello\"\n  hecate config show\n  hecate history --limit 10\n\nSet GHOSTNET_SEED for identity load/send/listen. Run without a command in a terminal for the shell."
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
    /// Install an exact Atlas SDK version with lifecycle scripts disabled.
    Setup {
        /// Exact SDK version to install (for example, 1.2.3).
        #[arg(long, value_name = "VERSION")]
        sdk_version: String,
    },

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

    /// Manage trusted Atlas peer public keys.
    Peer {
        #[command(subcommand)]
        action: PeerAction,
    },

    /// Send an end-to-end encrypted message to a peer.
    Send {
        /// Peer node ID (e.g. 0x...).
        peer: String,

        /// Message text (maximum 64 KiB UTF-8).
        message: String,

        /// Override the relay endpoint (wss:// only).
        #[arg(long)]
        endpoint: Option<String>,
    },

    /// Connect to the mesh and stream incoming messages.
    Listen {
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

    /// Restore an identity using GHOSTNET_SEED.
    Load {
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
pub enum PeerAction {
    /// Register a peer key verified through a trusted out-of-band channel.
    Add {
        /// Local label used by `hecate send`.
        name: String,
        /// Expected Atlas node ID (0x followed by 64 lowercase hex characters).
        node_id: String,
        /// Ed25519 public key (64 lowercase hex characters).
        public_key: String,
    },
    /// List trusted peer labels, node IDs, and public keys.
    List,
    /// Remove a locally trusted peer registration.
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
