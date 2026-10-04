//! x86-64 page tables.
//!
//! The 32-bit entry stub builds a bare 1 GiB identity map just to reach
//! long mode. This module replaces it with the real thing: fresh tables
//! from the PMM, the same identity window for bring-up (the kernel still
//! executes and logs through it), and the kernel image mapped into the
//! higher half at [`KERNEL_BASE`] so user space gets the low half when
//! address spaces arrive in M3.
//!
//! Layout of the top window (PML4[511]):
//!
//! ```text
//!   0xFFFF_FFFF_8000_0000   PDPT[510]: 1 GiB higher-half alias of
//!                           physical RAM (2 MiB pages) — the kernel
//!                           image lives at KERNEL_BASE + 1 MiB
//!   0xFFFF_FFFF_C000_0000   PDPT[511]: on-demand mappings — kernel
//!                           heap, scratch pages
//! ```
//!
//! PML4[510] self-maps the page directory for the recursive accessors
//! below: every table entry is reachable through a fixed virtual address
//! arithmetic derives from the virtual address being walked.
//!
//! Table frames come from the DMA zone: while the kernel runs through the
//! 1 GiB bring-up window, only sub-1 GiB frames are reachable through the
//! identity alias, and 16 MiB is far more RAM than page tables need.

#![allow(dead_code)] // helpers exercised by the self-test and later milestones

use super::pmm::{self, Zone};
use super::PAGE_SIZE;

// The 32-bit entry stub's identity-map PDPT (see `main.rs`). The new
// tables keep pointing at it for PML4[0], so the kernel keeps executing
// from the identity window until M3 tears that map down.
extern "C" {
    static _pdpt: u8;
}

/// Base of the higher-half kernel window (PML4[511], PDPT[510]).
pub const KERNEL_BASE: u64 = 0xFFFF_FFFF_8000_0000;

/// Base of the on-demand kernel window (PML4[511], PDPT[511]): kernel
/// heap and scratch pages, one PDPT slot above the physmap alias.
pub const HEAP_BASE: u64 = KERNEL_BASE + 1024 * 1024 * 1024;

/// PML4 index of the higher-half kernel window.
const PML4_KERNEL: usize = 511;

/// PML4 index of the recursive self-map.
const PML4_RECURSIVE: usize = 510;

/// PDPT slot of the 1 GiB higher-half alias inside the kernel window.
const PDPT_PHYSMAP: usize = 510;

/// Recursive self-map base: PML4[510] -> itself. Adding the shifted
/// indexes of a virtual address reaches each table entry on its path.
const REC_BASE: u64 = 0xFFFF_0000_0000_0000 | (510 << 39) | (510 << 30) | (510 << 21) | (510 << 12);

const PRESENT: u64 = 1 << 0;
const WRITE: u64 = 1 << 1;
const NO_CACHE: u64 = 1 << 4;
const HUGE: u64 = 1 << 7;

/// Address bits of a 4 KiB page-table entry.
const ADDR_MASK_4K: u64 = 0x000F_FFFF_FFFF_F000;

/// Address bits of a 2 MiB page-directory entry.
const ADDR_MASK_2M: u64 = 0x000F_FFFF_FFE0_0000;

/// Permission bits for a mapped page.
#[derive(Debug, Clone, Copy)]
pub struct Flags {
    pub writable: bool,
    pub no_cache: bool,
}

impl Flags {
    /// Read-write kernel mapping.
    pub const RW: Flags = Flags {
        writable: true,
        no_cache: false,
    };

    /// Read-only kernel mapping.
    pub const RO: Flags = Flags {
        writable: false,
        no_cache: false,
    };

    fn bits(self) -> u64 {
        let mut bits = PRESENT;
        if self.writable {
            bits |= WRITE;
        }
        if self.no_cache {
            bits |= NO_CACHE;
        }
        bits
    }
}

fn index_of(virt: u64, shift: u32) -> usize {
    ((virt >> shift) & 0x1FF) as usize
}

/// Virtual address of the PML4 entry for `virt`, through the self-map.
fn pml4_entry(virt: u64) -> *mut u64 {
    (REC_BASE + index_of(virt, 39) as u64 * 8) as *mut u64
}

