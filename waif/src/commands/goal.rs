use std::error::Error;
use std::fmt;

use clap::{Args, Subcommand};

use super::{print_tabular_row, relative_path};
use crate::protocol::WorkflowDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Subcommand)]
#[command(
    disable_help_subcommand = true,
    override_usage = "waif goal <accept>",
    after_help = "Example:\n  waif goal accept\n\nAcceptance requires explicit human \
                  approval, a GOOD drafting or amending goal, and no blocking open \
                  questions."
)]
pub enum GoalCommand {
    /// Accept a complete current goal after explicit human approval.
    #[command(
        long_about = "Accept the current drafting or amending goal after explicit human \
                      approval. The goal must be GOOD and have no blocking open questions. \
                      Only Status and Updated are changed; failed preconditions leave it \
                      unchanged."
    )]
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
#[command(after_help = "Example:\n  waif goal accept")]
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
