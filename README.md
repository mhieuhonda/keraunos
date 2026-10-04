<p align="center">
  <img alt="Keraunos" src="docs/assets/wordmark.svg" width="420">
</p>

<h3 align="center">the thunderbolt operating system</h3>

<p align="center">
  <strong>power · performance · speed — written in Rust</strong>
</p>

<p align="center">
  <a href="https://github.com/mhieuhonda/keraunos/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/mhieuhonda/keraunos/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="license: MIT" src="https://img.shields.io/badge/license-MIT-blue"></a>
  <a href="rust-toolchain.toml"><img alt="Rust 1.99" src="https://img.shields.io/badge/rust-1.99-orange"></a>
  <a href="docs/ROADMAP.md"><img alt="milestone: M2 complete" src="https://img.shields.io/badge/milestone-M2%20memory-green"></a>
</p>

---

**Keraunos** (Greek: *κεραυνός*, the thunderbolt) is a general-purpose
operating system under construction around one obsession: **going fast
without breaking things**. The kernel is Rust from the first instruction,
the architecture layer is designed to span everything from x86-64 servers
to ARM laptops to RISC-V boards — one kernel, every device class — and
the desktop is native Rust too: a compact text-mode shell, compositor
and theme ([`UI/`](UI/README.md)) that already renders on the boot
framebuffer today and grows with the kernel.

| What | Status |
| --- | --- |
| x86-64 boot (multiboot2 via GRUB2) | **boots** — banner, dual console (VGA + serial), memory map, verified end-to-end in CI on every push |
| Memory management ([`kernel/src/mm/`](kernel/src/mm)) | **online** — bitmap PMM with zone freelists, higher-half paging, slab heap, `/proc/meminfo`-style accounting, zero-leak stress self-test in CI |
| Desktop session ([`UI/`](UI/README.md)) | **renders** — splash, windowed shell, boot-log mirror, live memory facts, notifications on the text framebuffer |
| aarch64 / riscv64 | scaffolds compile (CI type-checks both); bring-up on the roadmap |
| Interrupts, scheduler, drivers, fs, net | designed, sequenced in the [roadmap](docs/ROADMAP.md) |
| Linux-ABI compat layer (run unmodified Linux userland) | planned (M6) |

## Screenshots

Real frames from the CI-built ISO booting in QEMU — the text console at
desktop handoff (meminfo table and subsystems on screen), then the
desktop session rendering the same numbers as windows:

| boot console | desktop session |
| --- | --- |
| ![boot console](docs/screenshots/boot-console.png) | ![desktop](docs/screenshots/desktop.png) |

## Why another OS?

* **Rust all the way down.** Memory safety where it historically hurt most:
  ring 0. No C in the kernel, ever — not even for the boot path.
* **Performance is a feature, not an afterthought.** Per-CPU data,
  tickless scheduling, zero-copy networking and lock-free paths are design
  constraints from day one, not optimizations bolted on later.
* **Hardware breadth as a goal.** One codebase, three architecture
  families (x86-64 / aarch64 / riscv64) behind a thin HAL — because an OS
  that only runs on one vendor's silicon is a demo, not infrastructure.
* **A desktop of its own.** The interface is native Rust sized to the
  kernel — no foreign stack to adapt to, no compatibility tax on the way
  up. What the kernel knows, the desktop shows; what the desktop draws,
  the kernel delivered.

## Quick start

```sh
# toolchain — rustup installs exactly the pinned 1.99 compiler
curl -sSf https://sh.rustup.rs | sh
git clone https://github.com/mhieuhonda/keraunos.git
cd keraunos

cargo build                     # kernel for x86-64 (bare metal)
cargo check --target aarch64-unknown-none
cargo check --target riscv64gc-unknown-none-elf

# boot it — no QEMU required
pip install -r tools/boot-unicorn/requirements.txt
python tools/boot-unicorn/boot_test.py
```

Expected output (excerpt):

```
+==========================================================+
|  K E R A U N O S                                         |
|  the thunderbolt operating system                        |
|  power - performance - speed, written in Rust            |
+==========================================================+
[INFO ] boot     boot protocol: multiboot2 via UnicornGRUB 2.12-emul
[INFO ] arch     GDT loaded (null / code64 / data)
[INFO ] mm       boot memory map: 2 regions, 1023 MiB + 0 KiB usable
[INFO ] mm       bitmap pmm armed: 261888 frames, 261821 free, 31 KiB bitmap
[INFO ] mm       paging: kernel mapped in the higher half at 0xffffffff80000000
[INFO ] mm       heap: slab caches online, 9 classes, 8 MiB window
...
[INFO ] mm       self-test: PASS -- 256 frame ops, 98 heap objects, 0 leaks
[INFO ] mm       MemTotal:     1047552 KiB
[INFO ] kernel   keraunos 0.1.0 framework ready -- BOOT OK
[INFO ] ui       session: splash on text surface
[INFO ] ui       console handoff: vga sink detached, display service owns the framebuffer
[INFO ] ui       compositor: system, memory, boot log, subsystems mapped
[INFO ] ui       notification: framework ready -- BOOT OK
```

