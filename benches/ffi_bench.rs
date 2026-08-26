//! Comprehensive C vs Rust benchmarks (Naive Rust & Optimized Rust via memchr / memmem::Finder).

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use memchr::memmem;
use sift_sys::{Arena, count_byte, find_bytes, find_many, hash_bytes, index_newlines};

const UNIT: &str = "fn sample(x: usize) -> usize {\n    let acc = x.wrapping_mul(31);\n    struct Frame { id: u32 }\n    acc + Frame { id: 1 }.id as usize\n}\n\nimpl Sample {\n    fn build() -> Self {\n        Self {}\n    }\n}\n\n";

/* ---------- Rust baseline implementations ---------- */

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

fn baseline_index_newlines(data: &[u8], output: &mut [usize]) -> usize {
    let mut count = 0;
    for (i, &b) in data.iter().enumerate() {
        if b == b'\n' {
            output[count] = i;
            count += 1;
            if count >= output.len() {
                break;
            }
        }
    }
    count
}

fn bench_find_bytes_oneshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_bytes_oneshot");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);

    let sizes = [
        ("1KB", 1024),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
    ];

    for (label, size) in sizes {
        let repeat_count = (size / UNIT.len()) + 1;
        let full_data = UNIT.repeat(repeat_count).into_bytes();
        let payload = &full_data[..size];

        group.throughput(Throughput::Bytes(size as u64));

        // 1. Beginning position
        let needle_beg = b"fn sample".as_slice();
        group.bench_with_input(
            BenchmarkId::new("c/beginning", label),
            &payload,
            |b, &data| {
                b.iter(|| find_bytes(black_box(data), black_box(needle_beg)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rust_memmem_oneshot/beginning", label),
            &payload,
            |b, &data| {
                b.iter(|| memmem::find(black_box(data), black_box(needle_beg)));
            },
        );

        // 2. Middle position
        let mid_offset = size / 2;
        let needle_mid = if size >= 64 {
            &payload[mid_offset..mid_offset + 12]
        } else {
            needle_beg
        };
        group.bench_with_input(BenchmarkId::new("c/middle", label), &payload, |b, &data| {
            b.iter(|| find_bytes(black_box(data), black_box(needle_mid)));
        });
        group.bench_with_input(
            BenchmarkId::new("rust_memmem_oneshot/middle", label),
            &payload,
            |b, &data| {
                b.iter(|| memmem::find(black_box(data), black_box(needle_mid)));
            },
        );

        // 3. Absent
        let needle_absent = b"__definitely_not_present_in_source__".as_slice();
        group.bench_with_input(BenchmarkId::new("c/absent", label), &payload, |b, &data| {
            b.iter(|| find_bytes(black_box(data), black_box(needle_absent)));
        });
        group.bench_with_input(
            BenchmarkId::new("rust_memmem_oneshot/absent", label),
            &payload,
            |b, &data| {
                b.iter(|| memmem::find(black_box(data), black_box(needle_absent)));
            },
        );
    }
    group.finish();
}

fn bench_find_bytes_reused_finder(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_bytes_reused_finder");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);

    let sizes = [
        ("1KB", 1024),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
        ("10MB", 10 * 1024 * 1024),
    ];

    for (label, size) in sizes {
        let repeat_count = (size / UNIT.len()) + 1;
        let full_data = UNIT.repeat(repeat_count).into_bytes();
        let payload = &full_data[..size];

        group.throughput(Throughput::Bytes(size as u64));

        // 1. Beginning position
        let needle_beg = b"fn sample".as_slice();
        let finder_beg = memmem::Finder::new(needle_beg);
        group.bench_with_input(
            BenchmarkId::new("c/beginning", label),
            &payload,
            |b, &data| {
                b.iter(|| find_bytes(black_box(data), black_box(needle_beg)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder/beginning", label),
            &payload,
            |b, &data| {
                b.iter(|| finder_beg.find(black_box(data)));
            },
        );

        // 2. Middle position
        let mid_offset = size / 2;
        let needle_mid = if size >= 64 {
            &payload[mid_offset..mid_offset + 12]
        } else {
            needle_beg
        };
        let finder_mid = memmem::Finder::new(needle_mid);
        group.bench_with_input(BenchmarkId::new("c/middle", label), &payload, |b, &data| {
            b.iter(|| find_bytes(black_box(data), black_box(needle_mid)));
        });
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder/middle", label),
            &payload,
            |b, &data| {
                b.iter(|| finder_mid.find(black_box(data)));
            },
        );

        // 3. End position
        let end_offset = size.saturating_sub(20);
        let needle_end = &payload[end_offset..];
        let finder_end = memmem::Finder::new(needle_end);
        group.bench_with_input(BenchmarkId::new("c/end", label), &payload, |b, &data| {
            b.iter(|| find_bytes(black_box(data), black_box(needle_end)));
        });
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder/end", label),
            &payload,
            |b, &data| {
                b.iter(|| finder_end.find(black_box(data)));
            },
        );

        // 4. Absent
        let needle_absent = b"__definitely_not_present_in_source__".as_slice();
        let finder_absent = memmem::Finder::new(needle_absent);
        group.bench_with_input(BenchmarkId::new("c/absent", label), &payload, |b, &data| {
            b.iter(|| find_bytes(black_box(data), black_box(needle_absent)));
        });
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder/absent", label),
            &payload,
            |b, &data| {
                b.iter(|| finder_absent.find(black_box(data)));
            },
        );

        // 5. Repetitive pattern
        let needle_rep = b"a".repeat(16);
        let rep_payload = b"a".repeat(size);
        let finder_rep = memmem::Finder::new(&needle_rep);
        group.bench_with_input(
            BenchmarkId::new("c/repetitive", label),
            &&rep_payload[..],
            |b, &data| {
                b.iter(|| find_bytes(black_box(data), black_box(&needle_rep)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder/repetitive", label),
            &&rep_payload[..],
            |b, &data| {
                b.iter(|| finder_rep.find(black_box(data)));
            },
        );
    }
    group.finish();
}

fn bench_batch_search(c: &mut Criterion) {
    let payload = UNIT.repeat(1000).into_bytes();
    let needles: &[&[u8]] = &[
        b"wrapping_mul",
        b"struct Frame",
        b"impl Sample",
        b"absent_symbol_alpha",
        b"absent_symbol_beta",
    ];

    let mut group = c.benchmark_group("batch_search");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(payload.len() as u64));

    group.bench_function("c_find_many", |b| {
        let mut out = [sift_sys::Match {
            needle_index: 0,
            offset: 0,
        }; 5];
        b.iter(|| {
            let _ = find_many(black_box(&payload), black_box(needles), black_box(&mut out));
        })
    });

    group.bench_function("repeated_memmem", |b| {
        b.iter(|| {
            let mut matches = Vec::with_capacity(needles.len());
            for (i, needle) in needles.iter().enumerate() {
                if let Some(offset) = memmem::find(black_box(&payload), black_box(needle)) {
                    matches.push((i, offset));
                }
            }
            matches
        })
    });

    group.finish();
}

fn bench_newline_index(c: &mut Criterion) {
    let payload = UNIT.repeat(1000).into_bytes();
    let mut out = vec![0usize; payload.len() / 5];

    let mut group = c.benchmark_group("index_newlines");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(payload.len() as u64));

    group.bench_function("c", |b| {
        b.iter(|| {
            let _ = index_newlines(black_box(&payload), black_box(&mut out));
        })
    });
    group.bench_function("rust_memchr_iter", |b| {
        b.iter(|| {
            let mut count = 0;
            for offset in memchr::memchr_iter(b'\n', black_box(&payload)) {
                out[count] = offset;
                count += 1;
            }
            count
        })
    });
    group.bench_function("rust_baseline", |b| {
        b.iter(|| baseline_index_newlines(black_box(&payload), black_box(&mut out)))
    });
    group.finish();
}

fn bench_primitives(c: &mut Criterion) {
    let data_vec = UNIT.repeat(2000).into_bytes();
    let data = data_vec.as_slice();

    let mut group = c.benchmark_group("hash_bytes");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("c", |b| b.iter(|| hash_bytes(black_box(data))));
    group.bench_function("rust", |b| b.iter(|| baseline_hash(black_box(data))));
    group.finish();

    let mut group = c.benchmark_group("count_byte");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("c", |b| b.iter(|| count_byte(black_box(data), b'\n')));
    group.bench_function("rust_iterator", |b| {
        b.iter(|| baseline_count(black_box(data), b'\n'))
    });
    group.bench_function("rust_memchr", |b| {
        b.iter(|| memchr::memchr_iter(b'\n', black_box(data)).count())
    });
    group.finish();

    let mut group = c.benchmark_group("arena_vs_vec_scratch");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);
    group.bench_function("c_arena_alloc_reset", |b| {
        let mut arena = Arena::new(65536).expect("arena");
        b.iter(|| {
            arena.reset();
            for _ in 0..50 {
                let _ = black_box(arena.alloc_bytes(64, 8));
            }
        })
    });
    group.bench_function("rust_vec_reuse", |b| {
        let mut buf = Vec::with_capacity(65536);
        b.iter(|| {
            buf.clear();
            for _ in 0..50 {
                buf.extend_from_slice(&[0u8; 64]);
                black_box(&buf);
            }
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_find_bytes_oneshot,
    bench_find_bytes_reused_finder,
    bench_batch_search,
    bench_newline_index,
    bench_primitives
);
criterion_main!(benches);
