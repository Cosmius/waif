use std::error::Error;
use std::path::Path;

use clap::Subcommand;

mod inspect;
mod new;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Subcommand)]
pub enum Command {
    /// Inspect the current workflow task.
    Inspect(inspect::InspectCommand),
    /// Create a new workflow task and make it current.
    New(new::NewCommand),
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Inspect(command) => command.run(),
            Self::New(command) => command.run(),
        }
    }
}

fn relative_path(workspace_dir: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}
