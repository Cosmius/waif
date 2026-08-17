use std::error::Error;
use std::path::Path;

use clap::Subcommand;

mod check;
mod fill;
mod inspect;
mod new;
mod switch;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Subcommand)]
pub enum Command {
    /// Check task artifacts for format errors.
    Check(check::CheckCommand),
    /// Replace known macros in a text file.
    Fill(fill::FillCommand),
    /// Inspect the current workflow task.
    Inspect(inspect::InspectCommand),
    /// Create a new workflow task and make it current.
    New(new::NewCommand),
    /// Select an existing workflow task.
    Switch(switch::SwitchCommand),
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Check(command) => command.run(),
            Self::Fill(command) => command.run(),
            Self::Inspect(command) => command.run(),
            Self::New(command) => command.run(),
            Self::Switch(command) => command.run(),
        }
    }
}

fn relative_path(workspace_dir: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}
