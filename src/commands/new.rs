use std::error::Error;

use clap::Args;

use super::{print_tabular_row, relative_path};
use crate::protocol::WorkflowDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
#[command(after_help = indoc::indoc! {"
    Example:
      waif new deterministic-goal-drafting"
})]
pub struct NewCommand {
    /// Slug containing lowercase letters, decimal digits, and hyphens.
    task_slug: String,
}

impl NewCommand {
    pub fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
        let task = workflow_dir.create_task(&self.task_slug)?;
        workflow_dir.switch_task(&task)?;

        print_tabular_row(
            "Workflow dir:",
            relative_path(&workspace_dir, &workflow_dir.path),
        );
        print_tabular_row("Current task:", task.name()?);
        print_tabular_row(
            "Current task dir:",
            relative_path(&workspace_dir, &task.path),
        );
        Ok(())
    }
}
