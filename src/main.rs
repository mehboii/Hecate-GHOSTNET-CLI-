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

fn main() {
    enable_ansi();

    let args = Cli::parse();

    if args.no_color {
        colored::control::set_override(false);
    }

    match args.command {
        // No subcommand → drop into the interactive Hecate shell.
        None => repl::run(),

        // A subcommand was given → run it once and exit (good for scripting).
        Some(command) => {
            banner::print_banner();
            banner::welcome();
            if let Err(err) = commands::dispatch(command) {
                eprintln!("{} {}", "error:".red().bold(), err);
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
