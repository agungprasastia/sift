//! Randomized property fuzzing test suite for native C primitives and lifecycles.

use sift_sys::{
    Arena, Match, NativeBuffer, count_byte, find_bytes, find_many_vec, hash_bytes,
    index_newlines_vec,
};

struct XorShift64(u64);

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u64() % bound as u64) as usize
        }
    }

    fn random_bytes(&mut self, max_len: usize) -> Vec<u8> {
        const ALPHABET: &[u8] = b"abcdefABCDEF0123\n\r\t\0 \x7f\xff{}()<>";
        let len = self.below(max_len);
        (0..len)
            .map(|_| ALPHABET[self.below(ALPHABET.len())])
            .collect()
    }
}

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

fn baseline_find_many(haystack: &[u8], needles: &[&[u8]]) -> Vec<Match> {
    let mut matches = Vec::new();
    for (i, needle) in needles.iter().enumerate() {
        if let Some(offset) = baseline_find(haystack, needle) {
            matches.push(Match {
                needle_index: i,
                offset,
            });
        }
    }
    matches
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

fn baseline_index_newlines(data: &[u8]) -> Vec<usize> {
    data.iter()
        .enumerate()
        .filter(|&(_, &b)| b == b'\n')
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn fuzz_byte_primitives_10k_runs() {
    let mut rng = XorShift64(0xFEED_CAFE_DEAD_BEEF);

    for _ in 0..10_000 {
        let hay = rng.random_bytes(300);
        let needle = if !hay.is_empty() && rng.below(2) == 0 {
            let start = rng.below(hay.len());
            let end = (start + rng.below(32)).min(hay.len());
            hay[start..end].to_vec()
        } else {
            rng.random_bytes(16)
        };

        // 1. find_bytes
        let res_c = find_bytes(&hay, &needle);
        let res_base = baseline_find(&hay, &needle);
        assert_eq!(res_c, res_base);

        // 2. hash_bytes
        assert_eq!(hash_bytes(&hay), baseline_hash(&hay));

        // 3. count_byte
        let target_byte = if rng.below(3) == 0 {
            b'\n'
        } else {
            (rng.next_u64() & 0xFF) as u8
        };
        assert_eq!(
            count_byte(&hay, target_byte),
            baseline_count(&hay, target_byte)
        );

        // 4. index_newlines
        assert_eq!(index_newlines_vec(&hay), baseline_index_newlines(&hay));

        // 5. find_many with random needle set including duplicates and empty
        let needle2 = rng.random_bytes(8);
        let needle3 = b"".to_vec();
        let needle_slices = [needle.as_slice(), needle2.as_slice(), needle3.as_slice()];
        assert_eq!(
            find_many_vec(&hay, &needle_slices),
            baseline_find_many(&hay, &needle_slices)
        );
    }
}

#[test]
fn fuzz_arena_allocations() {
    let mut rng = XorShift64(0x1337_C0DE_BABE_FACE);
    let mut arena = Arena::new(65536).expect("arena init");

    for _ in 0..500 {
        arena.reset();
        assert_eq!(arena.used(), 0);

        let count = rng.below(30) + 1;
        for _ in 0..count {
            let size = rng.below(512);
            let alignments = [1, 2, 4, 8, 16, 32, 64];
            let alignment = alignments[rng.below(alignments.len())];

            if let Some(buf) = arena.alloc_bytes(size, alignment) {
                assert_eq!(buf.len(), size);
                if size > 0 {
                    let ptr_val = buf.as_ptr() as usize;
                    assert_eq!(ptr_val % alignment, 0, "alignment mismatch");
                    buf.fill(0x5A);
                }
            }
        }
    }
}

#[test]
fn fuzz_buffer_growth_and_append() {
    let mut rng = XorShift64(0xAAAA_BBBB_CCCC_DDDD);

    for _ in 0..200 {
        let initial_cap = rng.below(64);
        let mut buf = NativeBuffer::with_capacity(initial_cap).expect("buf init");
        let mut shadow = Vec::new();

        let ops = rng.below(20) + 1;
        for _ in 0..ops {
            let chunk = rng.random_bytes(128);
            buf.append(&chunk).expect("append");
            shadow.extend_from_slice(&chunk);
            assert_eq!(buf.as_slice(), shadow.as_slice());
        }

        buf.clear();
        assert_eq!(buf.len(), 0);
    }
}
