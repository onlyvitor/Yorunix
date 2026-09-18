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

/// Converte um `u32` em 8 dígitos hexadecimais ASCII (maiúsculos, MSB primeiro).
///
/// Função pura: não toca no MMIO, por isso é testável no host (`cargo test`).
/// Sempre retorna zero-padded (ex.: `0x00F0000A` -> `b"00F0000A"`), para que
/// dumps de `eip/cs/eflags` tenham largura fixa e sejam fáceis de comparar.
/// Sem `core::fmt`/alloc — importante com `panic = "abort"` e `no_std`.
fn hex_digits(v: u32) -> [u8; 8] {
    // Tabela de conversão nibble (4 bits) -> ASCII. `b'A'` base garante
    // maiúsculas, padrão de dumps de kernel (`0x1234ABCD`, não `0x1234abcd`).
    let mut out = [b'0'; 8];
    for (i, slot) in out.iter_mut().enumerate() {
        // Extrai o nibble `i` do mais significativo para o menos significativo:
        // i=0 -> bits 28..31, i=7 -> bits 0..3.
        let shift = 28 - (i as u32) * 4;
        let nibble = ((v >> shift) & 0xF) as u8;
        // 0..9 -> '0'..'9', 10..15 -> 'A'..'F'.
        *slot = if nibble < 10 {
            b'0' + nibble
        } else {
            b'A' + (nibble - 10)
        };
    }
    out
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

/// Escreve `v` como `0xXXXXXXXX` (10 células) na posição `(row, col)`.
///
/// Modelo posicional (2B): sem cursor global, sem efeito colateral no `putstr`.
/// Ideal para dumps de exceção (`debug()` imprime `eip` na linha 1 sem apagar
/// o cabeçalho da linha 0). Não faz wrap para a próxima linha: se `col + 10`
/// ultrapassar `VGA_WIDTH`, trunca — evita que um dump corrompa o layout da
/// tela. Posição fora da tela (`row >= 25` ou `col >= 80`) é no-op seguro.
pub fn put_hex_at(v: u32, row: usize, col: usize) {
    // SAFETY: mesmo dono/endereço de `clear_screen`/`putstr` (MMIO 0xB8000,
    // único dono é o kernel). Cada escrita é `volatile` e precedida de
    // bound-check, então nunca escrevemos fora das 2000 células. Leitura da
    // cor atual via `read_volatile` segue o padrão do `putstr`.
    unsafe {
        // Rejeita origem inválida antes de tocar no hardware.
        if row >= VGA_HEIGHT || col >= VGA_WIDTH {
            return;
        }
        // Monta o texto `0x` + 8 dígitos em buffer local (sem alloc, sem fmt).
        let digits = hex_digits(v);
        let mut text = [b'0'; 10];
        text[0] = b'0';
        text[1] = b'x';
        text[2..].copy_from_slice(&digits);

        let buf = buffer();
        let base = row * VGA_WIDTH + col;
        // Largura restante nesta linha: garante truncamento sem wrap.
        let room = VGA_WIDTH - col;
        let len = if text.len() < room { text.len() } else { room };
        for (i, &b) in text.iter().enumerate().take(len) {
            let offset = base + i;
            // Preserva a cor atual da célula, como no `putstr`.
            let color = read_volatile(buf.add(offset)).color;
            let color = if color == 0 {
                COLOR_WHITE_ON_BLACK
            } else {
                color
            };
            write_volatile(buf.add(offset), ScreenCell { ascii: b, color });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_digits_zero_is_padded() {
        assert_eq!(hex_digits(0x0000_0000), *b"00000000");
    }

    #[test]
    fn hex_digits_typical_address() {
        // Endereço típico de `eip`: letras e números misturados.
        assert_eq!(hex_digits(0x1234_ABCD), *b"1234ABCD");
    }

    #[test]
    fn hex_digits_all_ones() {
        assert_eq!(hex_digits(0xFFFF_FFFF), *b"FFFFFFFF");
    }

    #[test]
    fn hex_digits_preserves_leading_zeros() {
        // Padding é load-bearing: sem ele `0xA` e `0xA0000000` seriam ambíguos.
        assert_eq!(hex_digits(0x00F0_000A), *b"00F0000A");
    }

    #[test]
    fn hex_digits_uses_uppercase() {
        // Garante `A..F` maiúsculos, não `a..f`.
        assert_eq!(hex_digits(0x00AB_CDEF), *b"00ABCDEF");
    }
}
