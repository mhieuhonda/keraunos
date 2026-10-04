//! VGA text mode driver (80x25, 16 colors) at physical `0xB8000`.
//!
//! All buffer access is volatile; the writer is single-threaded during boot
//! and protected by atomics afterwards. The hardware cursor is updated via
//! the CRTC ports so the screen matches the logical cursor position.

use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use crate::arch::x86_64::io::outb;

#[allow(dead_code)] // the full palette is part of the driver API surface
/// Standard VGA 16-color palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xa,
    LightCyan = 0xb,
    LightRed = 0xc,
    LightMagenta = 0xd,
    Yellow = 0xe,
    White = 0xf,
}

/// Foreground/background pair packed into one attribute byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(fg: Color, bg: Color) -> Self {
        ColorCode(((bg as u8) << 4) | (fg as u8))
    }
}

const COLUMNS: usize = 80;
const ROWS: usize = 25;
const VGA_BUFFER: *mut u16 = 0xB8000 as *mut u16;
const CRTC_INDEX_PORT: u16 = 0x3D4;
const CRTC_DATA_PORT: u16 = 0x3D5;

const DEFAULT_COLOR: ColorCode = ColorCode::new(Color::LightGray, Color::Black);
const BANNER_COLOR: ColorCode = ColorCode::new(Color::LightCyan, Color::Black);

static CURRENT_COLOR: AtomicU8 = AtomicU8::new(0);
static SAVED_COLOR: AtomicU8 = AtomicU8::new(0);
static CURSOR: AtomicUsize = AtomicUsize::new(0);

fn color_code() -> ColorCode {
    ColorCode(CURRENT_COLOR.load(Ordering::Relaxed))
}

/// Temporarily render output in `fg` (used for log levels / banner accents).
pub fn push_color(fg: Color) {
    SAVED_COLOR.store(CURRENT_COLOR.load(Ordering::Relaxed), Ordering::Relaxed);
    CURRENT_COLOR.store(ColorCode::new(fg, Color::Black).0, Ordering::Relaxed);
}

/// Revert the effect of [`push_color`].
pub fn pop_color() {
    CURRENT_COLOR.store(SAVED_COLOR.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// Highlight color used for the boot banner.
pub fn set_banner_color() {
    CURRENT_COLOR.store(BANNER_COLOR.0, Ordering::Relaxed);
}

/// Restore the default (light gray on black) rendering color.
pub fn set_default_color() {
    CURRENT_COLOR.store(DEFAULT_COLOR.0, Ordering::Relaxed);
}

/// Clear the screen and home the cursor.
pub fn init() {
    set_default_color();
    for i in 0..COLUMNS * ROWS {
        unsafe {
            write_volatile(VGA_BUFFER.add(i), entry(b' '));
        }
    }
    CURSOR.store(0, Ordering::Relaxed);
    sync_hardware_cursor();
}

/// Write a string, honoring `\n` (newline + scroll) and expanding tabs.
pub fn write_str(s: &str) {
    for b in s.bytes() {
        match b {
            b'\n' => new_line(),
            b'\r' => {}
            b'\t' => {
                for _ in 0..4 {
                    write_byte(b' ');
                }
            }
            0x20..=0x7e => write_byte(b),
            _ => write_byte(b'?'),
        }
    }
}

fn entry(byte: u8) -> u16 {
    ((color_code().0 as u16) << 8) | byte as u16
}

fn write_byte(byte: u8) {
    let pos = CURSOR.load(Ordering::Relaxed);
    if pos >= COLUMNS * ROWS {
        scroll();
    }
    let pos = CURSOR.load(Ordering::Relaxed);
    // SAFETY: `pos` is within `0..COLUMNS*ROWS` after the scroll check above
    // and `VGA_BUFFER` points at the mapped VGA text frame buffer.
    unsafe {
        write_volatile(VGA_BUFFER.add(pos), entry(byte));
    }
    CURSOR.store(pos + 1, Ordering::Relaxed);
    sync_hardware_cursor();
}

fn new_line() {
    let pos = CURSOR.load(Ordering::Relaxed);
    let line = pos / COLUMNS;
    if line + 1 >= ROWS {
        scroll();
        CURSOR.store((ROWS - 1) * COLUMNS, Ordering::Relaxed);
    } else {
        CURSOR.store((line + 1) * COLUMNS, Ordering::Relaxed);
    }
    sync_hardware_cursor();
}

fn scroll() {
    // Shift rows 1..ROWS one row up, then clear the last row.
    for row in 1..ROWS {
        for col in 0..COLUMNS {
            let src = row * COLUMNS + col;
            let dst = (row - 1) * COLUMNS + col;
            // SAFETY: both indices are within the 80x25 buffer.
            unsafe {
                let v = read_volatile(VGA_BUFFER.add(src));
                write_volatile(VGA_BUFFER.add(dst), v);
            }
        }
    }
    let last = (ROWS - 1) * COLUMNS;
    for col in 0..COLUMNS {
        // SAFETY: index within the 80x25 buffer.
        unsafe {
            write_volatile(VGA_BUFFER.add(last + col), entry(b' '));
        }
    }
    CURSOR.store(last, Ordering::Relaxed);
}

fn sync_hardware_cursor() {
    let pos = CURSOR.load(Ordering::Relaxed).min(0xFFFF) as u16;
    unsafe {
        outb(CRTC_INDEX_PORT, 0x0F);
        outb(CRTC_DATA_PORT, (pos & 0xFF) as u8);
        outb(CRTC_INDEX_PORT, 0x0E);
        outb(CRTC_DATA_PORT, (pos >> 8) as u8);
    }
}

/// Volatile read helper (see `core::ptr::read_volatile`).
///
/// # Safety
/// Caller guarantees `ptr` is readable and within the VGA buffer.
unsafe fn read_volatile(ptr: *mut u16) -> u16 {
    unsafe { core::ptr::read_volatile(ptr) }
}

/// Volatile write helper (see `core::ptr::write_volatile`).
///
/// # Safety
/// Caller guarantees `ptr` is writable and within the VGA buffer.
unsafe fn write_volatile(ptr: *mut u16, value: u16) {
    unsafe { core::ptr::write_volatile(ptr, value) }
}
