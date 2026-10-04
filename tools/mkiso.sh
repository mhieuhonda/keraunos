#!/usr/bin/env bash
# Build a bootable Keraunos ISO (GRUB2 + multiboot2 kernel).
#
# Usage: tools/mkiso.sh [kernel-elf] [output.iso]
#
# Requires: grub-mkrescue (grub-pc-bin / grub-common), xorriso, mtools.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KERNEL="${1:-$ROOT/target/x86_64-unknown-none/release/keraunos-kernel}"
OUT="${2:-$ROOT/build/keraunos.iso}"

if [ ! -f "$KERNEL" ]; then
    echo "error: kernel not found at $KERNEL" >&2
    echo "hint: run 'cargo build --release' first" >&2
    exit 1
fi

command -v grub-mkrescue >/dev/null || {
    echo "error: grub-mkrescue not found (Debian/Ubuntu: apt install grub-pc-bin xorriso mtools)" >&2
    exit 1
}

STAGING="$(mktemp -d)"
trap 'rm -rf "$STAGING"' EXIT

mkdir -p "$STAGING/boot/grub"
cp "$KERNEL" "$STAGING/boot/kernel.elf"
cp "$ROOT/tools/grub.cfg" "$STAGING/boot/grub/grub.cfg"

grub-mkrescue -o "$OUT" "$STAGING" --quiet

echo "ISO written to $OUT"
