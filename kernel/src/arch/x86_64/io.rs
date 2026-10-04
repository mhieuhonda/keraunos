//! Port-mapped I/O primitives for the x86-64 architecture.

/// Write one byte to an I/O port.
///
/// # Safety
/// The caller must ensure the port is safe to write on this platform.
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Read one byte from an I/O port.
///
/// # Safety
/// The caller must ensure the port is safe to read on this platform.
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            lateout("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

#[allow(dead_code)] // kept for the PIC/PIT bring-up in M1
/// Small I/O port write used to add latency between legacy device accesses.
///
/// # Safety
/// The caller must ensure port `0x80` is safe to touch (it is the POST card
/// / debug port on PC-compatible systems).
pub unsafe fn io_wait() {
    unsafe { outb(0x80, 0) };
}
