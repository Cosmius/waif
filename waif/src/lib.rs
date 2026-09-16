use std::error::Error;

use clap::Parser;

mod artifact;
mod commands;
mod macro_processor;
mod parser;
mod protocol;
mod schema;
mod schemas {
    pub mod goal;
    pub mod plan;
    pub mod review;
    pub mod step;
}
#[cfg(test)]
mod test_support;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn run() -> Result<()> {
    Cli::parse().command.run()
}

#[derive(Parser)]
#[command(
    name = "waif",
    disable_help_subcommand = true,
    override_usage = "waif <check|fill|goal|inspect|new|switch> [ARGUMENTS]",
    about = "Deterministic helpers for the Waif workflow",
    long_about = "Inspect and update mechanical Waif workflow state. Goal content, \
                  semantic review, and human acceptance remain outside the CLI.",
    after_help = "Examples:\n  waif inspect\n  waif new add-session-timeout\n  \
                  waif switch 20260916-1-add-session-timeout\n  waif check \
                  .waif/tasks/current/goal.md\n  waif fill \
                  .waif/tasks/current/goal.md\n  waif goal accept\n\nUse `waif \
                  <COMMAND> --help` for command details."
)]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,
}
