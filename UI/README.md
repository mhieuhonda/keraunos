# UI — the Keraunos desktop

The native interface stack: theme, shell and compositor over the kernel's
text surfaces, written in Rust like everything else in the system. The
shell mirrors the GNOME Shell layout, trimmed to what this desktop shows,
and dresses itself in the Keraunos icon and sound themes.

## What it is

* **compositor** — owns every pixel of the frame: back-buffer canvas,
  window chrome, wallpaper, top bar, notification toasts, one blit per
  frame.
* **shell** — the user-facing desktop model: system and memory views, the
  live boot log, the subsystem status board, session notifications.
* **theme** — the Keraunos look: storm-black palette, thunder accents,
  box-glyph chrome, per-window badges, the drizzle wallpaper.
* **assets** — the shipped artwork: the Keraunos icon theme (the default
  Yaru set), the Keraunos sound theme under freedesktop event names, and
  the shell theme artwork for the framebuffer sessions to come.

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

Boot order: splash (login sound), desktop base, mapped windows, ready
notification (system-ready sound). Each phase renders exactly one
deterministic frame, so serial logs, CI screen dumps and real hardware
always agree.

## Layout

```
UI/
├── Cargo.toml           # keraunos-ui, workspace member, zero dependencies
├── README.md            # this file
├── SOURCES.md           # upstream attribution (Yaru, GNOME Shell)
├── assets/
│   ├── icons/Keraunos/  # the Keraunos icon theme, all sizes and cursors
│   ├── sounds/Keraunos/ # stereo event sounds + index.theme
│   └── theme/           # shell artwork: start art, placeholders, OSD css
└── src/
    ├── lib.rs           # kernel contract, session phases, boot order
    ├── canvas.rs        # back buffer, drawing primitives, text helpers
    ├── theme.rs         # palette, styles, chrome glyphs, wallpaper
    ├── panel.rs         # the top bar: brand left, session facts right
    ├── overview.rs      # system, memory, boot log and subsystem windows
    ├── notifications.rs # toast banners
    ├── icons.rs         # badge → theme asset, size and glyph lookups
    └── sounds.rs        # event → sound file mapping
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
(virtio-gpu) moves the same shell onto real graphics, where the icon
theme, sound events and theme artwork in `assets/` take over from the
text-surface stand-ins.
