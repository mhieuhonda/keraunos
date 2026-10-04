//! Global Descriptor Table bring-up.
//!
//! Loads a minimal 64-bit GDT (null / kernel-code64 / kernel-data) and
//! reloads every segment register, including `CS` via a far return. This is
//! the foundation the IDT (milestone M1) builds its segment selectors on.

/// Limit of the GDT in bytes (three 8-byte entries).
const GDT_LIMIT: u16 = 24 - 1;

/// Selector offsets into the table.
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;

/// Descriptor: present, code, readable, long mode (L=1).
const CODE64: u64 = 0x0020_9A00_0000_0000;
/// Descriptor: present, data, writable.
const DATA64: u64 = 0x0000_9200_0000_0000;

#[allow(dead_code)] // the table is consumed by LGDT via its address
/// The three-entry GDT, 8-byte aligned by construction.
#[repr(align(8))]
struct Table {
    null: u64,
    code: u64,
    data: u64,
}

static GDT: Table = Table {
    null: 0,
    code: CODE64,
    data: DATA64,
};

/// Load the GDT and refresh all segment registers.
pub fn init() {
    // Build the 10-byte GDTR image (u16 limit + u64 base) in a byte buffer
    // so no unaligned references are formed.
    let mut gdtr = [0u8; 10];
    gdtr[0..2].copy_from_slice(&GDT_LIMIT.to_le_bytes());
    let base = &GDT as *const Table as u64;
    gdtr[2..10].copy_from_slice(&base.to_le_bytes());

    unsafe {
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) gdtr.as_ptr(),
            options(nostack, preserves_flags)
        );

        // Reload data segments; fs/gs are cleared until per-CPU areas exist.
        let data = KERNEL_DATA_SELECTOR as u32;
        let zero = 0u32;
        core::arch::asm!(
            "mov ds, {d:x}",
            "mov es, {d:x}",
            "mov ss, {d:x}",
            "mov fs, {z:x}",
            "mov gs, {z:x}",
            d = in(reg) data,
            z = in(reg) zero,
            options(preserves_flags)
        );

        // Reload CS with a far return through selector 0x08.
        core::arch::asm!(
            "lea {tmp}, [rip + 2f]",
            "push {code}",
            "push {tmp}",
            "retfq",
            "2:",
            tmp = out(reg) _,
            code = const KERNEL_CODE_SELECTOR,
        );
    }
}
