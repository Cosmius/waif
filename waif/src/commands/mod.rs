use std::error::Error;
use std::fmt::Display;
use std::path::Path;

use clap::Subcommand;

mod check;
mod fill;
mod goal;
mod init;
mod inspect;
mod new;
mod switch;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Subcommand)]
pub enum Command {
    /// Classify task artifacts as GOOD, INCOMPLETE, or BAD.
    #[command(
        long_about = "Check PATH as one artifact or task directory, or check the current \
                      task when PATH is omitted. Report every finding without mutation. \
                      GOOD and INCOMPLETE exit successfully; BAD exits unsuccessfully."
    )]
    Check(check::CheckCommand),
    /// Replace known macros in a text file after applicable safety checks.
    #[command(
        long_about = "Replace known macros in one PATH as plain text. No recognized \
                      occurrence leaves the file unchanged and reports that no fill is \
                      needed. Recognized artifacts are checked before overwrite; a BAD \
                      candidate is saved separately and leaves the target unchanged. \
                      Unrecognized paths skip candidate checks and recovery copies. A \
                      low-level overwrite failure is reported, may leave a partial target, \
                      and is not rolled back."
    )]
    Fill(fill::FillCommand),
    /// Perform lifecycle operations on the current goal.
    #[command(subcommand)]
    Goal(goal::GoalCommand),
    /// Set up an independent workflow or link to a supplied library.
    #[command(
        long_about = "Create an independent library and workflow repository, or use \
                      --link TARGET to link to a supplied library's canonical path. \
                      An existing library entry is left unchanged; report skipped \
                      setup and succeed without validating it."
    )]
    Init(init::InitCommand),
    /// Report current goal-stage context and the next mechanical action.
    #[command(long_about = "Report the local instance, library, and \
                      current goal state without arguments or mutation. Report \
                      acceptance readiness and the next mechanical action \
                      when determinable. Succeed only when the reported context is usable.")]
    Inspect(inspect::InspectCommand),
    /// Create and select a new workflow task from a validated slug.
    #[command(
        long_about = "Create and select the next date-sequenced task from one validated \
                      TASK_SLUG. Requires an initialized workflow; run waif init first. \
                      Invalid input or a collision creates \
                      and selects nothing."
    )]
    New(new::NewCommand),
    /// Select an existing task by its complete directory name.
    #[command(
        long_about = "Select one existing direct task child by its complete validated \
                      TASK_NAME. Missing, malformed, ambiguous, or indirect targets leave \
                      the current task reference unchanged."
    )]
    Switch(switch::SwitchCommand),
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Check(command) => command.run(),
            Self::Fill(command) => command.run(),
            Self::Goal(command) => command.run(),
            Self::Init(command) => command.run(),
            Self::Inspect(command) => command.run(),
            Self::New(command) => command.run(),
            Self::Switch(command) => command.run(),
        }
    }
}

fn relative_path(workspace_dir: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn print_tabular_row(label: &str, value: impl Display) {
    println!("{label:<20}{value}");
}
