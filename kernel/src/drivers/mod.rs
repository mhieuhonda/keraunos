//! Device drivers.
//!
//! Bring-up order is chosen to reach a usable developer console as fast as
//! possible: PIC/PIT -> PCI enumeration -> NVMe/AHCI -> virtio -> USB HID ->
//! GPU (virtio-gpu, then native) -> input/evdev -> network.
//!
//! The driver model (probe/DMA/IRQ plumbing over the HAL traits) is defined
//! in milestone M1-M2. Status: scaffold.
