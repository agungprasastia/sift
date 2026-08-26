//! Shared error type.

use std::path::PathBuf;

/// Errors surfaced across Sift crates.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
