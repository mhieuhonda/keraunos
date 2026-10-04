//! Kernel logging: `kinfo!` / `kerror!` fan out to every console sink.
//!
//! Formatting uses `core::fmt` only (integer, string and hex formatting are
//! fully supported in `no_std`), so no allocator is ever required to log.

/// Log an informational line.
///
/// ```ignore
/// kinfo!("mm", "frame {:#012x} ok", frame);
/// ```
#[macro_export]
macro_rules! kinfo {
    ($tag:expr, $($arg:tt)+) => {
        $crate::console::log_line("INFO ", $tag, format_args!($($arg)+))
    };
}

/// Log an error line. On VGA consoles this renders in red.
#[macro_export]
macro_rules! kerror {
    ($tag:expr, $($arg:tt)+) => {
        $crate::console::log_line("ERROR", $tag, format_args!($($arg)+))
    };
}
