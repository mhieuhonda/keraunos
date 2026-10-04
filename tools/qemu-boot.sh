#!/usr/bin/env bash
# Boot the Keraunos ISO under QEMU with the serial console on stdout.
#
# Usage: tools/qemu-boot.sh [iso] [timeout-seconds]
#
# Requires: qemu-system-x86_64.
#
# The kernel parks in a halt loop after printing its final "BOOT OK" line,
# so QEMU will not exit by itself; this script stops after the timeout and
# greps the captured serial log for the success marker.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ISO="${1:-$ROOT/build/keraunos.iso}"
TIMEOUT="${2:-60}"

if [ ! -f "$ISO" ]; then
    echo "error: ISO not found at $ISO" >&2
    echo "hint: run tools/mkiso.sh first" >&2
    exit 1
fi

command -v qemu-system-x86_64 >/dev/null || {
    echo "error: qemu-system-x86_64 not found (Debian/Ubuntu: apt install qemu-system-x86)" >&2
    exit 1
}

LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT

set +e
timeout "$TIMEOUT" qemu-system-x86_64 \
    -machine q35 -m 512M \
    -cdrom "$ISO" \
    -display none \
    -serial file:"$LOG" \
    -no-reboot
STATUS=$?
set -e

# 124 = timeout while the kernel sits in its halt loop: expected here.
if [ "$STATUS" -ne 0 ] && [ "$STATUS" -ne 124 ]; then
    echo "error: QEMU exited with status $STATUS" >&2
    cat "$LOG" >&2 || true
    exit 1
fi

cat "$LOG"

if grep -q "BOOT OK" "$LOG"; then
    echo "== QEMU BOOT TEST PASSED =="
else
    echo "== QEMU BOOT TEST FAILED (no BOOT OK in serial log) ==" >&2
    exit 1
fi
