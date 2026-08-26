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
