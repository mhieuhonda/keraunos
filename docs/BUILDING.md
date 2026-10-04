# Building and booting Keraunos

Everything you need to go from clone to a running kernel — with or
without real virtualization available on your machine.

## Prerequisites

| Tool | Version | Needed for |
| --- | --- | --- |
| Rust | 1.99 (pinned via [rust-toolchain.toml](../rust-toolchain.toml)) | building the kernel |
| rustup | any recent | installs the pinned toolchain + bare-metal targets automatically |
| Python 3 | ≥ 3.9 | the QEMU-free boot test |
| Unicorn | 2.1+ (`pip install unicorn`) | the QEMU-free boot test |
| QEMU | `qemu-system-x86` ≥ 6 | real boot test |
| GRUB2 + xorriso + mtools | `grub-pc-bin`, `xorriso`, `mtools` packages | building the bootable ISO |

`rustup` reads `rust-toolchain.toml` and installs the exact compiler the
project is validated against; the first `cargo` invocation also installs
the `x86_64-unknown-none`, `aarch64-unknown-none` and
`riscv64gc-unknown-none-elf` targets on demand.

## Build

```sh
cargo build                  # debug kernel for x86-64 bare metal
cargo build --release        # LTO, opt-level 3, single codegen unit
cargo check --target aarch64-unknown-none        # scaffold type-check
cargo check --target riscv64gc-unknown-none-elf  # scaffold type-check
```

Artifacts land in `target/x86_64-unknown-none/{debug,release}/keraunos-kernel`
— ELF64 executables linked at 1 MiB with a multiboot2 header, loadable by
GRUB2.

## Boot test (no QEMU — any laptop, any CI runner)

```sh
pip install -r tools/boot-unicorn/requirements.txt
python tools/boot-unicorn/boot_test.py
```

This runs the kernel on the Unicorn CPU emulator with an emulated GRUB2
hand-off and 16550 UART, asserting the full boot path up to `BOOT OK`.
Details: [tools/boot-unicorn/README.md](../tools/boot-unicorn/README.md).

## Boot test (real GRUB2 + QEMU)

```sh
# Debian/Ubuntu:
sudo apt install qemu-system-x86 grub-pc-bin xorriso mtools

cargo build --release
tools/mkiso.sh                       # -> build/keraunos.iso
tools/qemu-boot.sh                   # boots it, captures serial, greps BOOT OK
```

You should see the kernel banner and log on stdout (the serial console).
Add `-display gtk` to the QEMU invocation in `tools/qemu-boot.sh` if you
want to watch the VGA console render it too.

## Boot on real hardware

Write the ISO to a USB stick and boot it (CSM/legacy or UEFI with legacy
support — the current boot path is BIOS/multiboot2; the UEFI path is
milestone M2):

```sh
dd if=build/keraunos.iso of=/dev/sdX bs=4M status=progress && sync
```

The kernel is harmless to boot anywhere: it brings up consoles, parses the
firmware memory map, runs its self-checks and parks. It touches no disks,
no NICs, nothing else.

## Troubleshooting

* **`error: can't find crate for core`** — the bare-metal target isn't
  installed for the pinned toolchain: run `rustup target add
  x86_64-unknown-none` (rustup normally does this automatically from
  `.cargo/config.toml`'s default target).
* **`unicorn` import error** — install it into the same Python you're
  invoking (`python -m pip install unicorn`).
* **grub-mkrescue fails about `mformat`** — install `mtools`.
* **QEMU shows nothing on serial** — ensure `-serial stdio`/`-serial
  file:...` is passed; the kernel writes every log line to COM1.
* **Booted on unusual hardware and something looks off** — that's gold:
  open an issue with the full serial log and the machine's specs
  (CPU, firmware, RAM map). See [HARDWARE.md](HARDWARE.md) for what
  "unusual" is worth reporting at this stage.

## Continuous integration

Every push and PR runs (see [.github/workflows/ci.yml](../.github/workflows/ci.yml)):

1. `cargo fmt --check` + `cargo clippy` on the kernel,
2. debug **and** release builds for x86-64,
3. `cargo check` for aarch64 and riscv64 scaffolds,
4. the Unicorn boot test on both artifacts,
5. a real GRUB2/QEMU boot of the release ISO, grepping serial for
   `BOOT OK`.

If your change compiles but doesn't boot, CI will tell you before any
reviewer has to.
