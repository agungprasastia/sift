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
    for banned in ["secret.txt", "node_modules", "vendor", "ignored_dir"] {
        assert!(
            !all_paths.contains(banned),
            "{banned} leaked into the index"
        );
    }

    // Binary sniffing keeps notes.bin out while it still counts as scanned.
    assert!(!all_paths.contains("notes.bin"));

    let stats = index.stats();
    assert_eq!(stats.files_scanned, 10, "README.md + notes.bin + 8 sources");
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
