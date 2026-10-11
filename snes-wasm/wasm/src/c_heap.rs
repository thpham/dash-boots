//! C-ABI heap shim for wasm32: libxm's C code calls `malloc`/`free`, and
//! neither the `wasm32-unknown-unknown` std nor the `dlmalloc` crate export
//! those symbols. The core logic below is platform-neutral and covered by
//! native tests; on wasm the `#[no_mangle]` wrappers give the C ABI the
//! symbols it needs (on native targets they are off for obvious reasons).
//!
//! Layout bookkeeping: every block is a `HEADER`-byte size prefix followed by
//! the user data, so `free`/`realloc` can recover the `Layout` that Rust's
//! allocator requires.

use std::alloc::{alloc, dealloc, realloc as rust_realloc, Layout};

/// Alignment used for every block: this matches C's `max_align_t` on wasm32,
/// and it is the size of the header so user data keeps that alignment.
const ALIGN: usize = 16;
const HEADER: usize = ALIGN;

/// `malloc`: returns the data pointer (header lives right before it).
///
/// # Safety
/// `ptr` must be freed on the same wasm module instance; pointers from a
/// different allocator must not be passed to `c_free`.
pub unsafe fn c_malloc(size: usize) -> *mut u8 {
    malloc_block(size.max(1))
}

/// `free`: recovers the stored size from the header and deallocates.
///
/// # Safety
/// `ptr` must have come from `c_malloc`/`c_calloc` and must not be freed twice.
pub unsafe fn c_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let base = ptr.sub(HEADER);
    let user_size = (base as *mut usize).read_unaligned();
    let layout = Layout::from_size_align_unchecked(user_size + HEADER, ALIGN);
    dealloc(base, layout);
}

/// `realloc`: reads the old size from the header, copies to the new block.
///
/// # Safety
/// `ptr` must have come from `c_malloc`/`c_calloc`/`c_realloc`.
pub unsafe fn c_realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
    if ptr.is_null() {
        return c_malloc(new_size);
    }
    let base = ptr.sub(HEADER);
    let user_size = (base as *mut usize).read_unaligned();
    let old = Layout::from_size_align_unchecked(user_size + HEADER, ALIGN);
    let new_user = new_size.max(1);
    let new = Layout::from_size_align_unchecked(new_user + HEADER, ALIGN);
    let grown = rust_realloc(base, old, new.size());
    if grown.is_null() {
        return grown;
    }
    // The header is data too — rewrite it for the new block.
    (grown as *mut usize).write_unaligned(new_user);
    grown.add(HEADER)
}

/// `calloc`: zeroed allocation.
///
/// # Safety
/// The caller owns the returned pointer; free it with `c_free`.
pub unsafe fn c_calloc(n: usize, elem: usize) -> *mut u8 {
    let Some(total) = n.checked_mul(elem) else {
        return std::ptr::null_mut();
    };
    let ptr = malloc_block(total.max(1));
    if !ptr.is_null() {
        std::ptr::write_bytes(ptr, 0, total.max(1));
    }
    ptr
}

/// One block = `[u16/usize header: user_size][user data]`, aligned `ALIGN`.
unsafe fn malloc_block(user_size: usize) -> *mut u8 {
    let Some(full) = user_size.checked_add(HEADER) else {
        return std::ptr::null_mut();
    };
    let layout = Layout::from_size_align_unchecked(full, ALIGN);
    let base = alloc(layout);
    if base.is_null() {
        return base;
    }
    (base as *mut usize).write_unaligned(user_size);
    base.add(HEADER)
}

#[cfg(target_arch = "wasm32")]
mod abi {
    // Symbols libxm's C code references (see wasm-c-stubs/stdlib.h).
    use super::{c_calloc, c_free, c_malloc, c_realloc};

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn malloc(size: usize) -> *mut u8 {
        c_malloc(size)
    }
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn free(ptr: *mut u8) {
        c_free(ptr)
    }
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
        c_realloc(ptr, new_size)
    }
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn calloc(n: usize, elem: usize) -> *mut u8 {
        c_calloc(n, elem)
    }
}
