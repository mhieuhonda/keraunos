//! Bitmap physical-frame manager.
//!
//! Replaces the early bump allocator, which stays alive only long enough
//! to hand out the bitmap itself. Frames live in one bitmap spanning the
//! usable RAM of the boot memory map; allocation runs through per-zone
//! free lists so the hot path never scans.
//!
//! Zones follow the classic split: DMA (first 16 MiB, for legacy devices),
//! Normal (below 4 GiB) and High (everything above). Free lists are
//! tracked per NUMA node — multiboot2 carries no SRAT, so today every
//! frame belongs to node 0, but the tables are already node-indexed and
//! ACPI bring-up only has to fill them in.
//!
//! Single-core bring-up caveat: there is no lock yet. Interrupts (M1) and
//! SMP (M3) must add one before anything here runs concurrently.

use super::{align_up_page, PAGE_SIZE};
use crate::boot::multiboot2::{MemoryMap, RegionKind};

// The linker script exports the first byte after the kernel image.
extern "C" {
    static _kernel_end: u8;
}

/// Physical address immediately after the kernel image (page aligned by
/// the linker script).
pub(crate) fn kernel_end() -> u64 {
    // `_kernel_end` is a linker-defined constant; we only take its address
    // and never dereference it.
    core::ptr::addr_of!(_kernel_end) as u64
}

/// Floor for frame management: everything below 1 MiB belongs to the
/// firmware (IVT, BDA, EBDA, bootloader scratch) and is never handed out.
const FLOOR: u64 = 1024 * 1024;

/// Ceiling of the DMA zone.
const DMA_LIMIT: u64 = 16 * 1024 * 1024;

/// Ceiling of the Normal zone.
const NORMAL_LIMIT: u64 = 4 * 1024 * 1024 * 1024;

/// NUMA nodes tracked today. The SRAT (ACPI) can raise this later; the
/// free-list tables below are indexed by node already.
pub const NUMA_NODES: usize = 1;

/// Number of managed zones, per node.
pub const ZONE_COUNT: usize = 3;

/// Depth of each per-node, per-zone free list. When a list runs dry the
/// allocator falls back to a bitmap scan; when a free finds it full the
/// frame simply stays in the bitmap until the next scan finds it.
const FREELIST_CAP: usize = 256;

/// Managed zones, in address order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Below 16 MiB: legacy ISA DMA territory.
    Dma,
    /// Below 4 GiB: the general pool, and the only zone reachable through
    /// the 1 GiB bring-up identity window.
    Normal,
    /// 4 GiB and above.
    High,
}

impl Zone {
    /// All zones, in address order.
    pub const ALL: [Zone; ZONE_COUNT] = [Zone::Dma, Zone::Normal, Zone::High];

    fn index(self) -> usize {
        match self {
            Zone::Dma => 0,
            Zone::Normal => 1,
            Zone::High => 2,
        }
    }

    /// The zone a physical frame address falls into.
    fn of(frame: u64) -> Zone {
        if frame < DMA_LIMIT {
            Zone::Dma
        } else if frame < NORMAL_LIMIT {
            Zone::Normal
        } else {
            Zone::High
        }
    }

    /// Physical extent managed for this zone, clipped to the span.
    fn extent(self, span: (u64, u64)) -> (u64, u64) {
        let floor = match self {
            Zone::Dma => 0,
            Zone::Normal => DMA_LIMIT,
            Zone::High => NORMAL_LIMIT,
        };
        let ceil = match self {
            Zone::High => u64::MAX,
            zone => zone_limit(zone),
        };
        (floor.max(span.0), ceil.min(span.1))
    }
}

fn zone_limit(zone: Zone) -> u64 {
    match zone {
        Zone::Dma => DMA_LIMIT,
        Zone::Normal => NORMAL_LIMIT,
        Zone::High => u64::MAX,
    }
}

/// Per-zone frame accounting.
#[derive(Debug, Clone, Copy)]
pub struct ZoneStats {
    /// Frames the zone covers.
    pub total: u64,
    /// Frames currently free.
    pub free: u64,
}

