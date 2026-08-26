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
pub(super) struct ScanOutput {
    pub files: Vec<SourceFile>,
    /// Files the walker actually visited after directory pruning.
    pub files_scanned: u64,
}

/// Walks `root` and produces parsed [`SourceFile`]s in deterministic order.
///
/// Individual unreadable/non-source files are skipped silently; only a
/// broken walk root is fatal. `.gitignore` is honored even outside git
/// repositories (`require_git(false)`).
pub(super) fn scan(root: &Path) -> Result<ScanOutput, ScanError> {
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
    };
    for entry in walker {
        let entry = entry.map_err(|err| ScanError::Walk {
            path: root.to_path_buf(),
            source: Box::new(err),
        })?;
        if entry.depth() == 0 || !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        out.files_scanned += 1;
        if let Some(file) = process_file(entry.path()) {
            out.files.push(file);
        }
    }
    out.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Reads and parses one file; returns `None` for anything not worth indexing.
fn process_file(path: &Path) -> Option<SourceFile> {
    let language = Language::detect(path)?;

    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > MAX_FILE_SIZE {
        return None;
    }

    let bytes = std::fs::read(path).ok()?;

    // Binary sniff through the native accelerator: any NUL in the leading
    // chunk means "not source code".
    let sniff_len = bytes.len().min(BINARY_SNIFF_LEN);
    if sift_sys::count_byte(&bytes[..sniff_len], 0) > 0 {
        return None;
    }

    // tree-sitter node text requires valid UTF-8; enforce once here.
    let text = String::from_utf8(bytes).ok()?;

    let symbols = extract_symbols(language, text.as_bytes());
    let content_hash = sift_sys::hash_bytes(text.as_bytes());

    Some(SourceFile {
        path: path.to_path_buf(),
        language,
        symbols,
        content_hash,
    })
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
