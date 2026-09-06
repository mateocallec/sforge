use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no session named \"{0}\"")]
    SessionNotFound(String),

    #[error("a session named \"{0}\" already exists")]
    SessionNameConflict(String),

    #[error("session name cannot be empty")]
    EmptySessionName,

    #[error("failed to access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse session file {path}: {message}")]
    SessionParse { path: PathBuf, message: String },

    #[error("failed to write session file {path}: {message}")]
    SessionSerialize { path: PathBuf, message: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid --set-cookie value \"{0}\"")]
    InvalidSetCookie(String),

    #[error("invalid -H/--header value \"{0}\"")]
    InvalidHeader(String),

    #[error("editor \"{command}\" exited with a non-zero status")]
    EditorFailed { command: String },

    #[error("could not determine a config/data directory for this platform")]
    NoDataDir,

    #[error("invalid URL \"{0}\"")]
    InvalidUrl(String),

    #[error("unsupported URL scheme \"{0}\" (only http and https are supported)")]
    UnsupportedScheme(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("could not connect: {0}")]
    Connect(String),

    #[error("malformed HTTP response: {0}")]
    MalformedResponse(String),

    #[error("unsupported Content-Encoding \"{0}\"")]
    UnsupportedContentEncoding(String),

    #[error("failed to decompress response body: {0}")]
    Decompress(String),

    #[error("invalid -x/--proxy value \"{0}\"")]
    InvalidProxy(String),

    #[error("too many redirects (max {0})")]
    TooManyRedirects(u32),

    #[error("invalid -F/--form value \"{0}\"")]
    InvalidForm(String),
}

pub type Result<T> = std::result::Result<T, Error>;
