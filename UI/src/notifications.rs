//! Notification toasts: the corner banner the session raises when the
//! framework is ready, mirroring the GNOME message tray behavior.

use crate::canvas::{push_bytes, Canvas};
use crate::icons::BADGE_SESSION;
use crate::theme::*;
use crate::{Cell, Rect, SessionInfo, Style};

/// Toast geometry: pinned to the bottom-right of the status column.
pub const RECT_NOTE: Rect = Rect::new(56, 18, 24, 6);

/// Notification chrome: accent border, blue field, title in the header.
fn notification_frame(cv: &mut Canvas, rect: Rect) -> Rect {
    let last_x = rect.x + rect.w - 1;
    let last_y = rect.y + rect.h - 1;

    cv.fill(rect, Cell::new(b' ', ST_NOTE.fg, ST_NOTE.bg));
    for i in 0..rect.w {
        cv.put(
            (rect.x + i) as usize,
            rect.y as usize,
            Cell::new(BOX_H, ST_NOTE_BORDER.fg, ST_NOTE_BORDER.bg),
        );
        cv.put(
            (rect.x + i) as usize,
            last_y as usize,
            Cell::new(BOX_H, ST_NOTE_BORDER.fg, ST_NOTE_BORDER.bg),
        );
    }
    for i in 0..rect.h {
        cv.put(
            rect.x as usize,
            (rect.y + i) as usize,
            Cell::new(BOX_V, ST_NOTE_BORDER.fg, ST_NOTE_BORDER.bg),
        );
        cv.put(
            last_x as usize,
            (rect.y + i) as usize,
            Cell::new(BOX_V, ST_NOTE_BORDER.fg, ST_NOTE_BORDER.bg),
        );
    }
    cv.put(
        rect.x as usize,
        rect.y as usize,
        Cell::new(BOX_TL, ST_NOTE_BORDER.fg, ST_NOTE_BORDER.bg),
    );
    cv.put(
        last_x as usize,
        rect.y as usize,
        Cell::new(BOX_TR, ST_NOTE_BORDER.fg, ST_NOTE_BORDER.bg),
    );
    cv.put(
        rect.x as usize,
        last_y as usize,
        Cell::new(BOX_BL, ST_NOTE_BORDER.fg, ST_NOTE_BORDER.bg),
    );
    cv.put(
        last_x as usize,
        last_y as usize,
        Cell::new(BOX_BR, ST_NOTE_BORDER.fg, ST_NOTE_BORDER.bg),
    );

    let inner = rect.inset();
    cv.fill(
        Rect::new(inner.x, inner.y, inner.w, 1),
        Cell::new(b' ', ST_NOTE_TITLE.fg, ST_NOTE_TITLE.bg),
    );
    let glyph = BADGE_SESSION.glyph();
    let mut badge = [0u8; 22];
    badge[0] = glyph;
    badge[1] = b' ';
    badge[2..9].copy_from_slice(b"session");
    cv.field(
        inner.x as usize,
        inner.y as usize,
        &badge[..9],
        ST_NOTE_TITLE,
        inner.w as usize,
    );

    Rect::new(inner.x, inner.y + 1, inner.w, inner.h.saturating_sub(1))
}

pub fn render_notification(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
    let body = notification_frame(cv, rect);
    let x = body.x as usize;
    let w = body.w as usize;
    let mut y = body.y as usize;

    cv.field(x, y, b"framework ready --", ST_NOTE, w);
    y += 1;

    let mut line = [0u8; 32];
    let mut n = 0;
    push_bytes(&mut line, &mut n, b"BOOT OK  Keraunos ");
    push_bytes(&mut line, &mut n, info.version.as_bytes());
    cv.field(x, y, &line[..n], Style::on(LIGHT_CYAN, BLUE), w);
    y += 1;

    cv.field(
        x,
        y,
        b"desktop session active",
        Style::on(DARK_GRAY, BLUE),
        w,
    );
}
