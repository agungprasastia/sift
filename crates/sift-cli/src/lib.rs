//! Sift command line interface: `sift map`, `sift find`, `sift stats`.

use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};
use sift_index::RepositoryIndex;

/// Sift — context/token optimizer for coding AI (M0).
#[derive(Debug, Parser)]
#[command(name = "sift", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print a tree of source files and their symbols.
    Map {
        /// Repository root to scan.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Find symbols by name (case-sensitive substring).
    Find {
        /// Symbol name or fragment.
        symbol: String,
        /// Repository root to scan.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Print repository statistics.
    Stats {
        /// Repository root to scan.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

/// Runs one command, writing its report to `out`. Returns bytes written.
///
/// # Errors
/// Propagates filesystem/scan failures with context attached.
pub fn execute(command: Command, out: &mut impl Write) -> anyhow::Result<usize> {
    match command {
        Command::Map { path } => emit(out, render_map(&scan(&path)?)),
        Command::Find { symbol, path } => emit(out, render_find(&scan(&path)?, &symbol)),
        Command::Stats { path } => emit(out, render_stats(&scan(&path)?)),
    }
}

fn scan(path: &Path) -> anyhow::Result<RepositoryIndex> {
    RepositoryIndex::scan(path).with_context(|| format!("cannot scan `{}`", path.display()))
}

fn emit(out: &mut impl Write, report: String) -> anyhow::Result<usize> {
    out.write_all(report.as_bytes())?;
    Ok(report.len())
}

/// `sift map` rendering: directory groups with tree connectors.
#[must_use]
pub fn render_map(index: &RepositoryIndex) -> String {
    let mut grouped: std::collections::BTreeMap<String, Vec<(String, &sift_core::SourceFile)>> =
        std::collections::BTreeMap::new();

    for file in index.files() {
        let rel = file.path.strip_prefix(index.root()).unwrap_or(&file.path);
        let dir = rel
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = rel
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        grouped.entry(dir).or_default().push((name, file));
    }

    let mut body = String::new();
    for (dir, files) in grouped {
        if !body.is_empty() {
            body.push('\n');
        }
        let dir_display = if dir.is_empty() { "." } else { dir.as_str() };
        let _ = writeln!(body, "{dir_display}/");

        let file_count = files.len();
        for (i, (name, file)) in files.iter().enumerate() {
            let last_file = i + 1 == file_count;
            let connector = if last_file { "└──" } else { "├──" };
            let indent = if last_file { "    " } else { "│   " };
            let _ = writeln!(body, "{connector} {name}");

            let symbol_count = file.symbols.len();
            for (j, symbol) in file.symbols.iter().enumerate() {
                let sym_connector = if j + 1 == symbol_count {
                    "└──"
                } else {
                    "├──"
                };
                let _ = writeln!(
                    body,
                    "{indent}{sym_connector} {} {}",
                    symbol.kind.short_label(),
                    symbol.name
                );
            }
        }
    }
    body
}

/// `sift find` rendering: one block per match.
#[must_use]
pub fn render_find(index: &RepositoryIndex, query: &str) -> String {
    let blocks: Vec<String> = index
        .find(query)
        .iter()
        .map(|m| {
            format!(
                "{}\nkind: {}\nfile: {}\nline: {}\nlanguage: {}",
                m.symbol.name,
                m.symbol.kind.long_label(),
                m.relative_path(index.root()).to_string_lossy(),
                m.symbol.line,
                m.file.language
            )
        })
        .collect();

    let mut out = blocks.join("\n\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// `sift stats` rendering.
#[must_use]
pub fn render_stats(index: &RepositoryIndex) -> String {
    let stats = index.stats();
    let mut body = String::new();

    let _ = writeln!(body, "Files scanned: {}", stats.files_scanned);
    let _ = writeln!(body, "Source files: {}", stats.source_files);
    let _ = writeln!(body, "Symbols: {}", thousands(stats.symbols_total as u64));
    let _ = writeln!(body);
    let _ = writeln!(body, "Languages:");

    let width = stats
        .languages
        .iter()
        .map(|(lang, _)| lang.name().len())
        .max()
        .unwrap_or(0)
        .max(10)
        + 2;
    for (lang, count) in &stats.languages {
        let _ = writeln!(body, "{:<width$}{}", lang.name(), count, width = width);
    }

    body
}

/// Groups an integer with comma thousand separators (`1438` -> `1,438`).
#[must_use]
pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + len.saturating_sub(1) / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sift_core::Language;

    #[test]
    fn thousands_groups_digits() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_438), "1,438");
        assert_eq!(thousands(12_345_678), "12,345,678");
    }

    #[test]
    fn stats_of_empty_index_lists_no_languages() {
        let index = RepositoryIndex::from_parts(PathBuf::from("."), vec![], 0);
        let rendered = render_stats(&index);
        assert!(rendered.contains("Files scanned: 0"));
        assert!(!rendered.contains("Native engine"));
        assert!(!rendered.contains("Backend:"));
        assert!(!rendered.contains(Language::Rust.name()));
    }
}
