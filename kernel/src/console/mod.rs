//! Console subsystem: fan-out writer over all available sinks.
//!
//! On x86-64 the kernel drives two sinks simultaneously — the VGA text
//! buffer (what you see on screen) and the first 16550 UART (what lands in
//! `qemu -serial` / a serial cable). When the desktop session starts, the
//! VGA sink is detached (console handoff) and the display service takes
//! over the framebuffer; logging continues on the UART alone.
//!
//! Every formatted log line is also captured into a small mirror ring so
//! the desktop's boot-log view can show what happened before the handoff.

use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[cfg(target_arch = "x86_64")]
pub mod uart;
#[cfg(target_arch = "x86_64")]
pub mod vga;

/// Size of the boot-log mirror ring (about 8 lines of 80-column text are
/// expected; the ring keeps the most recent 1 KiB).
pub const MIRROR_BYTES: usize = 1024;

/// Initialize every console sink. Safe to call exactly once during boot.
pub fn early_init() {
    #[cfg(target_arch = "x86_64")]
    {
        vga::init();
        uart::init();
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        // No sink is wired on this scaffold yet; log_line/write_str drop output.
    }
}

static VGA_DETACHED: AtomicBool = AtomicBool::new(false);

/// Detach the VGA sink; called once by the display service at handoff.
pub fn detach_vga() {
    VGA_DETACHED.store(true, Ordering::Relaxed);
}

fn vga_active() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        !VGA_DETACHED.load(Ordering::Relaxed)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Write a raw string to every sink (VGA only until the console handoff).
pub fn write_str(s: &str) {
    #[cfg(target_arch = "x86_64")]
    {
        if vga_active() {
            vga::write_str(s);
        }
    }
    uart_write(s);
}

#[cfg(target_arch = "x86_64")]
fn uart_write(s: &str) {
    uart::write_str(s);
}

#[cfg(not(target_arch = "x86_64"))]
fn uart_write(s: &str) {
    let _ = s;
}

/// Write a formatted line (no level prefix) to every sink and the mirror.
/// Used by the subsystem status table.
pub fn write_line(args: fmt::Arguments) {
    use core::fmt::Write as _;
    let mut out = Fanout;
    let _ = out.write_fmt(args);
    let _ = out.write_str("\n");
}

/// Plain informational log entry point; the narration hook the desktop
/// session uses to keep the serial log in sync with its own phases.
pub fn log_info(tag: &str, msg: &str) {
    log_line("INFO ", tag, format_args!("{}", msg));
}

// ------------------------------------------------------------- log mirror

/// Fixed-size byte ring capturing formatted log lines, newest last.
///
/// Writes come only from `log_line`/`write_line` during single-threaded
/// boot; `snapshot` copies the written region out before the desktop
/// renders it, so no reader ever overlaps a writer.
struct Mirror {
    cell: UnsafeCell<[u8; MIRROR_BYTES]>,
    written: AtomicUsize,
}

// SAFETY: the mirror is written by the single boot thread only; snapshots
// copy out under the same thread before any read happens.
unsafe impl Sync for Mirror {}

impl Mirror {
    const fn new() -> Self {
        Self {
            cell: UnsafeCell::new([0; MIRROR_BYTES]),
            written: AtomicUsize::new(0),
        }
    }

    fn append(&self, bytes: &[u8]) {
        let written = self.written.load(Ordering::Relaxed);
        // SAFETY: exclusive boot-thread writer, per the Sync argument above.
        let ring = unsafe { &mut *self.cell.get() };
        for (i, &b) in bytes.iter().enumerate() {
            ring[(written + i) % MIRROR_BYTES] = b;
        }
        self.written.store(written + bytes.len(), Ordering::Relaxed);
    }

    /// Copy the captured region into `out` (linear, oldest first) and
    /// return the filled slice.
    fn snapshot<'a>(&self, out: &'a mut [u8; MIRROR_BYTES]) -> &'a [u8] {
        let written = self.written.load(Ordering::Relaxed);
        // SAFETY: shared read on the boot thread, concurrent with no writer.
        let ring = unsafe { &*self.cell.get() };
        if written <= MIRROR_BYTES {
            out[..written].copy_from_slice(&ring[..written]);
            &out[..written]
        } else {
            let start = written % MIRROR_BYTES;
            out[..MIRROR_BYTES - start].copy_from_slice(&ring[start..]);
            out[MIRROR_BYTES - start..].copy_from_slice(&ring[..start]);
            &out[..]
        }
    }
}

static MIRROR: Mirror = Mirror::new();

/// Linearized copy of the captured boot log; the caller provides storage.
pub fn mirror_snapshot(out: &mut [u8; MIRROR_BYTES]) -> &[u8] {
    MIRROR.snapshot(out)
}

// ---------------------------------------------------------------- logging

/// `fmt::Write` adapter that fans out into every sink and the mirror.
struct Fanout;

impl fmt::Write for Fanout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        MIRROR.append(s.as_bytes());
        uart_write(s);
        #[cfg(target_arch = "x86_64")]
        {
            if vga_active() {
                vga::write_str(s);
            }
        }
        Ok(())
    }
}

/// Formatted logging entry point used by the `kinfo!` / `kerror!` macros.
pub fn log_line(level: &str, tag: &str, args: fmt::Arguments) {
    #[cfg(target_arch = "x86_64")]
    let level_color = if level == "ERROR" {
        vga::Color::Red
    } else {
        vga::Color::LightCyan
    };
    #[cfg(target_arch = "x86_64")]
    vga::push_color(level_color);

    let mut out = Fanout;
    use core::fmt::Write as _;
    let _ = out.write_fmt(format_args!("[{}] {:<8} ", level, tag));
    let _ = fmt::write(&mut out, args);
    let _ = out.write_str("\n");

    #[cfg(target_arch = "x86_64")]
    vga::pop_color();
}
