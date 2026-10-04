//! The Keraunos look: palette, styles, chrome glyphs and wallpaper.
//!
//! On text surfaces the palette is the VGA hardware set; framebuffer
//! sessions project the same accents from the shell theme assets in
//! `assets/theme`, with `distributor-logo` as the brand mark.

use crate::canvas::Canvas;
use crate::Cell;

// ---------------------------------------------------------------- palette

// VGA-compatible 4-bit color indices; the kernel's surface maps them onto
// hardware attributes. Only the colors the theme actually uses are named.
pub const BLACK: u8 = 0x0;
pub const BLUE: u8 = 0x1;
pub const CYAN: u8 = 0x3;
pub const DARK_GRAY: u8 = 0x8;
pub const LIGHT_GREEN: u8 = 0xa;
pub const LIGHT_CYAN: u8 = 0xb;
pub const LIGHT_RED: u8 = 0xc;
pub const YELLOW: u8 = 0xe;
pub const WHITE: u8 = 0xf;

// Theme styles, assembled once and reused everywhere.
pub const ST_BODY: crate::Style = crate::Style::on(WHITE, BLACK);
pub const ST_MUTED: crate::Style = crate::Style::on(DARK_GRAY, BLACK);
pub const ST_ACCENT: crate::Style = crate::Style::on(LIGHT_CYAN, BLACK);
pub const ST_RULE: crate::Style = crate::Style::on(CYAN, BLACK);
pub const ST_BAR: crate::Style = crate::Style::on(WHITE, DARK_GRAY);
pub const ST_BAR_BRAND: crate::Style = crate::Style::on(LIGHT_CYAN, DARK_GRAY);
pub const ST_TITLE: crate::Style = crate::Style::on(WHITE, DARK_GRAY);
pub const ST_BORDER: crate::Style = crate::Style::on(WHITE, BLACK);
pub const ST_NOTE: crate::Style = crate::Style::on(WHITE, BLUE);
pub const ST_NOTE_TITLE: crate::Style = crate::Style::on(YELLOW, BLUE);
pub const ST_NOTE_BORDER: crate::Style = crate::Style::on(LIGHT_CYAN, BLACK);

// Single-line box glyphs (code page 437, rendered natively by VGA).
pub const BOX_H: u8 = 0xc4;
pub const BOX_V: u8 = 0xb3;
pub const BOX_TL: u8 = 0xda;
pub const BOX_TR: u8 = 0xbf;
pub const BOX_BL: u8 = 0xc0;
pub const BOX_BR: u8 = 0xd9;
pub const RULE: u8 = 0xcd;
pub const SHADE: u8 = 0xb0;

// ------------------------------------------------------------------ theme

/// The wallpaper: a quiet diagonal drizzle over the storm-black desktop.
pub fn wallpaper(x: usize, y: usize) -> Cell {
    match (x + y) % 12 {
        0 => Cell::new(SHADE, DARK_GRAY, BLACK),
        6 => Cell::new(b'/', DARK_GRAY, BLACK),
        _ => Cell::BLANK,
    }
}

/// Repaint the whole canvas with the wallpaper.
pub fn paint_wallpaper(cv: &mut Canvas) {
    let (w, h) = Canvas::extent();
    for y in 0..h {
        for x in 0..w {
            cv.cells[y * w + x] = wallpaper(x, y);
        }
    }
}
