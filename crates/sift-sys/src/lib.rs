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

    fn sift_arena_init(arena: *mut SiftArena, capacity: usize) -> i32;
    fn sift_arena_alloc(arena: *mut SiftArena, size: usize, alignment: usize) -> *mut u8;
    fn sift_arena_reset(arena: *mut SiftArena);
    fn sift_arena_destroy(arena: *mut SiftArena);

    fn sift_buffer_init(buffer: *mut SiftBuffer, initial_capacity: usize) -> i32;
    fn sift_buffer_reserve(buffer: *mut SiftBuffer, additional: usize) -> i32;
    fn sift_buffer_append(buffer: *mut SiftBuffer, data: *const u8, len: usize) -> i32;
    fn sift_buffer_clear(buffer: *mut SiftBuffer);
    fn sift_buffer_destroy(buffer: *mut SiftBuffer);

    fn sift_scanner_init(scanner: *mut SiftScanner, scratch_capacity: usize) -> i32;
    fn sift_scanner_reset(scanner: *mut SiftScanner);
    fn sift_scanner_destroy(scanner: *mut SiftScanner);

    fn sift_find_many(
        haystack: *const u8,
        haystack_len: usize,
        needles: *const SiftSlice,
        needle_count: usize,
        output: *mut SiftMatch,
        output_capacity: usize,
    ) -> usize;

    fn sift_index_newlines(
        data: *const u8,
        len: usize,
        output: *mut usize,
        output_capacity: usize,
    ) -> usize;
}

/// Errors that can occur when calling native allocators and data structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeError {
    AllocationFailed,
    Overflow,
    InvalidArgument,
    CapacityExceeded,
    NativeInvariantViolation,
}

impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllocationFailed => write!(f, "native allocation failed"),
            Self::Overflow => write!(f, "arithmetic overflow"),
            Self::InvalidArgument => write!(f, "invalid argument"),
            Self::CapacityExceeded => write!(f, "capacity exceeded"),
            Self::NativeInvariantViolation => write!(f, "native invariant violation"),
        }
    }
}

impl std::error::Error for NativeError {}

/// Convert a C `SiftStatus` integer code to a typed `Result<(), NativeError>`.
pub fn status_to_result(code: i32) -> Result<(), NativeError> {
    match code {
        0 => Ok(()),
        -1 => Err(NativeError::AllocationFailed),
        -2 => Err(NativeError::Overflow),
        -3 => Err(NativeError::InvalidArgument),
        -4 => Err(NativeError::CapacityExceeded),
        _ => Err(NativeError::NativeInvariantViolation),
    }
}

/// Raw C-compatible arena structure matching `SiftArena` in `sift_native.h`.
#[repr(C)]
#[derive(Debug)]
pub struct SiftArena {
    pub data: *mut u8,
    pub capacity: usize,
    pub used: usize,
}

/// Safe wrapper around the native `SiftArena` allocator.
#[derive(Debug)]
pub struct Arena {
    raw: SiftArena,
}

// SAFETY: Arena owns its backing buffer exclusively and does not share internal aliased state across threads.
unsafe impl Send for Arena {}

