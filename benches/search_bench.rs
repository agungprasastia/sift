//! Search benchmarks comparing pure Rust substring search implementations:
//! `str::contains`, `memmem::find` (one-shot), and `memmem::Finder` (reusable).

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use memchr::memmem;

fn generate_corpus(size: usize, seed: u64) -> Vec<u8> {
    let mut x = seed;
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789_ \n";
    let mut data = Vec::with_capacity(size);
    for _ in 0..size {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let idx = (x % ALPHABET.len() as u64) as usize;
        data.push(ALPHABET[idx]);
    }
    data
}

fn with_needle_at(mut data: Vec<u8>, offset: usize, needle: &[u8]) -> Vec<u8> {
    assert!(offset + needle.len() <= data.len());
    data[offset..offset + needle.len()].copy_from_slice(needle);
    data
}

fn bench_search_implementations(c: &mut Criterion) {
    let mut group = c.benchmark_group("substring_search_rust");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);

    let sizes = [
        ("1KB", 1024),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
    ];

    let needle_str = "__TARGET_SYMBOL_NAME__";
    let needle_bytes = needle_str.as_bytes();
    let finder = memmem::Finder::new(needle_bytes);

    for (label, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));

        let mid_pos = size / 2;
        let hay_mid = with_needle_at(generate_corpus(size, 0x1234), mid_pos, needle_bytes);
        let hay_str = String::from_utf8(hay_mid.clone()).expect("valid ascii corpus");

        group.bench_with_input(
            BenchmarkId::new("str_contains/middle", label),
            &hay_str,
            |b, text| {
                b.iter(|| black_box(text).contains(black_box(needle_str)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("memmem_oneshot/middle", label),
            &hay_mid,
            |b, data| {
                b.iter(|| memmem::find(black_box(data), black_box(needle_bytes)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("memmem_reused_finder/middle", label),
            &hay_mid,
            |b, data| {
                b.iter(|| finder.find(black_box(data)));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_search_implementations);
criterion_main!(benches);
