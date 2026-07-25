use std::{env, path::PathBuf, process};

use anyhow::{bail, Context, Result};
use tidy_core::Vault;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());

    match command.as_str() {
        "init" => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let summary = Vault::initialize(&path)
                .with_context(|| format!("failed to initialize vault at {}", path.display()))?;

            if summary.created {
                println!("Created Tidy vault at {}", summary.path.display());
            } else {
                println!("Opened existing Tidy vault at {}", summary.path.display());
            }
            println!("Index: {}", summary.database_path.display());
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            bail!("unknown command `{other}`\n\n{}", HELP);
        }
    }
}

fn print_help() {
    print!("{HELP}");
}

const HELP: &str = "\
tidy — local-first fetch engine and vault tools

USAGE:
    tidy <COMMAND>

COMMANDS:
    init [PATH]    Create or open a Tidy vault (default: current directory)
    help           Show this help

Note: discover / fetch / reindex arrive in later milestones.
";