impl Arena {
    /// Create a new native Arena with fixed capacity.
    ///
    /// # Errors
    /// Returns `NativeError::AllocationFailed` if native allocation fails.
    pub fn new(capacity: usize) -> Result<Self, NativeError> {
        let mut raw = SiftArena {
            data: std::ptr::null_mut(),
            capacity: 0,
            used: 0,
        };
        let ret = unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut raw` is a valid, uniquely referenced stack variable matching `SiftArena` layout.
            sift_arena_init(&mut raw, capacity)
        };
        status_to_result(ret)?;
        Ok(Self { raw })
    }

    /// Total capacity in bytes.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.raw.capacity
    }

    /// Bytes currently allocated.
    #[must_use]
    pub fn used(&self) -> usize {
        self.raw.used
    }

    /// Bytes remaining before exhaustion.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.raw.capacity.saturating_sub(self.raw.used)
    }

    /// Allocate `size` bytes aligned to `alignment`.
    ///
    /// Returns `None` if the arena is exhausted or alignment is invalid.
    pub fn alloc_bytes(&mut self, size: usize, alignment: usize) -> Option<&mut [u8]> {
        let ptr = unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut self.raw` is uniquely borrowed. `sift_arena_alloc` performs bounds and alignment checks.
            sift_arena_alloc(&mut self.raw, size, alignment)
        };
        if ptr.is_null() {
            None
        } else if size == 0 {
            Some(&mut [])
        } else {
            Some(unsafe {
                // SAFETY: [Category 8 — FFI boundary UB]
                // `ptr` is non-null, aligned, points to `size` allocated bytes owned exclusively by `self.raw`,
                // and lifetime is tied to `&mut self`.
                std::slice::from_raw_parts_mut(ptr, size)
            })
        }
    }

    /// Reset arena allocation offset to 0 without freeing backing buffer.
    pub fn reset(&mut self) {
        unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut self.raw` is valid.
            sift_arena_reset(&mut self.raw);
        }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut self.raw` is uniquely owned by `self`. `sift_arena_destroy` frees the buffer and zeroes fields.
            sift_arena_destroy(&mut self.raw);
        }
    }
}

/// Raw C-compatible buffer structure matching `SiftBuffer` in `sift_native.h`.
#[repr(C)]
#[derive(Debug)]
pub struct SiftBuffer {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

/// Safe wrapper around the native `SiftBuffer` dynamic byte buffer.
#[derive(Debug)]
pub struct NativeBuffer {
    raw: SiftBuffer,
}

// SAFETY: NativeBuffer owns its backing buffer exclusively and is safe to transfer between threads.
unsafe impl Send for NativeBuffer {}

impl NativeBuffer {
    /// Create a new native dynamic buffer with `initial_capacity`.
    ///
    /// # Errors
    /// Returns `NativeError` if initialization or allocation fails.
    pub fn with_capacity(initial_capacity: usize) -> Result<Self, NativeError> {
        let mut raw = SiftBuffer {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        };
        let ret = unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut raw` is a valid uniquely referenced stack variable matching `SiftBuffer` layout.
            sift_buffer_init(&mut raw, initial_capacity)
        };
        status_to_result(ret)?;
        Ok(Self { raw })
    }

    /// Current length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.raw.len
    }

    /// True if the buffer contains 0 bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.raw.len == 0
    }

    /// Total allocated capacity in bytes.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.raw.capacity
    }

    /// Reserve space for at least `additional` more bytes.
    ///
    /// # Errors
    /// Returns `NativeError` on allocation failure or integer overflow.
    pub fn reserve(&mut self, additional: usize) -> Result<(), NativeError> {
        let ret = unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut self.raw` is uniquely borrowed. `sift_buffer_reserve` handles reallocation safely.
            sift_buffer_reserve(&mut self.raw, additional)
        };
        status_to_result(ret)
    }

    /// Append a slice of bytes to the buffer.
    ///
    /// # Errors
    /// Returns `NativeError` on allocation failure or integer overflow.
    pub fn append(&mut self, data: &[u8]) -> Result<(), NativeError> {
        if data.is_empty() {
            return Ok(());
        }
        let ret = unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `data` is valid for reads of `data.len()` bytes. `&mut self.raw` is uniquely borrowed.
            sift_buffer_append(&mut self.raw, data.as_ptr(), data.len())
        };
        status_to_result(ret)
    }

    /// View the buffer content as an immutable byte slice.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        if self.raw.len == 0 || self.raw.data.is_null() {
            &[]
        } else {
            unsafe {
                // SAFETY: [Category 8 — FFI boundary UB]
                // `self.raw.data` points to `self.raw.len` initialized valid bytes with lifetime bounded by `&self`.
                std::slice::from_raw_parts(self.raw.data, self.raw.len)
            }
        }
    }

    /// Reset length to 0 without deallocating capacity.
    pub fn clear(&mut self) {
        unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut self.raw` is valid.
            sift_buffer_clear(&mut self.raw);
        }
    }
}

impl Drop for NativeBuffer {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut self.raw` is uniquely owned by `self`. `sift_buffer_destroy` frees the buffer and zeroes fields.
            sift_buffer_destroy(&mut self.raw);
        }
    }
}

