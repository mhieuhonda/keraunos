#!/usr/bin/env python3
"""Boot Keraunos under the Unicorn CPU emulator.

This is a protocol-level smoke test: we emulate what GRUB2 does — load the
ELF PT_LOAD segments, build a multiboot2 information block, then hand off
**in 32-bit protected mode with paging disabled** (EAX = multiboot2 magic,
EBX = MBI pointer), exactly as the multiboot2 specification prescribes.
The kernel's own entry stub performs the switch to 64-bit long mode
(PAE, identity page tables, EFER.LME, far jump); emulation runs until the
kernel parks in its halt loop.

Device model:
  * 16550 UART on COM1 — LSR always reports THR-empty; TX bytes captured.
  * CPUID — intercepted to present a stable "UnicornCPU" vendor string.
  * VGA text buffer at 0xB8000 — decoded at the end to show the 'screen'.

Usage: boot_test.py [path-to-elf]
"""

import struct
import sys

import unicorn
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UcError
from unicorn import UC_HOOK_INSN, UC_HOOK_MEM_INVALID, UC_HOOK_INTR
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDX,
    UC_X86_REG_RIP,
    UC_X86_INS_IN, UC_X86_INS_OUT, UC_X86_INS_CPUID,
)

ELF = sys.argv[1] if len(sys.argv) > 1 else \
    "/home/z/my-project/keraunos/target/x86_64-unknown-none/debug/keraunos-kernel"

MB2_BOOT_MAGIC = 0x36D76289
MBI_ADDR = 0x0004_0000          # low memory, below the 1 MiB kernel base
MEM_SIZE = 0x4000_0000          # 1 GiB of RAM

serial_out = bytearray()
COM1 = 0x3F8
LSR = COM1 + 5
THR_EMPTY = 1 << 5

# ---------------------------------------------------------------- ELF loader

def load_elf(uc, path):
    with open(path, "rb") as f:
        blob = f.read()
    assert blob[:4] == b"\x7fELF" and blob[4] == 2, "not an ELF64 file"
    e_entry, = struct.unpack_from("<Q", blob, 0x18)
    phoff, = struct.unpack_from("<Q", blob, 0x20)
    phentsize, phnum = struct.unpack_from("<HH", blob, 0x36)
    loads = []
    for i in range(phnum):
        off = phoff + i * phentsize
        p_type, p_flags = struct.unpack_from("<II", blob, off)
        p_offset, p_vaddr, p_paddr, p_filesz, p_memsz = struct.unpack_from("<QQQQQ", blob, off + 8)
        if p_type != 1:  # PT_LOAD
            continue
        uc.mem_write(p_paddr, blob[p_offset : p_offset + p_filesz])
        if p_memsz > p_filesz:  # .bss
            uc.mem_write(p_paddr + p_filesz, b"\x00" * (p_memsz - p_filesz))
        loads.append((p_paddr, p_memsz))
    return e_entry, loads

# ------------------------------------------------------- multiboot2 MBI builder

def mb2_tag(kind, payload: bytes) -> bytes:
    size = 8 + len(payload)
    pad = (-size) % 8
    return struct.pack("<HHI", kind, 0, size) + payload + b"\x00" * pad


def build_mbi() -> bytes:
    parts = []
    parts.append(mb2_tag(2, b"UnicornGRUB 2.12-emul\x00"))          # bootloader name
    parts.append(mb2_tag(1, b"console=uart log=verbose\x00"))       # command line
    mm = struct.pack("<II", 24, 0)                                  # entry_size, entry_version
    mm += struct.pack("<QQII", 0x0000_0000, 0x0010_0000, 2, 0)      # low 1 MiB reserved
    mm += struct.pack("<QQII", 0x0010_0000, MEM_SIZE - 0x0010_0000, 1, 0)  # usable RAM
    parts.append(mb2_tag(6, mm))                                    # memory map
    parts.append(struct.pack("<HHI", 0, 0, 8))                      # end tag
    body = b"".join(parts)
    return struct.pack("<II", 8 + len(body), 0) + body

# --------------------------------------------------------------- device model

def hook_in(uc, port, size, user_data):
    if port == LSR:
        return THR_EMPTY            # transmitter always ready
    return 0


def hook_out(uc, port, size, value, user_data):
    if port == COM1:
        serial_out.append(value & 0xFF)


def hook_cpuid(uc, user_data):
    # NOTE: 32-bit register ids only — on a MODE_32 unicorn the 64-bit ids
    # (RAX/RBX/...) are deprecated no-ops even after the kernel switches the
    # CPU into long mode.
    rax = uc.reg_read(UC_X86_REG_EAX)
    if rax == 0:
        uc.reg_write(UC_X86_REG_EAX, 0)
        uc.reg_write(UC_X86_REG_EBX, int.from_bytes(b"Unic", "little"))
        uc.reg_write(UC_X86_REG_EDX, int.from_bytes(b"ornC", "little"))
        uc.reg_write(UC_X86_REG_ECX, int.from_bytes(b"PU\x00\x00", "little"))
    else:
        uc.reg_write(UC_X86_REG_EAX, 0)
        uc.reg_write(UC_X86_REG_EBX, 0)
        uc.reg_write(UC_X86_REG_ECX, 0)
        uc.reg_write(UC_X86_REG_EDX, 0)
    return 1                       # handled: skip native execution


