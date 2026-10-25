use std::error::Error;
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use clap::Args;

use super::{print_tabular_row, relative_path};
use crate::protocol::{ArtifactClassification, Task, WorkflowDir};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
#[command(after_help = indoc::indoc! {"
    Example:
      waif inspect"
})]
pub struct InspectCommand {}

impl InspectCommand {
    pub fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
        print_tabular_row(
            "Workflow dir:",
            relative_path(&workspace_dir, &workflow_dir.path),
        );

        print_tabular_row(
            "Library:",
            relative_path(&workspace_dir, &workflow_dir.library_dir()),
        );

        let tasks_dir = workflow_dir.tasks_dir();
        if let Err(error) = fs::read_dir(&tasks_dir) {
            print_tabular_row("Current task:", "unavailable");
            print_tabular_row(
                "Next:",
                if error.kind() == ErrorKind::NotFound {
                    "Run waif init or repair library"
                } else {
                    "Check library"
                },
            );
            return Err(format!("{}: {error}", relative_path(&workspace_dir, &tasks_dir)).into());
        }

        let task = match workflow_dir.current_task() {
            Ok(Some(task)) => task,
            Ok(None) => {
                print_tabular_row("Current task:", "none");
                print_tabular_row("Next:", "Create or select task");
                return unusable();
            }
            Err(error) => {
                print_tabular_row("Current task:", "unavailable");
                print_tabular_row("Next:", "Repair or select task");
                return Err(format!(
                    "{}: {error}",
                    relative_path(&workspace_dir, &workflow_dir.current_link()),
                )
                .into());
            }
        };
        print_current_task(&workspace_dir, task)
    }
}

fn print_current_task(workspace_dir: &Path, task: Task) -> Result<()> {
    let task_name = task.name().map_err(|_| UnusableInspection)?;
    print_tabular_row("Current task:", task_name);
    print_tabular_row(
        "Current task dir:",
        relative_path(workspace_dir, &task.path),
    );
    if let Err(error) = fs::read_dir(&task.path) {
        print_tabular_row("Next:", "Repair or select task");
        return Err(format!("{}: {error}", relative_path(workspace_dir, &task.path)).into());
    }
    println!("Goal:");

    let goal_path = task.path.join("goal.md");
    let inspection = match task.inspect_goal() {
        Ok(Some(inspection)) => inspection,
        Ok(None) => {
            println!("  not created yet");
            print_tabular_row("Next:", "Draft goal");
            return Ok(());
        }
        Err(_) => {
            print_tabular_row("  Path:", relative_path(workspace_dir, &goal_path));
            print_tabular_row("  Status:", "BAD");
            print_tabular_row("Next:", "Check goal");
            return unusable();
        }
    };

    print_tabular_row("  Path:", relative_path(workspace_dir, &goal_path));
    if inspection.classification() == ArtifactClassification::Bad {
        print_tabular_row("  Status:", "BAD");
        print_tabular_row("Next:", "Check goal");
        return unusable();
    }

    let status = inspection.status().unwrap_or("BAD");
    print_tabular_row("  Status:", status);
    if let Some(ready) = inspection.ready_to_accept() {
        print_tabular_row("  Ready to accept:", if ready { "yes" } else { "no" });
    }

    if inspection.classification() == ArtifactClassification::Incomplete {
        print_tabular_row("Next:", "Fill goal");
    } else if matches!(status, "drafting" | "amending") {
        if inspection.has_open_questions() {
            print_tabular_row("Next:", "Revise goal");
        } else {
            print_tabular_row("Next:", "Revise or accept goal");
        }
    } else if status == "accepted" {
        print_tabular_row("Next:", "Draft plan");
    }
    Ok(())
}

fn unusable<T>() -> Result<T> {
    Err(Box::new(UnusableInspection))
}

#[derive(Debug)]
struct UnusableInspection;

impl fmt::Display for UnusableInspection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("workflow context is unusable")
    }
}

impl Error for UnusableInspection {}
