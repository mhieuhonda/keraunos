//! Scheduler.
//!
//! Design goal (see `docs/ARCHITECTURE.md`): a tickless, per-CPU,
//! multi-level feedback scheduler with SCHED_DEADLINE-style real-time
//! classes — built for latency-bound interactive sessions (the desktop in
//! `/UI`) and throughput-bound workloads at the same time.
//!
//! Status: scaffold — task structures, run queues and preemption arrive in
//! milestone M3, after interrupts (M1) and paging (M2).
