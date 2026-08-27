/* Byte counting / scanning — see native/include/sift_native.h for the full contract. */
#include "sift_native.h"

size_t sift_count_byte(const uint8_t *data, size_t len, uint8_t value)
{
    if (data == NULL) {
        return 0;
    }

    size_t count = 0;
    for (size_t i = 0; i < len; ++i) {
        if (data[i] == value) {
            ++count;
        }
    }
    return count;
}

size_t sift_index_newlines(
    const uint8_t *data,
    size_t len,
    size_t *output,
    size_t output_capacity)
{
    if (data == NULL || len == 0 || output == NULL || output_capacity == 0) {
        return 0;
    }

    size_t written = 0;
    for (size_t i = 0; i < len; ++i) {
        if (data[i] == '\n') {
            output[written] = i;
            ++written;
            if (written >= output_capacity) {
                break;
            }
        }
    }

    return written;
}
