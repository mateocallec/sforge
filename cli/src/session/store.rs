use std::io::Write;
use std::path::Path;

use crate::error::{Error, Result};

/// Writes `contents` to `path` atomically: a temp file in the same
/// directory is written and fsync'd, then renamed into place, so a reader
/// (or a crash mid-write) never observes a partially-written session file.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    tmp.write_all(contents).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    tmp.flush().map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;

    tmp.persist(path).map_err(|err| Error::Io {
        path: path.to_path_buf(),
        source: err.error,
    })?;

    Ok(())
}