/// LIFO free list fronting one zone of one node. Entries are physical
/// frame addresses; the capacity is fixed so managing the allocator never
/// needs an allocation of its own.
struct FreeList {
    entries: [u64; FREELIST_CAP],
    len: usize,
}

impl FreeList {
    const fn new() -> Self {
        FreeList {
            entries: [0; FREELIST_CAP],
            len: 0,
        }
    }

    fn push(&mut self, frame: u64) {
        if self.len < FREELIST_CAP {
            self.entries[self.len] = frame;
            self.len += 1;
        }
    }

    fn pop(&mut self) -> Option<u64> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.entries[self.len])
    }
}

impl Default for FreeList {
    fn default() -> Self {
        Self::new()
    }
}

struct ZoneState {
    stats: ZoneStats,
    /// Bit indexes owned by the zone.
    first_bit: u64,
    last_bit: u64,
    /// Where the next first-fit scan starts.
    cursor: u64,
}

/// The bitmap physical-frame manager, armed once and never disarmed.
struct Pmm {
    span_base: u64,
    /// One bit per 4 KiB frame across the usable span.
    bitmap: &'static mut [u64],
    bits: u64,
    free_frames: u64,
    total_frames: u64,
    zones: [ZoneState; ZONE_COUNT],
    freelists: [[FreeList; ZONE_COUNT]; NUMA_NODES],
}

/// Slot holding the armed PMM, following the console mirror pattern.
struct Slot {
    cell: core::cell::UnsafeCell<Option<Pmm>>,
}

// SAFETY: the PMM is armed once by the single boot thread and then used
// exclusively by that thread. Interrupts (M1) and SMP (M3) must add
// locking before any concurrent access exists.
unsafe impl Sync for Slot {}

static PMM: Slot = Slot {
    cell: core::cell::UnsafeCell::new(None),
};

/// Borrow the armed PMM. Panics if called before [`init`].
// SAFETY: exclusive access is guaranteed by the single boot thread, per
// the `Sync` argument on `Slot`.
fn pmm() -> &'static mut Pmm {
    let slot = unsafe { &mut *PMM.cell.get() };
    slot.as_mut().expect("pmm accessed before init")
}

impl Pmm {
    /// Bit index of a page-aligned physical address inside the span.
    fn bit_of(&self, frame: u64) -> Option<u64> {
        if frame < self.span_base || !(frame - self.span_base).is_multiple_of(PAGE_SIZE) {
            return None;
        }
        Some((frame - self.span_base) / PAGE_SIZE)
    }

    fn frame_of(&self, bit: u64) -> u64 {
        self.span_base + bit * PAGE_SIZE
    }

    /// Frame state. Bits past the managed span read as used.
    fn test(&self, bit: u64) -> bool {
        let (word, mask) = ((bit / 64) as usize, 1u64 << (bit % 64));
        match self.bitmap.get(word) {
            Some(w) => w & mask != 0,
            None => true,
        }
    }

    fn set(&mut self, bit: u64, used: bool) {
        let (word, mask) = ((bit / 64) as usize, 1u64 << (bit % 64));
        if let Some(w) = self.bitmap.get_mut(word) {
            if used {
                *w |= mask;
            } else {
                *w &= !mask;
            }
        }
    }

    /// First free bit in [from, last_bit), or `None`.
    fn scan_from(&self, zone: &ZoneState, from: u64) -> Option<u64> {
        (from..zone.last_bit).find(|&bit| !self.test(bit))
    }

    /// First free bit in the zone scanning from the cursor, wrapping once
    /// to catch frames below it.
    fn scan(&self, zone: &ZoneState) -> Option<u64> {
        match self.scan_from(zone, zone.cursor.max(zone.first_bit)) {
            Some(bit) => Some(bit),
            None => self.scan_from(zone, zone.first_bit),
        }
    }

    /// First run of `n` consecutive free bits in the zone.
    fn scan_run(&self, zone: &ZoneState, n: u64) -> Option<u64> {
        let mut run = 0;
        for bit in zone.first_bit..zone.last_bit {
            if self.test(bit) {
                run = 0;
            } else {
                run += 1;
                if run == n {
                    return Some(bit + 1 - n);
                }
            }
        }
        None
    }

