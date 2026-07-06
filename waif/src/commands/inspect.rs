use std::error::Error;
use std::path::Path;

use clap::Args;

use super::relative_path;
use crate::protocol::{Task, WorkflowDir};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
pub struct InspectCommand {}

impl InspectCommand {
    pub fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
        println!(
            "Workflow dir:       {}",
            relative_path(&workspace_dir, &workflow_dir.path)
        );
        match workflow_dir.current_task()? {
            Some(task) => print_current_task(&workspace_dir, task)?,
            None => println!("Current task:       none"),
        }
        Ok(())
    }
}

fn print_current_task(workspace_dir: &Path, task: Task) -> Result<()> {
    let task_name = task.name()?;

    println!("Current task:       {task_name}");
    println!(
        "Current task dir:   {}",
        relative_path(workspace_dir, &task.path)
    );
    println!("Goal:");
    match task.goal()? {
        Some(goal) => {
            println!(
                "  Path:             {}",
                relative_path(workspace_dir, goal.path())
            );
            println!(
                "  Status:           {}",
                goal.metadata_value("Status").unwrap_or("BAD VALUE")
            );
        }
        None => {
            println!("  not created yet");
        }
    }

    Ok(())
}
