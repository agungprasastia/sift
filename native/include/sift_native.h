/*
 * Sift native accelerator — C11 byte-level primitives (M0).
 *
 * OWNERSHIP & MEMORY SAFETY CONTRACT
 * ==================================
 *
 * Rust owns all memory. Every function in this header is a *stateless*
 * accelerator over borrowed memory:
 *
 *   - Inputs are `pointer + length` pairs; the callee borrows them only for
 *     the duration of the call and never stores, retains, or frees them.
 *   - No allocation happens here: no malloc/calloc/realloc/free.
 *   - No global mutable state. Every function is thread-safe and reentrant.
 *   - Source code is treated as raw bytes; NUL bytes are ordinary data and
 *     strings are never assumed to be null-terminated.
 *   - All lengths, capacities, offsets and counts use `size_t`; all index
 *     arithmetic stays inside `[data, data + data_len)`.
 *   - "Not found" is reported as `SIZE_MAX` (see SIFT_NOT_FOUND); callers in
 *     Rust re-validate every returned offset before use.
 *
 * NULL pointers are only valid together with a zero length. Passing
 * `NULL` with a non-zero length returns the documented defensive result
 * instead of dereferencing (Rust never does this, but the contract holds
 * regardless).
 */
#ifndef SIFT_NATIVE_H
#define SIFT_NATIVE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Sentinel returned by lookup functions when nothing was found. */
#define SIFT_NOT_FOUND SIZE_MAX

/*
 * Return the byte offset of the first occurrence of `needle` inside `data`.
 *
 * Contract:
 *   - `needle_len == 0` matches at offset 0 (same convention as Rust's
 *     `str::find("")`), regardless of `data_len`, even for empty input.
 *   - `needle_len > data_len` yields SIFT_NOT_FOUND.
 *   - `NULL` with any non-zero length yields SIFT_NOT_FOUND (defensive;
 *     never dereferences).
 *   - Data may contain arbitrary bytes including NUL.
 *
 * Complexity: O(data_len * needle_len), no allocation, stateless.
 */
size_t sift_find_bytes(const uint8_t *data, size_t data_len,
                       const uint8_t *needle, size_t needle_len);

/*
 * Compute a non-cryptographic FNV-1a 64-bit hash over `len` bytes.
 *
 * Contract:
 *   - Deterministic across runs, platforms and endianness-independent by
 *     construction (byte-wise algorithm).
 *   - Empty input (`NULL` or `len == 0`) yields the FNV-1a offset basis
 *     (14695981039346656037ULL), matching the canonical empty-input result.
 *   - `NULL` with non-zero length is treated as empty input (defensive).
 *
 * This hash is NOT suitable for cryptographic use.
 */
uint64_t sift_hash_bytes(const uint8_t *data, size_t len);

/*
 * Count occurrences of `value` among the first `len` bytes of `data`.
 *
 * Contract:
 *   - Result is always <= len.
 *   - `NULL` (any len) or `len == 0` yields 0 (defensive).
 *   - Data may contain arbitrary bytes including NUL.
 */
size_t sift_count_byte(const uint8_t *data, size_t len, uint8_t value);

#ifdef __cplusplus
}
#endif

#endif /* SIFT_NATIVE_H */
