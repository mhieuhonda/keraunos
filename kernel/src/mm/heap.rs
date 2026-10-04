//! Kernel heap: slab-cached `GlobalAlloc`.
//!
//! The heap lives in the on-demand kernel window at [`paging::HEAP_BASE`]
//! (PML4[511], PDPT[511]), backed by Normal-zone frames mapped in as the
//! arena grows. Nine size classes from 32 B to 8 KiB run as slab caches:
//! each slab is a contiguous frame run carved into equal slots, threaded
//! onto a LIFO free list stored inside the free slots themselves.
//!
//! Allocations larger than the top class, and any alignment the slab
//! slots cannot offer (8 bytes), go through the large path: a frame run
//! mapped whole, recycled through an exact-fit free list.
//!
//! Debug builds poison — freshly allocated payloads read `0xA5`, freed
//! payloads are scrubbed with `0x5A`, and every free verifies the slot
//! header, so use-after-free of a scrubbed slot and double frees are
//! caught at the next touch instead of corrupting a neighbour silently.
//!
//! Same bring-up caveat as the PMM: no lock yet, single boot thread.

#![allow(dead_code)] // stats surface to meminfo and the desktop below

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;

use super::paging;
use super::pmm::{self, Zone};
use super::PAGE_SIZE;

/// Payload sizes of the slab classes, in bytes.
const CLASS_PAYLOAD: [usize; CLASS_COUNT] = [32, 64, 128, 256, 512, 1024, 2048, 4096, 8192];
const CLASS_COUNT: usize = 9;

/// Frames per slab, per class: enough that even the 8 KiB slots pack
/// several to a slab.
const CLASS_SLAB_FRAMES: [u64; CLASS_COUNT] = [1, 1, 1, 1, 4, 4, 4, 16, 32];

/// Header marking an allocated slot.
const ALLOC_MAGIC: u32 = 0x4B455241; // "KERA"

/// Header marking a live slab.
const SLAB_MAGIC: u32 = 0x4B534C42; // "KSLB"

/// Class id stored in large-path headers.
const CLASS_LARGE: u16 = 0xFFFF;

/// Size of the slot header (magic + class), also the freelist link.
const HEADER: usize = 8;

/// Target heap window, in frames (8 MiB at 4 KiB pages).
const HEAP_FRAMES: u64 = 2048;

/// Exact-fit free list capacity for large spans.
const LARGE_FREE_CAP: usize = 16;

/// One slab size class.
struct Class {
    /// Bytes of payload per slot (header excluded).
    payload: usize,
    /// Frames per slab.
    slab_frames: u64,
    /// LIFO of free slot addresses (slot base, header included).
    free: u64,
    /// Slabs mapped for this class so far.
    slabs: u32,
    /// Slots handed out and not yet freed.
    in_use: u32,
    /// Slot capacity per slab, derived at init.
    slots_per_slab: u32,
}

impl Class {
    const fn new(payload: usize, slab_frames: u64) -> Self {
        Class {
            payload,
            slab_frames,
            free: 0,
            slabs: 0,
            in_use: 0,
            slots_per_slab: 0,
        }
    }

    fn slot_size(&self) -> usize {
        HEADER + self.payload
    }

    fn slab_bytes(&self) -> u64 {
        self.slab_frames * PAGE_SIZE
    }
}

/// A mapped-but-free large span awaiting an exact-fit reuse.
#[derive(Clone, Copy)]
struct LargeSpan {
    virt: u64,
    frames: u64,
}

struct Heap {
    /// Next unmapped frame offset inside the heap window.
    arena_top: u64,
    classes: [Class; CLASS_COUNT],
    large_free: [LargeSpan; LARGE_FREE_CAP],
    large_free_len: usize,
    /// Frames mapped for outstanding large allocations.
    large_in_use: u64,
    /// Payload bytes handed out and not yet freed, all classes.
    in_use_bytes: u64,
}

static HEAP: HeapSlot = HeapSlot {
    cell: UnsafeCell::new(None),
};

/// Slot holding the armed heap, following the console mirror pattern.
struct HeapSlot {
    cell: UnsafeCell<Option<Heap>>,
}

// SAFETY: the heap is created once by the single boot thread and then
// used exclusively by that thread. Interrupts (M1) and SMP (M3) must add
// locking before any concurrent access exists.
unsafe impl Sync for HeapSlot {}

/// Borrow the heap. Returns `None` before `init`.
// SAFETY: exclusive access is guaranteed by the single boot thread, per
// the `Sync` argument on `HeapSlot`.
fn heap() -> Option<&'static mut Heap> {
    let slot = unsafe { &mut *HEAP.cell.get() };
    slot.as_mut()
}

