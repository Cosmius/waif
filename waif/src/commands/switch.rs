use std::error::Error;

use clap::Args;

use super::{print_tabular_row, relative_path};
use crate::protocol::WorkflowDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
pub struct SwitchCommand {
    /// Complete workflow task-directory name.
    task_name: String,
}

impl SwitchCommand {
    pub fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
        let task = workflow_dir.task(&self.task_name)?;
        workflow_dir.switch_task(&task)?;

        print_tabular_row("Current task:", task.name()?);
        print_tabular_row(
            "Current task dir:",
            relative_path(&workspace_dir, &task.path),
        );
        print_tabular_row(
            "Current reference:",
            format!(
                "{} -> ./{}",
                relative_path(&workspace_dir, &workflow_dir.current_link()),
                task.name()?
            ),
        );
        Ok(())
    }
}
