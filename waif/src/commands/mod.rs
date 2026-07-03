use std::error::Error;

use clap::Subcommand;

mod inspect;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Subcommand)]
pub enum Command {
    /// Inspect the current workflow task.
    Inspect(inspect::InspectCommand),
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Inspect(command) => command.run(),
        }
    }
}
