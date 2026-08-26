//! Workspace-level acceptance test: scan the fixture repository and verify
//! all three CLI reports end to end through `sift_cli::execute`.

use std::path::{Path, PathBuf};

use sift_cli::{Command, execute};
use sift_index::RepositoryIndex;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("crates/sift-index/tests/fixtures/sample_repo")
}

#[test]
fn map_command_produces_report() {
    let mut sink: Vec<u8> = Vec::new();
    let written = execute(Command::Map { path: fixture() }, &mut sink).unwrap();

    assert!(written > 100);
    let text = String::from_utf8(sink).unwrap();
    assert!(text.contains("├── auth.rs"));
    assert!(text.contains("└── fn cleanup_session"));
}

#[test]
fn find_command_reports_matches() {
    let mut sink: Vec<u8> = Vec::new();
    execute(
        Command::Find {
            symbol: "cleanup_session".into(),
            path: fixture(),
        },
        &mut sink,
    )
    .unwrap();

    let text = String::from_utf8(sink).unwrap();
    assert!(text.starts_with("cleanup_session\nkind: function"));
    assert!(text.contains("line: 5"));
    assert!(text.contains("language: Rust"));
}

#[test]
fn stats_command_prints_native_backend() {
    let mut sink: Vec<u8> = Vec::new();
    execute(Command::Stats { path: fixture() }, &mut sink).unwrap();

    let text = String::from_utf8(sink).unwrap();
    assert!(text.contains("Files scanned: 11"));
    assert!(text.contains("Source files: 8"));
    assert!(text.contains("Symbols: 30"));
    assert!(text.contains("Native engine: enabled"));
    assert!(text.ends_with("Backend: C11\n"));
}

#[test]
fn index_and_renderers_agree_on_relative_paths() {
    let index = RepositoryIndex::scan(fixture()).unwrap();
    let hit = &index.find("authenticate")[0];
    assert_eq!(
        hit.relative_path(index.root()),
        Path::new("src").join("auth.rs")
    );
}
