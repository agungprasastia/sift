/* Native scanner lifecycle state — see sift_native.h for contract. */
#include "sift_native.h"

SiftStatus sift_scanner_init(SiftScanner *scanner, size_t scratch_capacity)
{
    if (scanner == NULL) {
        return SIFT_ERR_INVALID_ARGUMENT;
    }

    scanner->bytes_scanned = 0;
    scanner->scans = 0;

    return sift_arena_init(&scanner->scratch, scratch_capacity);
}

void sift_scanner_reset(SiftScanner *scanner)
{
    if (scanner != NULL) {
        sift_arena_reset(&scanner->scratch);
    }
}

void sift_scanner_destroy(SiftScanner *scanner)
{
    if (scanner != NULL) {
        sift_arena_destroy(&scanner->scratch);
        scanner->bytes_scanned = 0;
        scanner->scans = 0;
    }
}
