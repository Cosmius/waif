use std::error::Error;
use std::fmt;

use clap::{Args, Subcommand};

use super::{print_tabular_row, relative_path};
use crate::protocol::WorkflowDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Subcommand)]
pub enum GoalCommand {
    /// Accept the current goal after human approval.
    Accept(AcceptCommand),
}

impl GoalCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Accept(command) => command.run(),
        }
    }
}

#[derive(Args)]
pub struct AcceptCommand {}

impl AcceptCommand {
    fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
        let task = match workflow_dir.current_task() {
            Ok(Some(task)) => task,
            Ok(None) => {
                return Err(Box::new(AcceptContextError::NoCurrentTask {
                    workflow_path: relative_path(&workspace_dir, &workflow_dir.path),
                }));
            }
            Err(error) => {
                return Err(Box::new(AcceptContextError::Context {
                    workflow_path: relative_path(&workspace_dir, &workflow_dir.path),
                    source: error,
                }));
            }
        };
        let task_name = task.name()?.to_owned();
        let goal_path = task.path.join("goal.md");
        let workflow_path = relative_path(&workspace_dir, &workflow_dir.path);
        let goal_path_display = relative_path(&workspace_dir, &goal_path);

        if let Err(error) = task.accept_goal() {
            return Err(Box::new(AcceptContextError::Rejected {
                workflow_path,
                task_name,
                goal_path: goal_path_display,
                source: error,
            }));
        }

        print_tabular_row(
            "Workflow dir:",
            relative_path(&workspace_dir, &workflow_dir.path),
        );
        print_tabular_row("Current task:", task_name);
        print_tabular_row("Goal:", relative_path(&workspace_dir, &goal_path));
        print_tabular_row("Status:", "accepted");
        Ok(())
    }
}

#[derive(Debug)]
enum AcceptContextError {
    NoCurrentTask {
        workflow_path: String,
    },
    Context {
        workflow_path: String,
        source: Box<dyn Error>,
    },
    Rejected {
        workflow_path: String,
        task_name: String,
        goal_path: String,
        source: Box<dyn Error>,
    },
}

impl fmt::Display for AcceptContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCurrentTask { workflow_path } => write!(
                formatter,
                "Workflow dir: {workflow_path}\nCurrent task: none\nno current workflow task"
            ),
            Self::Context {
                workflow_path,
                source,
            } => write!(formatter, "Workflow dir: {workflow_path}\n{source}"),
            Self::Rejected {
                workflow_path,
                task_name,
                goal_path,
                source,
            } => write!(
                formatter,
                "Workflow dir: {workflow_path}\nCurrent task: {task_name}\nGoal: {goal_path}\n{source}"
            ),
        }
    }
}

impl Error for AcceptContextError {}
