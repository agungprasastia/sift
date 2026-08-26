//! Comprehensive and verified C vs Rust benchmarks on deterministic non-repetitive datasets.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use memchr::memmem;
use sift_sys::{Arena, count_byte, find_bytes, find_many, hash_bytes, index_newlines};

fn generate_corpus(size: usize, seed: u64) -> Vec<u8> {
    let mut x = seed;
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
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

/* ---------- Rust baseline implementations ---------- */

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

    let needle = b"__TARGET_NEEDLE_EXACT__".as_slice();
    let absent = b"__ABSENT_NEEDLE_EXACT__".as_slice();

    for (label, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));

        let hay_beg = with_needle_at(generate_corpus(size, 0x1111), 0, needle);
        assert_eq!(memmem::find(&hay_beg, needle), Some(0));
        assert_eq!(find_bytes(&hay_beg, needle), Some(0));

        group.bench_with_input(
            BenchmarkId::new("c/beginning", label),
            &hay_beg,
            |b, data| {
                b.iter(|| find_bytes(black_box(data), black_box(needle)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rust_memmem_oneshot/beginning", label),
            &hay_beg,
            |b, data| {
                b.iter(|| memmem::find(black_box(data), black_box(needle)));
            },
        );
        if size <= 10 * 1024 {
            group.bench_with_input(
                BenchmarkId::new("rust_naive/beginning", label),
                &hay_beg,
                |b, data| {
                    b.iter(|| baseline_find(black_box(data), black_box(needle)));
                },
            );
        }

        let mid_pos = size / 2;
        let hay_mid = with_needle_at(generate_corpus(size, 0x2222), mid_pos, needle);
        assert_eq!(memmem::find(&hay_mid, needle), Some(mid_pos));
        assert_eq!(find_bytes(&hay_mid, needle), Some(mid_pos));

        group.bench_with_input(BenchmarkId::new("c/middle", label), &hay_mid, |b, data| {
            b.iter(|| find_bytes(black_box(data), black_box(needle)));
        });
        group.bench_with_input(
            BenchmarkId::new("rust_memmem_oneshot/middle", label),
            &hay_mid,
            |b, data| {
                b.iter(|| memmem::find(black_box(data), black_box(needle)));
            },
        );

        let hay_absent = generate_corpus(size, 0x3333);
        assert_eq!(memmem::find(&hay_absent, absent), None);
        assert_eq!(find_bytes(&hay_absent, absent), None);

        group.bench_with_input(
            BenchmarkId::new("c/absent", label),
            &hay_absent,
            |b, data| {
                b.iter(|| find_bytes(black_box(data), black_box(absent)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rust_memmem_oneshot/absent", label),
            &hay_absent,
            |b, data| {
                b.iter(|| memmem::find(black_box(data), black_box(absent)));
            },
        );
        if size <= 10 * 1024 {
            group.bench_with_input(
                BenchmarkId::new("rust_naive/absent", label),
                &hay_absent,
                |b, data| {
                    b.iter(|| baseline_find(black_box(data), black_box(absent)));
                },
            );
        }
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

    let needle = b"__TARGET_NEEDLE_EXACT__".as_slice();
    let absent = b"__ABSENT_NEEDLE_EXACT__".as_slice();
    let finder_needle = memmem::Finder::new(needle);
    let finder_absent = memmem::Finder::new(absent);

    for (label, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));

        let hay_beg = with_needle_at(generate_corpus(size, 0x4444), 0, needle);
        assert_eq!(finder_needle.find(&hay_beg), Some(0));
        assert_eq!(find_bytes(&hay_beg, needle), Some(0));

        group.bench_with_input(
            BenchmarkId::new("c/beginning", label),
            &hay_beg,
            |b, data| {
                b.iter(|| find_bytes(black_box(data), black_box(needle)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder/beginning", label),
            &hay_beg,
            |b, data| {
                b.iter(|| finder_needle.find(black_box(data)));
            },
        );

        let mid_pos = size / 2;
        let hay_mid = with_needle_at(generate_corpus(size, 0x5555), mid_pos, needle);
        assert_eq!(finder_needle.find(&hay_mid), Some(mid_pos));
        assert_eq!(find_bytes(&hay_mid, needle), Some(mid_pos));

        group.bench_with_input(BenchmarkId::new("c/middle", label), &hay_mid, |b, data| {
            b.iter(|| find_bytes(black_box(data), black_box(needle)));
        });
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder/middle", label),
            &hay_mid,
            |b, data| {
                b.iter(|| finder_needle.find(black_box(data)));
            },
        );

        let end_pos = size - needle.len();
        let hay_end = with_needle_at(generate_corpus(size, 0x6666), end_pos, needle);
        assert_eq!(finder_needle.find(&hay_end), Some(end_pos));
        assert_eq!(find_bytes(&hay_end, needle), Some(end_pos));

        group.bench_with_input(BenchmarkId::new("c/end", label), &hay_end, |b, data| {
            b.iter(|| find_bytes(black_box(data), black_box(needle)));
        });
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder/end", label),
            &hay_end,
            |b, data| {
                b.iter(|| finder_needle.find(black_box(data)));
            },
        );

        let hay_absent = generate_corpus(size, 0x7777);
        assert_eq!(finder_absent.find(&hay_absent), None);
        assert_eq!(find_bytes(&hay_absent, absent), None);

        group.bench_with_input(
            BenchmarkId::new("c/absent", label),
            &hay_absent,
            |b, data| {
                b.iter(|| find_bytes(black_box(data), black_box(absent)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder/absent", label),
            &hay_absent,
            |b, data| {
                b.iter(|| finder_absent.find(black_box(data)));
            },
        );
    }
    group.finish();
}

fn bench_adversarial_repetitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("adversarial_repetitive");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);

    let sizes = [("1KB", 1024), ("100KB", 100 * 1024), ("1MB", 1024 * 1024)];

    for (label, size) in sizes {
        // Pattern: "aaaa...aab" searched inside "aaaa...aaaa"
        let needle = b"aaaaaaaaaaaaaaab".as_slice();
        let payload = vec![b'a'; size];
        let finder = memmem::Finder::new(needle);

        assert_eq!(find_bytes(&payload, needle), None);
        assert_eq!(finder.find(&payload), None);

        group.bench_with_input(BenchmarkId::new("c", label), &payload, |b, data| {
            b.iter(|| find_bytes(black_box(data), black_box(needle)));
        });
        group.bench_with_input(
            BenchmarkId::new("rust_reused_finder", label),
            &payload,
            |b, data| {
                b.iter(|| finder.find(black_box(data)));
            },
        );
    }
    group.finish();
}

fn bench_batch_search(c: &mut Criterion) {
    let corpus = generate_corpus(256 * 1024, 0x8888);
    let n1 = b"__N1__".as_slice();
    let n2 = b"__N2__".as_slice();
    let n3 = b"__N3__".as_slice();
    let n_absent = b"__N_ABS__".as_slice();

    let mut hay = corpus;
    hay = with_needle_at(hay, 1000, n1);
    hay = with_needle_at(hay, 50000, n2);
    hay = with_needle_at(hay, 150000, n3);

    let needles: &[&[u8]] = &[n1, n2, n3, n_absent];

    let mut group = c.benchmark_group("batch_search");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(hay.len() as u64));

    group.bench_function("c_find_many", |b| {
        let mut out = [sift_sys::Match {
            needle_index: 0,
            offset: 0,
        }; 4];
        b.iter(|| {
            let _ = find_many(black_box(&hay), black_box(needles), black_box(&mut out));
        })
    });

    group.bench_function("repeated_reused_finders", |b| {
        let finders: Vec<memmem::Finder> = needles.iter().copied().map(memmem::Finder::new).collect();
        b.iter(|| {
            let mut matches = Vec::with_capacity(needles.len());
            for (i, finder) in finders.iter().enumerate() {
                if let Some(offset) = finder.find(black_box(&hay)) {
                    matches.push((i, offset));
                }
            }
            matches
        })
    });

    group.finish();
}

fn bench_newline_index(c: &mut Criterion) {
    let mut payload = generate_corpus(256 * 1024, 0x9999);
    for i in (0..payload.len()).step_by(60) {
        payload[i] = b'\n';
    }
    let mut out = vec![0usize; payload.len() / 10];

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
    let data = generate_corpus(256 * 1024, 0xAAAA);

    let mut group = c.benchmark_group("hash_bytes");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("c", |b| b.iter(|| hash_bytes(black_box(&data))));
    group.bench_function("rust", |b| b.iter(|| baseline_hash(black_box(&data))));
    group.finish();

    let mut group = c.benchmark_group("count_byte");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(700));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("c", |b| b.iter(|| count_byte(black_box(&data), b'a')));
    group.bench_function("rust_iterator", |b| {
        b.iter(|| baseline_count(black_box(&data), b'a'))
    });
    group.bench_function("rust_memchr", |b| {
        b.iter(|| memchr::memchr_iter(b'a', black_box(&data)).count())
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
    bench_adversarial_repetitive,
    bench_batch_search,
    bench_newline_index,
    bench_primitives
);
criterion_main!(benches);
