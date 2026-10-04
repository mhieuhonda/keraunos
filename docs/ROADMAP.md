# Keraunos roadmap

Milestones are acceptance-test driven: a milestone is done when its
acceptance test runs green in CI — not when its tasks are all "90%".

## M0 — Kernel framework (complete)

**Goal:** a booting, verified, well-instrumented kernel skeleton.

Delivered:

* [x] Rust 1.99 pinned toolchain, `no_std` dependency-free kernel
* [x] x86-64 boot: multiboot2 header, entry stub, bss clear, boot stack
* [x] Dual console: VGA text (80x25, CRTC cursor) + 16550 UART, fan-out
      `kinfo!`/`kerror!` logging
* [x] Multiboot2 info parser: bootloader name, command line, memory map
* [x] GDT bring-up incl. CS reload via far return; CPUID vendor probe
* [x] Early physical bump allocator above the kernel image
* [x] aarch64 / riscv64 scaffolds, type-checked in CI
* [x] Boot tests: Unicorn emulation (no QEMU) + GRUB2/QEMU ISO, both
      asserting `BOOT OK` on every push
* [x] Native text-mode desktop shell in `UI/` (theme, shell, compositor)
      rendering on the boot framebuffer via the display service

## M1 — Interrupts and time

**Goal:** the kernel can react, not just report.

* [ ] IDT + TSS (x86-64): exception gates with proper stacks
* [ ] PIC remap + IO-APIC; MSI-X plumbing in the driver core
* [ ] Local APIC timer; tickless single-shot event framework
* [ ] GICv3 (aarch64) and SBI timer (riscv64) behind the HAL clock traits
* [ ] Kernel panic→dump: register + stack snapshot on fatal exceptions

**Acceptance:** a timer-driven counter thread prints on console; a
kernel unit test suite runs under `cargo test` via a custom harness.

## M2 — Memory management (complete)

**Goal:** real memory, real address spaces.

Delivered:

* [x] Bitmap PMM with zones (DMA / Normal / High) and per-node free
      lists; the bump allocator retires into it at boot
* [x] Higher-half kernel mapping at `0xFFFF_FFFF_8000_0000`; recursive
      page-table helpers (`map_page` / `unmap_page` / `translate`)
* [x] Kernel heap + nine slab size classes (poisoned in debug), wired as
      the `GlobalAlloc` — `alloc` crate types are live kernel-wide
* [x] Physical/virtual page accounting printed at boot in
      `/proc/meminfo` shape and mirrored on the desktop's memory window

**Acceptance:** the boot-time stress self-test churns frame runs across
all zones plus heap objects across every size class and verifies the
free counts return exactly to baseline — zero leaks, asserted on every
CI boot (`self-test: PASS -- 0 leaks`).

## M3 — SMP and the scheduler

**Goal:** every core earning its keep.

* [ ] SMP bring-up: SIPI (x86-64), PSCI (aarch64), HSM (riscv64)
* [ ] Per-CPU areas, per-CPU run queues, tickless preemption
* [ ] MLFQ + real-time class with deadline inheritance
* [ ] Affinity, load balancing, cgroup-style hierarchical accounting

**Acceptance:** N vCPU QEMU machine runs a CPU-bound + an interactive
workload with documented latency bounds; sched self-tests in CI.

## M4 — Drivers and filesystem

**Goal:** storage you can trust, pixels you can see.

* [ ] PCI/PCIe enumeration; driver probe/DMA/IRQ framework over the HAL
* [ ] virtio-blk + NVMe storage; ext2 r/w with journaling on top
* [ ] VFS with Linux path/fd semantics; RAMFS + initial asset mount
      (serving UI theme assets)
* [ ] virtio-gpu + basic DRM/KMS surface; USB HID input path

**Acceptance:** kernel mounts an ext2 image from CI artifacts, reads and
writes files, and paints a test pattern through virtio-gpu in QEMU.

## M5 — Networking and IPC

**Goal:** talk to the world, securely and quickly.

* [ ] virtio-net + E1000; zero-copy RX/TX rings
* [ ] TCP/IP stack (smoltcp integration behind an audit + hardening)
* [ ] Sockets with Linux semantics; unified typed-channel IPC; signals

**Acceptance:** kernel fetches a file over TCP from a CI container and
echoes it back bit-exact.

## M6 — Linux-ABI compatibility layer

**Goal:** run real software. Existing Linux userland — tools, runtimes,
whole distro userlands — boots beside the native desktop, unported.

* [ ] Syscall ABI: the practical subset real userland exercises (epoll,
      signalfd, memfd, pidfd, io_uring, eventfd, FUTEX, …)
* [ ] Process model: CLONE_VM/FILES semantics, exec, zombies
* [ ] Userland base image build (documented, reproducible)
* [ ] Session bring-up: init → services → a Wayland session compositor
      alongside the native desktop

**Acceptance:** an unmodified statically linked Linux binary runs to
completion under QEMU beside the native desktop session.

## M7 — Hardware breadth program

**Goal:** "runs everywhere" becomes a checklist, not a slogan. See
[HARDWARE.md](HARDWARE.md) for the matrix and the process for getting a
board onto it.

* [ ] UEFI boot path (EDK2-compatible) alongside BIOS/multiboot2
* [ ] Board bring-up cadence: one reference board per architecture family
      per quarter, community-owned
* [ ] Hardware-in-the-loop CI donors program

## How milestones relate

M1→M2→M3 are strictly sequential in design (M2's page tables and heap
landed first and M1's interrupt plumbing will hook into them); M4 and M5
can proceed in parallel once M3 lands. M6 needs substantial slices of M4
(fs) and M5 (sockets) but can start its ABI layer against M3. M7 is
continuous from M2 onward — every board bring-up hardens the HAL.
