//! Output rendering checks against the shared fixture repository.

use std::path::{Path, PathBuf};

use sift_cli::{render_find, render_map, render_stats};
use sift_index::RepositoryIndex;

fn fixture() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../sift-index/tests/fixtures/sample_repo")
}

#[test]
fn map_renders_tree_groups() {
    let index = RepositoryIndex::scan(fixture()).unwrap();
    let rendered = render_map(&index);

    assert!(rendered.contains("./"), "root-level group header");
    assert!(rendered.contains("src/\n"));
    assert!(rendered.contains("├── auth.rs"));
    assert!(rendered.contains("│   ├── struct User"));
    assert!(rendered.contains("│   ├── fn login"));
    assert!(rendered.contains("└── session.rs"));
    assert!(rendered.contains("    └── fn cleanup_session"));
    assert!(rendered.contains("interface Options"));
    assert!(rendered.contains("class Shape"));
}

#[test]
fn find_renders_one_block_per_match() {
    let index = RepositoryIndex::scan(fixture()).unwrap();
    let rendered = render_find(&index, "cleanup_session");

    let expected_file = Path::new("src").join("session.rs");
    let expected = format!(
        "cleanup_session\nkind: function\nfile: {}\nline: 5\nlanguage: Rust\n",
        expected_file.to_string_lossy()
    );
    assert_eq!(rendered, expected);

    let multiple = render_find(&index, "a");
    assert!(
        multiple.matches("\n\n").count() >= 1,
        "blocks separated by blank lines"
    );

    assert_eq!(render_find(&index, "no_such_symbol"), "");
}

#[test]
fn stats_renders_full_report() {
    let index = RepositoryIndex::scan(fixture()).unwrap();
    let rendered = render_stats(&index);

    assert!(rendered.starts_with("Files scanned: 11\n"));
    assert!(rendered.contains("Source files: 8\n"));
    assert!(rendered.contains("Symbols: 30\n"));
    assert!(rendered.contains("\nLanguages:\n"));
    assert!(!rendered.contains("Native engine"));
    assert!(!rendered.contains("Backend:"));
}
