//! Filesystem scanner: walks a repository, skips ignored/generated trees,
//! filters binary and oversized files, extracts symbols one file at a time
//! (streaming — never loads the whole repository into RAM).

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use sift_core::{Error, Language, SourceFile};
use sift_parser::extract_symbols;

/// Directories pruned regardless of `.gitignore` state.
const IGNORED_DIRS: [&str; 9] = [
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    "coverage",
    "vendor",
    ".cache",
];

/// Files larger than this (bytes) are skipped: M0 targets source files, not
/// generated artifacts or data dumps.
const MAX_FILE_SIZE: u64 = 1024 * 1024;

/// Leading bytes inspected for NUL when sniffing for binaries.
const BINARY_SNIFF_LEN: usize = 8192;

#[derive(Debug, Clone)]
pub struct ScanWarning {
    pub path: PathBuf,
    pub message: String,
}

impl ScanWarning {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Default, Clone)]
pub struct ScanDiagnostics {
    pub warnings: Vec<ScanWarning>,
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("cannot walk `{path}`: {source}")]
    Walk {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl From<ScanError> for Error {
    fn from(err: ScanError) -> Self {
        match err {
            ScanError::Walk { path, source } => {
                let io_err = source
                    .downcast::<std::io::Error>()
                    .map(|boxed| *boxed)
                    .unwrap_or_else(|other| std::io::Error::other(other.to_string()));
                Error::Io {
                    path,
                    source: io_err,
                }
            }
        }
    }
}

/// Result of one scan pass.
pub struct ScanOutput {
    pub files: Vec<SourceFile>,
    /// Files the walker actually visited after directory pruning.
    pub files_scanned: u64,
    pub diagnostics: ScanDiagnostics,
}

/// Walks `root` and produces parsed [`SourceFile`]s in deterministic order.
///
/// Root repository invalid is fatal. Individual unreadable files or
/// subtrees are skipped with diagnostics logged, allowing scanning of the rest.
pub fn scan(root: &Path) -> Result<ScanOutput, ScanError> {
    if !root.exists() {
        return Err(ScanError::Walk {
            path: root.to_path_buf(),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "root path does not exist",
            )),
        });
    }

    let walker = WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(|entry| {
            let is_ignored_dir = entry.file_type().is_some_and(|ft| ft.is_dir())
                && IGNORED_DIRS.contains(&entry.file_name().to_string_lossy().as_ref());
            !is_ignored_dir
        })
        .build();

    let mut out = ScanOutput {
        files: Vec::new(),
        files_scanned: 0,
        diagnostics: ScanDiagnostics::default(),
    };
    for entry_result in walker {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(err) => {
                let err_path = match &err {
                    ignore::Error::WithPath { path, .. } => path.clone(),
                    _ => root.to_path_buf(),
                };
                if err_path == root {
                    return Err(ScanError::Walk {
                        path: root.to_path_buf(),
                        source: Box::new(err),
                    });
                }
                out.diagnostics.warnings.push(ScanWarning {
                    path: err_path,
                    message: err.to_string(),
                });
                continue;
            }
        };

        if entry.depth() == 0 || !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        out.files_scanned += 1;
        match process_file(entry.path()) {
            Ok(Some(file)) => {
                out.files.push(file);
            }
            Ok(None) => {}
            Err(warning) => {
                out.diagnostics.warnings.push(warning);
            }
        }
    }
    out.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Reads and parses one file; returns `Ok(None)` for non-source or skipped files.
fn process_file(path: &Path) -> Result<Option<SourceFile>, ScanWarning> {
    let language = match Language::detect(path) {
        Some(lang) => lang,
        None => return Ok(None),
    };

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(err) => {
            return Err(ScanWarning {
                path: path.to_path_buf(),
                message: format!("cannot read metadata: {err}"),
            });
        }
    };

    if metadata.len() > MAX_FILE_SIZE {
        return Ok(None);
    }

    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(err) => {
            return Err(ScanWarning {
                path: path.to_path_buf(),
                message: format!("cannot read content: {err}"),
            });
        }
    };

    let sniff_len = bytes.len().min(BINARY_SNIFF_LEN);
    if sift_sys::count_byte(&bytes[..sniff_len], 0) > 0 {
        return Ok(None);
    }

    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };

    let symbols = extract_symbols(language, text.as_bytes());
    let content_hash = sift_sys::hash_bytes(text.as_bytes());

    Ok(Some(SourceFile {
        path: path.to_path_buf(),
        language,
        symbols,
        content_hash,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignored_dir_list_matches_spec() {
        assert_eq!(
            IGNORED_DIRS,
            [
                ".git",
                "target",
                "node_modules",
                "dist",
                "build",
                ".next",
                "coverage",
                "vendor",
                ".cache"
            ]
        );
    }
}
