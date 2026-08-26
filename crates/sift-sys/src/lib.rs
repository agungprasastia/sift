//! Safe bindings to the Sift C11 accelerator (`native/`).
//!
//! This is the **only** crate in the workspace allowed to contain `unsafe`
//! and raw C FFI declarations. Everything else consumes the safe functions
//! below, which enforce the memory-safety contract documented in
//! `native/include/sift_native.h`:
//!
//! ```text
//! Rust &[u8]  ->  ptr + len  ->  C computation (stateless)
//!             ->  offset/count/hash  ->  Rust validation
//! ```
//!
//! The C side never allocates, never retains pointers past the call, and
//! reports "not found" as `SIZE_MAX`; these wrappers translate that into
//! `Option<usize>` and re-validate every offset against the input length.
//!
//! # Proof strategy
//!
//! These FFI calls execute real compiled C and therefore cannot run under
//! Miri. Correctness is instead proven by differential testing against plain
//! Rust reference implementations (`tests/differential.rs`) covering fixed
//! edge cases plus deterministic pseudo-random inputs.

/// Name of the native backend, for reporting in CLI output.
pub const BACKEND_NAME: &str = "C11";

/// FNV-1a 64-bit offset basis; the hash of empty input.
pub const FNV1A_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

// On every tier-one target `usize` == `size_t` and `u8` == `uint8_t`.
unsafe extern "C" {
    fn sift_find_bytes(
        data: *const u8,
        data_len: usize,
        needle: *const u8,
        needle_len: usize,
    ) -> usize;
    fn sift_hash_bytes(data: *const u8, len: usize) -> u64;
    fn sift_count_byte(data: *const u8, len: usize, value: u8) -> usize;
}

/// Offset of the first occurrence of `needle` inside `haystack`.
///
/// An empty `needle` matches at offset `0`. Returns `None` when there is no
/// match (including when `needle` is longer than `haystack`).
#[must_use]
pub fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.is_empty() {
        return None;
    }

    let offset = unsafe {
        // SAFETY: [Category 8 — FFI boundary UB]
        // - `haystack`/`needle` are valid for reads of their full lengths for
        //   the entire call; that invariant is guaranteed by the Rust slice
        //   type itself.
        // - The C side is stateless per the header contract in
        //   `native/include/sift_native.h`: it borrows the pointers only for
        //   the duration of the call, stores nothing, allocates nothing.
        // - Signatures match the C prototypes exactly (`usize` == `size_t`,
        //   `u8` == `uint8_t` on all supported targets).
        sift_find_bytes(
            haystack.as_ptr(),
            haystack.len(),
            needle.as_ptr(),
            needle.len(),
        )
    };

    // Rust validates the C result before use: a match start can never be at
    // or past the end of the haystack, so this also rejects SIZE_MAX.
    if offset >= haystack.len() {
        None
    } else {
        Some(offset)
    }
}

/// Non-cryptographic FNV-1a 64-bit hash of `data`.
///
/// Empty input hashes to [`FNV1A_OFFSET_BASIS`]. Deterministic across runs
/// and platforms.
#[must_use]
pub fn hash_bytes(data: &[u8]) -> u64 {
    if data.is_empty() {
        return FNV1A_OFFSET_BASIS;
    }

    unsafe {
        // SAFETY: [Category 8 — FFI boundary UB]
        // Same contract as `find_bytes`: borrowed slice valid for its whole
        // length during the call; stateless callee that neither stores nor
        // frees the pointer.
        sift_hash_bytes(data.as_ptr(), data.len())
    }
}

/// Number of bytes in `data` equal to `value`.
#[must_use]
pub fn count_byte(data: &[u8], value: u8) -> usize {
    if data.is_empty() {
        return 0;
    }

    let count = unsafe {
        // SAFETY: [Category 8 — FFI boundary UB]
        // Same contract as `find_bytes`: borrowed slice valid for its whole
        // length during the call; stateless callee that neither stores nor
        // frees the pointer.
        sift_count_byte(data.as_ptr(), data.len(), value)
    };
    debug_assert!(count <= data.len(), "C returned count outside input bounds");
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_handles_empty_inputs() {
        assert_eq!(find_bytes(b"", b""), Some(0));
        assert_eq!(find_bytes(b"", b"a"), None);
        assert_eq!(find_bytes(b"abc", b""), Some(0));
        assert_eq!(find_bytes(b"abc", b"abcd"), None);
    }

    #[test]
    fn binary_data_with_nul_bytes_is_ordinary_input() {
        let data = [0x00, 0x01, b'a', 0x00, b'b'];
        assert_eq!(find_bytes(&data, &[b'a', 0x00]), Some(2));
        assert_eq!(count_byte(&data, 0x00), 2);
    }
}
