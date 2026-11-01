use std::error::Error;

use clap::Parser;

mod artifact;
mod commands;
mod git_repo;
mod macro_processor;
mod package;
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
    override_usage = "waif <check|fill|goal|init|inspect|new|package|plan|\
                      step|switch> [ARGUMENTS]",
    about = "Deterministic helpers for the Waif workflow",
    long_about = indoc::indoc! {"
        Inspect and update mechanical Waif workflow state. Artifact content,
        semantic review, and human acceptance remain outside the CLI."
    },
    after_help = indoc::indoc! {"
        Examples:
          waif init
          waif init --link /project/.waif/library
          waif inspect
          waif new add-session-timeout
          waif switch 20260916-1-add-session-timeout
          waif check .waif/current_task/goal.md
          waif fill .waif/current_task/goal.md
          waif goal accept
          waif plan accept
          waif step accept
          waif step commit -m \"Add session timeout\"
          waif step commit -F -
          waif package --target claude-plugin

        Use `waif <COMMAND> --help` for command details."
    }
)]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,
}
