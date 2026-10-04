//! The back buffer every frame is composed into, plus the drawing
//! primitives the shell layers use. One blit per frame, straight to the
//! kernel's surface.

use crate::theme::{ST_BODY, ST_MUTED};
use crate::{Cell, Style, Surface};

/// Columns and rows of the composed frame.
pub const W: usize = 80;
pub const H: usize = 25;

/// Back buffer the whole desktop is composed into; one blit per frame.
pub struct Canvas {
    pub(crate) cells: [Cell; W * H],
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            cells: [Cell::BLANK; W * H],
        }
    }

    /// Composed frame extent as (width, height).
    pub const fn extent() -> (usize, usize) {
        (W, H)
    }

    pub fn put(&mut self, x: usize, y: usize, cell: Cell) {
        if x < W && y < H {
            self.cells[y * W + x] = cell;
        }
    }

    /// Draw `s` clipped to `max_w` columns starting at `x`.
    pub fn field(&mut self, x: usize, y: usize, s: &[u8], style: Style, max_w: usize) {
        for (i, &b) in s.iter().take(max_w).enumerate() {
            self.put(x + i, y, Cell::new(b, style.fg, style.bg));
        }
    }

    pub fn fill(&mut self, rect: crate::Rect, cell: Cell) {
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                self.put(x as usize, y as usize, cell);
            }
        }
    }

    pub fn hline(&mut self, x: usize, y: usize, len: usize, glyph: u8, style: Style) {
        for i in 0..len {
            self.put(x + i, y, Cell::new(glyph, style.fg, style.bg));
        }
    }

    pub fn centered(&mut self, y: usize, s: &[u8], style: Style) {
        let x = (W - s.len().min(W)) / 2;
        self.field(x, y, s, style, W);
    }

    pub fn right(&mut self, y: usize, s: &[u8], style: Style) {
        let x = W - s.len().min(W);
        self.field(x, y, s, style, W);
    }

    /// One label/value row: dim label at `x`, value at `x + 10`.
    pub fn kv(&mut self, x: usize, y: usize, label: &str, value: &[u8], w: usize) {
        self.field(x, y, label.as_bytes(), ST_MUTED, 12);
        self.field(x + 10, y, value, ST_BODY, w.saturating_sub(10));
    }

    pub fn present(&self, surface: &mut dyn Surface) {
        let (sw, sh) = surface.size();
        let w = (sw as usize).min(W) as u16;
        let h = (sh as usize).min(H) as u16;
        surface.blit(&self.cells, W as u16, w, h);
    }
}

/// Draw window chrome (border + header bar) and return the content rect.
pub fn window(cv: &mut Canvas, rect: crate::Rect, badge: &crate::icons::Badge, title: &str) -> crate::Rect {
    use crate::theme::{BOX_BL, BOX_BR, BOX_H, BOX_TL, BOX_TR, BOX_V, ST_BORDER, ST_TITLE};

    let last_x = rect.x + rect.w - 1;
    let last_y = rect.y + rect.h - 1;

    cv.fill(
        crate::Rect::new(rect.x, rect.y, rect.w, 1),
        Cell::new(BOX_H, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.fill(
        crate::Rect::new(rect.x, last_y, rect.w, 1),
        Cell::new(BOX_H, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.fill(
        crate::Rect::new(rect.x, rect.y, 1, rect.h),
        Cell::new(BOX_V, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.fill(
        crate::Rect::new(last_x, rect.y, 1, rect.h),
        Cell::new(BOX_V, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.put(
        rect.x as usize,
        rect.y as usize,
        Cell::new(BOX_TL, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.put(
        last_x as usize,
        rect.y as usize,
        Cell::new(BOX_TR, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.put(
        rect.x as usize,
        last_y as usize,
        Cell::new(BOX_BL, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.put(
        last_x as usize,
        last_y as usize,
        Cell::new(BOX_BR, ST_BORDER.fg, ST_BORDER.bg),
    );

    let inner = rect.inset();
    cv.fill(
        crate::Rect::new(inner.x, inner.y, inner.w, 1),
        Cell::new(b' ', ST_TITLE.fg, ST_TITLE.bg),
    );
    // Opaque body: blank cells under the content so the wallpaper cannot
    // bleed through the window.
    cv.fill(
        crate::Rect::new(inner.x, inner.y + 1, inner.w, inner.h.saturating_sub(1)),
        Cell::BLANK,
    );
    let glyph = badge.glyph();
    let mut badge_line = [0u8; 24];
    badge_line[0] = glyph;
    badge_line[1] = b' ';
    let t = title.as_bytes();
    let n = t.len().min(badge_line.len() - 2);
    badge_line[2..2 + n].copy_from_slice(&t[..n]);
    cv.field(
        inner.x as usize,
        inner.y as usize,
        &badge_line[..2 + n],
        ST_TITLE,
        inner.w as usize,
    );

    crate::Rect::new(inner.x, inner.y + 1, inner.w, inner.h.saturating_sub(1))
}

// ------------------------------------------------------------------ text

/// Append `s` to a fixed buffer, dropping whatever does not fit.
pub fn push_bytes(buf: &mut [u8], len: &mut usize, s: &[u8]) {
    for &b in s {
        if *len < buf.len() {
            buf[*len] = b;
            *len += 1;
        }
    }
}

/// Decimal formatter; core::fmt machinery stays out of the hot path.
pub fn u64_to_dec(mut v: u64, out: &mut [u8; 20]) -> &[u8] {
    if v == 0 {
        out[0] = b'0';
        return &out[..1];
    }
    let mut i = out.len();
    while v > 0 {
        i -= 1;
        out[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    &out[i..]
}

pub fn kib_string(kib: u64, buf: &mut [u8; 32]) -> &[u8] {
    let mut num = [0u8; 20];
    let mut len = 0;
    push_bytes(buf, &mut len, u64_to_dec(kib / 1024, &mut num));
    push_bytes(buf, &mut len, b" MiB + ");
    push_bytes(buf, &mut len, u64_to_dec(kib % 1024, &mut num));
    push_bytes(buf, &mut len, b" KiB");
    &buf[..len]
}
