# boot-unicorn — boot Keraunos without QEMU

`boot_test.py` boots the kernel on a pure-CPU emulator ([Unicorn
engine](https://www.unicorn-engine.org/)) with no virtual machine, no
root, and no extra system packages. It is the fastest way to answer
"does the kernel still boot?" — ideal for laptops and CI runners that
cannot install QEMU.

The harness replicates what GRUB2 does at hand-off:

1. loads the kernel ELF `PT_LOAD` segments at their physical addresses;
2. builds a multiboot2 information block (bootloader name, command line,
   memory map) at a low-memory address;
3. sets `EAX` to the multiboot2 magic and `RBX` to the MBI pointer;
4. executes the entry point and runs until the kernel parks in its halt
   loop;
5. emulates the 16550 UART (capturing everything the kernel prints) and
   decodes the VGA text buffer to show what the "screen" would display.

## Usage

```sh
pip install -r tools/boot-unicorn/requirements.txt
cargo build                        # or --release
python tools/boot-unicorn/boot_test.py
# optionally point it at another image:
python tools/boot-unicorn/boot_test.py target/x86_64-unknown-none/release/keraunos-kernel
```

## What it proves

The test asserts, on the captured serial log: the banner, multiboot2
hand-off (bootloader name + command line tags), GDT bring-up, CPUID
vendor reporting, boot memory map parsing, the early bump allocator's
frame smoke test, the subsystem status table, and the final `BOOT OK`
marker.

## What it does not prove

Real hardware timing, the VGA CRTC, GRUB itself, interrupt delivery,
paging, and everything from milestone M1 onward — for those, use the
QEMU/GRUB path described in `docs/BUILDING.md`.
