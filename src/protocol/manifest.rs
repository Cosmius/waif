use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde::Serialize;

use super::Result;

pub(crate) const SCHEMA_VERSION: &str = "0.3";

#[derive(Serialize)]
struct Manifest {
    schema_version: String,
}

/// Create the library manifest; fail rather than overwrite an existing one.
pub(super) fn write_library_manifest(library_dir: &Path) -> Result<()> {
    write_manifest(&library_dir.join("waif.toml"))
}

/// Create the task manifest; fail rather than overwrite an existing one.
pub(super) fn write_task_manifest(task_dir: &Path) -> Result<()> {
    write_manifest(&task_dir.join("task.toml"))
}

fn write_manifest(path: &Path) -> Result<()> {
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION.to_owned(),
    };
    let contents = toml::to_string(&manifest)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(contents.as_bytes())?;
    Ok(())
}