/// Virtual address of the PDPT entry for `virt`.
fn pdpt_entry(virt: u64) -> *mut u64 {
    (REC_BASE + index_of(virt, 39) as u64 * (1 << 30) + index_of(virt, 30) as u64 * 8) as *mut u64
}

/// Virtual address of the PD entry for `virt`.
fn pd_entry(virt: u64) -> *mut u64 {
    let base =
        REC_BASE + index_of(virt, 39) as u64 * (1 << 30) + index_of(virt, 30) as u64 * (1 << 21);
    (base + index_of(virt, 21) as u64 * 8) as *mut u64
}

/// Virtual address of the PT entry for `virt`.
fn pt_entry(virt: u64) -> *mut u64 {
    let base = REC_BASE
        + index_of(virt, 39) as u64 * (1 << 30)
        + index_of(virt, 30) as u64 * (1 << 21)
        + index_of(virt, 21) as u64 * (1 << 12);
    (base + index_of(virt, 12) as u64 * 8) as *mut u64
}

/// Higher-half alias of a physical address inside the 1 GiB bring-up
/// window. The alias is how the kernel will reach physical RAM once the
/// identity map is torn down in M3.
pub fn higher_half(phys: u64) -> u64 {
    KERNEL_BASE + phys
}

/// Physical frame backing `virt`, if it is mapped. Handles both the
/// 2 MiB alias entries and 4 KiB leaf mappings.
pub fn translate(virt: u64) -> Option<u64> {
    // SAFETY: every address dereferenced here is inside the recursive
    // window, which the self-map keeps valid for as long as the tables
    // are installed.
    unsafe {
        let pml4e = pml4_entry(virt).read_volatile();
        if pml4e & PRESENT == 0 {
            return None;
        }
        let pdpte = pdpt_entry(virt).read_volatile();
        if pdpte & PRESENT == 0 {
            return None;
        }
        let pde = pd_entry(virt).read_volatile();
        if pde & PRESENT == 0 {
            return None;
        }
        if pde & HUGE != 0 {
            return Some((pde & ADDR_MASK_2M) | (virt & (2 * 1024 * 1024 - 1)));
        }
        let pte = pt_entry(virt).read_volatile();
        if pte & PRESENT == 0 {
            return None;
        }
        Some((pte & ADDR_MASK_4K) | (virt & (PAGE_SIZE - 1)))
    }
}

/// Map one 4 KiB page. Intermediate tables are allocated from the DMA
/// zone on demand. Returns `false` when a table frame ran out or an
/// argument is misaligned.
pub fn map_page(virt: u64, phys: u64, flags: Flags) -> bool {
    if !virt.is_multiple_of(PAGE_SIZE) || !phys.is_multiple_of(PAGE_SIZE) {
        return false;
    }
    // SAFETY: recursive-window accesses as in `translate`; table memory
    // is fresh DMA-zone RAM reachable through the identity alias.
    unsafe {
        if ensure_table(pml4_entry(virt)) {
            return false;
        }
        if ensure_table(pdpt_entry(virt)) {
            return false;
        }
        if ensure_table(pd_entry(virt)) {
            return false;
        }
        pt_entry(virt).write_volatile(phys | flags.bits());
        invlpg(virt);
    }
    true
}

/// Remove a 4 KiB mapping. Returns `false` when the page was not mapped.
pub fn unmap_page(virt: u64) -> bool {
    // SAFETY: recursive-window access, as in `translate`.
    unsafe {
        let pte = pt_entry(virt);
        let old = pte.read_volatile();
        if old & PRESENT == 0 {
            return false;
        }
        pte.write_volatile(0);
        invlpg(virt);
    }
    true
}

/// Point a table entry at a fresh DMA-zone table, unless already present.
/// Returns true on allocation failure.
///
/// SAFETY: `entry` must be a recursive-window table pointer.
unsafe fn ensure_table(entry: *mut u64) -> bool {
    let old = entry.read_volatile();
    if old & PRESENT != 0 {
        return false;
    }
    let Some(frame) = pmm::alloc(Zone::Dma) else {
        return true;
    };
    // SAFETY: the frame is DMA-zone RAM, zeroed through the identity
    // alias that stays installed for the whole bring-up.
    core::ptr::write_bytes(frame as *mut u64, 0, (PAGE_SIZE / 8) as usize);
    entry.write_volatile(frame | PRESENT | WRITE);
    false
}

