/* Fixed-capacity linear arena allocator — see sift_native.h for contract. */
#include "sift_native.h"
#include <stdlib.h>
#include <stdint.h>

SiftStatus sift_arena_init(SiftArena *arena, size_t capacity)
{
    if (arena == NULL) {
        return SIFT_ERR_INVALID_ARGUMENT;
    }

    arena->data = NULL;
    arena->capacity = 0;
    arena->used = 0;

    if (capacity == 0) {
        return SIFT_OK;
    }

    arena->data = (uint8_t *)malloc(capacity);
    if (arena->data == NULL) {
        return SIFT_ERR_ALLOC;
    }

    arena->capacity = capacity;
    arena->used = 0;
    return SIFT_OK;
}

void *sift_arena_alloc(SiftArena *arena, size_t size, size_t alignment)
{
    if (arena == NULL) {
        return NULL;
    }

    if (alignment == 0) {
        alignment = sizeof(void *) > 8 ? sizeof(void *) : 8;
    }

    /* Alignment must be a power of two */
    if ((alignment & (alignment - 1)) != 0) {
        return NULL;
    }

    if (arena->data == NULL) {
        if (size == 0) {
            return (void *)arena; /* non-null sentinel for 0-size on empty arena */
        }
        return NULL;
    }

    /* Check pointer arithmetic overflow when calculating aligned offset */
    uintptr_t current_addr = (uintptr_t)(arena->data + arena->used);
    uintptr_t mask = (uintptr_t)(alignment - 1);
    uintptr_t aligned_addr = (current_addr + mask) & ~mask;
    size_t padding = (size_t)(aligned_addr - current_addr);

    /* Check integer overflow for used + padding */
    if (SIZE_MAX - arena->used < padding) {
        return NULL;
    }
    size_t new_used_start = arena->used + padding;

    /* Check integer overflow for new_used_start + size */
    if (SIZE_MAX - new_used_start < size) {
        return NULL;
    }
    size_t new_used_end = new_used_start + size;

    if (new_used_end > arena->capacity) {
        return NULL; /* Out of capacity */
    }

    arena->used = new_used_end;
    return (void *)aligned_addr;
}

void sift_arena_reset(SiftArena *arena)
{
    if (arena != NULL) {
        arena->used = 0;
    }
}

void sift_arena_destroy(SiftArena *arena)
{
    if (arena != NULL) {
        if (arena->data != NULL) {
            free(arena->data);
        }
        arena->data = NULL;
        arena->capacity = 0;
        arena->used = 0;
    }
}
