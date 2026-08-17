use std::error::Error;

use clap::Args;

use super::relative_path;
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

        println!("Current task:       {}", task.name()?);
        println!(
            "Current task dir:   {}",
            relative_path(&workspace_dir, &task.path)
        );
        println!(
            "Current reference:  {} -> ./{}",
            relative_path(&workspace_dir, &workflow_dir.current_link()),
            task.name()?
        );
        Ok(())
    }
}