    fn claim(&mut self, bit: u64, zone: usize) {
        self.set(bit, true);
        self.zones[zone].stats.free -= 1;
        self.free_frames -= 1;
    }

    fn release(&mut self, bit: u64, frame: u64) {
        let zi = Zone::of(frame).index();
        self.set(bit, false);
        self.zones[zi].stats.free += 1;
        self.free_frames += 1;
        self.freelists[0][zi].push(frame);
    }
}

/// Arm the physical-frame manager over the boot memory map.
///
/// `bump` supplies the bitmap storage; whatever the bump has not handed
/// out by then returns to the pool, so nothing between the kernel image
/// and the end of RAM is lost when the bump retires.
pub fn init(map: &MemoryMap, bump: &mut super::bump::BumpAllocator) {
    // Span: every usable region above the 1 MiB floor.
    let mut span_base = u64::MAX;
    let mut span_end = 0u64;
    for region in map.regions() {
        if region.kind != RegionKind::Usable {
            continue;
        }
        let start = region.base.max(FLOOR);
        let end = region.base.saturating_add(region.len);
        if start < end {
            span_base = span_base.min(start);
            span_end = span_end.max(end);
        }
    }
    if span_base >= span_end {
        kerror!("mm", "no usable RAM above {} to manage", FLOOR);
        crate::arch::halt_forever();
    }
    let span_base = align_up_page(span_base);
    let span_end = align_up_page(span_end);

    // Bitmap from the bump, after which the bump retires.
    let bits = (span_end - span_base) / PAGE_SIZE;
    let words = bits.div_ceil(64) as usize;
    let bitmap_frames = words.div_ceil((PAGE_SIZE / 8) as usize) as u64;
    let bitmap_addr = match bump.alloc_contiguous(usize::try_from(bitmap_frames).unwrap()) {
        Some(addr) => addr,
        None => {
            kerror!(
                "mm",
                "no bump RAM for the {} KiB frame bitmap",
                words * 8 / 1024
            );
            crate::arch::halt_forever();
        }
    };
    // SAFETY: `bitmap_addr` points at freshly bump-allocated RAM, covered
    // by the identity mapping that early boot runs under.
    let bitmap = unsafe { core::slice::from_raw_parts_mut(bitmap_addr as *mut u64, words) };
    bitmap.fill(u64::MAX);

    let mut this = Pmm {
        span_base,
        bitmap,
        bits,
        free_frames: 0,
        total_frames: 0,
        zones: [
            ZoneState {
                stats: ZoneStats { total: 0, free: 0 },
                first_bit: 0,
                last_bit: 0,
                cursor: 0,
            },
            ZoneState {
                stats: ZoneStats { total: 0, free: 0 },
                first_bit: 0,
                last_bit: 0,
                cursor: 0,
            },
            ZoneState {
                stats: ZoneStats { total: 0, free: 0 },
                first_bit: 0,
                last_bit: 0,
                cursor: 0,
            },
        ],
        freelists: [Default::default(); NUMA_NODES],
    };

    // Free every usable frame; the reservations below take back what the
    // firmware, the kernel image, the bitmap and the bump hold.
    for region in map.regions() {
        if region.kind != RegionKind::Usable {
            continue;
        }
        let start = align_up_page(region.base.max(FLOOR)).max(span_base);
        let end = region.base.saturating_add(region.len).min(span_end);
        for frame in (start..end).step_by(PAGE_SIZE as usize) {
            if let Some(bit) = this.bit_of(frame) {
                this.set(bit, false);
            }
        }
    }

    // Reserved: the kernel image and everything the bump has handed out so
    // far (the bitmap itself, early page tables), plus low legacy memory.
    let reserved_until = kernel_end()
        .max(bump.cursor())
        .max(bitmap_addr + bitmap_frames * PAGE_SIZE);
    for frame in (FLOOR..reserved_until).step_by(PAGE_SIZE as usize) {
        if let Some(bit) = this.bit_of(frame) {
            this.set(bit, true);
        }
    }

    // Zone extents and accounting, straight off the bitmap.
    for (zi, zone) in Zone::ALL.into_iter().enumerate() {
        let (lo, hi) = zone.extent((span_base, span_end));
        this.zones[zi].first_bit = if lo >= hi {
            this.bits
        } else {
            this.bit_of(lo).unwrap()
        };
        this.zones[zi].last_bit = if lo >= hi {
            this.bits
        } else {
            this.bit_of(hi).unwrap()
        };
        let mut total = 0;
        let mut free = 0;
        for bit in this.zones[zi].first_bit..this.zones[zi].last_bit {
            total += 1;
            if !this.test(bit) {
                free += 1;
            }
        }
        this.zones[zi].stats = ZoneStats { total, free };
    }
    this.free_frames = this.zones.iter().map(|z| z.stats.free).sum();
    this.total_frames = this.zones.iter().map(|z| z.stats.total).sum();

    // Seed the node-0 free lists so the first allocations never scan.
    for (zi, _) in Zone::ALL.into_iter().enumerate() {
        let mut bit = this.zones[zi].first_bit;
        while bit < this.zones[zi].last_bit && this.freelists[0][zi].len < FREELIST_CAP {
            if !this.test(bit) {
                let frame = this.frame_of(bit);
                this.freelists[0][zi].push(frame);
            }
            bit += 1;
        }
    }

    kinfo!(
        "mm",
        "bitmap pmm armed: {} frames, {} free, {} KiB bitmap",
        this.total_frames,
        this.free_frames,
        this.bitmap.len() * 8 / 1024
    );

    *unsafe { &mut *PMM.cell.get() } = Some(this);
}