For the full path — a real GRUB2 ISO booted in QEMU — see
[docs/BUILDING.md](docs/BUILDING.md).

## Repository layout

```
keraunos/
├── kernel/            # the Rust kernel (no_std, first-party deps only)
│   └── src/
│       ├── main.rs    # entry point, multiboot2 header, boot sequence
│       ├── arch/      # x86-64 (gdt, cpu, io) + aarch64/riscv64 scaffolds
│       ├── boot/      # multiboot2 information parser
│       ├── console/   # VGA text + 16550 UART, fan-out logging, log mirror
│       ├── display/   # display service: post-handoff framebuffer owner
│       ├── mm/        # bitmap PMM, higher-half paging, slab heap,
│       │              #   meminfo accounting, boot-time stress self-test
│       ├── sched/     # scheduler scaffold (M3)
│       ├── hal/       # hardware abstraction layer scaffold (M1)
│       ├── drivers/   # driver model scaffold (M4)
│       ├── fs/        # VFS scaffold (M4)
│       ├── net/       # network stack scaffold (M5)
│       └── ipc/       # IPC scaffold (M5)
├── UI/                # the native desktop: theme, shell, compositor
│                      #   (keraunos-ui crate, zero deps, no unsafe)
├── tools/             # mkiso.sh, qemu-boot.sh, boot-unicorn/ (QEMU-free boot test)
├── docs/              # ARCHITECTURE, ROADMAP, HARDWARE, BUILDING, screenshots/
└── .github/           # CI: build + cross-arch + fmt/clippy + boot tests
```

## Documentation

* [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — how the kernel is put
  together, and how the desktop runs on it.
* [docs/ROADMAP.md](docs/ROADMAP.md) — milestones M1…M7 with concrete
  deliverables and acceptance criteria.
* [docs/HARDWARE.md](docs/HARDWARE.md) — the hardware support matrix and
  how "runs everywhere" is actually engineered.
* [docs/BUILDING.md](docs/BUILDING.md) — build, cross-check, boot (emulated
  and real), and troubleshooting.
* [UI/README.md](UI/README.md) — the native desktop: contract, layout
  and rendering model.

## Contributing

Keraunos is a community project in the making — contributions of every
kind are welcome: kernel code, boot logs from odd hardware, docs,
packaging, outreach. Start with
[CONTRIBUTING.md](CONTRIBUTING.md), then look for issues labeled
`good first issue`. Development coordination happens in
[GitHub Discussions](https://github.com/mhieuhonda/keraunos/discussions);
everyone follows the [Code of Conduct](CODE_OF_CONDUCT.md).

Two house rules worth knowing early:

1. **CI must boot the kernel.** A PR that compiles but doesn't reach
   `BOOT OK` isn't done.
2. **The desktop follows kernel bars.** `UI/` is native workspace code:
   no_std, fmt- and clippy-clean, and covered by the same boot tests.
   What it shows must come from the kernel — never invented.

## License

The kernel, the desktop and the repository tooling are [MIT](LICENSE) —
one license, one codebase, no vendored exceptions.

---

## Tiếng Việt

**Keraunos** (tiếng Hy Lạp: tia sấm Zeus) là hệ điều hành đang được xây
dựng bằng Rust với các mục tiêu: **mạnh mẽ, hiệu năng, tốc độ** — nhanh,
mượt, mạnh mẽ. Kernel được viết bằng Rust từ dòng lệnh đầu tiên; một kiến
trúc duy nhất nhắm tới **mọi họ phần cứng và thiết bị** (x86-64, ARM64,
RISC-V) qua một lớp HAL mỏng; giao diện là desktop thuần Rust
(`UI/`) — compositor, shell và theme gọn nhẹ đã hiển thị trực tiếp trên
framebuffer của kernel.

Trạng thái hiện tại: khung kernel đã hoàn thành — boot multiboot2, console
kép (VGA + serial), phân tích memory map — cùng toàn bộ quản lý bộ nhớ M2:
PMM bitmap theo vùng (DMA/Normal/High) với freelist từng node NUMA, ánh xạ
kernel ở nửa cao địa chỉ (higher half) với các hàm trợ lý bảng trang đệ quy,
heap slab chín lớp kích thước (poison trong bản debug, làm `GlobalAlloc` cho
cả kernel), kế toán bộ nhớ kiểu `/proc/meminfo` và bài test tự kiểm tra stress
chạy mỗi lần boot với cam kết **không rò rỉ**. Phiên làm việc desktop (splash,
cửa sổ, thông báo) hiển thị trực tiếp các con số bộ nhớ thật trên màn hình và
được CI xác thực boot lại ở mỗi commit. Lộ trình chi tiết:
[docs/ROADMAP.md](docs/ROADMAP.md). Mọi đóng góp đều được chào đón — xem
[CONTRIBUTING.md](CONTRIBUTING.md) (tiếng Anh) và thảo luận tại
[GitHub Discussions](https://github.com/mhieuhonda/keraunos/discussions).
