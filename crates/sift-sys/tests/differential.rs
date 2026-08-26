//! Differential tests: every C primitive must produce byte-identical results
//! to a simple Rust reference implementation on identical inputs.
//!
//! These tests are the designated correctness proof for the FFI boundary
//! (real compiled C cannot run under Miri). Inputs cover the defensive edge
//! cases from `sift_native.h` plus deterministic pseudo-random buffers.

use sift_sys::{count_byte, find_bytes, hash_bytes};

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
    data.iter().filter(|&&b| b == value).count()
}

/* ---------- deterministic PRNG (xorshift64*) ---------- */

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        debug_assert!(bound > 0);
        (self.next() % bound as u64) as usize
    }
}

/* ---------- fixed edge cases ---------- */

#[test]
fn fixed_cases_match_baseline() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"", b"a"),
        (b"abc", b""),
        (b"abc", b"c"),
        (b"abc", b"abcd"),          // needle longer than haystack
        (b"aaaa", b"aa"),           // repeated pattern
        (b"aaaab", b"aab"),         // repeated with tail
        (&[0u8; 16], &[0u8; 4]),    // NUL-heavy data
        (&[255u8; 8], &[255u8; 3]), // high bytes
        (b"hello\0world", b"\0w"),  // NUL inside both sides
        (b"mississippi", b"issip"), // overlapping candidates
    ];

    for &(haystack, needle) in cases {
        assert_eq!(
            find_bytes(haystack, needle),
            baseline_find(haystack, needle),
            "find mismatch for {haystack:?} / {needle:?}"
        );
        assert_eq!(
            hash_bytes(haystack),
            baseline_hash(haystack),
            "hash mismatch"
        );
        assert_eq!(
            count_byte(haystack, 0),
            baseline_count(haystack, 0),
            "count NUL"
        );
        assert_eq!(
            count_byte(haystack, b'a'),
            baseline_count(haystack, b'a'),
            "count 'a'"
        );
    }
}

#[test]
fn hash_is_consistent_and_discriminates() {
    let a = b"sift context optimizer";
    assert_eq!(hash_bytes(a), hash_bytes(a), "same input, same hash");
    assert_eq!(hash_bytes(b""), sift_sys::FNV1A_OFFSET_BASIS);
    // Deterministic known value of FNV-1a("a").
    assert_eq!(hash_bytes(b"a"), 0xaf63_dc4c_8601_ec8c);
}

#[test]
fn count_matches_baseline_on_binary_payload() {
    let payload: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
    for value in [0u8, 1, 127, 128, 255] {
        assert_eq!(count_byte(&payload, value), baseline_count(&payload, value));
    }
}

/* ---------- randomized differential sweep ---------- */

#[test]
fn randomized_inputs_match_baseline() {
    let mut rng = XorShift(0x5EED_2026_0826_DEAD);
    // Alphabet includes NUL and high bytes so binary data is ordinary input.
    const ALPHABET: &[u8] = b"abcxyz01 \n\t\0{}()";

    for _ in 0..1500 {
        let hay_len = rng.below(400);
        let haystack: Vec<u8> = (0..hay_len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len())])
            .collect();

        let needle: Vec<u8> = if !haystack.is_empty() && rng.below(2) == 0 {
            // Extract a real substring (often repeated patterns).
            let start = rng.below(haystack.len());
            let max_end = (start + rng.below(24) + 1).min(haystack.len());
            haystack[start..max_end].to_vec()
        } else {
            let n_len = rng.below(12);
            (0..n_len)
                .map(|_| ALPHABET[rng.below(ALPHABET.len())])
                .collect()
        };

        assert_eq!(
            find_bytes(&haystack, &needle),
            baseline_find(&haystack, &needle),
            "find mismatch: hay={haystack:?} needle={needle:?}"
        );
        assert_eq!(hash_bytes(&haystack), baseline_hash(&haystack));
        let value = ALPHABET[rng.below(ALPHABET.len())];
        assert_eq!(
            count_byte(&haystack, value),
            baseline_count(&haystack, value)
        );
    }
}
