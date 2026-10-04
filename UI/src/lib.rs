//! # keraunos-ui — the Keraunos desktop
//!
//! Theme, shell and compositor for the kernel's text surfaces, in one
//! deliberate file. The crate is `no_std`, allocator-free and contains no
//! `unsafe` at all — every hardware detail stays behind the [`Surface`]
//! contract, which the kernel implements for its framebuffer.
//!
//! Rendering is deterministic: a session phase maps to exactly one frame,
//! so serial logs, CI screen dumps and real hardware always agree.

#![no_std]
#![forbid(unsafe_code)]

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

/// Foreground/background pair for one cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: u8,
    pub bg: u8,
}

impl Style {
    pub const fn on(fg: u8, bg: u8) -> Self {
        Self { fg, bg }
    }
}

/// One character cell: a code-page byte plus a palette attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: u8,
    pub fg: u8,
    pub bg: u8,
}

impl Cell {
    pub const BLANK: Self = Self::new(b' ', WHITE, BLACK);

    pub const fn new(ch: u8, fg: u8, bg: u8) -> Self {
        Self { ch, fg, bg }
    }

    /// Packed attribute byte (fg | bg << 4), as the text surface expects it.
    pub fn attr(&self) -> u8 {
        (self.bg << 4) | self.fg
    }
}

// ------------------------------------------------------ kernel contract

/// The display a session renders onto. Implemented by the kernel's display
/// service; the UI never touches hardware itself.
pub trait Surface {
    /// Geometry as (columns, rows).
    fn size(&self) -> (u16, u16);

    /// Present a frame. `cells` holds at least `stride * h` entries
    /// row-major; the `w` x `h` top-left region is copied to the screen.
    fn blit(&mut self, cells: &[Cell], stride: u16, w: u16, h: u16);
}

/// Bring-up state of one kernel subsystem, mirrored on the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Online,
    Scaffold,
    Planned,
}

impl State {
    pub fn label(&self) -> &'static str {
        match self {
            State::Online => "online",
            State::Scaffold => "scaffold",
            State::Planned => "planned",
        }
    }

    fn color(&self) -> u8 {
        match self {
            State::Online => LIGHT_GREEN,
            State::Scaffold => YELLOW,
            State::Planned => DARK_GRAY,
        }
    }
}

/// One row of the subsystem table the shell displays.
#[derive(Debug, Clone, Copy)]
pub struct Subsystem {
    pub name: &'static str,
    pub state: State,
    pub note: &'static str,
}

/// Facts the kernel hands to the session. Everything shown on the desktop
/// comes from here, so the interface cannot drift from the truth.
pub struct SessionInfo<'a> {
    pub version: &'a str,
    pub arch: &'a str,
    pub vendor: &'a str,
    pub bootloader: &'a str,
    pub cmdline: Option<&'a str>,
    pub regions: usize,
    pub usable_kib: u64,
    pub subsystems: &'a [Subsystem],
    /// Captured boot log, newline-terminated lines, newest last.
    pub boot_log: &'a [u8],
    /// Serial narration hook; the UI owns no console of its own.
    pub log: fn(&str, &str),
}

impl SessionInfo<'_> {
    fn say(&self, msg: &str) {
        (self.log)("ui", msg);
    }
}

// ------------------------------------------------------------- geometry

/// Axis-aligned rectangle on the text grid.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }

    /// The rect shrunk by one cell on each side: a window's interior.
    fn inset(&self) -> Self {
        Self::new(
            self.x + 1,
            self.y + 1,
            self.w.saturating_sub(2),
            self.h.saturating_sub(2),
        )
    }
}

// ---------------------------------------------------------------- canvas

const W: usize = 80;
const H: usize = 25;

// Theme styles, assembled once and reused everywhere.
const ST_BODY: Style = Style::on(WHITE, BLACK);
const ST_MUTED: Style = Style::on(DARK_GRAY, BLACK);
const ST_ACCENT: Style = Style::on(LIGHT_CYAN, BLACK);
const ST_RULE: Style = Style::on(CYAN, BLACK);
const ST_BAR: Style = Style::on(WHITE, DARK_GRAY);
const ST_BAR_BRAND: Style = Style::on(LIGHT_CYAN, DARK_GRAY);
const ST_TITLE: Style = Style::on(WHITE, DARK_GRAY);
const ST_BORDER: Style = Style::on(WHITE, BLACK);
const ST_NOTE: Style = Style::on(WHITE, BLUE);
const ST_NOTE_TITLE: Style = Style::on(YELLOW, BLUE);
const ST_NOTE_BORDER: Style = Style::on(LIGHT_CYAN, BLACK);

