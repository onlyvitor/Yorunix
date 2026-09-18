//! Símbolos de suporte exigidos pelo compilador em freestanding.
//!
//! Quando linkamos a `staticlib` com `ld` direto (sem driver `rustc`/`gcc`),
//! ninguém fornece `memcpy/memset/memcmp` nem `rust_eh_personality` — o
//! backend LLVM os emite para loops de cópia, `slice` e tabelas de unwind.
//! Implementações byte-a-byte, sem dependência de libc.

use core::ffi::c_void;
use core::slice::{from_raw_parts, from_raw_parts_mut};

/// `rust_eh_personality` é referenciado pelas tabelas de unwind do `core`
/// mesmo com `panic = "abort"`. Como nunca fazemos unwind (todo pânico trava
/// em `hlt`), o corpo é inalcançável por construção.
#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

/// Equivalente freestanding do `memcpy` da libc.
///
/// # Safety
///
/// Chamador garante que `dest` e `src` apontam para regiões válidas de `n`
/// bytes, que não se sobrepõem, e que `dest` é gravável. Sobreposição é UB —
/// use `memmove`.
#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    // SAFETY: contrato C padrão documentado acima. Copia byte-a-byte sem
    // otimizações agressivas.
    let d = from_raw_parts_mut(dest.cast::<u8>(), n);
    let s = from_raw_parts(src.cast::<u8>(), n);
    d.copy_from_slice(s);
    dest
}

/// Equivalente freestanding do `memmove` da libc.
///
/// # Safety
///
/// Chamador garante que `dest` e `src` apontam para regiões válidas de `n`
/// bytes e que `dest` é gravável. Ao contrário do `memcpy`, sobreposição é
/// permitida.
#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    // SAFETY: mesmo contrato do `memcpy`, mas permite sobreposição (cópia
    // via buffer temporário lógico do `copy`, que lida com overlap).
    unsafe {
        core::ptr::copy(src.cast::<u8>(), dest.cast::<u8>(), n);
    }
    dest
}

/// Equivalente freestanding do `memset` da libc.
///
/// # Safety
///
/// Chamador garante que `s` aponta para uma região gravável de `n` bytes.
#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut c_void, c: i32, n: usize) -> *mut c_void {
    // SAFETY: chamador garante `n` bytes escrevíveis em `s`.
    unsafe {
        core::ptr::write_bytes(s.cast::<u8>(), c as u8, n);
    }
    s
}

/// Equivalente freestanding do `memcmp` da libc.
///
/// # Safety
///
/// Chamador garante que `s1` e `s2` apontam para regiões legíveis de `n`
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32 {
    // SAFETY: chamador garante `n` bytes legíveis em ambos os ponteiros.
    let a = from_raw_parts(s1.cast::<u8>(), n);
    let b = from_raw_parts(s2.cast::<u8>(), n);
    for i in 0..n {
        if a[i] != b[i] {
            return a[i] as i32 - b[i] as i32;
        }
    }
    0
}

/// Alias BSD de `memcmp`, referenciado por alguns objetos do `core`.
///
/// # Safety
///
/// Mesmo contrato do `memcmp`.
#[no_mangle]
pub unsafe extern "C" fn bcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32 {
    // SAFETY: ver `memcmp`.
    unsafe { memcmp(s1, s2, n) }
}
