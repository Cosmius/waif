use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::artifact::Artifact;
use crate::parser::{self, Diagnostic};
use crate::schemas::{goal, plan, review, step};

use super::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArtifactKind {
    Goal,
    Plan,
    Step,
    ImplementationReview,
    Generic,
}

pub struct ArtifactFile {
    path: PathBuf,
    contents: String,
}

impl ArtifactFile {
    pub fn read(path: PathBuf) -> Result<Self> {
        let contents = fs::read_to_string(&path)?;
        Ok(Self { path, contents })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    pub(crate) fn kind(&self) -> ArtifactKind {
        ArtifactKind::for_path(&self.path)
    }

    pub fn parse_goal(&self) -> Result<Artifact<'_>> {
        self.parse_with(goal::parse)
    }

    pub fn parse_plan(&self) -> Result<Artifact<'_>> {
        self.parse_with(plan::parse)
    }

    pub fn parse_step(&self) -> Result<Artifact<'_>> {
        self.parse_with(step::parse)
    }

    pub fn parse_review(&self) -> Result<Artifact<'_>> {
        self.parse_with(review::parse)
    }

    pub fn parse_generic(&self) -> Result<Artifact<'_>> {
        self.parse_with(parser::parse)
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        match self.kind() {
            ArtifactKind::Goal => goal::check(&self.contents),
            ArtifactKind::Plan => plan::check(&self.contents),
            ArtifactKind::Step => step::check(&self.path, &self.contents),
            ArtifactKind::ImplementationReview => review::check(&self.path, &self.contents),
            ArtifactKind::Generic => parser::parse(&self.contents).err().unwrap_or_default(),
        }
    }

    fn parse_with<'a>(
        &'a self,
        parse: impl FnOnce(&'a str) -> std::result::Result<Artifact<'a>, Vec<Diagnostic>>,
    ) -> Result<Artifact<'a>> {
        parse(&self.contents).map_err(|diagnostics| {
            Box::new(InvalidTaskArtifact {
                path: self.path.clone(),
                diagnostics,
            }) as Box<dyn Error>
        })
    }
}

impl ArtifactKind {
    fn for_path(path: &Path) -> Self {
        let Some(name) = path.file_name() else {
            return Self::Generic;
        };
        if name == OsStr::new("goal.md") {
            Self::Goal
        } else if name == OsStr::new("plan.md") {
            Self::Plan
        } else if name == OsStr::new("step.md") {
            Self::Step
        } else if name.to_str().is_some_and(is_numbered_review_name) {
            Self::ImplementationReview
        } else {
            Self::Generic
        }
    }
}

pub(super) fn is_numbered_review_name(name: &str) -> bool {
    let Some(number) = name
        .strip_prefix("review")
        .and_then(|name| name.strip_suffix(".md"))
    else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
}

#[derive(Debug)]
struct InvalidTaskArtifact {
    path: PathBuf,
    diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for InvalidTaskArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                formatter.write_str("\n")?;
            }
            write!(
                formatter,
                "{}:{}: {}",
                self.path.display(),
                diagnostic.line(),
                diagnostic.message()
            )?;
        }
        Ok(())
    }
}

impl Error for InvalidTaskArtifact {}
