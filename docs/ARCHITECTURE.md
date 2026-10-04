# Keraunos architecture

How the pieces fit — today, and by design. This document is written for
contributors who want a mental model in ten minutes, not for textbook
completeness; module-level rustdoc carries the fine print.

## Design principles

1. **Fast paths first.** Data structures are chosen for the hot path
   (per-CPU, lock-free, cache-line aware) and generalized only when
   measured. A subsystem that cannot state its performance contract does
   not merge.
2. **Rust from instruction one.** Assembly exists only where the
   architecture forces it (entry stubs, port I/O, CPU control); each such
   block is wrapped in `unsafe` with a documented contract and a Rust
   facade.
3. **One kernel, three architecture families.** All architecture
   specifics live behind `src/arch/` and the `src/hal/` traits. A driver
   or subsystem written against the HAL compiles for x86-64, aarch64 and
   riscv64 unchanged.
4. **Compatibility over reinvention.** The userspace ABI target is
   Linux: implementing that ABI faithfully lets real software run
   unported (roadmap M6), which beats porting the world to a new one.
5. **Fail loud, halt clean.** Early-boot failures print a precise,
   tag-formatted diagnostic on every console sink before parking the CPU.

## Boot sequence (x86-64, current)

```
GRUB2 (multiboot2)
   │  EAX = 0x36d76289, RBX = MBI address, long mode + identity paging
   ▼
_start (global_asm, .text.entry @ 1 MiB + 0x40)
   │  preserve magic/MBI · switch to boot stack · zero .bss
   ▼
kmain
   ├─ console::early_init()      VGA 80x25 + 16550 COM1 (115200 8N1)
   ├─ print_banner()
   ├─ multiboot2::Info parse     bootloader name · cmdline · memory map
   ├─ gdt::init()                null/code64/data + segment reload (incl. CS)
   ├─ cpu::vendor_string()       CPUID leaf 0
   ├─ mm::bump                   first usable region above _kernel_end + MBI
   ├─ mm::pmm::init              bitmap PMM, zones + per-node freelists; bump retires
   ├─ mm::paging::init           fresh PML4, higher-half kernel map, CR3 switch
   ├─ mm::heap::init             slab heap over the PMM; alloc crate goes live
   ├─ smoke tests                frame + heap round trips
   ├─ mm::selftest::run          stress loop: zero leaks or no BOOT OK
   ├─ mm::meminfo                /proc/meminfo-shaped accounting snapshot
   ├─ print_subsystem_table()
   ├─ "keraunos <ver> framework ready -- BOOT OK"
   ├─ display::take_over()       console handoff: VGA sink → display service
   └─ ui::boot_session()         splash → desktop → windows → notification
                                  → halt loop (cli; hlt)
```

Every line above is asserted by two independent boot tests (Unicorn
emulation and GRUB2/QEMU) on every push.

## Module map

| Path | Responsibility | Status |
| --- | --- | --- |
| `kernel/src/main.rs` | entry, boot orchestration, panic handler | online |
| `kernel/src/arch/x86_64/` | GDT, CPUID, port I/O, halt | online |
| `kernel/src/arch/{aarch64,riscv64}/` | halt scaffolds + bring-up notes | scaffold |
| `kernel/src/boot/multiboot2.rs` | tag parser: names, cmdline, memory map | online |
| `kernel/src/console/` | fan-out writer: VGA text, UART, log mirror; `kinfo!`/`kerror!` | online |
| `kernel/src/display/` | display service: post-handoff framebuffer owner | online |
| `UI/` (keraunos-ui) | native desktop: theme, shell, compositor over `Surface` | online |
| `kernel/src/mm/` | bitmap PMM, zones + per-node free lists, higher-half paging, slab heap, meminfo, boot self-test | online (M2) |
| `kernel/src/hal/` | clock/irq/SMP/cache/DMA traits | drafting M1 |
| `kernel/src/sched/` | tickless per-CPU run queues | M3 |
| `kernel/src/drivers/` | PCI/NVMe/virtio/USB/GPU model | M4 |
| `kernel/src/fs/` | VFS (Linux path+fd semantics), RAMFS, ext2 | M4 |
| `kernel/src/net/` | zero-copy stack, virtio-net first | M5 |
| `kernel/src/ipc/` | typed channels; POSIX emulation on top | M5/M6 |

## Memory layout (current)

