<p align="center">
  <img alt="Keraunos" src="docs/assets/wordmark.svg" width="420">
</p>

<h3 align="center">the thunderbolt operating system</h3>

<p align="center">
  <strong>power · performance · speed — written in Rust</strong>
</p>

<p align="center">
  <a href="https://github.com/mhieuhonda/keraunos/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/mhieuhonda/keraunos/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="license: MIT (kernel) + upstream licenses (UI/)" src="https://img.shields.io/badge/license-MIT%20(kernel)%20·%20upstream%20(UI)-blue"></a>
  <a href="rust-toolchain.toml"><img alt="Rust 1.99" src="https://img.shields.io/badge/rust-1.99-orange"></a>
  <a href="docs/ROADMAP.md"><img alt="milestone: M0 complete" src="https://img.shields.io/badge/milestone-M0%20framework-green"></a>
</p>

---

**Keraunos** (Greek: *κεραυνός*, the thunderbolt) is a general-purpose
operating system under construction around one obsession: **going fast
without breaking things**. The kernel is Rust from the first instruction,
the architecture layer is designed to span everything from x86-64 servers
to ARM laptops to RISC-V boards, and the desktop target is a familiar,
production-grade stack: **GNOME Shell on mutter, wearing the Yaru theme**
— staged unmodified in [`UI/`](UI/README.md) and destined to run on
Keraunos' Linux-ABI compatibility layer.

| What | Status |
| --- | --- |
| x86-64 boot (multiboot2 via GRUB2) | **boots** — banner, dual console (VGA + serial), memory map, early frame allocator, verified end-to-end in CI on every push |
| aarch64 / riscv64 | scaffolds compile (CI type-checks both); bring-up on the roadmap |
| Interrupts, paging, scheduler, drivers, fs, net | designed, sequenced in the [roadmap](docs/ROADMAP.md) |
| Linux-ABI compat layer (run unmodified GNOME) | planned (M6) |
| GNOME mutter + gnome-shell + Yaru | **imported & pinned** in [`UI/`](UI/README.md) |

## Why another OS?

* **Rust all the way down.** Memory safety where it historically hurt most:
  ring 0. No C in the kernel, ever — not even for the boot path.
* **Performance is a feature, not an afterthought.** Per-CPU data,
  tickless scheduling, zero-copy networking and lock-free paths are design
  constraints from day one, not optimizations bolted on later.
* **Hardware breadth as a goal.** One codebase, three architecture
  families (x86-64 / aarch64 / riscv64) behind a thin HAL — because an OS
  that only runs on one vendor's silicon is a demo, not infrastructure.
* **A desktop people already know.** Rather than inventing a new UI, run
  the stack hundreds of millions of users touch daily — GNOME —
  unmodified. Compatibility is the honest kind: no forks, no drift.

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
[INFO ] mm       early bump allocator armed above the kernel image
...
[INFO ] kernel   keraunos 0.1.0 framework ready -- BOOT OK
```

For the full path — a real GRUB2 ISO booted in QEMU — see
[docs/BUILDING.md](docs/BUILDING.md).

## Repository layout

```
keraunos/
├── kernel/            # the Rust kernel (no_std, dependency-free today)
│   └── src/
│       ├── main.rs    # entry point, multiboot2 header, boot sequence
│       ├── arch/      # x86-64 (gdt, cpu, io) + aarch64/riscv64 scaffolds
│       ├── boot/      # multiboot2 information parser
│       ├── console/   # VGA text + 16550 UART, fan-out logging
│       ├── mm/        # boot memory map + early bump allocator
│       ├── sched/     # scheduler scaffold (M3)
│       ├── hal/       # hardware abstraction layer scaffold (M1)
│       ├── drivers/   # driver model scaffold (M4)
│       ├── fs/        # VFS scaffold (M4)
│       ├── net/       # network stack scaffold (M5)
│       └── ipc/       # IPC scaffold (M5)
├── UI/                # staged default interface: GNOME mutter + gnome-shell + Yaru
│                      #   (verbatim upstream codebases, pinned commits, read-only)
├── tools/             # mkiso.sh, qemu-boot.sh, boot-unicorn/ (QEMU-free boot test)
├── docs/              # ARCHITECTURE, ROADMAP, HARDWARE, BUILDING
└── .github/           # CI: build + cross-arch + fmt/clippy + boot tests
```

## Documentation

* [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — how the kernel is put
  together, and how GNOME is meant to run on it.
* [docs/ROADMAP.md](docs/ROADMAP.md) — milestones M1…M7 with concrete
  deliverables and acceptance criteria.
* [docs/HARDWARE.md](docs/HARDWARE.md) — the hardware support matrix and
  how "runs everywhere" is actually engineered.
* [docs/BUILDING.md](docs/BUILDING.md) — build, cross-check, boot (emulated
  and real), and troubleshooting.
* [UI/README.md](UI/README.md) — provenance, licenses and the read-only
  policy for the staged interface stack.

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
2. **`UI/` is read-only.** It's a vendored, pinned snapshot of upstream
   GNOME/Yaru; changes belong upstream, and upgrades land here as
   deliberate re-imports.

## License

The kernel and repository tooling are [MIT](LICENSE). The `UI/`
directory contains verbatim third-party codebases that keep their own
upstream licenses (GPL-2.0-or-later for mutter and gnome-shell; GPL-3.0 /
CC-BY-SA for Yaru). See [UI/README.md](UI/README.md) for details.

---

## Tiếng Việt

**Keraunos** (tiếng Hy Lạp: tia sấm Zeus) là hệ điều hành đang được xây
dựng bằng Rust với một mục tiêu duy nhất: **mạnh mẽ, hiệu năng, tốc độ**.
Kernel được viết bằng Rust từ dòng lệnh đầu tiên; kiến trúc nhắm tới mọi
họ phần cứng (x86-64, ARM64, RISC-V) qua một lớp HAL mỏng; giao diện
mặc định là bộ GNOME (mutter + gnome-shell) cùng theme Yaru — được nhập
 nguyên vẹn trong [`UI/`](UI/README.md) và sẽ chạy không cần chỉnh sửa
qua lớp tương thích Linux-ABI.

Trạng thái hiện tại: khung kernel đã hoàn thành — boot multiboot2, console
kép (VGA + serial), phân tích memory map, bộ cấp phát frame sớm — và được
CI xác thực boot lại ở mỗi commit. Lộ trình chi tiết: 
[docs/ROADMAP.md](docs/ROADMAP.md). Mọi đóng góp đều được chào đón — xem
[CONTRIBUTING.md](CONTRIBUTING.md) (tiếng Anh) và thảo luận tại
[GitHub Discussions](https://github.com/mhieuhonda/keraunos/discussions).
