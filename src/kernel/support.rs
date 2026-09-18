//! Support symbols required by the compiler in freestanding.
//!
//! When linking the `staticlib` with plain `ld` (without the `rustc`/`gcc` driver),
//! nobody provides `memcpy/memset/memcmp` or `rust_eh_personality` — the
//! LLVM backend emits them for copy loops, `slice`s, and unwind tables.
//! Byte-by-byte implementations, with no libc dependency.

use core::ffi::c_void;
use core::slice::{from_raw_parts, from_raw_parts_mut};

/// `rust_eh_personality` is referenced by the `core` unwind tables
/// even with `panic = "abort"`. Since we never unwind (every panic halts
/// in `hlt`), the body is unreachable by construction.
#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

/// Freestanding equivalent of libc `memcpy`.
///
/// # Safety
///
/// Caller guarantees that `dest` and `src` point to valid `n`-byte regions
/// that do not overlap, and that `dest` is writable. Overlap is UB —
/// use `memmove`.
#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    // SAFETY: standard C contract documented above. Byte-by-byte copy without
    // aggressive optimizations.
    let d = from_raw_parts_mut(dest.cast::<u8>(), n);
    let s = from_raw_parts(src.cast::<u8>(), n);
    d.copy_from_slice(s);
    dest
}

/// Freestanding equivalent of libc `memmove`.
///
/// # Safety
///
/// Caller guarantees that `dest` and `src` point to valid `n`-byte regions
/// and that `dest` is writable. Unlike `memcpy`, overlap is
/// allowed.
#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    // SAFETY: same contract as `memcpy`, but allows overlap (copy
    // via the logical temporary buffer of `copy`, which handles overlap).
    unsafe {
        core::ptr::copy(src.cast::<u8>(), dest.cast::<u8>(), n);
    }
    dest
}

/// Freestanding equivalent of libc `memset`.
///
/// # Safety
///
/// Caller guarantees that `s` points to a writable `n`-byte region.
#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut c_void, c: i32, n: usize) -> *mut c_void {
    // SAFETY: caller guarantees `n` writable bytes at `s`.
    unsafe {
        core::ptr::write_bytes(s.cast::<u8>(), c as u8, n);
    }
    s
}

/// Freestanding equivalent of libc `memcmp`.
///
/// # Safety
///
/// Caller guarantees that `s1` and `s2` point to readable `n`-byte regions.
#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32 {
    // SAFETY: caller guarantees `n` readable bytes at both pointers.
    let a = from_raw_parts(s1.cast::<u8>(), n);
    let b = from_raw_parts(s2.cast::<u8>(), n);
    for i in 0..n {
        if a[i] != b[i] {
            return a[i] as i32 - b[i] as i32;
        }
    }
    0
}

/// BSD alias for `memcmp`, referenced by some `core` objects.
///
/// # Safety
///
/// Same contract as `memcmp`.
#[no_mangle]
pub unsafe extern "C" fn bcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32 {
    // SAFETY: see `memcmp`.
    unsafe { memcmp(s1, s2, n) }
}
