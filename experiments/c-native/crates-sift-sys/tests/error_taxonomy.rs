//! Comprehensive tests for native status codes, error taxonomy, and invariant verification.

use sift_sys::{Arena, NativeBuffer, NativeError, NativeScanner, status_to_result};

#[test]
fn status_code_mapping() {
    assert_eq!(status_to_result(0), Ok(()));
    assert_eq!(status_to_result(-1), Err(NativeError::AllocationFailed));
    assert_eq!(status_to_result(-2), Err(NativeError::Overflow));
    assert_eq!(status_to_result(-3), Err(NativeError::InvalidArgument));
    assert_eq!(status_to_result(-4), Err(NativeError::CapacityExceeded));

    // Unknown error code maps to NativeInvariantViolation
    assert_eq!(
        status_to_result(1),
        Err(NativeError::NativeInvariantViolation)
    );
    assert_eq!(
        status_to_result(-99),
        Err(NativeError::NativeInvariantViolation)
    );
    assert_eq!(
        status_to_result(42),
        Err(NativeError::NativeInvariantViolation)
    );
}

#[test]
fn arena_alignment_rejections() {
    let mut arena = Arena::new(256).expect("arena");
    // Alignments that are not powers of 2 must return None
    assert!(arena.alloc_bytes(16, 3).is_none());
    assert!(arena.alloc_bytes(16, 5).is_none());
    assert!(arena.alloc_bytes(16, 7).is_none());
    assert!(arena.alloc_bytes(16, 15).is_none());

    // Valid power of 2 alignments
    assert!(arena.alloc_bytes(16, 1).is_some());
    assert!(arena.alloc_bytes(16, 2).is_some());
    assert!(arena.alloc_bytes(16, 4).is_some());
    assert!(arena.alloc_bytes(16, 8).is_some());
    assert!(arena.alloc_bytes(16, 16).is_some());
}

#[test]
fn buffer_overflow_rejection() {
    let mut buf = NativeBuffer::with_capacity(16).expect("buf");
    // Requesting reserve that would cause usize/size_t overflow must fail with Overflow error
    let res = buf.reserve(usize::MAX);
    assert_eq!(res, Err(NativeError::Overflow));
}

#[test]
fn native_error_display() {
    assert_eq!(
        format!("{}", NativeError::AllocationFailed),
        "native allocation failed"
    );
    assert_eq!(format!("{}", NativeError::Overflow), "arithmetic overflow");
    assert_eq!(
        format!("{}", NativeError::InvalidArgument),
        "invalid argument"
    );
    assert_eq!(
        format!("{}", NativeError::CapacityExceeded),
        "capacity exceeded"
    );
    assert_eq!(
        format!("{}", NativeError::NativeInvariantViolation),
        "native invariant violation"
    );
}

#[test]
fn scanner_lifecycle_resets_scratch_arena() {
    let mut scanner = NativeScanner::new(1024).expect("scanner");
    assert_eq!(scanner.scratch_capacity(), 1024);
    assert_eq!(scanner.scratch_used(), 0);
    scanner.record_scan(512);
    assert_eq!(scanner.bytes_scanned(), 512);
    assert_eq!(scanner.scans(), 1);
    scanner.reset();
    assert_eq!(scanner.scratch_used(), 0);
    assert_eq!(scanner.bytes_scanned(), 512);
}
