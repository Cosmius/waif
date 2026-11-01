use std::error::Error;

use clap::Args;

use crate::package::{self, PackageTarget};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
#[command(after_help = indoc::indoc! {"
    Examples:
      waif package --target claude-plugin
      waif package --target skill-zip"
})]
pub struct PackageCommand {
    /// Package to build.
    #[arg(long, value_enum)]
    target: PackageTarget,
}

impl PackageCommand {
    pub fn run(self) -> Result<()> {
        package::build(self.target)
    }
}
