//! NativeScanner lifecycle and state tests.

use sift_sys::NativeScanner;

#[test]
fn scanner_lifecycle_and_accounting() {
    let mut scanner = NativeScanner::new(4096).expect("scanner creation");
    assert_eq!(scanner.bytes_scanned(), 0);
    assert_eq!(scanner.scans(), 0);
    assert_eq!(scanner.scratch_capacity(), 4096);
    assert_eq!(scanner.scratch_used(), 0);

    scanner.record_scan(1024);
    scanner.record_scan(2048);
    assert_eq!(scanner.bytes_scanned(), 3072);
    assert_eq!(scanner.scans(), 2);

    scanner.reset();
    // Accounting counters preserved across reset
    assert_eq!(scanner.bytes_scanned(), 3072);
    assert_eq!(scanner.scans(), 2);
    assert_eq!(scanner.scratch_used(), 0);
}

#[test]
fn scanner_zero_scratch_capacity() {
    let mut scanner = NativeScanner::new(0).expect("zero scratch scanner");
    assert_eq!(scanner.scratch_capacity(), 0);
    scanner.record_scan(500);
    assert_eq!(scanner.bytes_scanned(), 500);
    assert_eq!(scanner.scans(), 1);
}
