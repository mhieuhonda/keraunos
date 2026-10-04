//! Boot-time memory self-test: the M2 acceptance gate.
//!
//! Stress loop over every allocator the kernel now owns — bitmap PMM
//! zones, slab heap, page-table helpers — with deterministic (xorshift)
//! patterns so failures reproduce. The run passes only when every free
//! counter returns exactly to its baseline: zero leaks, or the boot
//! fails loudly and CI catches it.

use super::paging;
use super::pmm::{self, Zone};
use super::PAGE_SIZE;
use alloc::vec::Vec;

/// Deterministic xorshift64* — reproducible stress patterns, no
/// dependency needed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Top of the 1 GiB bring-up identity window: only frames below it can
/// be written through the identity alias during the stress loop.
const IDENTITY_LIMIT: u64 = 1024 * 1024 * 1024;

/// Run every stress phase; panics on leak or corruption, so a failing
/// kernel never reports `BOOT OK`.
pub fn run() {
    let mut rng = Rng(0x4B45_5241_4E4F_5307);
    let frame_ops = stress_frames(&mut rng);
    let objects = stress_heap(&mut rng);
    stress_paging();
    kinfo!(
        "mm",
        "self-test: PASS -- {} frame ops, {} heap objects, 0 leaks",
        frame_ops,
        objects
    );
}

/// Allocate and free frame runs across the zones in a churn pattern,
/// writing a unique pattern into every frame the identity window
/// reaches. The free count must land exactly where it started.
fn stress_frames(rng: &mut Rng) -> u64 {
    let free_before = pmm::free_frames();
    let mut live: Vec<(u64, u64)> = Vec::new();
    let mut ops = 0u64;
    let zones = [Zone::Dma, Zone::Normal, Zone::High];

    for _round in 0..64u64 {
        for _ in 0..4u64 {
            // Rotate through the zones with a random stride.
            let zone = zones[rng.below(zones.len() as u64) as usize];
            let run = 1 + rng.below(8);
            ops += 1;
            let Some(base) = pmm::alloc_contiguous(zone, run) else {
                continue; // zone exhausted: fine, the loop moves on
            };
            if base + run * PAGE_SIZE <= IDENTITY_LIMIT {
                for f in 0..run {
                    let pattern = 0x4B10_C000 | base | f;
                    // SAFETY: the frame was just allocated, lies inside
                    // the identity window and nothing else maps it yet.
                    unsafe {
                        let word = (base + f * PAGE_SIZE) as *mut u64;
                        word.write_volatile(pattern);
                        if word.read_volatile() != pattern {
                            panic!("self-test: frame {:#x} lost content", base + f * PAGE_SIZE);
                        }
                    }
                }
            }
            live.push((base, run));
        }
        // Free roughly half of what is live, churn style.
        let mut i = 0;
        while i < live.len() {
            if rng.below(2) == 0 {
                let (base, run) = live.swap_remove(i);
                pmm::free_contiguous(base, run);
            } else {
                i += 1;
            }
        }
    }
    for (base, run) in live {
        pmm::free_contiguous(base, run);
    }
    if pmm::free_frames() != free_before {
        panic!(
            "self-test: pmm leaked {} frames",
            free_before - pmm::free_frames()
        );
    }
    ops
}

/// Grow, shuffle and drop heap objects across every size class; the
/// in-use byte count must return to the baseline.
fn stress_heap(rng: &mut Rng) -> u32 {
    let before = super::heap::in_use_bytes();
    let mut live: Vec<(Vec<u8>, u8)> = Vec::new();
    let mut objects = 0u32;

    for round in 0..96u32 {
        // Sizes straddle the class boundaries on purpose.
        let size = (1usize << rng.below(12)) + rng.below(17) as usize;
        let fill = rng.next() as u8;
        let mut buf = alloc::vec![fill; size.max(1)];
        buf[size.max(1) - 1] = fill ^ 0xFF;
        live.push((buf, fill ^ 0xFF));
        objects += 1;
        if round % 8 == 7 {
            // Drain a few, verifying the tail marker survived.
            for _ in 0..4 {
                if let Some((buf, tail)) = live.pop() {
                    let last = buf[buf.len() - 1];
                    if last != tail {
                        panic!(
                            "self-test: heap object corrupted (tail {:#x} != {:#x})",
                            last, tail
                        );
                    }
                }
            }
        }
    }

    // Large path: 16 KiB spans, recycled through the exact-fit list.
    for _ in 0..2 {
        let big = alloc::vec![0x5Au8; 16 * 1024];
        if big.len() != 16 * 1024 || big[42] != 0x5Au8 {
            panic!("self-test: large allocation lost content");
        }
        objects += 1;
    }

    for (buf, tail) in live {
        let last = buf[buf.len() - 1];
        if last != tail {
            panic!(
                "self-test: heap object corrupted (tail {:#x} != {:#x})",
                last, tail
            );
        }
    }
    let after = super::heap::in_use_bytes();
    if after != before {
        panic!(
            "self-test: heap leaked {} bytes",
            after.saturating_sub(before)
        );
    }
    objects
}

/// Map, touch, translate and unmap a scratch page at the top of the
/// on-demand kernel window.
fn stress_paging() {
    let Some(frame) = pmm::alloc(Zone::Normal) else {
        panic!("self-test: no frame for the paging stress");
    };
    let scratch = paging::SCRATCH_BASE;

    if !paging::map_page(scratch, frame, paging::Flags::RW) {
        panic!("self-test: map_page failed for scratch {:#x}", scratch);
    }
    // SAFETY: the page was just mapped read-write and nothing else
    // touches this window slot.
    unsafe {
        let ptr = scratch as *mut u64;
        ptr.write_volatile(0x5041_4749_4E47_4F4B); // "PAGINGOK"
        if ptr.read_volatile() != 0x5041_4749_4E47_4F4B {
            panic!("self-test: scratch page lost content");
        }
    }
    if paging::translate(scratch) != Some(frame) {
        panic!("self-test: translate({:#x}) lost the frame", scratch);
    }
    if !paging::unmap_page(scratch) || paging::translate(scratch).is_some() {
        panic!("self-test: unmap_page left the scratch mapping behind");
    }
    pmm::free(frame);
}
