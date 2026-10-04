//! Memory management subsystem.
//!
//! Bring-up order (see `docs/ROADMAP.md`): boot memory map -> bump
//! allocator -> bitmap PMM (M2) -> page tables with the kernel mapped in
//! the higher half (M2) -> slab heap (M2) -> user address spaces (M3+).

pub mod bump;
#[cfg(target_arch = "x86_64")]
pub mod paging;
pub mod pmm;

/// Canonical page/frame size on every supported architecture.
pub const PAGE_SIZE: u64 = 4096;

/// Round `addr` up to the next page boundary.
pub fn align_up_page(addr: u64) -> u64 {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}
