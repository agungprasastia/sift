//! Arena lifecycle and memory safety tests.

use sift_sys::Arena;

#[test]
fn arena_init_and_alloc() {
    let mut arena = Arena::new(1024).expect("arena creation failed");
    assert_eq!(arena.capacity(), 1024);
    assert_eq!(arena.used(), 0);
    assert_eq!(arena.remaining(), 1024);

    let slice1 = arena.alloc_bytes(64, 8).expect("allocation failed");
    assert_eq!(slice1.len(), 64);
    slice1.fill(0xAA);
    assert_eq!(arena.used(), 64);
    assert_eq!(arena.remaining(), 960);

    let slice2 = arena.alloc_bytes(128, 16).expect("allocation failed");
    assert_eq!(slice2.len(), 128);
    slice2.fill(0xBB);
    assert!(arena.used() >= 192);

    arena.reset();
    assert_eq!(arena.used(), 0);
    assert_eq!(arena.remaining(), 1024);

    // Can allocate again after reset
    let slice3 = arena.alloc_bytes(512, 8).expect("allocation failed");
    slice3.fill(0xCC);
    assert_eq!(arena.used(), 512);
}

#[test]
fn arena_exhaustion_returns_none() {
    let mut arena = Arena::new(128).expect("arena creation");
    let slice = arena.alloc_bytes(100, 8).expect("first alloc");
    assert_eq!(slice.len(), 100);

    // Next alloc exceeding capacity fails gracefully
    assert!(arena.alloc_bytes(50, 8).is_none());
    assert_eq!(arena.used(), 100);
}

#[test]
fn arena_alignment_behavior() {
    let mut arena = Arena::new(1024).expect("arena creation");
    let slice1 = arena.alloc_bytes(1, 1).expect("1-byte alloc");
    assert_eq!(slice1.len(), 1);

    // 64-byte alignment
    let slice2 = arena.alloc_bytes(8, 64).expect("64-byte aligned alloc");
    let ptr = slice2.as_ptr() as usize;
    assert_eq!(ptr % 64, 0, "must be 64-byte aligned");

    // Invalid non-power-of-two alignment
    assert!(
        arena.alloc_bytes(8, 3).is_none(),
        "non power of 2 alignment fails"
    );
}

#[test]
fn arena_zero_size_alloc() {
    let mut arena = Arena::new(64).expect("arena creation");
    let slice = arena.alloc_bytes(0, 8).expect("zero-size alloc");
    assert_eq!(slice.len(), 0);
    assert_eq!(arena.used(), 0);
}

#[test]
fn arena_empty_capacity() {
    let mut arena = Arena::new(0).expect("zero-capacity arena creation");
    assert_eq!(arena.capacity(), 0);
    assert_eq!(arena.used(), 0);
    assert!(arena.alloc_bytes(10, 8).is_none());
}
