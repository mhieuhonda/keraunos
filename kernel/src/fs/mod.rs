//! Filesystem stack.
//!
//! Plan (see `docs/ROADMAP.md`): a VFS layer with Linux-compatible path and
//! fd semantics, a RAMFS for early boot and `/proc`-style introspection,
//! then an ext2 read/write implementation chosen for simplicity without
//! sacrificing the Linux-ABI goal (milestone M4). The `/UI` tree is mounted
//! read-only at boot as the initial asset store.
//!
//! Status: scaffold.
