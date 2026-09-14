//! VGA text-mode driver — safe wrapper over MMIO 0xB8000.
//!
//! Corrige bugs do `drivers/vga.c` original:
//! - `clear_screen` escrevia `0x00` (NUL) e iterava `i < 80*25` com passo 2
//!   (limpava só metade). Agora escreve `b' '` em todas as 2000 células.
//! - Escritas agora usam `write_volatile` para o compilador não eliminar o MMIO.
//! - `putstr` original não tratava `\n`, wrap nem limite de tela.

use core::ptr::{read_volatile, write_volatile};

pub const VGA_WIDTH: usize = 80;
pub const VGA_HEIGHT: usize = 25;
pub const VGA_ADDR: usize = 0xB8000;

/// Branco brilhante sobre preto, mesmo `WHITE_COLOR 0x0F` do C.
pub const COLOR_WHITE_ON_BLACK: u8 = 0x0F;

#[repr(C)]
#[derive(Clone, Copy)]
struct ScreenCell {
    ascii: u8,
    color: u8,
}

const BLANK_CELL: ScreenCell = ScreenCell {
    ascii: b' ',
    color: COLOR_WHITE_ON_BLACK,
};

fn buffer() -> *mut ScreenCell {
    VGA_ADDR as *mut ScreenCell
}

/// Limpa toda a tela para espaços em branco.
pub fn clear_screen() {
    // SAFETY: 0xB8000 é o buffer VGA text-mode mapeado pelo hardware/BIOS em
    // x86. O kernel é o único dono após o boot, escrita volatile de 2000
    // células dentro dos limites é sempre válida. Chamado com interrupções
    // já desabilitadas no `_start` ou em contexto single-core de boot.
    unsafe {
        let buf = buffer();
        let total = VGA_WIDTH * VGA_HEIGHT;
        for i in 0..total {
            write_volatile(buf.add(i), BLANK_CELL);
        }
    }
}

/// Escreve `s` a partir do topo-esquerda, com `\n`, wrap e truncamento seguro.
///
/// Mantém cursor interno simples (sem scroll por enquanto — mesma semântica
/// visível do `putstr` original para paridade no QEMU).
pub fn putstr(s: &str) {
    // SAFETY: mesmo dono/endereço de `clear_screen`. Leitura volatile do
    // cursor não é necessária (estado local); cada escrita é volatile e com
    // bound-check `if offset >= total { break }`.
    unsafe {
        let buf = buffer();
        let total = VGA_WIDTH * VGA_HEIGHT;
        let mut offset: usize = 0;
        for &b in s.as_bytes() {
            if b == b'\n' {
                // Avança para o início da próxima linha.
                let row = offset / VGA_WIDTH;
                offset = (row + 1) * VGA_WIDTH;
                if offset >= total {
                    break;
                }
                continue;
            }
            if offset >= total {
                break;
            }
            // Preserva a cor atual da célula (permite futuros destaques),
            // mas garante um caractere imprimível.
            let color = read_volatile(buf.add(offset)).color;
            let color = if color == 0 {
                COLOR_WHITE_ON_BLACK
            } else {
                color
            };
            write_volatile(
                buf.add(offset),
                ScreenCell {
                    ascii: if b == 0 { b' ' } else { b },
                    color,
                },
            );
            offset += 1;
        }
    }
}
