# VGA text mode — the first driver

**Source:** `src/vga.rs`

## How it works

### The framebuffer

In VGA text mode the screen is an **80×25 grid of 2-byte cells** mapped into physical memory at `0xB8000` (`src/vga.rs:11-13`):

```
cell = [ ascii (u8) | attribute (u8) ]
                     └─ high nibble: background, low nibble: foreground
COLOR_WHITE_ON_BLACK = 0x0F  → white on black (src/vga.rs:16)
```

`ScreenCell` (`src/vga.rs:18-23`) is `#[repr(C)]` over exactly those two bytes. The video card continuously reads this RAM and renders it to the display — writing to `0xB8000` *is* the display update. This is **memory-mapped I/O**: ordinary memory addresses that are actually a window into hardware.

### The operations

- **`clear_screen`** (`src/vga.rs:60-72`) — writes the same blank cell (`b' '`, `0x0F`) to all 80×25 = 2000 cells with `write_volatile`.
- **`putstr`** (`src/vga.rs:78-117`) — walks the string byte by byte from the top-left:
  - `\n` jumps to the start of the next row (`offset = (row + 1) * VGA_WIDTH`)
  - writes stop at the screen boundary (`offset >= total → break`) — no overruns past cell 1999
  - each cell's **existing color is read first** (`read_volatile`) and preserved, falling back to `0x0F` if it's zero — a hook for future colored output
  - NUL bytes are rendered as spaces instead of garbage glyphs
- **`hex_digits`** (`src/vga.rs:40-57`) — pure `u32` → 8 uppercase hex ASCII digits, zero-padded, no MMIO / `fmt` / alloc, so it runs in `cargo test` on the host.
- **`put_hex_at`** (`src/vga.rs:126-160`) — positional `0xXXXXXXXX` (10 cells) writer at `(row, col)` with no global cursor and no side effect on `putstr`:
  - `base = row * VGA_WIDTH + col`, truncates if `col + 10` exceeds the line — never wraps
  - out-of-screen origin (`row >= 25` or `col >= 80`) is a safe no-op, ideal for exception dumps without erasing other rows

Scrolling is intentionally **not implemented yet** — parity with the original C driver's visible behavior was the goal; a scrolling cursor is on the roadmap.

### Why `volatile` — the core lesson

From the optimizer's perspective, `clear_screen` is a loop that writes the same address range 2000 times where nobody ever reads the values. A compiler is allowed to collapse or elide such stores entirely — for *normal* memory that's a legal optimization, for MMIO it deletes your driver. **Volatile accesses are observable**: the compiler must perform them, in order, exactly as written.

That's why every framebuffer touch is `write_volatile` / `read_volatile` (`src/vga.rs:69, 101, 107, 151, 157`), and why the SAFETY comment on each `unsafe` block (`src/vga.rs:61-64, 79-81, 127-130`) documents the ownership argument: after boot, the kernel is the sole owner of the VGA buffer, and every access is bounds-checked against `0xB8000 + 2000` cells.

## Why we did this — bugs inherited from the C original

The module docstring (`src/vga.rs:1-7`) records what the C version got wrong; each fix is a small OS-dev lesson:

| C bug | Symptom | Rust fix |
|---|---|---|
| `clear_screen` wrote `0x00` (NUL) and iterated `i < 80*25` with **step 2** | Screen only half cleared; NULs render as garbage glyphs | Blank cell is `b' '`, loop covers all 2000 cells (`vga.rs:60-72`) |
| No `volatile` on MMIO stores | Compiler permitted to eliminate "dead" stores — driver could vanish under optimization | `write_volatile`/`read_volatile` everywhere |
| `putstr` had no `\n`, wrap, or bounds handling | Bytes written past the visible screen into adjacent memory | Full line handling + bound checks (`vga.rs:78-117`) |

(Full migration log in [rust-for-osdev.md](rust-for-osdev.md#migration-log-bugs-found-in-the-c-original).)

## Why it matters

The VGA driver is the kernel's first **device**, and MMIO is the pattern behind every device register you will ever touch — serial ports, PIC, PCI configuration, APIC. Three transferable rules:

1. **Device memory is not normal memory.** Reads and writes have side effects; `volatile` is the compiler contract that preserves them.
2. **Every access needs bounds and invariants written down.** The `SAFETY:` comments are the design doc living next to the code that relies on it.
3. **A driver is an API, not a pile of register pokes.** `pub fn clear_screen() / putstr() / put_hex_at()` hide the framebuffer details so the rest of the kernel never needs to know cell layout — when the driver moves to a framebuffer or serial console later, nothing else changes.

## Going deeper

- [OSDev Wiki — VGA Hardware](https://wiki.osdev.org/VGA_Hardware) — text mode memory map, attribute bytes, CRTC registers
- [OSDev Wiki — "Why is my framebuffer not updating?"](https://wiki.osdev.org/"Why"articles) — volatile and optimization pitfalls
- [The Rustonomicon — Volatile Access](https://doc.rust-lang.org/nomicon/aliasing.html#volatile-accesses) — the exact semantics `write_volatile` guarantees
- [C `volatile` vs MMIO (Embedded Rust book)](https://docs.rust-embedded.org/book/) — memory-mapped peripherals from Rust
