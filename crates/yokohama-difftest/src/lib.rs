//! Dev-only differential-testing harness for the Yokohama engine (Phase 2.9).
//!
//! The ENGINE drives seeded self-play and applies moves; the oracle
//! (`yokohama-oracle`) only enumerates legal moves per state; this crate
//! compares the two per state and writes a self-contained divergence
//! artifact (full `GameState` snapshot) when they disagree.
//!
//! This crate is openly IMPLEMENTATION-AWARE: it reads engine internals and
//! calls engine pricing functions freely. The phase plan's independence rule
//! protects `yokohama-oracle`, never this harness.

pub mod artifact;
pub mod compare;
pub mod drive;
pub mod runner;
pub mod scoring;
