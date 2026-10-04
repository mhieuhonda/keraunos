---
name: Boot log from hardware
about: Report a boot result from a machine — success or failure both help
labels: boot-log
---

**Machine**

* Make / model:
* CPU:
* RAM:
* Firmware: [BIOS / UEFI / CSM]
* Boot medium: [USB stick / internal disk / netboot]

**Result**

* [ ] Reached `BOOT OK`
* [ ] Booted partially (describe where it stops)
* [ ] Did not boot (describe what happened)

**Boot log**

```
(paste serial capture if available; otherwise a photo of the VGA screen)
```

**Notes**

Anything unusual: weird memory maps, peripherials that grab resources,
secure boot state, etc. If it booted: you'll be added to the L1 list in
docs/HARDWARE.md — say the name/URL you want the credit to point to.