/// Allocate one frame from `zone` on node 0. Returns the physical address.
pub fn alloc(zone: Zone) -> Option<u64> {
    alloc_on(0, zone)
}

/// Allocate one frame from `zone` on a specific NUMA node.
pub fn alloc_on(node: usize, zone: Zone) -> Option<u64> {
    let this = pmm();
    let zi = zone.index();
    // Free list first; entries taken by a run allocation are skipped.
    while let Some(frame) = this.freelists[node][zi].pop() {
        if let Some(bit) = this.bit_of(frame) {
            if !this.test(bit) {
                this.claim(bit, zi);
                return Some(frame);
            }
        }
    }
    // Bitmap scan fallback; park the cursor behind the hit.
    let bit = this.scan(&this.zones[zi])?;
    this.zones[zi].cursor = bit + 1;
    this.claim(bit, zi);
    Some(this.frame_of(bit))
}

/// Allocate `frames` contiguous frames from `zone` on node 0. Returns the
/// physical base address of the run.
pub fn alloc_contiguous(zone: Zone, frames: u64) -> Option<u64> {
    let this = pmm();
    let zi = zone.index();
    if frames == 0 {
        return None;
    }
    let bit = this.scan_run(&this.zones[zi], frames)?;
    this.zones[zi].cursor = bit + frames;
    for b in bit..bit + frames {
        this.claim(b, zi);
    }
    Some(this.frame_of(bit))
}

/// Return one frame to its zone on node 0. A double free panics: the
/// bitmap would hand the frame out twice, which nothing downstream
/// survives.
pub fn free(frame: u64) {
    let this = pmm();
    let bit = this
        .bit_of(frame)
        .unwrap_or_else(|| panic!("free of unaligned frame {:#x}", frame));
    if this.test(bit) {
        this.release(bit, frame);
    } else {
        panic!("double free of frame {:#x}", frame);
    }
}

/// Return `frames` frames starting at `base` to their zones.
pub fn free_contiguous(base: u64, frames: u64) {
    for i in 0..frames {
        free(base + i * PAGE_SIZE);
    }
}

/// Frame accounting for one zone.
pub fn zone_stats(zone: Zone) -> ZoneStats {
    pmm().zones[zone.index()].stats
}

/// Free frames across all zones and nodes.
pub fn free_frames() -> u64 {
    pmm().free_frames
}
