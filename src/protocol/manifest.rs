use std::cmp::Ordering;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Result;
use crate::parser::Severity;

pub(crate) const SCHEMA_VERSION: &str = "0.3";

#[derive(Serialize, Deserialize)]
struct Manifest {
    schema_version: String,
    #[serde(flatten)]
    unknown: toml::Table,
}

/// A read manifest: its version state and any keys this WAIF does not know.
pub(crate) struct ManifestCheck {
    state: ManifestState,
    unknown_keys: Vec<String>,
}

impl ManifestCheck {
    /// The diagnostics for this manifest, without the manifest path.
    pub(crate) fn findings(&self) -> Vec<(Severity, String)> {
        self.state
            .finding()
            .into_iter()
            .chain(
                self.unknown_keys
                    .iter()
                    .map(|key| (Severity::Warning, format!("unknown key `{key}` ignored"))),
            )
            .collect()
    }

    /// The value shown for this manifest by `waif inspect`.
    pub(crate) fn label(&self) -> &str {
        self.state.label()
    }

    /// The `waif inspect` next action for a manifest that stops inspection.
    pub(crate) fn next_action(&self) -> Option<&'static str> {
        self.state.next_action()
    }
}

/// State of a library or task manifest compared with the current schema.
enum ManifestState {
    Current,
    Missing,
    Older(String),
    Newer(String),
    Invalid(String),
}

impl ManifestState {
    fn finding(&self) -> Option<(Severity, String)> {
        match self {
            Self::Current => None,
            Self::Missing => Some((
                Severity::Warning,
                format!(
                    "no schema version, older than {SCHEMA_VERSION}; migrate with \
                     migration/README.md"
                ),
            )),
            Self::Older(version) => Some((
                Severity::Error,
                format!(
                    "schema version {version} is older than {SCHEMA_VERSION}; migrate with \
                     migration/README.md and migration/v{version}.md"
                ),
            )),
            Self::Newer(version) => Some((
                Severity::Error,
                format!(
                    "schema version {version} is newer than {SCHEMA_VERSION}; written by a \
                     newer WAIF"
                ),
            )),
            Self::Invalid(message) => Some((Severity::Error, message.clone())),
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::Current => SCHEMA_VERSION,
            Self::Missing => "none",
            Self::Older(version) | Self::Newer(version) => version,
            Self::Invalid(_) => "invalid",
        }
    }

    fn next_action(&self) -> Option<&'static str> {
        match self {
            Self::Current | Self::Missing => None,
            Self::Older(_) => Some("Migrate schema"),
            Self::Newer(_) => Some("Update WAIF"),
            Self::Invalid(_) => Some("Repair manifests"),
        }
    }
}

pub(crate) fn library_manifest(library_dir: &Path) -> PathBuf {
    library_dir.join("waif.toml")
}

pub(crate) fn task_manifest(task_dir: &Path) -> PathBuf {
    task_dir.join("task.toml")
}

/// Read a manifest and compare its version with the current schema version.
pub(crate) fn read_manifest(path: &Path) -> ManifestCheck {
    let invalid = |message| ManifestCheck {
        state: ManifestState::Invalid(message),
        unknown_keys: Vec::new(),
    };
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return ManifestCheck {
                state: ManifestState::Missing,
                unknown_keys: Vec::new(),
            }
        }
        Err(error) => return invalid(error.to_string()),
    };
    let manifest = match toml::from_str::<Manifest>(&contents) {
        Ok(manifest) => manifest,
        Err(error) => return invalid(format!("invalid manifest: {}", error.message())),
    };
    let version = manifest.schema_version;
    let Some(parsed) = parse_version(&version) else {
        return invalid(format!(
            "schema_version `{version}` is not a MAJOR.MINOR version"
        ));
    };
    let current = parse_version(SCHEMA_VERSION).expect("current schema version should parse");
    let state = match parsed.cmp(&current) {
        Ordering::Less => ManifestState::Older(version),
        Ordering::Equal => ManifestState::Current,
        Ordering::Greater => ManifestState::Newer(version),
    };
    ManifestCheck {
        state,
        unknown_keys: manifest.unknown.keys().cloned().collect(),
    }
}

/// Parse exactly two `.`-separated decimal components.
fn parse_version(version: &str) -> Option<[u64; 2]> {
    let components = version
        .split('.')
        .map(|component| {
            if component.bytes().all(|byte| byte.is_ascii_digit()) {
                component.parse().ok()
            } else {
                None
            }
        })
        .collect::<Option<Vec<u64>>>()?;
    components.try_into().ok()
}

/// Create the library manifest; fail rather than overwrite an existing one.
pub(super) fn write_library_manifest(library_dir: &Path) -> Result<()> {
    write_manifest(&library_manifest(library_dir))
}

/// Create the task manifest; fail rather than overwrite an existing one.
pub(super) fn write_task_manifest(task_dir: &Path) -> Result<()> {
    write_manifest(&task_manifest(task_dir))
}

fn write_manifest(path: &Path) -> Result<()> {
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION.to_owned(),
        unknown: toml::Table::new(),
    };
    let contents = toml::to_string(&manifest)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(contents.as_bytes())?;
    Ok(())
}
