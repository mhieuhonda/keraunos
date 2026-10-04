//! The top panel: one bar across the first row, brand on the left and the
//! live session facts on the right — the shell's take on the GNOME panel.

use crate::canvas::{push_bytes, u64_to_dec, Canvas, W};
use crate::theme::{ST_BAR, ST_BAR_BRAND};
use crate::{Cell, Rect, SessionInfo};

/// Draw the panel across the top row of the frame.
pub fn top_bar(cv: &mut Canvas, info: &SessionInfo) {
    cv.fill(
        Rect::new(0, 0, W as u16, 1),
        Cell::new(b' ', ST_BAR.fg, ST_BAR.bg),
    );

    let mut left = [0u8; 32];
    let mut n = 0;
    push_bytes(&mut left, &mut n, b" Keraunos ");
    push_bytes(&mut left, &mut n, info.version.as_bytes());
    cv.field(0, 0, &left[..n], ST_BAR_BRAND, W);

    let mut num = [0u8; 20];
    let mut right = [0u8; 40];
    let mut n = 0;
    push_bytes(&mut right, &mut n, info.arch.as_bytes());
    push_bytes(&mut right, &mut n, b" | ");
    push_bytes(
        &mut right,
        &mut n,
        u64_to_dec(info.usable_kib / 1024, &mut num),
    );
    push_bytes(&mut right, &mut n, b" MiB | ");
    push_bytes(&mut right, &mut n, info.vendor.as_bytes());
    cv.right(0, &right[..n], ST_BAR);
}