/// Arm the heap: nothing is mapped yet — slabs grow on demand out of the
/// Normal zone, one contiguous run at a time.
pub fn init() {
    let mut classes = [
        Class::new(32, 1),
        Class::new(64, 1),
        Class::new(128, 1),
        Class::new(256, 1),
        Class::new(512, 4),
        Class::new(1024, 4),
        Class::new(2048, 4),
        Class::new(4096, 16),
        Class::new(8192, 32),
    ];
    for class in &mut classes {
        let slots = (class.slab_bytes() as usize - HEADER) / class.slot_size();
        class.slots_per_slab = slots as u32;
    }
    let this = Heap {
        arena_top: 0,
        classes,
        large_free: [LargeSpan { virt: 0, frames: 0 }; LARGE_FREE_CAP],
        large_free_len: 0,
        large_in_use: 0,
        in_use_bytes: 0,
    };
    kinfo!(
        "mm",
        "heap: slab caches online, {} classes, {} MiB window at {:#x}",
        CLASS_COUNT,
        HEAP_FRAMES * PAGE_SIZE / 1024 / 1024,
        paging::HEAP_BASE
    );
    *unsafe { &mut *HEAP.cell.get() } = Some(this);
}

/// Map the next slab for class `ci` at the arena top and thread its slot
/// freelist. Returns false when the PMM or the window ran out.
fn grow(this: &mut Heap, ci: usize) -> bool {
    let class = &mut this.classes[ci];
    if this.arena_top + class.slab_frames > HEAP_FRAMES {
        return false;
    }
    let Some(phys) = pmm::alloc_contiguous(Zone::Normal, class.slab_frames) else {
        return false;
    };
    let virt = paging::HEAP_BASE + this.arena_top * PAGE_SIZE;
    this.arena_top += class.slab_frames;
    for i in 0..class.slab_frames {
        let page = virt + i * PAGE_SIZE;
        if !paging::map_page(page, phys + i * PAGE_SIZE, paging::Flags::RW) {
            return false;
        }
    }

    // Slab header, then the slot freelist threaded through the payload.
    // SAFETY: `virt..virt + slab_bytes` was just mapped from fresh frames.
    unsafe {
        (virt as *mut u64).write_volatile((SLAB_MAGIC as u64) << 32);
        let mut next = 0u64;
        for s in (0..class.slots_per_slab).rev() {
            let slot = virt + HEADER as u64 + s as u64 * class.slot_size() as u64;
            (slot as *mut u64).write_volatile(next);
            next = slot;
        }
        class.free = next;
        class.slabs += 1;
    }
    true
}

/// Allocate `size` bytes with `align <= 8`; larger requests take the
/// large path. Returns null when the heap is not armed or memory ran out.
///
/// SAFETY: callers must not alias slots the heap has already handed out.
unsafe fn kalloc(layout: Layout) -> *mut u8 {
    let Some(this) = heap() else {
        return ptr::null_mut();
    };

    if layout.size() > CLASS_PAYLOAD[CLASS_COUNT - 1] || layout.align() > 8 {
        return kalloc_large(this, layout);
    }

    // Smallest class that fits.
    let ci = CLASS_PAYLOAD
        .iter()
        .position(|&payload| payload >= layout.size())
        .unwrap_or(CLASS_COUNT - 1);

    if this.classes[ci].free == 0 && !grow(this, ci) {
        return ptr::null_mut();
    }
    let class = &mut this.classes[ci];
    let slot = class.free;
    // SAFETY: `slot` is a mapped slot base threaded onto the free list.
    unsafe {
        class.free = (slot as *mut u64).read_volatile();
        (slot as *mut u64).write_volatile((ALLOC_MAGIC as u64) << 32 | ci as u64);
        #[cfg(debug_assertions)]
        ptr::write_bytes((slot + HEADER as u64) as *mut u8, 0xA5, class.payload);
        class.in_use += 1;
        this.in_use_bytes += class.payload as u64;
        (slot + HEADER as u64) as *mut u8
    }
}

