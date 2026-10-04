//! 16550 UART driver on COM1 (`0x3F8`), 115200 8N1.
//!
//! The UART is the debug console: everything the kernel logs is duplicated
//! here, which is what `qemu -serial mon:stdio` (or a real serial cable)
//! shows. Transmission is polling-based — interrupt-driven TX/RX arrives
//! with the interrupt subsystem in milestone M1.

use crate::arch::x86_64::io::{inb, outb};

const COM1: u16 = 0x3F8;

/// Interrupt Enable Register.
const IER: u16 = COM1 + 1;
/// FIFO Control Register.
const FCR: u16 = COM1 + 2;
/// Line Control Register.
const LCR: u16 = COM1 + 3;
/// Modem Control Register.
const MCR: u16 = COM1 + 4;
/// Line Status Register.
const LSR: u16 = COM1 + 5;

/// LSR bit: transmitter holding register empty (safe to write a byte).
const LSR_THR_EMPTY: u8 = 1 << 5;

/// Program COM1 as 115200 baud, 8 data bits, no parity, FIFOs enabled.
pub fn init() {
    unsafe {
        outb(IER, 0x00); // disable interrupts while programming
        outb(LCR, 0x80); // DLAB on
        outb(COM1, 0x01); // divisor low: 115200 baud
        outb(IER, 0x00); // divisor high
        outb(LCR, 0x03); // 8N1, DLAB off
        outb(FCR, 0xC7); // enable + clear FIFOs, 14-byte threshold
        outb(MCR, 0x0B); // DTR + RTS + OUT2
    }
}

/// Write a string byte by byte, blocking until each byte is accepted.
pub fn write_str(s: &str) {
    for b in s.bytes() {
        write_byte(b);
    }
}

fn write_byte(byte: u8) {
    // Map newline to CRLF so bare terminals render line breaks correctly.
    if byte == b'\n' {
        write_byte(b'\r');
    }
    while unsafe { inb(LSR) } & LSR_THR_EMPTY == 0 {
        core::hint::spin_loop();
    }
    unsafe { outb(COM1, byte) };
}
