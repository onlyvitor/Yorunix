//! Código Rust específico da arquitetura.
//!
//! Atualmente só `x86` (32-bit). Novas arquiteturas entrariam como
//! módulos irmãos (ex.: `x86_64`), sem tocar em `kernel/`.

pub mod x86;
