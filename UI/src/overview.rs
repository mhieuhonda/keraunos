//! The overview: the window board the session opens onto, three columns —
//! system and memory, the boot log, the subsystem status board.

use crate::canvas::{kib_string, push_bytes, u64_to_dec, window, Canvas};
use crate::icons::{BADGE_LOG, BADGE_MEMORY, BADGE_STATUS, BADGE_SYSTEM};
use crate::theme::*;
use crate::{Rect, SessionInfo, Style};

// 80x25 layout: one top bar, three columns (system + memory, boot log,
// subsystems) and the notification toast held by `notifications`.
pub const RECT_SYSTEM: Rect = Rect::new(0, 1, 24, 12);
pub const RECT_MEMORY: Rect = Rect::new(0, 14, 24, 11);
pub const RECT_LOG: Rect = Rect::new(25, 1, 30, 24);
pub const RECT_STATUS: Rect = Rect::new(56, 1, 24, 24);

pub fn render_system(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
    let body = window(cv, rect, &BADGE_SYSTEM, "system");
    let x = body.x as usize;
    let w = body.w as usize;
    let mut y = body.y as usize;

    cv.field(x, y, b"Keraunos", ST_ACCENT, w);
    y += 1;
    cv.field(x, y, b"the thunderbolt", ST_BODY, w);
    y += 1;
    cv.field(x, y, b"operating system", ST_BODY, w);
    y += 1;
    cv.field(x, y, b"fast, smooth, powerful", ST_MUTED, w);
    y += 2;
    cv.hline(x, y, w, BOX_H, ST_MUTED);
    y += 1;

    cv.kv(x, y, "vendor", info.vendor.as_bytes(), w);
    y += 1;
    cv.kv(x, y, "boot", info.bootloader.as_bytes(), w);
    y += 1;
    cv.kv(x, y, "arch", info.arch.as_bytes(), w);
    if let Some(cmd) = info.cmdline {
        y += 1;
        cv.kv(x, y, "cmdline", cmd.as_bytes(), w);
    }
}

pub fn render_memory(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
    let body = window(cv, rect, &BADGE_MEMORY, "memory");
    let x = body.x as usize;
    let w = body.w as usize;
    let mut y = body.y as usize;

    cv.field(x, y, b"usable memory", ST_MUTED, w);
    y += 1;
    let mut buf = [0u8; 32];
    let usable = kib_string(info.usable_kib, &mut buf);
    cv.field(x, y, usable, ST_BODY, w);
    y += 2;

    let mut num = [0u8; 20];
    let mut line = [0u8; 32];
    let mut n = 0;
    push_bytes(&mut line, &mut n, u64_to_dec(info.regions as u64, &mut num));
    push_bytes(&mut line, &mut n, b" regions");
    cv.kv(x, y, "boot map", &line[..n], w);
    y += 1;
    cv.kv(x, y, "frames", b"bump armed", w);
    y += 1;
    cv.kv(x, y, "page size", b"4 KiB", w);
    y += 1;
    cv.kv(x, y, "paging", b"identity map", w);
}

pub fn render_log(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
    let body = window(cv, rect, &BADGE_LOG, "boot log");
    let data = info.boot_log;
    let mut pos = tail_start(data, body.h as usize);
    let mut y = body.y;

    while pos < data.len() && y < body.y + body.h {
        let rest = &data[pos..];
        let end = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
        let line = &rest[..end];
        // Formatted lines arrive as "[LEVEL] tag     message".
        let text = if line.len() > 8 && line[0] == b'[' {
            &line[8..]
        } else {
            line
        };
        let style = if line.len() >= 6 && &line[1..6] == b"ERROR" {
            Style::on(LIGHT_RED, BLACK)
        } else {
            ST_BODY
        };
        cv.field(body.x as usize, y as usize, text, style, body.w as usize);
        pos += end + 1;
        y += 1;
    }
}

/// Offset of the oldest byte of the last `keep` newline-terminated lines.
fn tail_start(data: &[u8], keep: usize) -> usize {
    if keep == 0 || data.is_empty() {
        return data.len();
    }
    let mut remaining = keep;
    let mut i = data.len();
    while i > 0 {
        i -= 1;
        if data[i] == b'\n' {
            remaining -= 1;
            if remaining == 0 {
                return i + 1;
            }
        }
    }
    0
}

pub fn render_status(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
    let body = window(cv, rect, &BADGE_STATUS, "subsystems");
    let x = body.x as usize;
    let w = body.w as usize;
    let bottom = (body.y + body.h) as usize;
    let mut y = body.y as usize;

    for s in info.subsystems {
        if y >= bottom {
            break;
        }
        cv.field(x, y, s.name.as_bytes(), ST_MUTED, 12);
        let state = Style::on(s.state.color(), BLACK);
        cv.field(
            x + 13,
            y,
            s.state.label().as_bytes(),
            state,
            w.saturating_sub(13),
        );
        y += 1;
    }

    y += 1;
    if y < bottom {
        cv.hline(x, y, w, BOX_H, ST_MUTED);
        y += 1;
    }
    if y < bottom {
        cv.field(x, y, b"milestone M0 complete", ST_MUTED, w);
        y += 1;
    }
    if y < bottom {
        cv.field(x, y, b"next: M1 interrupts", ST_MUTED, w);
    }
}
