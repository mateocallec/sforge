pub mod cookie;
pub mod index;
pub mod store;
pub mod template;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{Error, Result};
use cookie::CookieEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub meta: Meta,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub headers: Vec<HeaderEntry>,
    #[serde(default)]
    pub cookies: Vec<CookieEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub id: String,
    pub name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Auth {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<AuthKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthKind {
    Basic,
    Bearer,
    Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
}

/// What to replace on a `session setup` inline invocation. A category left
/// `None` is untouched — only categories actually present on the command
/// line get replaced, so e.g. refreshing cookies from the browser extension
/// never wipes a hand-added header or auth block.
#[derive(Debug, Default)]
pub struct SessionPatch {
    pub cookies: Option<Vec<CookieEntry>>,
    pub headers: Option<Vec<HeaderEntry>>,
    pub auth: Option<Auth>,
}

pub struct Store {
    sessions_dir: PathBuf,
}

impl Store {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    fn session_dir(&self, id: Uuid) -> PathBuf {
        self.sessions_dir.join(id.to_string())
    }

    fn session_file(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join("session.toml")
    }

    pub fn load_by_name(&self, name: &str) -> Result<Session> {
        let index = index::Index::load(&self.sessions_dir)?;
        let id = index
            .find(name)
            .ok_or_else(|| Error::SessionNotFound(name.to_string()))?;
        self.load(id)
    }

    pub fn load(&self, id: Uuid) -> Result<Session> {
        let path = self.session_file(id);
        let raw = std::fs::read_to_string(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        toml_edit::de::from_str(&raw).map_err(|e| Error::SessionParse {
            path,
            message: e.to_string(),
        })
    }

    pub fn list(&self) -> Result<Vec<(Uuid, String)>> {
        let index = index::Index::load(&self.sessions_dir)?;
        Ok(index.list())
    }

    /// The on-disk path of an existing session's data file, for callers
    /// (the TUI's `e` edit action) that already have the session's id and
    /// just need somewhere to point `$EDITOR`.
    pub fn path_for(&self, id: Uuid) -> PathBuf {
        self.session_file(id)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        let mut index = index::Index::load(&self.sessions_dir)?;
        let id = index.remove(name)?;
        let dir = self.session_dir(id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|source| Error::Io { path: dir, source })?;
        }
        Ok(())
    }

    /// Applies `patch` per-category to the named session, creating it (with
    /// a fresh UUID) if it doesn't exist yet. This is what lets the
    /// browser extension's copy button double as both create and update:
    /// it always emits a full `--set-cookie` set, which fully replaces the
    /// cookie list, while leaving any hand-added headers/auth untouched.
    pub fn replace(&self, name: &str, patch: SessionPatch) -> Result<Session> {
        if name.trim().is_empty() {
            return Err(Error::EmptySessionName);
        }

        let mut index = index::Index::load(&self.sessions_dir)?;
        let now = OffsetDateTime::now_utc();

        let (id, mut session) = match index.find(name) {
            Some(id) => (id, self.load(id)?),
            None => {
                let id = Uuid::new_v4();
                let session = Session {
                    meta: Meta {
                        id: id.to_string(),
                        name: name.to_string(),
                        created_at: now,
                        updated_at: now,
                    },
                    auth: Auth::default(),
                    headers: Vec::new(),
                    cookies: Vec::new(),
                };
                index.insert(id, name)?;
                (id, session)
            }
        };

        if let Some(cookies) = patch.cookies {
            session.cookies = cookies;
        }
        if let Some(headers) = patch.headers {
            session.headers = headers;
        }
        if let Some(auth) = patch.auth {
            session.auth = auth;
        }
        session.meta.updated_at = now;

        let path = self.session_file(id);
        let toml =
            toml_edit::ser::to_string_pretty(&session).map_err(|e| Error::SessionSerialize {
                path: path.clone(),
                message: e.to_string(),
            })?;
        store::write_atomic(&path, toml.as_bytes())?;

        Ok(session)
    }

    /// Ensures a session directory + file exists for `name` (writing a
    /// fresh commented template if it doesn't), returning its id and path
    /// ready to hand to `$EDITOR`.
    pub fn ensure_editable(&self, name: &str) -> Result<(Uuid, PathBuf)> {
        if name.trim().is_empty() {
            return Err(Error::EmptySessionName);
        }

        let mut index = index::Index::load(&self.sessions_dir)?;
        let id = match index.find(name) {
            Some(id) => id,
            None => {
                let id = Uuid::new_v4();
                index.insert(id, name)?;
                let now = OffsetDateTime::now_utc();
                let now_str = now
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default();
                let content = template::render(&id.to_string(), name, &now_str);
                store::write_atomic(&self.session_file(id), content.as_bytes())?;
                id
            }
        };
        Ok((id, self.session_file(id)))
    }

    /// Bumps `meta.updated_at` via a surgical in-place document edit, so
    /// hand-written comments/formatting from an `$EDITOR` session survive —
    /// a full reserialize here would defeat the point of TOML's
    /// comment-friendliness on the one write path where it actually matters.
    pub fn touch_updated_at(&self, id: Uuid) -> Result<()> {
        let path = self.session_file(id);
        let raw = std::fs::read_to_string(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        let mut doc = raw
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| Error::SessionParse {
                path: path.clone(),
                message: e.to_string(),
            })?;

        let now = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        doc["meta"]["updated_at"] = toml_edit::value(now);

        store::write_atomic(&path, doc.to_string().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().to_path_buf());
        (dir, store)
    }

    #[test]
    fn replace_creates_a_new_session() {
        let (_dir, store) = temp_store();
        let cookie = CookieEntry::parse_set_cookie("a=b; Domain=x.test", None).unwrap();
        let patch = SessionPatch {
            cookies: Some(vec![cookie]),
            ..Default::default()
        };
        let session = store.replace("first", patch).unwrap();
        assert_eq!(session.meta.name, "first");
        assert_eq!(session.cookies.len(), 1);

        let reloaded = store.load_by_name("first").unwrap();
        assert_eq!(reloaded.cookies.len(), 1);
    }

    #[test]
    fn replace_only_touches_categories_present_in_the_patch() {
        let (_dir, store) = temp_store();
        let cookie = CookieEntry::parse_set_cookie("a=b; Domain=x.test", None).unwrap();
        store
            .replace(
                "acme",
                SessionPatch {
                    cookies: Some(vec![cookie]),
                    headers: Some(vec![HeaderEntry {
                        name: "X-Debug".into(),
                        value: "1".into(),
                    }]),
                    ..Default::default()
                },
            )
            .unwrap();

        // Simulates the extension re-running setup with a refreshed cookie
        // set only — the hand-added header must survive untouched.
        let new_cookie = CookieEntry::parse_set_cookie("a=c; Domain=x.test", None).unwrap();
        let updated = store
            .replace(
                "acme",
                SessionPatch {
                    cookies: Some(vec![new_cookie]),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(updated.cookies[0].value, "c");
        assert_eq!(updated.headers.len(), 1);
        assert_eq!(updated.headers[0].name, "X-Debug");
    }

    #[test]
    fn ensure_editable_then_touch_updated_at_preserves_comments() {
        let (_dir, store) = temp_store();
        let (id, path) = store.ensure_editable("edited").unwrap();
        let before = std::fs::read_to_string(&path).unwrap();
        assert!(before.contains("# SessionForge session file"));

        store.touch_updated_at(id).unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.contains("# SessionForge session file"));

        let session = store.load(id).unwrap();
        assert_eq!(session.meta.name, "edited");
    }

    #[test]
    fn delete_removes_the_session_directory() {
        let (_dir, store) = temp_store();
        store.replace("temp", SessionPatch::default()).unwrap();
        store.delete("temp").unwrap();
        assert!(store.load_by_name("temp").is_err());
    }
}
