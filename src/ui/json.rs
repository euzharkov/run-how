//! `--json` output: the normalised model.
//!
//! Every document starts with `"schema": 1`; bump it when a field changes meaning or goes
//! away (adding fields is compatible). `root` is the absolute repository path; every other
//! path (`projects[].path`, `actions[].working_directory`, `ci[].file`) is relative to it
//! with `/` separators on every platform, exactly as the scanner produces them.
//!
//! JSON always carries every discovered action, including the ones the listing keeps behind
//! `--all`: they are marked `"hidden": true` (internal or lifecycle scripts) or
//! `"confidence": "low"`, so a consumer applies its own filter.

use crate::model::{Action, CiPipeline, Repo};
use serde::Serialize;

pub const SCHEMA: u32 = 1;

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

#[derive(Serialize)]
struct ActionEnvelope<'a> {
    schema: u32,
    version: &'static str,
    /// The project's path relative to the root (`.` for the root).
    project: &'a str,
    action: &'a Action,
    /// The script body the command resolves to, when it differs from the command (what the
    /// terminal view shows as `runs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    runs: Option<&'a str>,
}

/// `rhow why <x> --json` / `rhow <id> --json`: one action as data, `None` when no action
/// matches (the same lookup as the terminal view).
pub fn render_action(repo: &Repo, id: &str) -> Option<String> {
    let (project, action) = super::find_action(repo, id)?;
    let env = ActionEnvelope {
        schema: SCHEMA,
        version: env!("CARGO_PKG_VERSION"),
        project: &project.path,
        action,
        runs: action.raw.as_deref().filter(|raw| *raw != action.command),
    };
    Some(serde_json::to_string_pretty(&env).unwrap_or_else(|_| "{}".into()))
}
