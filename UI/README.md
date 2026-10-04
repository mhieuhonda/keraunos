# UI — the Keraunos desktop

The native interface stack: theme, shell and compositor over the kernel's
text surfaces, written in Rust like everything else in the system. The
whole desktop is one deliberate file — small enough to read in a sitting,
with nothing hidden behind layers of scaffolding.

## What it is

* **compositor** — owns every pixel of the frame: back-buffer canvas,
  window chrome, wallpaper, top bar, notification toasts, one blit per
  frame.
* **shell** — the user-facing desktop model: system and memory views, the
  live boot log, the subsystem status board, session notifications.
* **theme** — the Keraunos look: storm-black palette, thunder accents,
  box-glyph chrome, per-window badges, the drizzle wallpaper.

## Kernel contract

The crate is `no_std`, allocator-free and `unsafe`-free. It never touches
hardware: the kernel's display service implements the `Surface` trait and
hands over a `SessionInfo` containing every fact the desktop shows —
version, architecture, CPU vendor, memory map, subsystem table and the
captured boot log. If the kernel did not report it, the desktop cannot
display it.

```
kernel kmain ──handoff──▶ display service ──Surface──▶ boot_session()
   serial log ◀────────────── narration callback ◀──────────┘
```

Boot order: splash, desktop base, mapped windows, notification. Each phase
renders exactly one deterministic frame, so serial logs, CI screen dumps
and real hardware always agree.

## Layout

```
UI/
├── Cargo.toml   # keraunos-ui, workspace member, zero dependencies
├── README.md    # this file
└── src/lib.rs   # palette, canvas, theme, apps, compositor, session
```

## Running it

The desktop ships inside the kernel image, so building and booting the
kernel is building and booting the UI:

```sh
cargo build                        # kernel + desktop for x86-64
python tools/boot-unicorn/boot_test.py   # asserts BOOT OK + desktop frames
```

Both CI boot tests (Unicorn and GRUB2/QEMU) decode the VGA screen and
verify the desktop rendered. Milestone M1 (interrupts) wires input; M4
(virtio-gpu) moves the same shell onto real graphics.
