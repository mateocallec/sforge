use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::session::store;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexEntry {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexFile {
    version: u32,
    sessions: Vec<IndexEntry>,
}

impl Default for IndexFile {
    fn default() -> Self {
        Self {
            version: 1,
            sessions: Vec::new(),
        }
    }
}

/// The session name → session UUID lookup, backed by `index.json`.
///
/// Session names are arbitrary UTF-8 and are never used as filesystem path
/// components; the index is what makes name-based lookup possible while
/// on-disk storage stays keyed by UUID. Names are NFC-normalized before
/// comparison so visually-identical names typed with different Unicode
/// representations can't silently collide or fork.
pub struct Index {
    path: PathBuf,
    file: IndexFile,
}

fn normalize(name: &str) -> String {
    name.nfc().collect()
}

impl Index {
    pub fn load(sessions_dir: &Path) -> Result<Self> {
        let path = sessions_dir.join("index.json");
        let file = if path.exists() {
            let raw = std::fs::read_to_string(&path).map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
            serde_json::from_str(&raw)?
        } else {
            IndexFile::default()
        };
        Ok(Self { path, file })
    }

    pub fn find(&self, name: &str) -> Option<Uuid> {
        let target = normalize(name);
        self.file
            .sessions
            .iter()
            .find(|e| normalize(&e.name) == target)
            .and_then(|e| Uuid::parse_str(&e.id).ok())
    }

    pub fn list(&self) -> Vec<(Uuid, String)> {
        self.file
            .sessions
            .iter()
            .filter_map(|e| Uuid::parse_str(&e.id).ok().map(|id| (id, e.name.clone())))
            .collect()
    }

    pub fn insert(&mut self, id: Uuid, name: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err(Error::EmptySessionName);
        }
        let target = normalize(name);
        if self
            .file
            .sessions
            .iter()
            .any(|e| normalize(&e.name) == target)
        {
            return Err(Error::SessionNameConflict(name.to_string()));
        }
        self.file.sessions.push(IndexEntry {
            id: id.to_string(),
            name: name.to_string(),
        });
        self.save()
    }

    pub fn remove(&mut self, name: &str) -> Result<Uuid> {
        let target = normalize(name);
        let pos = self
            .file
            .sessions
            .iter()
            .position(|e| normalize(&e.name) == target)
            .ok_or_else(|| Error::SessionNotFound(name.to_string()))?;
        let entry = self.file.sessions.remove(pos);
        self.save()?;
        Uuid::parse_str(&entry.id).map_err(|_| Error::SessionNotFound(name.to_string()))
    }

    fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.file)?;
        store::write_atomic(&self.path, json.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_index() -> (tempfile::TempDir, Index) {
        let dir = tempfile::tempdir().unwrap();
        let index = Index::load(dir.path()).unwrap();
        (dir, index)
    }

    #[test]
    fn insert_then_find_round_trips() {
        let (_dir, mut index) = temp_index();
        let id = Uuid::new_v4();
        index.insert(id, "acme-admin").unwrap();
        assert_eq!(index.find("acme-admin"), Some(id));
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let (_dir, mut index) = temp_index();
        index.insert(Uuid::new_v4(), "dup").unwrap();
        let result = index.insert(Uuid::new_v4(), "dup");
        assert!(matches!(result, Err(Error::SessionNameConflict(_))));
    }

    #[test]
    fn nfc_normalization_prevents_visual_duplicate_names() {
        // "e\u{0301}" (e + combining acute accent) normalizes to the same
        // NFC form as "\u{00e9}" (precomposed é).
        let (_dir, mut index) = temp_index();
        index.insert(Uuid::new_v4(), "caf\u{0065}\u{0301}").unwrap();
        let result = index.insert(Uuid::new_v4(), "caf\u{00e9}");
        assert!(matches!(result, Err(Error::SessionNameConflict(_))));
    }

    #[test]
    fn weird_characters_are_supported_in_names() {
        let (_dir, mut index) = temp_index();
        let id = Uuid::new_v4();
        let name = "acme QA \u{2013} Andr\u{00e9}'s \"test\" profile \u{1f600}";
        index.insert(id, name).unwrap();
        assert_eq!(index.find(name), Some(id));
    }

    #[test]
    fn empty_name_is_rejected() {
        let (_dir, mut index) = temp_index();
        let result = index.insert(Uuid::new_v4(), "   ");
        assert!(matches!(result, Err(Error::EmptySessionName)));
    }

    #[test]
    fn remove_then_find_returns_none() {
        let (_dir, mut index) = temp_index();
        let id = Uuid::new_v4();
        index.insert(id, "gone-soon").unwrap();
        let removed = index.remove("gone-soon").unwrap();
        assert_eq!(removed, id);
        assert_eq!(index.find("gone-soon"), None);
    }

    #[test]
    fn removing_unknown_name_errors() {
        let (_dir, mut index) = temp_index();
        let result = index.remove("never-existed");
        assert!(matches!(result, Err(Error::SessionNotFound(_))));
    }

    #[test]
    fn persists_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        {
            let mut index = Index::load(dir.path()).unwrap();
            index.insert(id, "persisted").unwrap();
        }
        let reloaded = Index::load(dir.path()).unwrap();
        assert_eq!(reloaded.find("persisted"), Some(id));
    }
}
