//! Console subsystem: fan-out writer over all available sinks.
//!
//! On x86-64 the kernel drives two sinks simultaneously — the VGA text
//! buffer (what you see on screen) and the first 16550 UART (what lands in
//! `qemu -serial` / a serial cable). Other architectures currently have no
//! wired sink and silently drop output until their bring-up lands.

use core::fmt;

#[cfg(target_arch = "x86_64")]
pub mod uart;
#[cfg(target_arch = "x86_64")]
pub mod vga;

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

/// Write a raw string to every sink.
pub fn write_str(s: &str) {
    #[cfg(target_arch = "x86_64")]
    {
        vga::write_str(s);
        uart::write_str(s);
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = s;
    }
}

/// Formatted logging entry point used by the `kinfo!` / `kerror!` macros.
pub fn log_line(level: &str, tag: &str, args: fmt::Arguments) {
    /// `fmt::Write` adapter that fans out into [`write_str`].
    struct Fanout;

    impl fmt::Write for Fanout {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            write_str(s);
            Ok(())
        }
    }

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
