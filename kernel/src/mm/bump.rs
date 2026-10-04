//! Early physical-frame bump allocator.
//!
//! Hands out 4 KiB frames from the first usable RAM region located entirely
//! above the kernel image. It exists purely for early boot (page-table
//! construction, boot allocations); the bitmap PMM replaces it once the
//! allocator subsystem comes online in milestone M2.

use super::{align_up_page, PAGE_SIZE};
use crate::boot::multiboot2::{MemoryMap, RegionKind};

// The linker script exports the first byte after the kernel image.
extern "C" {
    static _kernel_end: u8;
}

/// Physical address immediately after the kernel image.
fn kernel_end() -> u64 {
    // `_kernel_end` is a linker-defined constant; we only take its address
    // and never dereference it.
    core::ptr::addr_of!(_kernel_end) as u64
}

/// Bump allocator over a single usable region.
pub struct BumpAllocator {
    cursor: u64,
    end: u64,
}

impl BumpAllocator {
    /// Find the first usable region (or tail of one) that lies above the
    /// kernel image and the boot information block, and arm the allocator
    /// over it.
    pub fn from_memory_map(map: &MemoryMap, above: u64) -> Option<Self> {
        let floor = kernel_end().max(above);
        for region in map.regions() {
            if region.kind != RegionKind::Usable {
                continue;
            }
            let start = align_up_page(region.base.max(floor));
            let end = region.base.saturating_add(region.len);
            if start < end {
                return Some(BumpAllocator { cursor: start, end });
            }
        }
        None
    }

    /// Allocate `frames` contiguous physical frames. Returns the physical
    /// base address of the run, or `None` when exhausted.
    pub fn alloc_contiguous(&mut self, frames: usize) -> Option<u64> {
        let span = (frames as u64).checked_mul(PAGE_SIZE)?;
        let next = self.cursor.checked_add(span)?;
        if next > self.end {
            return None;
        }
        let frame = self.cursor;
        self.cursor = next;
        Some(frame)
    }

    #[allow(dead_code)] // surfaced to diagnostics once the PMM lands
    /// Frames still available to this allocator.
    pub fn remaining_frames(&self) -> u64 {
        self.end.saturating_sub(self.cursor) / PAGE_SIZE
    }

    /// Next physical address the bump would hand out. The PMM reads this
    /// once to learn how much early-allocated RAM to keep reserved.
    pub fn cursor(&self) -> u64 {
        self.cursor
    }
}
