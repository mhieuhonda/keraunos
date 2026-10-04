//! # Keraunos kernel
//!
//! Entry point and early boot sequence for the Keraunos operating system.
//!
//! The kernel is `no_std` and dependency-free. Today it boots on x86-64 via
//! GRUB2/multiboot2, brings up a dual console (VGA text + debug UART), parses
//! the boot memory map, arms an early physical-frame allocator and reports
//! subsystem status. Everything beyond this bring-up is tracked in
//! `docs/ROADMAP.md`.
//!
//! Architecture scaffolds for aarch64 and riscv64 live under `src/arch/` and
//! are type-checked in CI (`cargo check`) — see `docs/HARDWARE.md`.

#![no_std]
#![no_main]
// Non-x86-64 scaffolds have no boot protocol wired yet, so the multiboot2
// parser and friends sit dormant there; silence dead-code analysis for them.
#![cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]

#[macro_use]
mod klog;

mod arch;
mod boot;
mod console;
mod drivers;
mod fs;
mod hal;
mod ipc;
mod mm;
mod net;
mod sched;

#[cfg(target_arch = "x86_64")]
use core::arch::global_asm;

#[cfg(target_arch = "x86_64")]
use boot::multiboot2::MULTIBOOT2_BOOT_MAGIC;

/// Keraunos release string, baked in from the workspace version at compile time.
const VERSION: &str = env!("CARGO_PKG_VERSION");

// The multiboot2 header and the 64-bit entry stub are x86-64 specific. The
// aarch64/riscv64 scaffolds provide their own `_start` below.
#[cfg(target_arch = "x86_64")]
global_asm! {
    // ------------------------------------------------------------------
    // Multiboot2 header (must live within the first 32 KiB of the image).
    // Requests: command line, bootloader name, memory map. Declares the
    // entry point explicitly so flat and ELF layouts both work.
    // ------------------------------------------------------------------
    ".section .multiboot2_header, \"a\"",
    ".align 8",
    "_mb2_header_start:",
    ".long 0xE85250D6",                                              // magic
    ".long 0",                                                       // architecture: i386
    ".long _mb2_header_end - _mb2_header_start",                     // header length
    ".long -(0xE85250D6 + 0 + (_mb2_header_end - _mb2_header_start))", // checksum
    // information request tag: command line(1), bootloader name(2), memory map(6)
    // (tags are 8-byte aligned; GRUB advances by padded size)
    ".align 8",
    ".short 1", ".short 0", ".long 20",
    ".long 1", ".long 2", ".long 6",
    // entry address tag
    ".align 8",
    ".short 3", ".short 0", ".long 12", ".long _start",
    // end tag
    ".align 8",
    ".short 0", ".short 0", ".long 8",
    "_mb2_header_end:",

    // ------------------------------------------------------------------
    // Entry point. GRUB2 hands off in 32-bit protected mode with paging
    // disabled (EAX = multiboot2 magic, EBX = MBI pointer), even for
    // ELF64 images. The 32-bit stub saves the hand-off registers, loads
    // the boot GDT, builds identity page tables (1 GiB, 2 MiB pages),
    // enables PAE + EFER.LME + paging and far-jumps into 64-bit mode.
    // The 64-bit continuation then installs the boot stack, zeroes .bss
    // and hands (magic, mbi) to kmain.
    // ------------------------------------------------------------------
    ".section .text.entry",
    ".align 16",
    ".code32",
    ".global _start",
    "_start:",
    "    mov esi, eax",                          // save multiboot2 magic (survives the stub)
    "    mov ebp, ebx",                          // save MBI pointer (EDI is used by rep stosd)
    "    cld",
    "    lgdt [boot_gdt_ptr]",
    // --- build identity page tables in .boot_pgtables (12 KiB) ---
    // (lea reg, [symbol] loads the symbol's address; AT&T's $-immediate
    // syntax is unusable inside global_asm! because $ marks operands)
    "    lea edi, [_pml4]",
    "    xor eax, eax",
    "    mov ecx, 3072",                         // 3 pages of dwords
    "    rep stosd",
    "    lea eax, [_pdpt]",
    "    or eax, 3",                             // PRESENT | WRITE
    "    mov [_pml4], eax",
    "    lea eax, [_pd]",
    "    or eax, 3",                             // PRESENT | WRITE
    "    mov [_pdpt], eax",
    "    lea edi, [_pd]",
    "    mov ecx, 512",
    "    xor eax, eax",
    "1:  mov edx, eax",
    "    shl edx, 21",                            // i-th 2 MiB frame
    "    or edx, 0x83",                          // PS | RW | PRESENT
    "    mov [edi], edx",
    "    add edi, 8",
    "    inc eax",
    "    loop 1b",
    // --- enable PAE, load CR3, set EFER.LME, enable paging ---
    "    mov eax, cr4",
    "    or eax, 0x20",                           // PAE
    "    mov cr4, eax",
    "    lea eax, [_pml4]",
    "    mov cr3, eax",
    "    mov ecx, 0xC0000080",                    // IA32_EFER
    "    rdmsr",
    "    or eax, 0x100",                          // LME
    "    wrmsr",
    "    mov eax, cr0",
    "    or eax, 0x80000000",                     // PG (PE is already set)
    "    mov cr0, eax",
    // Far jump into the 64-bit code segment (selector 0x08). Encoded
    // manually (0xEA = JMP ptr16:32) because the assembler's $-syntax for
    // far jumps collides with inline-asm operand markers.
    "    .byte 0xEA",
    "    .long keraunos_long_mode",
    "    .short 0x08",

    ".code64",
    ".global keraunos_long_mode",
    "keraunos_long_mode:",
    "    mov ax, 0x10",                           // data selectors
    "    mov ds, ax",
    "    mov es, ax",
    "    mov ss, ax",
    "    mov fs, ax",
    "    mov gs, ax",
    "    mov rdx, rsi",                           // magic
    "    mov r8, rbp",                            // MBI pointer (saved in EBP by the 32-bit stub)
    "    lea rsp, [rip + _stack_top]",            // boot stack
    "    lea rdi, [rip + __bss_start]",
    "    lea rcx, [rip + __bss_end]",
    "    sub rcx, rdi",
    "    xor eax, eax",
    "    rep stosb",                              // zero .bss
    "    mov rdi, rdx",                           // kmain(magic, mbi)
    "    mov rsi, r8",
    "    call kmain",
    "2:  cli",                                    // kmain never returns; belt & braces
    "    hlt",
    "    jmp 2b",

    // ------------------------------------------------------------------
    // Boot GDT: null / 64-bit code / data. Used by the 32-bit stub for
    // the mode switch; kmain loads its own GDT afterwards (harmless).
    // ------------------------------------------------------------------
    ".section .rodata",
    ".align 8",
    "boot_gdt:",
    "    .quad 0",
    "    .quad 0x00209A0000000000",               // code64: L=1, D=0
    "    .quad 0x0000920000000000",               // data64
    "boot_gdt_end:",
    "boot_gdt_ptr:",
    "    .word boot_gdt_end - boot_gdt - 1",
    "    .long boot_gdt",
}

