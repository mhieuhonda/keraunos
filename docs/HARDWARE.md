# Hardware support matrix

Keraunos' compatibility goal is stated plainly: **one kernel that spans
the device classes people actually compute on**. That is engineered, not
wished for — by constraining all hardware specifics behind `src/arch/`
and the `src/hal/` traits, and by an explicit board-bring-up program
(roadmap M7).

## Architecture families

| Family | Targets | State |
| --- | --- | --- |
| x86-64 | PC/AT-compatible, UEFI class 3, cloud VMs (QEMU/KVM, Hyper-V, EC2 bare metal) | **boots** (BIOS/multiboot2 path online; UEFI path is M7) |
| aarch64 | ARMv8-A, EL1/EL2 | scaffold, type-checked in CI; bring-up M1-M2 (GICv3, PSCI, stage-2 aware) |
| riscv64 | RV64GC, S-mode under OpenSBI | scaffold, type-checked in CI; bring-up after aarch64 |

Why these three families: they cover effectively every general-purpose
computing device shipped today — servers, desktops, laptops, tablets,
phones-class SoCs, SBCs and microcontrollers running Linux-class
software. 32-bit-only and exotic ISAs are out of scope for now; if one
matters to you, the HAL boundary is where it would plug in — open a
discussion.

## Verified environments

| Environment | Boot path | Verified by |
| --- | --- | --- |
| Unicorn x86-64 emulation | multiboot2 hand-off | CI, every push (`tools/boot-unicorn`) |
| QEMU q35, 512M, GRUB2 ISO | BIOS → GRUB2 → multiboot2 | CI, every push (`tools/mkiso.sh` + `tools/qemu-boot.sh`) |
| Physical PCs | BIOS/CSM → GRUB2 | community boot logs — file an issue titled `boot log: <machine>` |

## Bring-up levels

Each platform is tracked at one of four levels:

* **L0 scaffold** — compiles for the ISA; no bring-up yet (aarch64,
  riscv64 today).
* **L1 boots** — reaches `BOOT OK` with consoles and memory map.
* **L2 usable** — storage + input + graphics work (interactive use).
* **L3 certified** — hardware-in-the-loop CI coverage via donors.

## Target platforms (M7 program)

| Class | Devices |
| --- | --- |
| Development | QEMU (all three ISAs), virt platforms |
| Desktops/laptops | generic x86-64 PCs, Apple Silicon virtualized |
| SBCs | Raspberry Pi 4/5 (aarch64), VisionFive 2 (riscv64), Rockchip RK3588 |
| Servers | any UEFI x86-64; Ampere Altra-class aarch64 |
| Handhelds/embedded | Steam Deck sibling hardware (x86-64), community-nominated boards |

## Reporting hardware results

The single most valuable early contribution is a **boot log**:

1. Build the ISO ([docs/BUILDING.md](BUILDING.md)).
2. Boot it on hardware nobody has reported yet, with a serial capture if
   possible.
3. Open an issue titled `boot log: <make/model>` with the log, firmware
   type (BIOS/UEFI/CSM), CPU, RAM, and what the VGA screen showed.

Reaches `BOOT OK`? The machine goes on the L1 list verbatim. Doesn't?
You just found the kernel's next bug — and the log is the reproducer.
