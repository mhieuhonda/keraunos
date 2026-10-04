//! Networking.
//!
//! Plan (see `docs/ROADMAP.md`): virtio-net first (development/QEMU), then
//! Intel E1000/E1000E, Realtek r8169 and USB CDC-NCM. The stack itself is
//! zero-copy end to end: NIC rings -> kernel packet buffers -> socket
//! queues, with io_uring-style async completion as the primary user API.
//!
//! Status: scaffold — arrives in milestone M5 after userspace exists.
