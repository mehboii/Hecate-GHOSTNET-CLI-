pub mod config_cmd;
pub mod doctor;
pub mod history_cmd;
pub mod identity;
pub mod info;
pub mod listen;
pub mod send;
pub mod setup;
pub mod status;

use crate::cli::Commands;
use anyhow::Result;

/// Run a single parsed command. Shared by one-shot mode and the interactive shell.
pub fn dispatch(command: Commands) -> Result<()> {
    match command {
        Commands::Setup { sdk_version } => setup::run(sdk_version),
        Commands::Info => info::run(),
        Commands::Identity { action } => match action {
            crate::cli::IdentityAction::List => identity::list(),
            crate::cli::IdentityAction::Remove { name } => identity::remove(name),
            action => identity::run(action),
        },
        Commands::Config { action } => config_cmd::run(action),
        Commands::Doctor => doctor::run(),
        Commands::Status => status::run(),
        Commands::History { limit, json } => history_cmd::run(limit, json),
        Commands::Send {
            peer,
            message,
            endpoint,
        } => send::run(peer, message, endpoint),
        Commands::Listen { endpoint } => listen::run(endpoint),
    }
}
