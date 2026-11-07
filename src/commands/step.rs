use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};

use super::{print_tabular_row, relative_path};
use crate::protocol::{Task, WorkflowDir};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Subcommand)]
#[command(
    disable_help_subcommand = true,
    override_usage = "waif step <accept|commit>",
    after_help = indoc::indoc! {"
        Examples:
          waif step accept
          waif step commit -m \"Add session timeout\"
          waif step commit -F -

        Acceptance acts on the step directory with the greatest number. It
        requires explicit human approval, an accepted goal and plan, a GOOD
        step without open questions whose coverage names pending plan items,
        and the recorded task branch checked out. Already accepted steps
        return an error.

        Commit requires the same preconditions on an accepted step, commits
        exactly the staged changes, then marks the step done with the commit
        hash and marks plan items with complete coverage done."
    }
)]
pub enum StepCommand {
    /// Accept the current drafting or amending step after explicit human
    /// approval.
    #[command(long_about = indoc::indoc! {"
        Accept the current step, the step directory with the greatest
        number, after explicit human approval. Requires an accepted goal and
        plan, a GOOD step without open questions whose coverage entries
        each name an existing pending plan item, and the recorded task
        branch checked out. Only Status and Updated are changed; failed
        preconditions leave every file unchanged."
    })]
    Accept(AcceptCommand),
    /// Commit the staged changes of the current accepted step.
    #[command(long_about = indoc::indoc! {"
        Commit exactly the staged changes for the current accepted step,
        taking the message from -m MESSAGE or -F FILE, where FILE - reads
        standard input. Requires the step acceptance preconditions, a
        non-empty message, staged changes, and no unmerged index entries;
        failed preconditions leave every file and the repository unchanged.
        After the commit, the step records the commit hash and becomes done,
        and plan items with complete coverage become done. A failure after
        the commit reports the hash and the artifact that was not updated."
    })]
    Commit(CommitCommand),
}

impl StepCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Accept(command) => command.run(),
            Self::Commit(command) => command.run(),
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
        let context = StepContext::resolve()?;
        if let Err(error) = context.task.accept_step(&context.workspace_dir) {
            return Err(context.rejected(error));
        }
        context.print();
        print_tabular_row("Status:", "accepted");
        Ok(())
    }
}

#[derive(Args)]
#[group(required = true, multiple = false)]
#[command(after_help = indoc::indoc! {"
    Examples:
      waif step commit -m \"Add session timeout\"
      waif step commit -F message.txt
      waif step commit -F -"
})]
pub struct CommitCommand {
    /// Use MESSAGE as the commit message.
    #[arg(short = 'm', long, value_name = "MESSAGE")]
    message: Option<String>,
    /// Read the commit message from FILE; `-` reads standard input.
    #[arg(short = 'F', long, value_name = "FILE")]
    file: Option<PathBuf>,
}

impl CommitCommand {
    fn run(self) -> Result<()> {
        let message = match (self.message, self.file) {
            (Some(message), _) => message,
            (None, Some(file)) => read_message(&file)?,
            (None, None) => unreachable!("clap requires -m or -F"),
        };
        let context = StepContext::resolve()?;
        let commit = match context.task.commit_step(&context.workspace_dir, &message) {
            Ok(commit) => commit,
            Err(error) => return Err(context.rejected(error)),
        };
        context.print();
        print_tabular_row("Commit:", &commit.hash);
        print_tabular_row("Status:", "done");
        let done = if commit.done_items.is_empty() {
            "none".to_owned()
        } else {
            commit.done_items.join(", ")
        };
        print_tabular_row("Plan items done:", done);
        Ok(())
    }
}

/// Read a commit message from FILE, or from standard input for `-`.
fn read_message(file: &Path) -> Result<String> {
    let mut message = String::new();
    let result = if file == Path::new("-") {
        io::stdin().read_to_string(&mut message).map(|_| ())
    } else {
        fs::read_to_string(file).map(|contents| message = contents)
    };
    result.map_err(|error| format!("{}: {error}", file.display()))?;
    Ok(message)
}

/// The workflow, task, and current step a step command acts on.
struct StepContext {
    workspace_dir: PathBuf,
    workflow_path: String,
    task: Task,
    task_name: String,
    step_path: PathBuf,
}

impl StepContext {
    fn resolve() -> Result<Self> {
        let workspace_dir = std::env::current_dir()?;
        let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
        let workflow_path = relative_path(&workspace_dir, &workflow_dir.path);
        let task = match workflow_dir.current_task() {
            Ok(Some(task)) => task,
            Ok(None) => {
                return Err(Box::new(StepContextError::NoCurrentTask { workflow_path }));
            }
            Err(error) => {
                return Err(Box::new(StepContextError::Context {
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
                return Err(Box::new(StepContextError::Context {
                    workflow_path,
                    source: source.into(),
                }));
            }
        };
        Ok(Self {
            workspace_dir,
            workflow_path,
            task,
            task_name,
            step_path,
        })
    }

    fn rejected(self, source: Box<dyn Error>) -> Box<dyn Error> {
        Box::new(StepContextError::Rejected {
            step_path: relative_path(&self.workspace_dir, &self.step_path),
            workflow_path: self.workflow_path,
            task_name: self.task_name,
            source,
        })
    }

    fn print(&self) {
        print_tabular_row("Workflow dir:", &self.workflow_path);
        print_tabular_row("Current task:", &self.task_name);
        let step_path = relative_path(&self.workspace_dir, &self.step_path);
        print_tabular_row("Step:", step_path);
    }
}

#[derive(Debug)]
enum StepContextError {
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

impl fmt::Display for StepContextError {
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

impl Error for StepContextError {}