/// Multiboot2 hand-off, called by the boot stub above.
#[cfg(target_arch = "x86_64")]
#[no_mangle]
extern "C" fn kmain(magic: u32, mbi_addr: usize) -> ! {
    console::early_init();
    print_banner();

    if magic != MULTIBOOT2_BOOT_MAGIC {
        kerror!("boot", "unexpected hand-off magic {:#010x}; halting", magic);
        arch::halt_forever();
    }

    // SAFETY: `mbi_addr` is the multiboot2 information pointer passed by the
    // bootloader in EBX; it is identity-mapped and 8-byte aligned per spec.
    let mbi = unsafe { boot::multiboot2::Info::from_addr(mbi_addr) };
    kinfo!(
        "boot",
        "boot protocol: multiboot2 via {}",
        mbi.bootloader_name()
    );
    if let Some(cmdline) = mbi.cmdline() {
        if !cmdline.is_empty() {
            kinfo!("boot", "kernel command line: \"{}\"", cmdline);
        }
    }

    // Architecture bring-up.
    arch::x86_64::gdt::init();
    kinfo!("arch", "GDT loaded (null / code64 / data)");
    let vendor = arch::x86_64::cpu::vendor_string();
    let vendor = core::str::from_utf8(&vendor).unwrap_or("unknown");
    kinfo!("arch", "cpu vendor: {} | long mode active | BSP", vendor);

    // Memory bring-up.
    let map = mbi.memory_map();
    let usable = map.usable_bytes();
    kinfo!(
        "mm",
        "boot memory map: {} regions, {} MiB + {} KiB usable",
        map.regions().len(),
        usable / 1024 / 1024,
        (usable % (1024 * 1024)) / 1024
    );

    let mut frame_alloc = match mm::bump::BumpAllocator::from_memory_map(&map) {
        Some(alloc) => alloc,
        None => {
            kerror!(
                "mm",
                "no usable memory region above the kernel image; halting"
            );
            arch::halt_forever();
        }
    };
    kinfo!("mm", "early bump allocator armed above the kernel image");
    match frame_alloc.alloc_contiguous(1) {
        Some(frame) => kinfo!("mm", "frame smoke test: {:#012x} (4 KiB) ok", frame),
        None => kerror!("mm", "frame allocation failed"),
    }

    print_subsystem_table();

    kinfo!("kernel", "keraunos {} framework ready -- BOOT OK", VERSION);
    arch::halt_forever();
}

/// Scaffold entry for architectures without a wired boot protocol yet.
#[cfg(not(target_arch = "x86_64"))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    console::early_init();
    kinfo!(
        "boot",
        "keraunos scaffold on {}: bring-up pending (see docs/HARDWARE.md)",
        arch::arch_name()
    );
    arch::halt_forever();
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    kerror!("panic", "{}", info);
    arch::halt_forever();
}

/// Boot banner on every console sink.
fn print_banner() {
    #[cfg(target_arch = "x86_64")]
    console::vga::set_banner_color();
    console::write_str("\n");
    console::write_str("+==========================================================+\n");
    console::write_str("|  K E R A U N O S                                         |\n");
    console::write_str("|  the thunderbolt operating system                        |\n");
    console::write_str("|  power - performance - speed, written in Rust            |\n");
    console::write_str("+==========================================================+\n");
    #[cfg(target_arch = "x86_64")]
    console::vga::set_default_color();
}

/// Subsystem bring-up status, kept in sync with docs/ROADMAP.md.
fn print_subsystem_table() {
    kinfo!("status", "subsystem bring-up status:");
    console::write_str("    console      online    vga text + debug uart\n");
    console::write_str("    boot         online    multiboot2\n");
    console::write_str("    arch         online    x86-64 long mode\n");
    console::write_str("    mm           online    memory map + bump frames\n");
    console::write_str("    interrupts   planned   M1\n");
    console::write_str("    scheduler    scaffold  M3\n");
    console::write_str("    drivers      scaffold  M4\n");
    console::write_str("    fs           planned   M4\n");
    console::write_str("    net          planned   M5\n");
    console::write_str("    ipc          planned   M5\n");
    console::write_str("    linux-abi    planned   M6 (unmodified userland)\n");
    console::write_str("    ui           staged    GNOME mutter/shell + Yaru in /UI\n");
}