// Single-line box glyphs (code page 437, rendered natively by VGA).
const BOX_H: u8 = 0xc4;
const BOX_V: u8 = 0xb3;
const BOX_TL: u8 = 0xda;
const BOX_TR: u8 = 0xbf;
const BOX_BL: u8 = 0xc0;
const BOX_BR: u8 = 0xd9;
const RULE: u8 = 0xcd;
const SHADE: u8 = 0xb0;

// Per-window badge glyphs — the icon set, one byte a piece.
const ICON_SYSTEM: u8 = 0x0f;
const ICON_MEMORY: u8 = 0xdb;
const ICON_LOG: u8 = 0x10;
const ICON_STATUS: u8 = 0xf0;
const ICON_SESSION: u8 = 0x07;

/// Back buffer the whole desktop is composed into; one blit per frame.
struct Canvas {
    cells: [Cell; W * H],
}

impl Canvas {
    fn new() -> Self {
        Self {
            cells: [Cell::BLANK; W * H],
        }
    }

    fn put(&mut self, x: usize, y: usize, cell: Cell) {
        if x < W && y < H {
            self.cells[y * W + x] = cell;
        }
    }

    /// Draw `s` clipped to `max_w` columns starting at `x`.
    fn field(&mut self, x: usize, y: usize, s: &[u8], style: Style, max_w: usize) {
        for (i, &b) in s.iter().take(max_w).enumerate() {
            self.put(x + i, y, Cell::new(b, style.fg, style.bg));
        }
    }

    fn fill(&mut self, rect: Rect, cell: Cell) {
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                self.put(x as usize, y as usize, cell);
            }
        }
    }

    fn hline(&mut self, x: usize, y: usize, len: usize, glyph: u8, style: Style) {
        for i in 0..len {
            self.put(x + i, y, Cell::new(glyph, style.fg, style.bg));
        }
    }

    fn centered(&mut self, y: usize, s: &[u8], style: Style) {
        let x = (W - s.len().min(W)) / 2;
        self.field(x, y, s, style, W);
    }

    fn right(&mut self, y: usize, s: &[u8], style: Style) {
        let x = W - s.len().min(W);
        self.field(x, y, s, style, W);
    }

    /// One label/value row: dim label at `x`, value at `x + 10`.
    fn kv(&mut self, x: usize, y: usize, label: &str, value: &[u8], w: usize) {
        self.field(x, y, label.as_bytes(), ST_MUTED, 12);
        self.field(x + 10, y, value, ST_BODY, w.saturating_sub(10));
    }

    fn present(&self, surface: &mut dyn Surface) {
        let (sw, sh) = surface.size();
        let w = (sw as usize).min(W) as u16;
        let h = (sh as usize).min(H) as u16;
        surface.blit(&self.cells, W as u16, w, h);
    }
}

// ------------------------------------------------------------------ theme

/// The wallpaper: a quiet diagonal drizzle over the storm-black desktop.
fn wallpaper(x: usize, y: usize) -> Cell {
    match (x + y) % 12 {
        0 => Cell::new(SHADE, DARK_GRAY, BLACK),
        6 => Cell::new(b'/', DARK_GRAY, BLACK),
        _ => Cell::BLANK,
    }
}

/// Append `s` to a fixed buffer, dropping whatever does not fit.
fn push_bytes(buf: &mut [u8], len: &mut usize, s: &[u8]) {
    for &b in s {
        if *len < buf.len() {
            buf[*len] = b;
            *len += 1;
        }
    }
}

