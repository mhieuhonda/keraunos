//! CPU identification and control for x86-64.

/// Return the 12-character CPUID vendor string (e.g. `GenuineIntel`,
/// `AuthenticAMD`, `KVMKVMKVM`) as a NUL-terminated byte array.
///
/// CPUID writes the vendor bytes into EBX, but RBX is a reserved register
/// on this target that LLVM may hold live values in (function pointers,
/// loop counters, …) across the call. The vendor bytes are exported through
/// a compiler-allocated register and RBX is saved/restored around the
/// instruction so the compiler's bookkeeping stays intact. Omitting this
/// save/restore compiles fine and crashes in optimized builds only —
/// a bug class the dual-profile boot tests in CI exist to catch.
pub fn vendor_string() -> [u8; 13] {
    let ebx: u32;
    let ecx: u32;
    let edx: u32;
    unsafe {
        core::arch::asm!(
            "push rbx",         // save: cpuid destroys EBX, and rbx is reserved
            "cpuid",
            "mov {v:e}, ebx",  // export the vendor bytes before restoring
            "pop rbx",
            inout("eax") 0u32 => _,       // leaf 0 in, max leaf out (unused)
            v = out(reg) ebx,
            lateout("ecx") ecx,
            lateout("edx") edx,
            options(nomem)      // note: stack + flags are clobbered on purpose
        );
    }
    let mut vendor = [0u8; 13];
    vendor[0..4].copy_from_slice(&ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&ecx.to_le_bytes());
    vendor[12] = 0;
    vendor
}