```
physical                         virtual (higher half, PML4[511])
0x000000        ┌──────────      0xFFFF_FFFF_8000_0000  ┌──────────
                │ firmware                        alias │ physmap: RAM at
0x100000        │ kernel image                    ────► │ KERNEL_BASE+phys
 _kernel_end ─► │ bitmap · tables · MBI                 │ (2 MiB pages)
                │ ...                                   0xFFFF_FFFF_C000_0000
                └──────────                             │ heap arena +
 end of RAM ─►                                         │ scratch pages
                                                       └──────────
PML4[510] = PML4 (recursive self-map) — table entries of any address
reachable by OR-ing its indexes into the recursive base.
```

The identity window (PML4[0], the boot stub's 1 GiB map) keeps the
kernel running until M3 moves the program counter into the higher half;
then it is torn down and `KERNEL_BASE + phys` becomes the only way to
reach RAM.

### The allocators, in boot order

1. **Bump allocator** — a cursor over the first usable region above the
   kernel image and the MBI. Exists long enough to hand out the PMM's
   own bitmap.
2. **Bitmap PMM** — one bit per 4 KiB frame across all usable RAM, split
   into DMA (< 16 MiB), Normal (< 4 GiB) and High zones with fixed-depth
   free lists per NUMA node (node 0 until the SRAT is parsed). The bump's
   remaining span returns to the pool when it retires.
3. **Slab heap** — the PMM's biggest customer: nine size classes from
   32 B to 8 KiB, each a cache of equal slots threaded through a LIFO
   free list, backed by contiguous frame runs mapped into the on-demand
   window. Debug builds poison allocations (`0xA5`) and scrub frees
   (`0x5A`); every free re-verifies the slot header, so double frees
   panic instead of corrupting. Larger or over-aligned requests go
   through an exact-fit large-span path. Wired as the kernel's
   `GlobalAlloc`, so `Box`, `Vec` and friends work kernel-wide.
4. **meminfo** — every layer publishes accounting (`ZoneStats`, class
   stats, table-frame count); `mm::meminfo` gathers a snapshot and prints
   it `/proc`-style at boot and hands the same numbers to the desktop.

### The acceptance gate

`mm::selftest` runs at every boot before the subsystem table: frame runs
churned across all three zones with write/read verification inside the
identity window, heap objects grown, shuffled and dropped across every
size class plus the large path, and a scratch page mapped, translated
and unmapped through the recursive helpers. Each phase compares its free
counter against its baseline — any drift panics, the boot dies before
`BOOT OK`, and CI catches it. The zero-leak property is thus re-proven
on every push, not assumed.

## How the desktop runs today

The desktop is the `keraunos-ui` crate (see [UI/README.md](../UI/README.md)):
text-mode compositor, shell and theme in one dependency-free, unsafe-free
file. The contract with the kernel is narrow on purpose:

* **Surface.** `display::TextDisplay` implements the UI crate's
  `Surface` trait: `size()` reports the 80x25 geometry, `blit()` copies a
  composed frame into the VGA framebuffer. The UI never touches
  hardware.
* **Session facts.** `SessionInfo` carries everything the desktop may
  show — version, architecture, CPU vendor, memory map, the meminfo
  snapshot (`MemFacts`), the subsystem table and a snapshot of the boot
  log. Nothing on screen is invented.
* **Console handoff.** `TextDisplay::take_over()` detaches the VGA console
  sink and hides the hardware cursor; logging continues on the UART while
  the mirror ring keeps feeding the boot-log view.

Rendering is deterministic: each session phase (splash, base, mapped
windows, notification) produces exactly one frame, so the serial log, the
CI screen dump and real hardware always agree. Milestone M1 (interrupts)
wires input; M4 (virtio-gpu) moves the same shell onto real graphics —
the `Surface` trait is the seam both plug into.

## Coding conventions

* `rustfmt` defaults, clippy-clean, no `unsafe` without a `// SAFETY:`
  comment stating the contract.
* Logging through `kinfo!`/`kerror!` with an 8-char-max subsystem tag —
  the console fans lines to every sink automatically.
* Panic handler is terminal: kernels never unwind. New code must not
  assume unwinding.
* Dependency policy: third-party crates enter the kernel only after an
  audit, and only behind feature flags; the one dependency today is the
  first-party desktop crate (`keraunos-ui`, path-only).
