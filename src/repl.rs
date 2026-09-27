//! Interactive Hecate shell. Entered when `hecate` is run with no subcommand.

use std::io::{self, BufRead, Write};

use clap::Parser;
use colored::Colorize;

use crate::cli::Cli;
use crate::{banner, commands};

pub fn run() {
    banner::print_banner();
    banner::welcome();
    println!(
        "  {}",
        "Type help for commands, exit to leave.".white()
    );
    println!();

    let stdin = io::stdin();

    loop {
        print!("{} ", banner::brand("hecate›"));
        let _ = io::stdout().flush();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                // EOF (Ctrl-Z on Windows, Ctrl-D elsewhere).
                println!();
                break;
            }
            Ok(_) => {}
            Err(_) => break,
        }

        // Strip a stray UTF-8 BOM (e.g. when input is piped) before trimming.
        let line = line.trim_start_matches('\u{feff}').trim();
        if line.is_empty() {
            continue;
        }

        // Shell built-ins.
        match line {
            "exit" | "quit" | ":q" => {
                println!("{}", "Leaving the mesh. Stay private.".white());
                break;
            }
            "clear" | "cls" => {
                print!("\x1b[2J\x1b[H");
                let _ = io::stdout().flush();
                banner::print_banner();
                continue;
            }
            _ => {}
        }

        let mut tokens = match tokenize(line) {
            Ok(tokens) => tokens,
            Err(err) => {
                eprintln!("{} {}", "error:".red().bold(), err);
                continue;
            }
        };

        // Be forgiving if the user habitually types the full `hecate …`
        // command inside the shell — drop a leading program name.
        if tokens
            .first()
            .is_some_and(|t| t.eq_ignore_ascii_case("hecate"))
        {
            tokens.remove(0);
        }
        if tokens.is_empty() {
            continue;
        }

        // Reuse the same clap parser; prepend the program name it expects.
        let mut argv = Vec::with_capacity(tokens.len() + 1);
        argv.push("hecate".to_string());
        argv.extend(tokens);

        match Cli::try_parse_from(&argv) {
            Ok(parsed) => {
                if let Some(command) = parsed.command {
                    if let Err(err) = commands::dispatch(command) {
                        eprintln!("{} {}", "error:".red().bold(), err);
                    }
                }
            }
            Err(err) => {
                // clap renders help text and usage errors here.
                print!("{err}");
            }
        }
        println!();
    }
}

/// Split a line into arguments, honoring single and double quotes so messages
/// and seed phrases with spaces work: `send 0xabc "hello world"`.
fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut has_token = false;
    let mut in_single = false;
    let mut in_double = false;

    for ch in input.chars() {
        match ch {
            // Straight or “smart” single quotes.
            '\'' | '\u{2018}' | '\u{2019}' if !in_double => {
                in_single = !in_single;
                has_token = true;
            }
            // Straight or “smart” double quotes.
            '"' | '\u{201C}' | '\u{201D}' if !in_single => {
                in_double = !in_double;
                has_token = true;
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if has_token {
                    tokens.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            c => {
                current.push(c);
                has_token = true;
            }
        }
    }

    if in_single || in_double {
        return Err("unterminated quote".to_string());
    }
    if has_token {
        tokens.push(current);
    }
    Ok(tokens)
}
