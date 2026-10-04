//! Architecture dispatch layer.
//!
//! `hal/` owns the portable abstractions; this module only picks the
//! implementation that matches `cfg(target_arch)`.

#[cfg(target_arch = "aarch64")]
pub mod aarch64;
#[cfg(target_arch = "riscv64")]
pub mod riscv64;
#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[allow(dead_code)] // used on the non-x86_64 boot path only
/// Canonical architecture name for logs and diagnostics.
pub fn arch_name() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "aarch64"
    }
    #[cfg(target_arch = "riscv64")]
    {
        "riscv64"
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        "unknown"
    }
}

#[cfg(target_arch = "aarch64")]
pub use aarch64::halt_forever;
#[cfg(target_arch = "riscv64")]
pub use riscv64::halt_forever;
#[cfg(target_arch = "x86_64")]
pub use x86_64::halt_forever;
