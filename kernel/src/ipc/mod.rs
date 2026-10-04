//! Inter-process communication.
//!
//! Plan (see `docs/ARCHITECTURE.md`): a single unified primitive — typed,
//! capability-scoped channels (synchronous rendezvous with async queues on
//! top) — plus POSIX signals, pipes and shared memory emulated over them
//! for Linux-ABI compatibility (milestone M6).
//!
//! Status: scaffold.
