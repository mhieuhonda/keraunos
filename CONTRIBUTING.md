# Contributing to Keraunos

Thanks for wanting to help build a faster operating system. This guide
gets you from clone to a merged PR without mystery.

## Ground rules

* **Be kind.** The [Code of Conduct](CODE_OF_CONDUCT.md) is enforced, not
  decorative.
* **CI must boot.** A PR is done when the kernel *builds on three
  architectures, passes fmt/clippy, and reaches `BOOT OK` under both boot
  tests* — not when the diff looks finished.
* **`UI/` is read-only.** It is a vendored, pinned snapshot of upstream
  GNOME (mutter, gnome-shell) and Yaru. Changes to those projects belong
  upstream; upgrades here are deliberate re-import PRs that update the
  pinned SHAs in [UI/README.md](UI/README.md).
* **No diaries.** Commit messages, docs and PR descriptions are written
  for the community's future readers — state what, why, and how to
  verify, in the imperative. Progress narratives belong in your own
  space.

## Environment

```sh
curl -sSf https://sh.rustup.rs | sh     # rust-toolchain.toml pins 1.99
git clone https://github.com/mhieuhonda/keraunos.git
cd keraunos
cargo build                              # x86-64 kernel
python tools/boot-unicorn/boot_test.py   # quick boot verification
```

Full details, including the QEMU/GRUB path: [docs/BUILDING.md](docs/BUILDING.md).

## Where to start

* Issues labeled [`good first issue`](https://github.com/mhieuhonda/keraunos/labels/good%20first%20issue)
  are scoped for a first PR (docs counts!).
* [`docs/ROADMAP.md`](docs/ROADMAP.md) milestone M1 is the active front —
  IDT, exception gates, APIC timer — with design notes in
  [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
* Found a bug while booting on odd hardware? A serial log attached to an
  issue is a full contribution (see [docs/HARDWARE.md](docs/HARDWARE.md)).

## Code standards

* `cargo fmt` defaults; `cargo clippy` with zero warnings on changed code.
* Every `unsafe` block carries a `// SAFETY:` comment stating the
  invariant it upholds.
* Kernel logging goes through `kinfo!`/`kerror!` with a short subsystem
  tag; no `println!`-style raw writes outside `console/`.
* New modules document their performance contract in rustdoc — see
  existing modules for the expected tone and depth.
* Dependencies: proposed in an issue first, audited, then feature-gated.
  The kernel is dependency-free today and that is a feature.

## Commit messages

Conventional Commits, imperative mood:

```
feat(mm): add bitmap PMM zone allocator

Zone freelists are per-NUMA-node with a lock-free pop fast path.
Boot self-test allocates/frees 10k frames per zone and asserts
zero leaks; Unicorn + QEMU boot tests updated to count frames.

Closes #42
```

## Pull requests

1. Branch from `main`: `feat/<topic>`, `fix/<topic>`, `docs/<topic>`.
2. One logical change per PR; the kernel stays bootable at every commit
   (bisectability is the whole point of the CI boot tests).
3. PR description: what + why + how verified. Link the issue.
4. CI green on all five jobs → review (usually within a few days) →
   squash-merged with your commit message.
5. Large design changes (new subsystem, ABI surface) start as a
   GitHub Discussion or a design-doc PR into `docs/` — never as a
   3,000-line surprise.

## Issue guidelines

* **Kernel/boot bugs:** full boot log (serial capture if possible),
  build profile (debug/release), QEMU version or hardware specs.
* **Feature requests:** name the user-visible outcome and, if known, the
  milestone it maps to in the roadmap.
* **Boot logs from hardware:** title them `boot log: <make/model>` (see
  docs/HARDWARE.md) — these feed the support matrix directly.

## Communication

* [GitHub Discussions](https://github.com/mhieuhonda/keraunos/discussions)
  for design talk, roadmap debate and show-and-tell.
* Issues for actionable work with acceptance criteria.
* Security issues go through [SECURITY.md](SECURITY.md), never public
  issues.
