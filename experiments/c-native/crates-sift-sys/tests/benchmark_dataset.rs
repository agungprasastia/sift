//! Verification of benchmark dataset generator and positional integrity.

use memchr::memmem;

pub struct BenchmarkCorpus {
    pub data: Vec<u8>,
}

impl BenchmarkCorpus {
    pub fn new(size: usize, seed: u64) -> Self {
        let mut x = seed;
        // Generate lower-case ASCII alphanumeric background text [a-z0-9]
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
        Self { data }
    }

    pub fn with_needle_at(mut self, offset: usize, needle: &[u8]) -> (Vec<u8>, usize) {
        assert!(offset + needle.len() <= self.data.len());
        self.data[offset..offset + needle.len()].copy_from_slice(needle);
        (self.data, offset)
    }
}

#[test]
fn verify_corpus_positional_guarantees() {
    let needle = b"__NEEDLE_TARGET_XYZ__";
    let absent = b"__ABSENT_TARGET_XYZ__";

    for &size in &[1024, 10 * 1024, 100 * 1024, 1024 * 1024] {
        // 1. Beginning
        let (hay_beg, off_beg) = BenchmarkCorpus::new(size, 0x1234).with_needle_at(0, needle);
        assert_eq!(off_beg, 0);
        assert_eq!(memmem::find(&hay_beg, needle), Some(0));

        // 2. Middle
        let mid_pos = size / 2;
        let (hay_mid, off_mid) = BenchmarkCorpus::new(size, 0x5678).with_needle_at(mid_pos, needle);
        assert_eq!(off_mid, mid_pos);
        assert_eq!(memmem::find(&hay_mid, needle), Some(mid_pos));

        // 3. End
        let end_pos = size - needle.len();
        let (hay_end, off_end) = BenchmarkCorpus::new(size, 0x9ABC).with_needle_at(end_pos, needle);
        assert_eq!(off_end, end_pos);
        assert_eq!(memmem::find(&hay_end, needle), Some(end_pos));

        // 4. Absent
        let hay_absent = BenchmarkCorpus::new(size, 0xDEF0).data;
        assert_eq!(memmem::find(&hay_absent, absent), None);
    }
}
