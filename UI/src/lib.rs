//! # keraunos-ui — the Keraunos desktop
//!
//! Theme, shell and compositor for the kernel's text surfaces. The crate
//! is `no_std`, allocator-free and contains no `unsafe` at all — every
//! hardware detail stays behind the [`Surface`] contract, which the kernel
//! implements for its framebuffer.
//!
//! The shell mirrors the GNOME Shell layout, trimmed to what this desktop
//! shows: [`panel`] draws the top bar, [`overview`] maps the window board,
//! [`notifications`] raises toasts, and [`theme`] dresses it all. [`icons`]
//! and [`sounds`] resolve chrome into the Keraunos icon and sound themes
//! under `UI/assets`.
//!
//! Rendering is deterministic: a session phase maps to exactly one frame,
//! so serial logs, CI screen dumps and real hardware always agree.

#![no_std]
#![forbid(unsafe_code)]

pub mod canvas;
pub mod icons;
pub mod notifications;
pub mod overview;
pub mod panel;
pub mod sounds;
pub mod theme;

use canvas::Canvas;

// ------------------------------------------------------ kernel contract

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
    pub const BLANK: Self = Self::new(b' ', theme::WHITE, theme::BLACK);

    pub const fn new(ch: u8, fg: u8, bg: u8) -> Self {
        Self { ch, fg, bg }
    }

    /// Packed attribute byte (fg | bg << 4), as the text surface expects it.
    pub fn attr(&self) -> u8 {
        (self.bg << 4) | self.fg
    }
}

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
            State::Online => theme::LIGHT_GREEN,
            State::Scaffold => theme::YELLOW,
            State::Planned => theme::DARK_GRAY,
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

    fn say_sound(&self, file: &str) {
        let mut msg = [0u8; 64];
        let mut n = 0;
        canvas::push_bytes(&mut msg, &mut n, b"sound: ");
        canvas::push_bytes(&mut msg, &mut n, sounds::THEME_DIR.as_bytes());
        canvas::push_bytes(&mut msg, &mut n, b"/");
        canvas::push_bytes(&mut msg, &mut n, file.as_bytes());
        // Serial narration is ASCII-safe by construction of the theme path.
        let text = core::str::from_utf8(&msg[..n]).unwrap_or("sound: (unavailable)");
        self.say(text);
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
    pub(crate) fn inset(&self) -> Self {
        Self::new(
            self.x + 1,
            self.y + 1,
            self.w.saturating_sub(2),
            self.h.saturating_sub(2),
        )
    }
}

// ---------------------------------------------------------------- session

fn splash(cv: &mut Canvas, info: &SessionInfo) {
    theme::paint_wallpaper(cv);
    cv.hline(24, 6, 32, theme::RULE, theme::ST_RULE);
    cv.hline(24, 16, 32, theme::RULE, theme::ST_RULE);

    cv.centered(8, b"K E R A U N O S", theme::ST_ACCENT);
    cv.centered(10, b"the thunderbolt operating system", theme::ST_BODY);
    cv.centered(12, b"fast, smooth, powerful", theme::ST_RULE);

    let mut line = [0u8; 24];
    let mut n = 0;
    canvas::push_bytes(&mut line, &mut n, b"version ");
    canvas::push_bytes(&mut line, &mut n, info.version.as_bytes());
    cv.centered(14, &line[..n], theme::ST_MUTED);
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
    theme::paint_wallpaper(cv);
    panel::top_bar(cv, info);

    if phase >= Phase::Mapped {
        overview::render_system(cv, overview::RECT_SYSTEM, info);
        overview::render_memory(cv, overview::RECT_MEMORY, info);
        overview::render_log(cv, overview::RECT_LOG, info);
        overview::render_status(cv, overview::RECT_STATUS, info);
    }
    if phase == Phase::Notified {
        notifications::render_notification(cv, notifications::RECT_NOTE, info);
    }
}

/// Render and present the whole desktop bring-up, then return; the caller
/// parks the CPU. Deterministic: same info in, same frames out, every boot.
pub fn boot_session(surface: &mut dyn Surface, info: &SessionInfo) {
    let mut cv = Canvas::new();

    info.say("session: splash on text surface");
    sounds::play(sounds::Event::Login, info);
    splash(&mut cv, info);
    cv.present(surface);

    info.say("compositor: top bar and wallpaper mapped");
    desktop(&mut cv, info, Phase::Base);
    cv.present(surface);

    info.say("compositor: system, memory, boot log, subsystems mapped");
    desktop(&mut cv, info, Phase::Mapped);
    cv.present(surface);

    info.say("notification: framework ready -- BOOT OK");
    sounds::play(sounds::Event::SystemReady, info);
    desktop(&mut cv, info, Phase::Notified);
    cv.present(surface);
}
