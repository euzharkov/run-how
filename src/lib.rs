//! `rhow` — discover how to run and operate a repository.
//!
//! The crate is organised as a pipeline:
//!
//! * [`repo`]     — find the repository root and scan its directory tree (read-only).
//! * [`discover`] — ecosystem adapters that turn files on disk into normalised [`model::Action`]s.
//! * [`analyze`]  — static analysis of shell command strings (tokenising, tool recognition).
//! * [`explain`]  — deterministic plain-English descriptions built from the analysis.
//! * [`risk`]     — classification of destructive / external actions.
//! * [`notes`]    — practical notes: long-running, device, network, git.
//! * [`ci`]       — GitHub Actions and GitLab CI pipelines as a structure of jobs and steps.
//! * [`runtime`]  — local runtime suggestions (e.g. Colima when Docker is missing).
//! * [`ui`]       — terminal and JSON rendering.
//!
//! `rhow` never executes project code, never contacts a network service and never writes to
//! the repository. It shows commands; it is not a task runner.

pub mod analyze;
pub mod ci;
pub mod discover;
pub mod explain;
pub mod model;
pub mod notes;
pub mod repo;
pub mod risk;
pub mod runtime;
pub mod support;
pub mod ui;

pub use discover::{discover, Options};
pub use model::*;
