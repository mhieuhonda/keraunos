//! AArch64 (ARMv8-A) scaffold.
//!
//! Bring-up order once active: EL2/EL1 stage configuration, custom
//! translation tables, GICv3 interrupt controller, architectural timer,
//! PSCI for SMP. See `docs/HARDWARE.md` for the target matrix.

/// Park the CPU until the next interrupt, forever.
pub fn halt_forever() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
