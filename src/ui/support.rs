//! Rendering for `rhow support`: the compatibility registry, and — inside a repository — how
//! its own declared versions compare against it. See [`crate::support`] for the data.

use super::{pad, width, Style};
use crate::model::Repo;
use crate::support::{self, Compat, Finding};
use serde::Serialize;

fn compat_marker(style: &Style, c: Compat) -> String {
    match c {
        Compat::Supported => String::new(),
        Compat::Newer => {
            if style.color {
                format!("  {}", style.yellow("● newer than verified"))
            } else {
                "  [newer than verified]".to_string()
            }
        }
        Compat::Unclear => format!("  {}", style.dim("? unclear")),
    }
}

/// Plain-text report: the registry, the ecosystems with no version check, and — when `repo`
/// has any declared versions at all — a per-project comparison against the registry.
pub fn render(repo: Option<&Repo>, style: &Style) -> String {
    let mut o = String::new();
    o.push_str(&style.bold("Toolsets this build of rhow has been verified against"));
    o.push('\n');
    let name_w = support::registry()
        .iter()
        .map(|e| width(&e.name))
        .max()
        .unwrap_or(4)
        .clamp(4, 24);
    for e in support::registry() {
        o.push_str(&format!(
            "  {}  {}\n",
            style.cyan(&pad(&e.name, name_w)),
            e.verified
        ));
    }

    o.push('\n');
    o.push_str(&style.bold("Also discovered, with no single version to check"));
    o.push('\n');
    let others: Vec<&str> = support::others().iter().map(|e| e.name.as_str()).collect();
    o.push_str(&wrap(&others.join(", "), 96, "  "));
    o.push('\n');

    let Some(repo) = repo else { return o };
    let findings = support::check(repo);
    if findings.is_empty() {
        return o;
    }
    o.push('\n');
    o.push_str(&style.bold(&format!("Detected in {}", repo.name)));
    o.push('\n');
    let name_w = findings
        .iter()
        .map(|f| width(&f.name))
        .max()
        .unwrap_or(4)
        .clamp(4, 24);
    let val_w = findings
        .iter()
        .map(|f| width(&f.value))
        .max()
        .unwrap_or(4)
        .clamp(4, 20);
    for f in &findings {
        let project = if f.project == "." {
            String::new()
        } else {
            format!("  ({})", f.project)
        };
        o.push_str(&format!(
            "  {}  {}  {}{}{}\n",
            style.cyan(&pad(&f.name, name_w)),
            pad(&f.value, val_w),
            style.dim(&f.source),
            project,
            compat_marker(style, f.compat),
        ));
    }
    if findings.iter().any(|f| f.compat == Compat::Newer) {
        o.push('\n');
        o.push_str(&style.dim(
            "A marked version is newer than this build of rhow has been verified against.\n\
             It will very likely still work; new syntax in that version may not be recognised yet.",
        ));
        o.push('\n');
    }
    o
}

/// Wrap a comma-joined list to `width` columns, prefixing every line with `indent`.
fn wrap(items: &str, width: usize, indent: &str) -> String {
    let mut out = String::new();
    let mut line = String::new();
    for part in items.split(", ") {
        let candidate = if line.is_empty() {
            part.to_string()
        } else {
            format!("{line}, {part}")
        };
        if candidate.len() > width && !line.is_empty() {
            out.push_str(indent);
            out.push_str(&line);
            out.push_str(",\n");
            line = part.to_string();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        out.push_str(indent);
        out.push_str(&line);
        out.push('\n');
    }
    out
}

#[derive(Serialize)]
struct Envelope {
    version: &'static str,
    registry: &'static [support::Entry],
    other: &'static [support::Other],
    findings: Vec<Finding>,
}

pub fn render_json(repo: Option<&Repo>) -> String {
    let env = Envelope {
        version: env!("CARGO_PKG_VERSION"),
        registry: support::registry(),
        other: support::others(),
        findings: repo.map(support::check).unwrap_or_default(),
    };
    serde_json::to_string_pretty(&env).unwrap_or_else(|_| "{}".into())
}
