//! Repository scanner throughput over the workspace's own source tree.

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use sift_index::RepositoryIndex;

fn bench_scanner(c: &mut Criterion) {
    let tree = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("crates");

    let mut group = c.benchmark_group("repository_scan");
    group.bench_function("scan_crates_tree", |b| {
        b.iter(|| match RepositoryIndex::scan(black_box(tree.as_path())) {
            Ok(index) => index.files().len(),
            Err(_) => 0,
        })
    });
    group.finish();
}

criterion_group!(benches, bench_scanner);
criterion_main!(benches);
