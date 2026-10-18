mod banner;
mod cli;
mod commands;
mod config;
mod history;
mod node;
mod repl;

use clap::Parser;
use cli::Cli;
use colored::Colorize;
use std::io::IsTerminal;

fn main() {
    enable_ansi();

    let args = Cli::parse();

    if args.no_color || std::env::var_os("NO_COLOR").is_some() || !std::io::stdout().is_terminal() {
        colored::control::set_override(false);
    }

    match args.command {
        // No subcommand → drop into the interactive Hecate shell.
        None if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() => repl::run(),
        None => {
            use clap::CommandFactory;
            let _ = Cli::command().print_help();
            println!();
        }

        // A subcommand was given → run it once and exit (good for scripting).
        Some(command) => {
            if let Err(err) = commands::dispatch(command) {
                if args.debug {
                    eprintln!("{} {err:#}", "error:".red().bold());
                } else {
                    eprintln!("{} {}", "error:".red().bold(), err);
                }
                std::process::exit(1);
            }
        }
    }
}

/// Enable ANSI escape handling on legacy Windows consoles so colors render
/// instead of printing raw `←[1;95m` sequences. No-op everywhere else.
#[cfg(windows)]
fn enable_ansi() {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        STD_OUTPUT_HANDLE,
    };
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() {
            return;
        }
        let mut mode: u32 = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

#[cfg(not(windows))]
fn enable_ansi() {}
