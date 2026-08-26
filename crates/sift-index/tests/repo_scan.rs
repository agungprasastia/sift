//! End-to-end scan of the deterministic fixture repository.

use std::path::{Path, PathBuf};

use sift_core::{Language, SymbolKind};
use sift_index::RepositoryIndex;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample_repo")
}

#[test]
fn scans_expected_files_and_symbols() {
    let index = RepositoryIndex::scan(fixture()).unwrap();

    assert_eq!(index.files().len(), 8, "source files indexed");

    // Ignored trees and gitignored files never make it into the index.
    let all_paths: String = index
        .files()
        .iter()
        .map(|f| f.path.to_string_lossy().into_owned())
        .collect();
    for banned in [
        "secret.txt",
        "node_modules",
        "vendor",
        "ignored_dir",
        "ignored.rs",
        "hidden.rs",
        "binary.rs",
    ] {
        assert!(
            !all_paths.contains(banned),
            "{banned} leaked into the index"
        );
    }

    // Binary sniffing keeps notes.bin and binary.rs out while they still count as scanned.
    assert!(!all_paths.contains("notes.bin"));
    assert!(!all_paths.contains("binary.rs"));

    let stats = index.stats();
    assert_eq!(
        stats.files_scanned, 11,
        "README.md + notes.bin + binary.rs + 8 sources"
    );
    assert_eq!(stats.source_files, 8);
    assert_eq!(stats.symbols_total, 30);

    // Ties are ordered by enum declaration order for determinism.
    assert_eq!(
        stats.languages,
        vec![
            (Language::Rust, 2),
            (Language::C, 1),
            (Language::Cpp, 1),
            (Language::Go, 1),
            (Language::JavaScript, 1),
            (Language::TypeScript, 1),
            (Language::Python, 1),
        ]
    );
}

#[test]
fn finds_symbols_with_location_metadata() {
    let index = RepositoryIndex::scan(fixture()).unwrap();

    let hits = index.find("cleanup_session");
    assert_eq!(hits.len(), 1);

    let hit = &hits[0];
    assert_eq!(hit.symbol.name, "cleanup_session");
    assert_eq!(hit.symbol.kind, SymbolKind::Function);
    assert_eq!(hit.symbol.line, 5);
    assert_eq!(hit.file.language, Language::Rust);
    assert_eq!(
        hit.relative_path(index.root()),
        Path::new("src").join("session.rs")
    );
}

#[test]
fn substring_search_returns_every_match() {
    let index = RepositoryIndex::scan(fixture()).unwrap();

    let hits = index.find("area");
    let names: Vec<&str> = hits.iter().map(|m| m.symbol.name.as_str()).collect();
    assert!(names.contains(&"point_area"));
    assert!(names.contains(&"total_area"));
    assert!(names.contains(&"area"));

    assert!(index.find("definitely_not_present").is_empty());
}

#[test]
fn scanner_handles_nonexistent_root() {
    let bad_path = fixture().join("definitely_nonexistent_path_xyz");
    let result = RepositoryIndex::scan(bad_path);
    assert!(result.is_err(), "nonexistent root must fail");
}

#[test]
fn scanner_diagnostics_and_warning_accessors() {
    let warning = sift_index::ScanWarning {
        path: PathBuf::from("test.rs"),
        message: "unreadable file".to_string(),
    };
    assert_eq!(warning.path(), Path::new("test.rs"));
    assert_eq!(warning.message(), "unreadable file");

    let index = RepositoryIndex::scan(fixture()).unwrap();
    assert_eq!(index.warnings_count(), 0);
    assert_eq!(index.stats().warnings_count, 0);
}
