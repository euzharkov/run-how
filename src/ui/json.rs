//! `--json` output: the normalised model.
//!
//! Every document starts with `"schema": 2`; bump it when a field changes meaning or goes
//! away (adding fields is compatible). `root` is the absolute repository path; every other
//! path (`projects[].path`, `actions[].working_directory`, `ci[].file`) is relative to it
//! with `/` separators on every platform, exactly as the scanner produces them.
//!
//! JSON carries exactly the actions the listing shows; nothing is hidden. Schema 2 dropped
//! `hidden`: an action another one covers is no longer reported at all.

use crate::model::{CiPipeline, Repo};
use serde::Serialize;

pub const SCHEMA: u32 = 2;

#[derive(Serialize)]
struct Envelope<'a> {
    schema: u32,
    version: &'static str,
    #[serde(flatten)]
    repo: &'a Repo,
}

pub fn render(repo: &Repo) -> String {
    let env = Envelope {
        schema: SCHEMA,
        version: env!("CARGO_PKG_VERSION"),
        repo,
    };
    serde_json::to_string_pretty(&env).unwrap_or_else(|_| "{}".into())
}

#[derive(Serialize)]
struct CiEnvelope<'a> {
    schema: u32,
    version: &'static str,
    ci: &'a [CiPipeline],
}

/// `rhow --ci --json`: only the pipelines.
pub fn render_ci(repo: &Repo) -> String {
    let env = CiEnvelope {
        schema: SCHEMA,
        version: env!("CARGO_PKG_VERSION"),
        ci: &repo.ci,
    };
    serde_json::to_string_pretty(&env).unwrap_or_else(|_| "{}".into())
}
