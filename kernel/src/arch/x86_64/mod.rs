//! x86-64 implementation: long-mode boot, CPU probing, port I/O, GDT.

pub mod cpu;
pub mod gdt;
pub mod io;

/// Disable interrupts and park the CPU forever.
pub fn halt_forever() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli", "hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
