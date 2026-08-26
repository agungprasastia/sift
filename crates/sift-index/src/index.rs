//! The repository index built from a scan, plus search and stats.

use std::path::{Path, PathBuf};

use sift_core::{Language, SourceFile, Symbol};

use super::scanner::{ScanError, scan};

/// A symbol together with the file it was found in.
#[derive(Debug, Clone, Copy)]
pub struct Match<'a> {
    pub file: &'a SourceFile,
    pub symbol: &'a Symbol,
}

impl Match<'_> {
    /// Path of the containing file relative to the scanned root.
    #[must_use]
    pub fn relative_path(&self, root: &Path) -> PathBuf {
        self.file
            .path
            .strip_prefix(root)
            .unwrap_or(&self.file.path)
            .to_path_buf()
    }
}

/// Aggregated repository statistics.
#[derive(Debug, Clone)]
pub struct Stats {
    pub files_scanned: u64,
    pub source_files: usize,
    pub symbols_total: usize,
    /// Per-language source-file counts, sorted by count desc then name asc.
    pub languages: Vec<(Language, usize)>,
}

/// Immutable index of one repository scan.
#[derive(Debug, Clone)]
pub struct RepositoryIndex {
    root: PathBuf,
    files: Vec<SourceFile>,
    files_scanned: u64,
}

impl RepositoryIndex {
    /// Scans `root` and builds the index.
    ///
    /// # Errors
    /// Returns [`ScanError`] when the walk root itself cannot be read.
    pub fn scan(root: impl Into<PathBuf>) -> Result<Self, ScanError> {
        let root = root.into();
        let output = scan(&root)?;
        Ok(Self {
            root,
            files: output.files,
            files_scanned: output.files_scanned,
        })
    }

    /// The root this index was built from.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Assembles an index from pre-scanned parts; the entry point for
    /// in-memory indexes in later milestones.
    #[must_use]
    pub fn from_parts(root: PathBuf, files: Vec<SourceFile>, files_scanned: u64) -> Self {
        let mut sorted = files;
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        Self {
            root,
            files: sorted,
            files_scanned,
        }
    }

    /// All indexed source files, sorted by path.
    #[must_use]
    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// Case-sensitive substring search over symbol names.
    #[must_use]
    pub fn find(&self, query: &str) -> Vec<Match<'_>> {
        self.files
            .iter()
            .flat_map(|file| {
                file.symbols
                    .iter()
                    .map(move |symbol| Match { file, symbol })
            })
            .filter(|m| m.symbol.name.contains(query))
            .collect()
    }

    /// Aggregated statistics for CLI reporting.
    #[must_use]
    pub fn stats(&self) -> Stats {
        let mut per_language: Vec<(Language, usize)> = Vec::new();
        for file in &self.files {
            match per_language
                .iter_mut()
                .find(|(lang, _)| *lang == file.language)
            {
                Some((_, n)) => *n += 1,
                None => per_language.push((file.language, 1)),
            }
        }
        per_language.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        Stats {
            files_scanned: self.files_scanned,
            source_files: self.files.len(),
            symbols_total: self.files.iter().map(|f| f.symbols.len()).sum(),
            languages: per_language,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sift_core::SymbolKind;

    fn sample(path: &str, name: &str, kind: SymbolKind) -> SourceFile {
        SourceFile {
            path: PathBuf::from(path),
            language: Language::Rust,
            symbols: vec![Symbol {
                name: name.to_string(),
                kind,
                line: 1,
            }],
            content_hash: 0,
        }
    }

    #[test]
    fn find_matches_substrings_across_files() {
        let index = RepositoryIndex::from_parts(
            PathBuf::from("."),
            vec![
                sample("src/a.rs", "cleanup_session", SymbolKind::Function),
                sample("src/b.rs", "cleanup_all", SymbolKind::Function),
                sample("src/c.rs", "login", SymbolKind::Function),
            ],
            3,
        );

        let hits = index.find("cleanup");
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|m| m.symbol.name.contains("cleanup")));
        assert!(index.find("zzz").is_empty());
    }

    #[test]
    fn stats_orders_languages_by_count_then_name() {
        let files = vec![
            sample("a.rs", "x", SymbolKind::Function),
            sample("b.rs", "y", SymbolKind::Function),
            sample("c.go", "z", SymbolKind::Function),
        ];
        let mut files = files;
        files[2].language = Language::Go;
        let index = RepositoryIndex::from_parts(PathBuf::from("."), files, 9);

        let stats = index.stats();
        assert_eq!(stats.source_files, 3);
        assert_eq!(stats.symbols_total, 3);
        assert_eq!(stats.files_scanned, 9);
        assert_eq!(
            stats.languages,
            vec![(Language::Rust, 2), (Language::Go, 1)]
        );
    }
}
