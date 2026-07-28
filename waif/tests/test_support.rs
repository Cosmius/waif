#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) struct TestDir {
    pub(crate) path: PathBuf,
}

impl TestDir {
    pub(crate) fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        let name = sanitize_name(name);
        let path = std::env::temp_dir().join(format!("waif-test-{name}-{unique}"));
        fs::create_dir_all(&path).expect("test directory should be created");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect()
}
