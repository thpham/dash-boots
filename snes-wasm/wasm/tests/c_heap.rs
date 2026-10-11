//! Native coverage of the wasm C-heap shim's logic (see `src/c_heap.rs` —
//! the `#[no_mangle]` wrappers are wasm-gated, but the bookkeeping is shared).

use snes_wasm::c_heap::{c_calloc, c_free, c_malloc, c_realloc};

#[test]
fn malloc_free_roundtrip_tolerates_repeated_cycles() {
    for i in 0..64 {
        let size = 17 + i * 23;
        let p = unsafe { c_malloc(size) };
        assert!(!p.is_null());
        unsafe { std::ptr::write_bytes(p, 0xAB, size) };
        unsafe { c_free(p) };
    }
}

#[test]
fn realloc_grows_and_preserves_contents() {
    let old_size = 32;
    let p = unsafe { c_malloc(old_size) };
    assert!(!p.is_null());
    unsafe { std::ptr::write_bytes(p, 0xCD, old_size) };
    let new_size = 320;
    let q = unsafe { c_realloc(p, new_size) };
    assert!(!q.is_null());
    let bytes = unsafe { std::slice::from_raw_parts(q, new_size) };
    assert!(bytes[..old_size].iter().all(|&b| b == 0xCD));
    unsafe { c_free(q) };
}

#[test]
fn realloc_from_null_is_malloc() {
    let q = unsafe { c_realloc(std::ptr::null_mut(), 64) };
    assert!(!q.is_null());
    unsafe { c_free(q) };
}

#[test]
fn calloc_zeroes_smallest_blocks_too() {
    let p = unsafe { c_calloc(1, 1) };
    assert!(!p.is_null());
    let b = unsafe { std::slice::from_raw_parts(p, 1) };
    assert_eq!(b[0], 0);
    unsafe { c_free(p) };
}

#[test]
fn oversized_request_returns_null() {
    let p = unsafe { c_malloc(usize::MAX - 8) };
    assert!(p.is_null());
}
