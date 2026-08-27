/*
 * Sift native accelerator — C11 byte-level primitives & scratch resources.
 *
 * OWNERSHIP & MEMORY SAFETY CONTRACT
 * ==================================
 *
 * The native layer contains both stateless byte primitives and explicitly-owned
 * stateful scratch resources:
 *
 *   - Stateless byte primitives (sift_find_bytes, sift_hash_bytes, sift_count_byte,
 *     sift_find_many, sift_index_newlines) receive borrowed `pointer + length` pairs,
 *     store no pointers past the call, keep no global state, and allocate nothing.
 *   - Managed native components (SiftArena, SiftBuffer, SiftScanner) have explicit
 *     ownership lifecycles (init -> use -> reset -> destroy). SiftArena and SiftBuffer
 *     own their backing data allocations until explicitly destroyed.
 *   - Rust owns application semantics and long-lived state.
 *   - All lengths, capacities, offsets and counts use `size_t`.
 *   - "Not found" is reported as `SIZE_MAX` (SIFT_NOT_FOUND); callers in
 *     Rust re-validate every returned offset against input bounds.
 *
 * NULL pointers are only valid together with a zero length. Passing
 * `NULL` with a non-zero length returns the documented defensive error/result
 * instead of dereferencing.
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
 * Unified native status / error codes.
 */
typedef enum {
    SIFT_OK = 0,
    SIFT_ERR_ALLOC = -1,
    SIFT_ERR_OVERFLOW = -2,
    SIFT_ERR_INVALID_ARGUMENT = -3,
    SIFT_ERR_CAPACITY = -4
} SiftStatus;

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

/*
 * Fixed-capacity linear arena allocator.
 *
 * Contract:
 *   - SiftArena owns arena->data.
 *   - Memory returned by sift_arena_alloc is borrowed.
 *   - Borrowed memory must NOT be freed individually.
 *   - Borrowed memory remains valid until sift_arena_reset or sift_arena_destroy.
 *   - sift_arena_destroy zeroes internal fields and frees arena->data.
 */
typedef struct {
    uint8_t *data;
    size_t capacity;
    size_t used;
} SiftArena;

/*
 * Initialize an arena with the requested capacity in bytes.
 * Returns SIFT_OK on success, or SiftStatus error code.
 */
SiftStatus sift_arena_init(SiftArena *arena, size_t capacity);

/*
 * Allocate `size` bytes aligned to `alignment` from the arena.
 * `alignment` must be a power of two (or 0 for default 8-byte alignment).
 * Returns pointer to aligned memory, or NULL on failure / overflow / out-of-memory.
 */
void *sift_arena_alloc(SiftArena *arena, size_t size, size_t alignment);

/*
 * Reset arena usage back to 0 without releasing the backing buffer.
 */
void sift_arena_reset(SiftArena *arena);

/*
 * Release backing buffer and zero out arena structure.
 */
void sift_arena_destroy(SiftArena *arena);

/*
 * Dynamic growable byte buffer.
 *
 * Contract:
 *   - SiftBuffer owns buffer->data.
 *   - Reallocates safely with temporary pointer; on failure old data remains valid.
 *   - Integer overflow checked before arithmetic.
 *   - sift_buffer_destroy frees buffer->data and zeroes fields.
 */
typedef struct {
    uint8_t *data;
    size_t len;
    size_t capacity;
} SiftBuffer;

/*
 * Initialize buffer with `initial_capacity` bytes.
 * Returns SIFT_OK on success, or SiftStatus error code.
 */
SiftStatus sift_buffer_init(SiftBuffer *buffer, size_t initial_capacity);

/*
 * Ensure capacity for at least `additional` more bytes beyond `len`.
 * Returns SIFT_OK on success, or SiftStatus error code.
 */
SiftStatus sift_buffer_reserve(SiftBuffer *buffer, size_t additional);

/*
 * Append `len` bytes from `data` to buffer.
 * Returns SIFT_OK on success, or SiftStatus error code.
 */
SiftStatus sift_buffer_append(SiftBuffer *buffer, const uint8_t *data, size_t len);

/*
 * Reset buffer length to 0 without releasing capacity.
 */
void sift_buffer_clear(SiftBuffer *buffer);

/*
 * Release backing buffer and zero out buffer structure.
 */
void sift_buffer_destroy(SiftBuffer *buffer);

/*
 * Stateful native scanner context.
 *
 * Contract:
 *   - SiftScanner owns its internal scratch arena.
 *   - Does not store borrowed pointers from callers.
 *   - Explicit lifecycle: init -> use / reset -> destroy.
 */
typedef struct {
    SiftArena scratch;
    size_t bytes_scanned;
    size_t scans;
} SiftScanner;

/*
 * Initialize native scanner with `scratch_capacity` scratch arena.
 * Returns SIFT_OK on success, or SiftStatus error code.
 */
SiftStatus sift_scanner_init(SiftScanner *scanner, size_t scratch_capacity);

/*
 * Reset scratch arena usage while preserving scanner instance.
 */
void sift_scanner_reset(SiftScanner *scanner);

/*
 * Release internal scratch arena and zero out scanner structure.
 */
void sift_scanner_destroy(SiftScanner *scanner);

/*
 * =========================================================================
 * Batch Searching & Indexing Primitives
 * =========================================================================
 */

/* Read-only slice representation passed from Rust. */
typedef struct {
    const uint8_t *data;
    size_t len;
} SiftSlice;

/* Search match result associating needle index with its byte offset. */
typedef struct {
    size_t needle_index;
    size_t offset;
} SiftMatch;

/*
 * Search for first occurrence of each needle in `needles` inside `haystack`.
 *
 * Contract:
 *   - Rust owns haystack, needles, and output.
 *   - C borrows all memory for duration of call only.
 *   - Empty needles match at offset 0.
 *   - Needles not found are omitted from output.
 *   - Writes at most `output_capacity` matches.
 *   - Returns total number of matches written (always <= output_capacity).
 */
size_t sift_find_many(
    const uint8_t *haystack,
    size_t haystack_len,
    const SiftSlice *needles,
    size_t needle_count,
    SiftMatch *output,
    size_t output_capacity
);

/*
 * Record byte offsets of every newline ('\n') in `data` into `output`.
 *
 * Contract:
 *   - Rust owns data and output buffers.
 *   - C borrows memory for duration of call only.
 *   - Writes at most `output_capacity` offsets.
 *   - Returns count of newline offsets written (always <= output_capacity).
 */
size_t sift_index_newlines(
    const uint8_t *data,
    size_t len,
    size_t *output,
    size_t output_capacity
);

#ifdef __cplusplus
}
#endif

#endif /* SIFT_NATIVE_H */
