//! Memory management subsystem.
//!
//! Milestone plan (see `docs/ROADMAP.md`): boot memory map -> bump
//! allocator (today) -> PMM bitmap -> upper-half page tables -> kernel heap
//! -> per-CPU slab caches -> user address spaces.

pub mod bump;

/// Canonical page/frame size on every supported architecture.
pub const PAGE_SIZE: u64 = 4096;

/// Round `addr` up to the next page boundary.
pub fn align_up_page(addr: u64) -> u64 {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}
