use std::error::Error;

use clap::Parser;

mod artifact;
mod commands;
mod goal;
mod parser;
mod plan;
mod protocol;
mod schema;
#[cfg(test)]
mod test_support;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn run() -> Result<()> {
    Cli::parse().command.run()
}

#[derive(Parser)]
#[command(name = "waif", about = "Deterministic helpers for the Waif workflow")]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,
}
