//! C vs plain-Rust baselines over a realistic ~250 KB source payload.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use sift_sys::{count_byte, find_bytes, hash_bytes};

const UNIT: &str = "fn sample(x: usize) -> usize {\n    let acc = x.wrapping_mul(31);\n    struct Frame { id: u32 }\n    acc + Frame { id: 1 }.id as usize\n}\n\nimpl Sample {\n    fn build() -> Self {\n        Self {}\n    }\n}\n\n";

/* ---------- Rust reference implementations ---------- */

fn baseline_find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > haystack.len() {
        return None;
    }
    let last = haystack.len() - needle.len();
    (0..=last).find(|&i| &haystack[i..i + needle.len()] == needle)
}

fn baseline_hash(data: &[u8]) -> u64 {
    const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    data.iter().fold(BASIS, |acc, &byte| {
        (acc ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

fn baseline_count(data: &[u8], value: u8) -> usize {
    data.iter().filter(|&&byte| byte == value).count()
}

fn bench_ffi(c: &mut Criterion) {
    let data_vec = UNIT.repeat(2000).into_bytes();
    let data = data_vec.as_slice();
    let present = b"wrapping_mul".as_slice();
    let absent = b"zzz_never_present".as_slice();

    let mut group = c.benchmark_group("find_bytes");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("c/present", |b| {
        b.iter(|| find_bytes(black_box(data), black_box(present)))
    });
    group.bench_function("rust/present", |b| {
        b.iter(|| baseline_find(black_box(data), black_box(present)))
    });
    group.bench_function("c/absent", |b| {
        b.iter(|| find_bytes(black_box(data), black_box(absent)))
    });
    group.bench_function("rust/absent", |b| {
        b.iter(|| baseline_find(black_box(data), black_box(absent)))
    });
    group.finish();

    let mut group = c.benchmark_group("hash_bytes");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("c", |b| b.iter(|| hash_bytes(black_box(data))));
    group.bench_function("rust", |b| b.iter(|| baseline_hash(black_box(data))));
    group.finish();

    let mut group = c.benchmark_group("count_byte");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("c", |b| b.iter(|| count_byte(black_box(data), b'\n')));
    group.bench_function("rust", |b| {
        b.iter(|| baseline_count(black_box(data), b'\n'))
    });
    group.finish();
}

criterion_group!(benches, bench_ffi);
criterion_main!(benches);
