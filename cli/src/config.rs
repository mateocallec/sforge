use std::path::PathBuf;

use directories::ProjectDirs;

use crate::error::{Error, Result};

pub struct Config {
    pub sessions_dir: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let data_dir = match std::env::var_os("SFORGE_DATA_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => ProjectDirs::from("io", "sforge", "sforge")
                .ok_or(Error::NoDataDir)?
                .data_dir()
                .to_path_buf(),
        };

        let sessions_dir = data_dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir).map_err(|source| Error::Io {
            path: sessions_dir.clone(),
            source,
        })?;

        Ok(Self { sessions_dir })
    }
}
