use std::error::Error;
use std::fmt;

use clap::{Args, Subcommand};

use super::{print_tabular_row, relative_path};
use crate::protocol::WorkflowDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Subcommand)]
#[command(
    disable_help_subcommand = true,
    override_usage = "waif step <accept>",
    after_help = indoc::indoc! {"
        Example:
          waif step accept

        Acceptance acts on the step directory with the greatest number. It
        requires explicit human approval, an accepted goal and plan, a GOOD
        step without open questions whose coverage names pending plan items,
        and the recorded task branch checked out. Already accepted steps
        return an error."
    }
)]
pub enum StepCommand {
    /// Accept the current drafting or amending step after explicit human
    /// approval.
    #[command(long_about = indoc::indoc! {"
        Accept the current step, the step directory with the greatest
        number, after explicit human approval. Requires an accepted goal and
        plan, a GOOD step without blocking open questions, one coverage entry
        per plan item, each naming an existing pending plan item, and the
        recorded task branch checked out. Only Status and Updated are
        changed; failed preconditions leave every file unchanged."
    })]
    Accept(AcceptCommand),
}

impl StepCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Accept(command) => command.run(),
        }
    }
}

#[derive(Args)]
#[command(after_help = indoc::indoc! {"
    Example:
      waif step accept"
})]
pub struct AcceptCommand {}

impl AcceptCommand {
    fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
        let workflow_path = relative_path(&workspace_dir, &workflow_dir.path);
        let task = match workflow_dir.current_task() {
            Ok(Some(task)) => task,
            Ok(None) => {
                return Err(Box::new(AcceptContextError::NoCurrentTask {
                    workflow_path,
                }));
            }
            Err(error) => {
                return Err(Box::new(AcceptContextError::Context {
                    workflow_path,
                    source: error,
                }));
            }
        };
        let task_name = task.name()?.to_owned();
        let step_path = match task.current_step_path() {
            Ok(path) => path,
            Err(error) => {
                let source = format!("Current task: {task_name}\n{error}");
                return Err(Box::new(AcceptContextError::Context {
                    workflow_path,
                    source: source.into(),
                }));
            }
        };
        let step_path_display = relative_path(&workspace_dir, &step_path);

        if let Err(error) = task.accept_step(&workspace_dir) {
            return Err(Box::new(AcceptContextError::Rejected {
                workflow_path,
                task_name,
                step_path: step_path_display,
                source: error,
            }));
        }

        print_tabular_row("Workflow dir:", workflow_path);
        print_tabular_row("Current task:", task_name);
        print_tabular_row("Step:", relative_path(&workspace_dir, &step_path));
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
        step_path: String,
        source: Box<dyn Error>,
    },
}

impl fmt::Display for AcceptContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCurrentTask { workflow_path } => write!(
                formatter,
                "Workflow dir: {workflow_path}\nCurrent task: none\n\
                 no current workflow task"
            ),
            Self::Context {
                workflow_path,
                source,
            } => write!(formatter, "Workflow dir: {workflow_path}\n{source}"),
            Self::Rejected {
                workflow_path,
                task_name,
                step_path,
                source,
            } => write!(
                formatter,
                "Workflow dir: {workflow_path}\nCurrent task: {task_name}\n\
                 Step: {step_path}\n{source}"
            ),
        }
    }
}

impl Error for AcceptContextError {}
