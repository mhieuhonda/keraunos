---
name: Kernel / boot bug
about: The kernel misbehaves — crash, wrong output, boot failure
labels: bug
---

**Environment**

* Build profile: [debug / release]
* Boot method: [Unicorn test / QEMU ISO (machine type) / real hardware (make + model)]
* QEMU version / firmware (BIOS or UEFI, version):
* CPU + RAM:

**Kernel output**

```
(paste the full serial / console log; for hardware, a photo of the VGA
screen works if serial isn't captured)
```

**What happened**

A clear description of the wrong behavior, and where in the boot log it
diverges from the expected sequence (banner → protocol tags → GDT →
memory map → bump allocator → subsystem table → BOOT OK).

**What you expected**

**Reproduction**

1. ...
2. ...

**Extra context**

.config / command lines, kernel command line if passed, anything unusual
about the memory map or the machine.