/// Raw C-compatible scanner structure matching `SiftScanner` in `sift_native.h`.
#[repr(C)]
#[derive(Debug)]
pub struct SiftScanner {
    pub scratch: SiftArena,
    pub bytes_scanned: usize,
    pub scans: usize,
}

/// Safe wrapper around the native `SiftScanner` lifecycle state.
#[derive(Debug)]
pub struct NativeScanner {
    raw: SiftScanner,
}

// SAFETY: NativeScanner owns its internal scratch arena and state exclusively.
unsafe impl Send for NativeScanner {}

impl NativeScanner {
    /// Create a new native scanner with `scratch_capacity` bytes of scratch arena.
    ///
    /// # Errors
    /// Returns `NativeError` if native initialization fails.
    pub fn new(scratch_capacity: usize) -> Result<Self, NativeError> {
        let mut raw = SiftScanner {
            scratch: SiftArena {
                data: std::ptr::null_mut(),
                capacity: 0,
                used: 0,
            },
            bytes_scanned: 0,
            scans: 0,
        };
        let ret = unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut raw` is a valid uniquely referenced stack variable matching `SiftScanner` layout.
            sift_scanner_init(&mut raw, scratch_capacity)
        };
        status_to_result(ret)?;
        Ok(Self { raw })
    }

    /// Total bytes scanned recorded by this instance.
    #[must_use]
    pub fn bytes_scanned(&self) -> usize {
        self.raw.bytes_scanned
    }

    /// Total scan passes performed.
    #[must_use]
    pub fn scans(&self) -> usize {
        self.raw.scans
    }

    /// Total scratch arena capacity.
    #[must_use]
    pub fn scratch_capacity(&self) -> usize {
        self.raw.scratch.capacity
    }

    /// Bytes currently used in the scratch arena.
    #[must_use]
    pub fn scratch_used(&self) -> usize {
        self.raw.scratch.used
    }

    /// Record bytes scanned and increment scan count.
    pub fn record_scan(&mut self, bytes: usize) {
        self.raw.bytes_scanned = self.raw.bytes_scanned.saturating_add(bytes);
        self.raw.scans = self.raw.scans.saturating_add(1);
    }

    /// Reset scratch arena while preserving scanner counters.
    pub fn reset(&mut self) {
        unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut self.raw` is valid.
            sift_scanner_reset(&mut self.raw);
        }
    }
}

impl Drop for NativeScanner {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: [Category 8 — FFI boundary UB]
            // `&mut self.raw` is uniquely owned by `self`. `sift_scanner_destroy` frees scratch and zeroes fields.
            sift_scanner_destroy(&mut self.raw);
        }
    }
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

    if offset == usize::MAX {
        return None;
    }

    // Rust validates the C result before use: check that the entire match
    // fits within haystack bounds (including checked_add overflow guard).
    let end = offset.checked_add(needle.len())?;
    if end > haystack.len() {
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

/// Raw C-compatible slice descriptor matching `SiftSlice` in `sift_native.h`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SiftSlice {
    pub data: *const u8,
    pub len: usize,
}

/// Raw C-compatible match entry matching `SiftMatch` in `sift_native.h`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiftMatch {
    pub needle_index: usize,
    pub offset: usize,
}

/// Safe batch search match result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub needle_index: usize,
    pub offset: usize,
}