/// Large path: whole frame run, exact-fit recycled.
fn kalloc_large(this: &mut Heap, layout: Layout) -> *mut u8 {
    let frames = (layout.size() as u64 + HEADER as u64).div_ceil(PAGE_SIZE);

    // Exact-fit reuse keeps repeated large cycles from growing the arena.
    for i in 0..this.large_free_len {
        if this.large_free[i].frames == frames {
            let virt = this.large_free[i].virt;
            this.large_free[i] = this.large_free[this.large_free_len - 1];
            this.large_free_len -= 1;
            // SAFETY: the span stays mapped from its first allocation.
            unsafe {
                (virt as *mut u64).write_volatile((ALLOC_MAGIC as u64) << 32 | CLASS_LARGE as u64);
                #[cfg(debug_assertions)]
                ptr::write_bytes(
                    (virt + HEADER as u64) as *mut u8,
                    0xA5,
                    (frames * PAGE_SIZE - HEADER as u64) as usize,
                );
                this.large_in_use += frames;
                this.in_use_bytes += layout.size() as u64;
                return (virt + HEADER as u64) as *mut u8;
            }
        }
    }

    if this.arena_top + frames > HEAP_FRAMES {
        return ptr::null_mut();
    }
    let Some(phys) = pmm::alloc_contiguous(Zone::Normal, frames) else {
        return ptr::null_mut();
    };
    let virt = paging::HEAP_BASE + this.arena_top * PAGE_SIZE;
    this.arena_top += frames;
    for i in 0..frames {
        if !paging::map_page(
            virt + i * PAGE_SIZE,
            phys + i * PAGE_SIZE,
            paging::Flags::RW,
        ) {
            return ptr::null_mut();
        }
    }
    // SAFETY: the run was just mapped and its header slot written below.
    unsafe {
        (virt as *mut u64).write_volatile((ALLOC_MAGIC as u64) << 32 | CLASS_LARGE as u64);
        this.large_in_use += frames;
        this.in_use_bytes += layout.size() as u64;
        (virt + HEADER as u64) as *mut u8
    }
}

/// Free a pointer from `kalloc`. A corrupted or doubly freed slot panics:
/// silently continuing would hand the same slot out twice.
///
/// SAFETY: `ptr` must have come from `kalloc` and not be freed before.
unsafe fn kfree(ptr: *mut u8, size: usize) {
    let Some(this) = heap() else {
        return;
    };
    let slot = unsafe { ptr.sub(HEADER) } as u64;
    // SAFETY: `slot` carries the header written at allocation time; the
    // magic check below rejects anything else before the slot is touched.
    let header = unsafe { (slot as *mut u64).read_volatile() };
    if header >> 32 != ALLOC_MAGIC as u64 {
        panic!(
            "heap: free of unallocated or corrupt slot at {:#x} (header {:#018x})",
            slot, header
        );
    }
    let ci = (header & 0xFFFF) as u16;

    if ci == CLASS_LARGE {
        let frames = (size as u64 + HEADER as u64).div_ceil(PAGE_SIZE);
        if this.large_free_len < LARGE_FREE_CAP {
            this.large_free[this.large_free_len] = LargeSpan { virt: slot, frames };
            this.large_free_len += 1;
        }
        this.large_in_use -= frames;
        this.in_use_bytes -= size as u64;
        #[cfg(debug_assertions)]
        // SAFETY: the span is mapped for as long as it sits on the free list.
        unsafe {
            ptr::write_bytes(ptr, 0x5A, size);
        }
        return;
    }

    if ci as usize >= CLASS_COUNT {
        panic!("heap: corrupt class {} at {:#x}", ci, slot);
    }
    let class = &mut this.classes[ci as usize];
    // SAFETY: the slot is mapped and, per the magic check above, allocated.
    unsafe {
        (slot as *mut u64).write_volatile(class.free);
        #[cfg(debug_assertions)]
        ptr::write_bytes(ptr, 0x5A, class.payload);
    }
    class.free = slot;
    class.in_use -= 1;
    this.in_use_bytes -= class.payload as u64;
}

struct KHeap;

// SAFETY: `kalloc`/`kfree` hand out only mapped, header-guarded slots and
// run on the single boot thread; see the `Sync` argument on `HeapSlot`.
unsafe impl GlobalAlloc for KHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: contract of `GlobalAlloc::alloc`; the slot arithmetic in
        // `kalloc` is header-guarded against foreign pointers.
        unsafe { kalloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }
        // SAFETY: contract of `GlobalAlloc::dealloc`: `ptr` came from
        // `alloc` with the same layout.
        unsafe { kfree(ptr, layout.size()) }
    }
}

#[global_allocator]
static KHEAP: KHeap = KHeap;

/// Bytes of payload currently allocated across all classes.
pub fn in_use_bytes() -> u64 {
    heap().map(|h| h.in_use_bytes).unwrap_or(0)
}

/// Frames of the heap window currently mapped for the arena.
pub fn arena_frames() -> u64 {
    heap().map(|h| h.arena_top).unwrap_or(0)
}

/// Heap window capacity, in frames.
pub fn capacity_frames() -> u64 {
    HEAP_FRAMES
}

/// Per-class snapshot for accounting: (payload, in-use objects, slabs).
pub fn class_stats() -> [(usize, u32, u32); CLASS_COUNT] {
    let mut out = [(0usize, 0u32, 0u32); CLASS_COUNT];
    if let Some(h) = heap() {
        for (ci, class) in h.classes.iter().enumerate() {
            out[ci] = (class.payload, class.in_use, class.slabs);
        }
    }
    out
}
