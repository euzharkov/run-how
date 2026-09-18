//! `--json` output: the normalised model.

use crate::model::Repo;
use serde::Serialize;

#[derive(Serialize)]
struct Envelope<'a> {
    version: &'static str,
    #[serde(flatten)]
    repo: &'a Repo,
}

pub fn render(repo: &Repo) -> String {
    let env = Envelope {
        version: env!("CARGO_PKG_VERSION"),
        repo,
    };
    serde_json::to_string_pretty(&env).unwrap_or_else(|_| "{}".into())
}
