//! Hardware Abstraction Layer.
//!
//! The HAL is the portability seam that lets one kernel run on everything
//! from a watch to a server: clocks, timers, interrupt controllers, SMP
//! bring-up, caches, DMA and power management are expressed here as traits,
//! and each architecture family provides implementations.
//!
//! Status: interface drafting begins in milestone M1 alongside the x86-64
//! IDT; design notes live in `docs/ARCHITECTURE.md`.
