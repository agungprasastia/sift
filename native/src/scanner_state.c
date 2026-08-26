/* Native scanner lifecycle state — see sift_native.h for contract. */
#include "sift_native.h"

int sift_scanner_init(SiftScanner *scanner, size_t scratch_capacity)
{
    if (scanner == NULL) {
        return -1;
    }

    scanner->bytes_scanned = 0;
    scanner->scans = 0;

    if (sift_arena_init(&scanner->scratch, scratch_capacity) != 0) {
        return -1;
    }

    return 0;
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
