/* Dynamic growable byte buffer — see sift_native.h for contract. */
#include "sift_native.h"
#include <stdlib.h>
#include <string.h>

SiftStatus sift_buffer_init(SiftBuffer *buffer, size_t initial_capacity)
{
    if (buffer == NULL) {
        return SIFT_ERR_INVALID_ARGUMENT;
    }

    buffer->data = NULL;
    buffer->len = 0;
    buffer->capacity = 0;

    if (initial_capacity == 0) {
        return SIFT_OK;
    }

    buffer->data = (uint8_t *)malloc(initial_capacity);
    if (buffer->data == NULL) {
        return SIFT_ERR_ALLOC;
    }

    buffer->capacity = initial_capacity;
    return SIFT_OK;
}

SiftStatus sift_buffer_reserve(SiftBuffer *buffer, size_t additional)
{
    if (buffer == NULL) {
        return SIFT_ERR_INVALID_ARGUMENT;
    }

    /* Check integer overflow for len + additional */
    if (additional > SIZE_MAX - buffer->len || (buffer->len + additional) > (SIZE_MAX / 2)) {
        return SIFT_ERR_OVERFLOW;
    }

    size_t required = buffer->len + additional;
    if (required <= buffer->capacity) {
        return SIFT_OK; /* Already enough capacity */
    }

    /* Geometric growth with overflow guards */
    size_t new_capacity = buffer->capacity > 0 ? buffer->capacity : 8;
    while (new_capacity < required) {
        if (new_capacity > SIZE_MAX / 2) {
            new_capacity = required;
            break;
        }
        new_capacity *= 2;
    }

    /* Reallocate using temporary pointer to avoid leaking on failure */
    uint8_t *new_data = (uint8_t *)realloc(buffer->data, new_capacity);
    if (new_data == NULL) {
        return SIFT_ERR_ALLOC;
    }

    buffer->data = new_data;
    buffer->capacity = new_capacity;
    return SIFT_OK;
}

SiftStatus sift_buffer_append(SiftBuffer *buffer, const uint8_t *data, size_t len)
{
    if (buffer == NULL) {
        return SIFT_ERR_INVALID_ARGUMENT;
    }
    if (len == 0) {
        return SIFT_OK;
    }
    if (data == NULL) {
        return SIFT_ERR_INVALID_ARGUMENT;
    }

    SiftStatus reserve_status = sift_buffer_reserve(buffer, len);
    if (reserve_status != SIFT_OK) {
        return reserve_status;
    }

    memcpy(buffer->data + buffer->len, data, len);
    buffer->len += len;
    return SIFT_OK;
}

void sift_buffer_clear(SiftBuffer *buffer)
{
    if (buffer != NULL) {
        buffer->len = 0;
    }
}

void sift_buffer_destroy(SiftBuffer *buffer)
{
    if (buffer != NULL) {
        if (buffer->data != NULL) {
            free(buffer->data);
        }
        buffer->data = NULL;
        buffer->len = 0;
        buffer->capacity = 0;
    }
}
