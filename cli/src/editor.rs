use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

#[cfg(target_os = "windows")]
fn default_editor() -> &'static str {
    "notepad"
}

#[cfg(not(target_os = "windows"))]
fn default_editor() -> &'static str {
    "nano"
}

pub fn resolve() -> String {
    std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| default_editor().to_string())
}

/// Opens `path` in the resolved editor, blocking until it exits, with the
/// editor's stdio inherited so interactive terminal editors work correctly.
pub fn open(path: &Path) -> Result<()> {
    let editor = resolve();
    let mut parts = shlex::split(&editor).unwrap_or_default();
    if parts.is_empty() {
        parts.push(editor);
    }
    let program = parts.remove(0);

    let status = Command::new(&program)
        .args(&parts)
        .arg(path)
        .status()
        .map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;

    if !status.success() {
        return Err(Error::EditorFailed { command: program });
    }
    Ok(())
}
