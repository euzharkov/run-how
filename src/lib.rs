//! `rhow` — discover how to run and operate a repository.
//!
//! The crate is organised as a pipeline:
//!
//! * [`repo`]     — find the repository root and scan its directory tree (read-only).
//! * [`discover`] — ecosystem adapters that turn files on disk into normalised [`model::Action`]s.
//! * [`analyze`]  — static analysis of shell command strings (tokenising, tool recognition).
//! * [`explain`]  — deterministic plain-English descriptions built from the analysis.
//! * [`risk`]     — classification of destructive / external actions.
//! * [`runtime`]  — local runtime suggestions (e.g. Colima when Docker is missing).
//! * [`exec`]     — running a discovered action through its native tool.
//! * [`ui`]       — terminal and JSON rendering.
//!
//! Discovery never executes project code, never contacts a network service and never
//! writes to the repository.

pub mod analyze;
pub mod discover;
pub mod exec;
pub mod explain;
pub mod model;
pub mod repo;
pub mod risk;
pub mod runtime;
pub mod support;
pub mod ui;

pub use discover::{discover, Options};
pub use model::*;
