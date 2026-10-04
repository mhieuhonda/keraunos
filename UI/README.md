# UI — the staged default interface stack

This directory holds the **verbatim codebases** of the projects that will
become Keraunos' default desktop experience. They are imported as-is, at
pinned upstream commits, Keraunos will run
them unmodified on top of its Linux-ABI compatibility layer (roadmap
milestone M6, see `docs/ROADMAP.md`).

## Why these projects?

* **mutter** — the compositor and window manager (Wayland). It owns every
  pixel on screen, so it is where "fast" is most visible: Keraunos'
  performance-first kernel + a compositing stack that already drives most
  of the Linux desktop world.
* **gnome-shell** — the shell itself: top bar, overview, notifications,
  app launching, the whole user-facing desktop model.
* **Yaru** — the Ubuntu visual identity: icons, themes, cursors, sounds and
  wallpapers; the look users recognize instantly.

Together they form a complete, production-grade desktop — and a demanding
one, which is exactly what a performance-focused kernel wants as its
first showcase workload.

## Imports

| Component   | Upstream                                           | Imported commit                                   | License                  |
| ----------- | -------------------------------------------------- | -------------------------------------------------- | ------------------------ |
| mutter      | <https://gitlab.gnome.org/GNOME/mutter> (GitHub mirror: <https://github.com/GNOME/mutter>) | `285f54d394b6c041dcdab90d79ecdb761af655ec` | GPL-2.0-or-later         |
| gnome-shell | <https://gitlab.gnome.org/GNOME/gnome-shell> (GitHub mirror: <https://github.com/GNOME/gnome-shell>) | `f9cd9aaedf046ab75ab6d5e9d110ec126ff34995` | GPL-2.0-or-later         |
| Yaru        | <https://github.com/ubuntu/yaru>                   | `7f18973e05607c609c4b972f0dd9bff36fa2a13e`        | GPL-3.0 / CC-BY-SA (per-directory) |

Imported: 2026-10-04, shallow clone of the default branch (`main`,
`main`, `master` respectively), history excluded by design — this tree is
a vendored codebase, not a fork. **No file inside `mutter/`,
`gnome-shell/` or `yaru/` has been changed.** Verify it yourself:

```sh
# compares the vendored tree against a fresh clone of the pinned commit
git clone https://github.com/GNOME/mutter.git /tmp/mutter
git -C /tmp/mutter checkout 285f54d394b6c041dcdab90d79ecdb761af655ec
diff -r --exclude=.git /tmp/mutter UI/mutter
```

## Policies

1. **Read-only by default.** Keraunos must run these codebases unmodified —
   the compat layer adapts to them, not the other way around. Patches that
   "fix" UI projects locally are rejected by default.
2. **Upstream pinning.** Each import records its commit SHA in the table
   above and in the importing commit message. Upgrades are deliberate,
   reviewable re-imports.
3. **Licensing.** Each subdirectory retains its own upstream license files
   and headers. The repository's top-level MIT license does not apply to
   the contents of `mutter/`, `gnome-shell/` or `yaru/`.
4. **Upstream contributions welcome.** Improvements to these projects
   should go upstream (GNOME / Ubuntu); Keraunos then re-imports them here.

## Layout

```
UI/
├── mutter/        # compositor / window manager (C, Meson)
├── gnome-shell/   # the shell (JS + C, Meson)
└── yaru/          # icons, themes, sounds, wallpapers (SVG/PNG/CSS/audio)
```
