use std::error::Error;
use std::path::PathBuf;

use clap::Args;

use super::{print_tabular_row, relative_path};
use crate::protocol::{Initialisation, WorkflowDir};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
#[command(after_help = indoc::indoc! {"
    Examples:
      waif init
      waif init --link /project/.waif/library"
})]
pub struct InitCommand {
    /// Link to this library instead of creating an independent one.
    #[arg(long, value_name = "TARGET")]
    link: Option<PathBuf>,
}

impl InitCommand {
    pub fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
        let outcome = workflow_dir.initialise(self.link.as_deref())?;
        print_tabular_row(
            "Workflow dir:",
            relative_path(&workspace_dir, &workflow_dir.path),
        );
        print_tabular_row(
            "Library:",
            relative_path(&workspace_dir, &workflow_dir.library_dir()),
        );
        match outcome {
            Initialisation::Skipped => println!("Library already exists; setup skipped"),
            Initialisation::Created(Some(error)) => {
                print_tabular_row("Workflow git:", format!("not initialised ({error})"));
            }
            Initialisation::Created(None) => (),
        }
        Ok(())
    }
}
