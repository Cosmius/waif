use std::error::Error;
use std::fmt;

use clap::{Args, Subcommand};

use super::{print_tabular_row, relative_path};
use crate::protocol::WorkflowDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Subcommand)]
#[command(
    disable_help_subcommand = true,
    override_usage = "waif plan <accept>",
    after_help = indoc::indoc! {"
        Example:
          waif plan accept

        Acceptance requires explicit human approval, a GOOD plan and accepted goal, and no blocking open questions. The recorded task branch is established before metadata updates. Repeated acceptance preserves accepted-plan bytes."
    }
)]
pub enum PlanCommand {
    /// Accept the current plan or restore its branch after explicit human approval.
    #[command(
        long_about = "Accept or reaccept the current plan after explicit human approval. \
                      Requires a GOOD plan, a GOOD accepted goal, and no blocking open questions. \
                      Establish the recorded task branch before updating Status and Updated. \
                      Already accepted plans preserve every byte, including on branch restoration. \
                      Git failures leave metadata unchanged; a later metadata failure reports \
                      completed branch setup for agent repair."
    )]
    Accept(AcceptCommand),
}

impl PlanCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Accept(command) => command.run(),
        }
    }
}

#[derive(Args)]
#[command(after_help = indoc::indoc! {"
    Example:
      waif plan accept"
})]
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
        let plan_path = task.path.join("plan.md");
        let workflow_path = relative_path(&workspace_dir, &workflow_dir.path);
        let plan_path_display = relative_path(&workspace_dir, &plan_path);

        if let Err(error) = task.accept_plan(&workspace_dir) {
            return Err(Box::new(AcceptContextError::Rejected {
                workflow_path,
                task_name,
                plan_path: plan_path_display,
                source: error,
            }));
        }

        print_tabular_row(
            "Workflow dir:",
            relative_path(&workspace_dir, &workflow_dir.path),
        );
        print_tabular_row("Current task:", task_name);
        print_tabular_row("Plan:", relative_path(&workspace_dir, &plan_path));
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
        plan_path: String,
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
                plan_path,
                source,
            } => write!(
                formatter,
                "Workflow dir: {workflow_path}\nCurrent task: {task_name}\nPlan: {plan_path}\n{source}"
            ),
        }
    }
}

impl Error for AcceptContextError {}