/// Decimal formatter; core::fmt machinery stays out of the hot path.
fn u64_to_dec(mut v: u64, out: &mut [u8; 20]) -> &[u8] {
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

fn kib_string(kib: u64, buf: &mut [u8; 32]) -> &[u8] {
    let mut num = [0u8; 20];
    let mut len = 0;
    push_bytes(buf, &mut len, u64_to_dec(kib / 1024, &mut num));
    push_bytes(buf, &mut len, b" MiB + ");
    push_bytes(buf, &mut len, u64_to_dec(kib % 1024, &mut num));
    push_bytes(buf, &mut len, b" KiB");
    &buf[..len]
}

/// Draw window chrome (border + header bar) and return the content rect.
fn window(cv: &mut Canvas, rect: Rect, icon: u8, title: &str) -> Rect {
    let last_x = rect.x + rect.w - 1;
    let last_y = rect.y + rect.h - 1;

    cv.fill(
        Rect::new(rect.x, rect.y, rect.w, 1),
        Cell::new(BOX_H, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.fill(
        Rect::new(rect.x, last_y, rect.w, 1),
        Cell::new(BOX_H, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.fill(
        Rect::new(rect.x, rect.y, 1, rect.h),
        Cell::new(BOX_V, ST_BORDER.fg, ST_BORDER.bg),
    );
    cv.fill(
        Rect::new(last_x, rect.y, 1, rect.h),
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
        Rect::new(inner.x, inner.y, inner.w, 1),
        Cell::new(b' ', ST_TITLE.fg, ST_TITLE.bg),
    );
    // Opaque body: blank cells under the content so the wallpaper cannot
    // bleed through the window.
    cv.fill(
        Rect::new(inner.x, inner.y + 1, inner.w, inner.h.saturating_sub(1)),
        Cell::BLANK,
    );
    let mut badge = [0u8; 24];
    badge[0] = icon;
    badge[1] = b' ';
    let t = title.as_bytes();
    let n = t.len().min(badge.len() - 2);
    badge[2..2 + n].copy_from_slice(&t[..n]);
    cv.field(
        inner.x as usize,
        inner.y as usize,
        &badge[..2 + n],
        ST_TITLE,
        inner.w as usize,
    );

    Rect::new(inner.x, inner.y + 1, inner.w, inner.h.saturating_sub(1))
}

// --------------------------------------------------------------- desktop
//
// 80x25 layout: one top bar, three columns (system + memory, boot log,
// subsystems) and a notification toast in the bottom-right corner.

const RECT_SYSTEM: Rect = Rect::new(0, 1, 24, 12);
const RECT_MEMORY: Rect = Rect::new(0, 14, 24, 11);
const RECT_LOG: Rect = Rect::new(25, 1, 30, 24);
const RECT_STATUS: Rect = Rect::new(56, 1, 24, 24);
const RECT_NOTE: Rect = Rect::new(56, 18, 24, 6);

fn top_bar(cv: &mut Canvas, info: &SessionInfo) {
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

fn render_system(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
    let body = window(cv, rect, ICON_SYSTEM, "system");
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

fn render_memory(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
    let body = window(cv, rect, ICON_MEMORY, "memory");
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

fn render_log(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
    let body = window(cv, rect, ICON_LOG, "boot log");
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

fn render_status(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
    let body = window(cv, rect, ICON_STATUS, "subsystems");
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
    let mut badge = [0u8; 22];
    badge[0] = ICON_SESSION;
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

fn render_notification(cv: &mut Canvas, rect: Rect, info: &SessionInfo) {
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

// ---------------------------------------------------------------- session

fn splash(cv: &mut Canvas, info: &SessionInfo) {
    for y in 0..H {
        for x in 0..W {
            cv.cells[y * W + x] = wallpaper(x, y);
        }
    }
    cv.hline(24, 6, 32, RULE, ST_RULE);
    cv.hline(24, 16, 32, RULE, ST_RULE);

    cv.centered(8, b"K E R A U N O S", ST_ACCENT);
    cv.centered(10, b"the thunderbolt operating system", ST_BODY);
    cv.centered(12, b"fast, smooth, powerful", ST_RULE);

    let mut line = [0u8; 24];
    let mut n = 0;
    push_bytes(&mut line, &mut n, b"version ");
    push_bytes(&mut line, &mut n, info.version.as_bytes());
    cv.centered(14, &line[..n], ST_MUTED);
}

/// Session phases, in bring-up order. Each maps to exactly one frame;
/// the splash is its own renderer, the rest compose the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    Base,
    Mapped,
    Notified,
}

fn desktop(cv: &mut Canvas, info: &SessionInfo, phase: Phase) {
    for y in 0..H {
        for x in 0..W {
            cv.cells[y * W + x] = wallpaper(x, y);
        }
    }
    top_bar(cv, info);

    if phase >= Phase::Mapped {
        render_system(cv, RECT_SYSTEM, info);
        render_memory(cv, RECT_MEMORY, info);
        render_log(cv, RECT_LOG, info);
        render_status(cv, RECT_STATUS, info);
    }
    if phase == Phase::Notified {
        render_notification(cv, RECT_NOTE, info);
    }
}

/// Render and present the whole desktop bring-up, then return; the caller
/// parks the CPU. Deterministic: same info in, same frames out, every boot.
pub fn boot_session(surface: &mut dyn Surface, info: &SessionInfo) {
    let mut cv = Canvas::new();

    info.say("session: splash on text surface");
    splash(&mut cv, info);
    cv.present(surface);

    info.say("compositor: top bar and wallpaper mapped");
    desktop(&mut cv, info, Phase::Base);
    cv.present(surface);

    info.say("compositor: system, memory, boot log, subsystems mapped");
    desktop(&mut cv, info, Phase::Mapped);
    cv.present(surface);

    info.say("notification: framework ready -- BOOT OK");
    desktop(&mut cv, info, Phase::Notified);
    cv.present(surface);
}
