//! Architecture-specific Rust code.
//!
//! Currently only `x86` (32-bit). New architectures would come in as
//! sibling modules (e.g. `x86_64`), without touching `kernel/`.

pub mod x86;