fn cr3_write(frame: u64) {
    // SAFETY: `frame` is a page-aligned table base; writing CR3 just
    // switches the active tables.
    unsafe {
        core::arch::asm!("mov cr3, {0}", in(reg) frame, options(nostack, preserves_flags));
    }
}

fn invlpg(virt: u64) {
    // SAFETY: invalidates one TLB entry; no memory or stack effect.
    unsafe {
        core::arch::asm!("invlpg [{0}]", in(reg) virt, options(nostack, preserves_flags));
    }
}

/// Build the boot page tables and switch over. Idempotence is not
/// attempted: called exactly once, from `kmain`.
pub fn init() {
    // The PML4 goes into the first DMA frame so the self-map base below
    // is stable; the remaining tables follow wherever the PMM puts them.
    let Some(pml4) = pmm::alloc_contiguous(Zone::Dma, 1) else {
        kerror!("mm", "no frame for the boot PML4");
        crate::arch::halt_forever();
    };

    // SAFETY: DMA-zone frames, reachable and zeroable through the
    // identity alias while the boot tables are still installed.
    unsafe {
        core::ptr::write_bytes(pml4 as *mut u64, 0, (PAGE_SIZE / 8) as usize);
    }

    // SAFETY: addresses inside the freshly zeroed PML4 frame.
    unsafe {
        let pml4p = pml4 as *mut u64;

        // PML4[0]: the boot stub's 1 GiB identity map, still carrying the
        // running kernel, the VGA buffer and every low-memory structure.
        let boot_pdpt = core::ptr::addr_of!(_pdpt) as u64;
        pml4p.write_volatile(boot_pdpt | PRESENT | WRITE);

        // PML4[511] -> PDPT of the higher-half kernel window.
        let pdpt = pmm::alloc(Zone::Dma).unwrap_or_else(|| {
            kerror!("mm", "no frame for the kernel-window PDPT");
            crate::arch::halt_forever();
        });
        core::ptr::write_bytes(pdpt as *mut u64, 0, (PAGE_SIZE / 8) as usize);
        pml4p
            .add(PML4_KERNEL)
            .write_volatile(pdpt | PRESENT | WRITE);

        // PDPT[510]: 1 GiB of RAM aliased at KERNEL_BASE, 2 MiB pages.
        let pd = pmm::alloc(Zone::Dma).unwrap_or_else(|| {
            kerror!("mm", "no frame for the higher-half PD");
            crate::arch::halt_forever();
        });
        core::ptr::write_bytes(pd as *mut u64, 0, (PAGE_SIZE / 8) as usize);
        (pdpt as *mut u64)
            .add(PDPT_PHYSMAP)
            .write_volatile(pd | PRESENT | WRITE);
        let pdp = pd as *mut u64;
        for i in 0..512usize {
            let frame = (i as u64) * 2 * 1024 * 1024;
            pdp.add(i).write_volatile(frame | PRESENT | WRITE | HUGE);
        }

        // PML4[510]: the recursive self-map.
        pml4p
            .add(PML4_RECURSIVE)
            .write_volatile(pml4 | PRESENT | WRITE);
    }

    cr3_write(pml4);

    // The kernel image must read identically through both windows.
    for (i, word) in identity_probe().iter().enumerate() {
        // SAFETY: KERNEL_BASE + 1 MiB is the higher-half alias of the
        // kernel text, mapped by the physmap PD built above.
        let addr = KERNEL_BASE + 1024 * 1024 + i as u64 * 8;
        let alias = unsafe { core::ptr::read_volatile(addr as *const u64) };
        if alias != *word {
            kerror!("mm", "higher-half alias mismatch at offset {:#x}", i * 8);
            crate::arch::halt_forever();
        }
    }

    kinfo!(
        "mm",
        "paging: kernel mapped in the higher half at {:#x}, tables switched",
        KERNEL_BASE
    );
}

/// The first kernel-image words as seen through the identity map; the
/// higher-half alias must match them word for word.
fn identity_probe() -> &'static [u64; 16] {
    // SAFETY: the kernel is linked at physical 1 MiB, in the identity
    // window; reading 128 bytes of .text cannot fault.
    unsafe { &*((1024 * 1024) as *const [u64; 16]) }
}
