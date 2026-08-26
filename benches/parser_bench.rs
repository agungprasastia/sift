//! Symbol extraction throughput on a synthetic multi-declaration Rust file.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use sift_core::Language;
use sift_parser::extract_symbols;

const UNIT: &str = "pub struct Item { pub id: u32 }\n\npub fn process(item: &Item) -> u32 {\n    item.id.wrapping_add(7)\n}\n\nimpl Item {\n    pub fn new(id: u32) -> Self {\n        Self { id }\n    }\n\n    pub const KIND: &'static str = \"item\";\n}\n\n";

fn bench_extraction(c: &mut Criterion) {
    let source = UNIT.repeat(300);

    let mut group = c.benchmark_group("symbol_extraction");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("rust_source", |b| {
        b.iter(|| extract_symbols(Language::Rust, black_box(source.as_bytes())).len())
    });
    group.finish();
}

criterion_group!(benches, bench_extraction);
criterion_main!(benches);
