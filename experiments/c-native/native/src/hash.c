/* FNV-1a 64-bit hashing — see native/include/sift_native.h for the full contract. */
#include "sift_native.h"

uint64_t sift_hash_bytes(const uint8_t *data, size_t len)
{
    const uint64_t fnv1a_offset_basis = 14695981039346656037ULL;
    const uint64_t fnv1a_prime = 1099511628211ULL;

    uint64_t hash = fnv1a_offset_basis;
    if (data == NULL) {
        return hash; /* defensive: treat NULL as empty input */
    }

    for (size_t i = 0; i < len; ++i) {
        hash ^= (uint64_t)data[i];
        hash *= fnv1a_prime;
    }
    return hash;
}
