//! NativeBuffer lifecycle and memory safety tests.

use sift_sys::NativeBuffer;

#[test]
fn buffer_init_append_and_growth() {
    let mut buf = NativeBuffer::with_capacity(16).expect("buffer creation");
    assert_eq!(buf.len(), 0);
    assert!(buf.is_empty());
    assert_eq!(buf.capacity(), 16);
    assert_eq!(buf.as_slice(), b"");

    buf.append(b"hello ").expect("append 1");
    assert_eq!(buf.len(), 6);
    assert!(!buf.is_empty());
    assert_eq!(buf.as_slice(), b"hello ");

    buf.append(b"world! This is a longer string that exceeds initial capacity.")
        .expect("append 2");
    assert!(buf.len() > 16);
    assert!(buf.capacity() >= buf.len());
    assert!(buf.as_slice().starts_with(b"hello world!"));

    buf.clear();
    assert_eq!(buf.len(), 0);
    assert!(buf.is_empty());
    assert_eq!(buf.as_slice(), b"");

    // Reuse after clear
    buf.append(b"reused buffer content").expect("append 3");
    assert_eq!(buf.as_slice(), b"reused buffer content");
}

#[test]
fn buffer_zero_capacity_growth() {
    let mut buf = NativeBuffer::with_capacity(0).expect("zero-capacity buffer");
    assert_eq!(buf.capacity(), 0);
    assert_eq!(buf.len(), 0);

    buf.append(b"dynamically grown from zero").expect("append");
    assert_eq!(buf.as_slice(), b"dynamically grown from zero");
    assert!(buf.capacity() >= buf.len());
}

#[test]
fn buffer_reserve_expansion() {
    let mut buf = NativeBuffer::with_capacity(10).expect("buffer creation");
    buf.reserve(1000).expect("reserve 1000 bytes");
    assert!(buf.capacity() >= 1000);
    assert_eq!(buf.len(), 0);
}