/// Search for occurrences of multiple `needles` inside `haystack` in a single FFI call.
///
/// Returns the number of matches written to `output`.
///
/// # Errors
/// Returns `NativeError::NativeInvariantViolation` if C returns invalid counts or offsets.
pub fn find_many(
    haystack: &[u8],
    needles: &[&[u8]],
    output: &mut [Match],
) -> Result<usize, NativeError> {
    if needles.is_empty() || output.is_empty() {
        return Ok(0);
    }

    let c_needles: Vec<SiftSlice> = needles
        .iter()
        .map(|n| SiftSlice {
            data: n.as_ptr(),
            len: n.len(),
        })
        .collect();

    let mut c_output = vec![
        SiftMatch {
            needle_index: 0,
            offset: 0
        };
        output.len()
    ];

    let count = unsafe {
        // SAFETY: [Category 8 — FFI boundary UB]
        // - `haystack` is valid for `haystack.len()` reads.
        // - `c_needles` contains valid pointers and lengths to each borrowed slice in `needles`.
        // - `c_output` is a valid mutable buffer of capacity `output.len()`.
        // - The C implementation borrows memory only for the call duration and allocates nothing.
        sift_find_many(
            haystack.as_ptr(),
            haystack.len(),
            c_needles.as_ptr(),
            c_needles.len(),
            c_output.as_mut_ptr(),
            c_output.len(),
        )
    };

    if count > output.len() {
        return Err(NativeError::NativeInvariantViolation);
    }

    let mut written = 0;
    for &m in &c_output[..count] {
        if m.needle_index >= needles.len() || m.offset == usize::MAX {
            return Err(NativeError::NativeInvariantViolation);
        }
        let needle_len = needles[m.needle_index].len();
        let end = m
            .offset
            .checked_add(needle_len)
            .ok_or(NativeError::NativeInvariantViolation)?;
        if end > haystack.len() {
            return Err(NativeError::NativeInvariantViolation);
        }
        output[written] = Match {
            needle_index: m.needle_index,
            offset: m.offset,
        };
        written += 1;
    }
    Ok(written)
}

/// Search for multiple `needles` inside `haystack` and return all matches in a `Vec`.
///
/// # Errors
/// Returns `NativeError::NativeInvariantViolation` if C returns invalid counts or offsets.
pub fn find_many_vec(haystack: &[u8], needles: &[&[u8]]) -> Result<Vec<Match>, NativeError> {
    if needles.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = vec![
        Match {
            needle_index: 0,
            offset: 0
        };
        needles.len()
    ];
    let count = find_many(haystack, needles, &mut out)?;
    out.truncate(count);
    Ok(out)
}

/// Write byte offsets of newline ('\n') characters in `data` into `output`.
///
/// Returns the number of offsets written (always `<= output.len()`).
///
/// # Errors
/// Returns `NativeError::NativeInvariantViolation` if C returns invalid counts, out-of-bound
/// offsets, non-newline target bytes, or non-monotonic offsets.
pub fn index_newlines(data: &[u8], output: &mut [usize]) -> Result<usize, NativeError> {
    if data.is_empty() || output.is_empty() {
        return Ok(0);
    }

    let count = unsafe {
        // SAFETY: [Category 8 — FFI boundary UB]
        // - `data` is valid for reads of `data.len()` bytes.
        // - `output` is a valid mutable slice of `usize` with `output.len()` elements.
        // - The C function only reads `data` and writes up to `output.len()` elements.
        sift_index_newlines(data.as_ptr(), data.len(), output.as_mut_ptr(), output.len())
    };

    if count > output.len() {
        return Err(NativeError::NativeInvariantViolation);
    }

    let mut prev_offset: Option<usize> = None;
    for &offset in &output[..count] {
        if offset >= data.len() || data[offset] != b'\n' {
            return Err(NativeError::NativeInvariantViolation);
        }
        if let Some(prev) = prev_offset
            && offset <= prev
        {
            return Err(NativeError::NativeInvariantViolation);
        }
        prev_offset = Some(offset);
    }
    Ok(count)
}

/// Collect all byte offsets of newline ('\n') characters into a `Vec<usize>`.
///
/// # Errors
/// Returns `NativeError::NativeInvariantViolation` if C returns corrupted offset data.
pub fn index_newlines_vec(data: &[u8]) -> Result<Vec<usize>, NativeError> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    let total_newlines = count_byte(data, b'\n');
    if total_newlines == 0 {
        return Ok(Vec::new());
    }
    let mut out = vec![0usize; total_newlines];
    let count = index_newlines(data, &mut out)?;
    out.truncate(count);
    Ok(out)
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
