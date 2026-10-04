//! Page accounting and the early `/proc/meminfo` analogue.
//!
//! Everything the kernel knows about its own memory use, gathered from
//! the PMM, the heap and the page-table layer into one snapshot and
//! printed in `/proc/meminfo` shape at boot. The desktop's memory window
//! renders the same numbers — one source of truth, no invented facts.

#![allow(dead_code)] // the snapshot feeds the desktop session as well

use super::heap;
use super::pmm::{self, Zone, ZoneStats};
use super::PAGE_SIZE;

/// Frame accounting as seen at snapshot time, all zones summed.
#[derive(Debug, Clone, Copy)]
pub struct Snapshot {
    /// Managed RAM, KiB.
    pub total_kib: u64,
    /// Free frames, KiB.
    pub free_kib: u64,
    /// Kernel image, boot data and allocator metadata, KiB.
    pub reserved_kib: u64,
    /// Per-zone frame accounting, in zone order (DMA, Normal, High).
    pub zones: [ZoneStats; 3],
    /// Slab payload bytes currently allocated, KiB.
    pub heap_used_kib: u64,
    /// Heap arena frames mapped so far, KiB.
    pub heap_arena_kib: u64,
    /// Heap objects handed out and not freed.
    pub heap_objects: u32,
    /// Slabs mapped for the size classes.
    pub heap_slabs: u32,
    /// Frames spent on page tables themselves.
    pub table_frames: u64,
}

impl Snapshot {
    /// Numbers from every accounting source the M2 subsystem exposes.
    pub fn take() -> Snapshot {
        let total_kib = pmm::total_frames_kib();
        let free_kib = pmm::free_frames() * PAGE_SIZE / 1024;
        let zones = [
            pmm::zone_stats(Zone::Dma),
            pmm::zone_stats(Zone::Normal),
            pmm::zone_stats(Zone::High),
        ];
        let classes = heap::class_stats();
        Snapshot {
            total_kib,
            free_kib,
            reserved_kib: total_kib.saturating_sub(free_kib),
            zones,
            heap_used_kib: heap::in_use_bytes() / 1024,
            heap_arena_kib: heap::arena_frames() * PAGE_SIZE / 1024,
            heap_objects: classes.iter().map(|c| c.1).sum(),
            heap_slabs: classes.iter().map(|c| c.2).sum(),
            table_frames: super::paging::table_frames(),
        }
    }

    /// Log the snapshot in `/proc/meminfo` shape, one line per field.
    pub fn print(&self) {
        kinfo!("mm", "MemTotal:     {} KiB", self.total_kib);
        kinfo!("mm", "MemFree:      {} KiB", self.free_kib);
        kinfo!("mm", "MemReserved:  {} KiB", self.reserved_kib);
        let names = ["Dma", "Normal", "High"];
        for (name, stats) in names.iter().zip(self.zones.iter()) {
            kinfo!(
                "mm",
                "Zone{}:  {} of {} KiB free",
                name,
                stats.free * PAGE_SIZE / 1024,
                stats.total * PAGE_SIZE / 1024
            );
        }
        kinfo!(
            "mm",
            "Slab:         {} KiB in {} objects / {} slabs",
            self.heap_used_kib,
            self.heap_objects,
            self.heap_slabs
        );
        kinfo!(
            "mm",
            "HeapArena:    {} of {} KiB mapped",
            self.heap_arena_kib,
            heap::capacity_frames() * PAGE_SIZE / 1024
        );
        kinfo!("mm", "PageTables:   {} frames", self.table_frames);
    }
}
