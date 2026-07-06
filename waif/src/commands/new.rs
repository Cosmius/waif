use std::error::Error;

use clap::Args;

use super::relative_path;
use crate::protocol::WorkflowDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
pub struct NewCommand {
    /// Lowercase-hyphenated task slug.
    task_slug: String,
}

impl NewCommand {
    pub fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
        let mut git_error = None;
        if !workflow_dir.exists() {
            git_error = workflow_dir.initialise()?;
        }
        let task = workflow_dir.create_task(&self.task_slug)?;
        workflow_dir.switch_task(&task)?;

        println!(
            "Workflow dir:       {}",
            relative_path(&workspace_dir, &workflow_dir.path)
        );
        println!("Current task:       {}", task.name()?);
        println!(
            "Current task dir:   {}",
            relative_path(&workspace_dir, &task.path)
        );
        if let Some(error) = git_error {
            println!("Workflow git:       not initialised ({error})");
        }
        Ok(())
    }
}
