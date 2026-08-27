/* Byte substring search — see native/include/sift_native.h for the full contract. */
#include "sift_native.h"

size_t sift_find_bytes(const uint8_t *data, size_t data_len,
                       const uint8_t *needle, size_t needle_len)
{
    if (needle_len == 0) {
        return 0; /* empty needle matches at position 0 */
    }
    if (data == NULL || needle == NULL) {
        return SIFT_NOT_FOUND; /* NULL is only valid with length 0 */
    }
    if (needle_len > data_len) {
        return SIFT_NOT_FOUND;
    }

    const size_t last = data_len - needle_len;
    for (size_t i = 0; i <= last; ++i) {
        size_t j = 0;
        while (j < needle_len && data[i + j] == needle[j]) {
            ++j;
        }
        if (j == needle_len) {
            return i;
        }
    }
    return SIFT_NOT_FOUND;
}

size_t sift_find_many(
    const uint8_t *haystack,
    size_t haystack_len,
    const SiftSlice *needles,
    size_t needle_count,
    SiftMatch *output,
    size_t output_capacity)
{
    if (needles == NULL || output == NULL || output_capacity == 0 || needle_count == 0) {
        return 0;
    }

    size_t written = 0;
    for (size_t i = 0; i < needle_count; ++i) {
        if (written >= output_capacity) {
            break;
        }

        const uint8_t *needle_data = needles[i].data;
        size_t needle_len = needles[i].len;

        size_t offset = sift_find_bytes(haystack, haystack_len, needle_data, needle_len);
        if (offset != SIFT_NOT_FOUND) {
            output[written].needle_index = i;
            output[written].offset = offset;
            ++written;
        }
    }

    return written;
}
