//! RISC-V 64 (RV64GC) scaffold.
//!
//! Bring-up order once active: M-mode to S-mode hand-off (OpenSBI), Sv39
//! translation tables, PLIC/CLINT interrupt plumbing, SBI console for early
//! logs. See `docs/HARDWARE.md` for the target matrix.

/// Park the hart until the next interrupt, forever.
pub fn halt_forever() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}
