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
4. **Compatibility over reinvention.** Userspace is the Linux ABI. The
   way to run GNOME unmodified (see below) is to implement that ABI
   faithfully, not to port the world to a new one.
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
   ├─ mm::bump                   first usable region above _kernel_end
   ├─ print_subsystem_table()
   └─ "keraunos <ver> framework ready -- BOOT OK"  → halt loop (cli; hlt)
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
| `kernel/src/console/` | fan-out writer: VGA text, UART; `kinfo!`/`kerror!` | online |
| `kernel/src/mm/` | page constants, boot bump allocator | online; PMM/paging M2 |
| `kernel/src/hal/` | clock/irq/SMP/cache/DMA traits | drafting M1 |
| `kernel/src/sched/` | tickless per-CPU run queues | M3 |
| `kernel/src/drivers/` | PCI/NVMe/virtio/USB/GPU model | M4 |
| `kernel/src/fs/` | VFS (Linux path+fd semantics), RAMFS, ext2 | M4 |
| `kernel/src/net/` | zero-copy stack, virtio-net first | M5 |
| `kernel/src/ipc/` | typed channels; POSIX emulation on top | M5/M6 |

## Memory model plan

`mm` grows in this order, each step replacing the previous frontier:

1. **Boot bump allocator** (today): hands 4 KiB frames from the first
   usable region above the kernel image — enough to build page tables.
2. **PMM bitmap** (M2): full physical-frame accounting, zones
   (DMA32/DMA/normal), per-NUMA-node freelists.
3. **Higher-half mapping** (M2): kernel at `-2 GiB` via `PDPT`
   self-reference, per-CPU `CR3` for ASID-tagged TLB control.
4. **Kernel heap + slabs** (M2): sized-object caches, poison-on-free in
   debug builds.
5. **User address spaces** (M3): copy-on-write forks, demand paging,
   `mmap` with Linux semantics ( prerequisite for the compat layer).

## How GNOME is meant to run here

The `/UI` directory holds mutter, gnome-shell and Yaru verbatim (see
[UI/README.md](../UI/README.md)). The plan that makes that make sense:

* **M6 — Linux-ABI compat layer.** Keraunos implements the syscall ABI
  (the practical subset that a GNOME session actually exercises: epoll,
  wayland sockets, `memfd`, `io_uring`,drm/io_uring KMS paths, …) with
  Linux-compatible semantics, including the `epoll`+`signalfd` patterns
  GLib's main loop depends on.
* **Runtime environment.** The userland base (glibc or a compat libc,
  systemd-less boot, dbus broker, Wayland stack) is targeted exactly as
  upstream builds it — no patched GNOME.
* **Graphics.** mutter needs a KMS/DRM surface: virtio-gpu first, then
  native drivers behind the same HAL device API. The kernel's
  zero-copy buffer management hands mutter scanout buffers without extra
  copies.
* **Yaru** lands as the default icon/theme/sound package — pure data on
  the initial read-only asset filesystem.

The acceptance test for M6 is blunt: boot Keraunos on QEMU with a GPU,
launch an unmodified `gnome-shell` binary built from `/UI` sources, get
the Yaru-styled desktop.

## Coding conventions

* `rustfmt` defaults, clippy-clean, no `unsafe` without a `// SAFETY:`
  comment stating the contract.
* Logging through `kinfo!`/`kerror!` with an 8-char-max subsystem tag —
  the console fans lines to every sink automatically.
* Panic handler is terminal: kernels never unwind. New code must not
  assume unwinding.
* Dependency policy: crates enter the kernel only after an audit, and
  only behind feature flags; today the kernel is dependency-free.