def hook_intr(uc, intno, user_data):
    print(f"!! CPU raised interrupt vector {intno} at rip={uc.reg_read(UC_X86_REG_RIP):#x}")
    uc.emu_stop()


def hook_mem_invalid(uc, access, address, size, value, user_data):
    print(f"!! invalid memory access: addr={address:#x} size={size} value={value:#x}")
    return False

# ------------------------------------------------------------------------ run

uc = Uc(UC_ARCH_X86, UC_MODE_32)   # GRUB hands off in 32-bit protected mode
uc.mem_map(0, MEM_SIZE)

uc.hook_add(UC_HOOK_INSN, hook_in, None, 1, 0, UC_X86_INS_IN)
uc.hook_add(UC_HOOK_INSN, hook_out, None, 1, 0, UC_X86_INS_OUT)
uc.hook_add(UC_HOOK_INSN, hook_cpuid, None, 1, 0, UC_X86_INS_CPUID)
uc.hook_add(UC_HOOK_INTR, hook_intr)
uc.hook_add(UC_HOOK_MEM_INVALID, hook_mem_invalid)

entry, loads = load_elf(uc, ELF)
print(f"loaded ELF: entry={entry:#x}, PT_LOADs: " + ", ".join(f"{p:#x}+{m:#x}" for p, m in loads))

mbi = build_mbi()
uc.mem_write(MBI_ADDR, mbi)
print(f"multiboot2 MBI: {len(mbi)} bytes @ {MBI_ADDR:#x}")

uc.reg_write(UC_X86_REG_EAX, MB2_BOOT_MAGIC)
uc.reg_write(UC_X86_REG_EBX, MBI_ADDR)
uc.reg_write(UC_X86_REG_RIP, entry)

try:
    # The debug build's unoptimized loops (bitmap sweeps, slab poisoning)
    # need real instruction headroom; release fits far below this.
    uc.emu_start(entry, 0, timeout=120_000_000, count=400_000_000)
except UcError as e:
    print(f"!! emulation stopped with error: {e} (rip={uc.reg_read(UC_X86_REG_RIP):#x})")

text = bytes(serial_out).decode("ascii", "replace").replace("\r", "")
print("\n---------------- serial console ----------------")
print(text, end="" if text.endswith("\n") else "\n")
print("------------------------------------------------")

vga = uc.mem_read(0xB8000, 80 * 25 * 2)
screen_lines = []
for row in range(25):
    chars = []
    for col in range(80):
        ch, attr = vga[(row * 80 + col) * 2 : (row * 80 + col) * 2 + 2]
        chars.append(chr(ch) if 0x20 <= ch < 0x7F else " ")
    line = "".join(chars).rstrip()
    if line:
        screen_lines.append(line)
print("---------------- VGA screen --------------------")
for line in screen_lines:
    print(line)
print("------------------------------------------------")

checks = {
    "banner": "K E R A U N O S" in text,
    "bootloader tag": "multiboot2 via UnicornGRUB" in text,
    "cmdline tag": "console=uart log=verbose" in text,
    "cpu vendor": "UnicornCPU" in text,
    "memory map": "usable" in text,
    "bump alloc": "bump allocator armed" in text,
    # M2: bitmap PMM, higher-half paging, slab heap.
    "pmm armed": "bitmap pmm armed" in text,
    "higher-half paging": "kernel mapped in the higher half" in text,
    "slab heap": "heap: slab caches online" in text,
    "heap smoke": "heap smoke test" in text,
    "meminfo": "MemTotal" in text,
    "self-test pass": "self-test: PASS" in text and "0 leaks" in text,
    "frame smoke": "frame smoke test" in text,
    "subsystem table": "subsystem bring-up status" in text,
    "session handoff": "console handoff: vga sink detached" in text,
    "compositor": "compositor: system, memory, boot log, subsystems mapped" in text,
    "final": "BOOT OK" in text,
    # Desktop assertions on the decoded VGA screen.
    "desktop top bar": any(l.startswith(" Keraunos ") for l in screen_lines),
    "desktop windows": "usable memory" in "\n".join(screen_lines),
    "desktop paging row": "higher-half" in "\n".join(screen_lines),
    "desktop notification": "BOOT OK  Keraunos" in "\n".join(screen_lines),
}
print("checks:")
all_ok = True
for name, ok in checks.items():
    print(f"  {'PASS' if ok else 'FAIL'}  {name}")
    all_ok &= ok
print("RESULT:", "BOOT TEST PASSED" if all_ok else "BOOT TEST FAILED")
sys.exit(0 if all_ok else 1)
