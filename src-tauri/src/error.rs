//! Błędy zwracane z komend Tauri do front-endu.
//!
//! Kształt JSON: `{ "kind": "...", "message": "..." }`.
//! `kind` — stabilny identyfikator dla JS, `message` — tekst dla użytkownika.

use serde::ser::SerializeStruct;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("nieprawidłowy URL: {0}")]
    BadUrl(String),
    #[error("host niedozwolony: {0}")]
    BadHost(String),
    #[error("błąd HTTP: {0}")]
    Http(String),
    #[error("błąd IO: {0}")]
    Io(String),
    #[error("niezgodny hash: oczekiwano {expected}, otrzymano {got} ({path})")]
    HashMismatch {
        expected: String,
        got: String,
        path: String,
    },
    #[error("nieprawidłowa ścieżka: {0}")]
    BadPath(String),
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            Self::BadUrl(_) => "bad_url",
            Self::BadHost(_) => "bad_host",
            Self::Http(_) => "http",
            Self::Io(_) => "io",
            Self::HashMismatch { .. } => "hash_mismatch",
            Self::BadPath(_) => "bad_path",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_builder() || err.is_redirect() {
            Self::BadUrl(err.to_string())
        } else {
            Self::Http(err.to_string())
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Http(format!("nieprawidłowy JSON: {err}"))
    }
}
