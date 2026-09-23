//! The "supported toolsets" registry: which tool/schema versions this build of `rhow` has
//! actually been verified against, and a checker that compares a discovered repository's own
//! declared versions (a Cargo edition, a `go.mod` directive, a Taskfile schema, …) against it.
//!
//! This exists so a future toolchain jump — Rust's next edition, a new Taskfile schema, a
//! .NET target framework we haven't seen, a Terraform major version — shows up as "newer than
//! verified" instead of `rhow` silently going stale. Nothing here runs a subprocess or contacts
//! a network: every declared version comes from a file an adapter already reads (see
//! [`crate::model::ToolVersion`]); `rhow support` only ever compares strings that are already
//! part of the normal, read-only discovery pass.
//!
//! **Maintaining this**: when you verify `rhow` against a newer version of a tool (a new Rust
//! edition, a new Taskfile schema, …), bump that tool's `max` and `verified` in `support.toml`
//! at the repository root, then paste the table `cargo test` prints into README.md. That file
//! is the entire "support matrix"; this module only embeds and interprets it.

use crate::model::{Repo, ToolVersion};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Compat {
    /// Within the range this build of rhow has been verified against.
    Supported,
    /// Newer than anything rhow has been verified against. It will very likely still work —
    /// this is a heads-up, not an error — but new syntax or defaults in that version may not
    /// be recognised yet.
    Newer,
    /// The declared value isn't in a form the comparator can safely reduce to a number (a
    /// compound range, a pre-release tag, …). rhow makes no claim either way.
    Unclear,
}

/// How a declared value is judged against [`Entry::max`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Compare {
    /// The declared value, component by component, must not exceed `max`.
    AtMost,
    /// Like `AtMost` after stripping a simple `>=`/`^`/`~`/`=` prefix; compound constraints
    /// are `Unclear`.
    LowerBound,
    /// `netN.N` target frameworks only; `net48` and `netstandard*` are `Unclear`.
    DotnetTfm,
}

/// One row of the support matrix. Deserialised from `support.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Entry {
    /// Matches a [`ToolVersion::tool`].
    pub id: String,
    pub name: String,
    pub category: String,
    /// Config file(s)/field(s) an adapter reads to find the declared version.
    pub config: String,
    /// The highest version this build has been verified against, as numeric components.
    pub max: Vec<u32>,
    pub compare: Compare,
    /// One-line human description of what has been verified.
    pub verified: String,
}

impl Entry {
    /// Compare a declared value against this entry's baseline.
    pub fn check(&self, declared: &str) -> Compat {
        match self.compare {
            Compare::AtMost => at_most(declared, &self.max),
            Compare::LowerBound => range_at_most(declared, &self.max),
            Compare::DotnetTfm => dotnet_tfm(declared, &self.max),
        }
    }
}

/// Ecosystems `rhow` discovers but that have no single "version" field worth comparing.
/// Listed for completeness in `rhow support`, not compared.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Other {
    pub name: String,
    pub category: String,
    pub config: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Matrix {
    #[serde(rename = "tool")]
    pub tools: Vec<Entry>,
    #[serde(rename = "other", default)]
    pub others: Vec<Other>,
}

/// The support matrix, embedded from `support.toml` at the repository root.
pub const SOURCE: &str = include_str!("../../support.toml");

/// The parsed support matrix. Parsed once; a malformed file is a build-time bug caught by the
/// tests, so it panics with the TOML error rather than reporting nothing.
pub fn matrix() -> &'static Matrix {
    static M: OnceLock<Matrix> = OnceLock::new();
    M.get_or_init(|| toml::from_str(SOURCE).expect("support.toml is valid"))
}

/// The tools with a version baseline.
pub fn registry() -> &'static [Entry] {
    &matrix().tools
}

/// Ecosystems listed without a version check.
pub fn others() -> &'static [Other] {
    &matrix().others
}

/// Look up an entry by [`ToolVersion::tool`].
pub fn entry(id: &str) -> Option<&'static Entry> {
    registry().iter().find(|e| e.id == id)
}

// ---------------------------------------------------------------------------------------
// Comparators
// ---------------------------------------------------------------------------------------

fn leading_ints(s: &str) -> Vec<u32> {
    s.split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect()
}

/// Compare against a fixed "verified up to" tuple, using only as many numeric components as
/// the declared value provides (so `"1.22"` against a baseline of `(1, 23, 0)` compares just
/// major.minor). An empty or non-numeric value is `Unclear`, never guessed at.
fn at_most(declared: &str, max: &[u32]) -> Compat {
    let got = leading_ints(declared);
    if got.is_empty() {
        return Compat::Unclear;
    }
    for (g, m) in got.iter().zip(max.iter()) {
        if g > m {
            return Compat::Newer;
        }
        if g < m {
            return Compat::Supported;
        }
    }
    Compat::Supported
}

/// Strip a simple `>=`/`^`/`~`/`=` prefix for comparison, but refuse (return `None`) on
/// anything that combines constraints (`<`, `,`, `||`, `!`, a hyphen range) — those need a
/// real range parser to judge correctly, and a wrong pass/fail is worse than no verdict.
fn simple_lower_bound(v: &str) -> Option<&str> {
    let v = v.trim();
    if v.contains('<')
        || v.contains(',')
        || v.contains("||")
        || v.contains('!')
        || v.contains(" - ")
    {
        return None;
    }
    Some(v.trim_start_matches(['>', '=', '^', '~', ' ']))
}

fn range_at_most(declared: &str, max: &[u32]) -> Compat {
    match simple_lower_bound(declared) {
        Some(v) => at_most(v, max),
        None => Compat::Unclear,
    }
}

fn dotnet_tfm(v: &str, max: &[u32]) -> Compat {
    // "net8.0" / "net9.0" / "netstandard2.1" / "net48" — only the modern "netN.N" shape has a
    // meaningful ordering here; classic .NET Framework (`net48`) and `netstandard*` are Unclear.
    let v = v.trim().to_ascii_lowercase();
    let Some(rest) = v.strip_prefix("net") else {
        return Compat::Unclear;
    };
    if rest.starts_with("standard") || rest.starts_with("coreapp") || !rest.contains('.') {
        return Compat::Unclear;
    }
    at_most(rest, max)
}

/// The matrix as a Markdown table, the block README.md carries under "Version support".
pub fn markdown_table() -> String {
    let mut o = String::from(
        "| Tool | Verified against | Read from |
|---|---|---|
",
    );
    for e in registry() {
        o.push_str(&format!(
            "| {} | {} | {} |
",
            e.name, e.verified, e.config
        ));
    }
    o
}

// ---------------------------------------------------------------------------------------
// Checking a discovered repository
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Project path, `.` for the root.
    pub project: String,
    pub tool: &'static str,
    pub name: String,
    pub value: String,
    pub source: String,
    pub compat: Compat,
}

/// Compare every version a discovered repository declares against the support matrix.
pub fn check(repo: &Repo) -> Vec<Finding> {
    let mut out = Vec::new();
    for p in &repo.projects {
        for v in &p.versions {
            out.push(finding(&p.path, v));
        }
    }
    out
}

fn finding(project: &str, v: &ToolVersion) -> Finding {
    let (name, compat) = match entry(v.tool) {
        Some(e) => (e.name.clone(), e.check(&v.value)),
        None => (v.tool.to_string(), Compat::Unclear),
    };
    Finding {
        project: project.to_string(),
        tool: v.tool,
        name,
        value: v.value.clone(),
        source: v.source.clone(),
        compat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(id: &str, v: &str) -> Compat {
        entry(id)
            .unwrap_or_else(|| panic!("no `{id}` in support.toml"))
            .check(v)
    }

    #[test]
    fn matrix_parses_and_ids_are_unique() {
        let mut ids: Vec<&str> = registry().iter().map(|e| e.id.as_str()).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(n, ids.len(), "duplicate ids in support.toml");
        assert!(registry().iter().all(|e| !e.max.is_empty()));
        assert!(!others().is_empty());
    }

    #[test]
    fn baselines_compare_as_declared() {
        assert_eq!(check("cargo-edition", "2015"), Compat::Supported);
        assert_eq!(check("cargo-edition", "2024"), Compat::Supported);
        assert_eq!(check("cargo-edition", "2027"), Compat::Newer);
        assert_eq!(check("go", "1.22"), Compat::Supported);
        assert_eq!(check("go", "1.26"), Compat::Supported);
        assert_eq!(check("go", "1.27"), Compat::Newer);
        assert_eq!(check("taskfile", "3"), Compat::Supported);
        assert_eq!(check("taskfile", "4"), Compat::Newer);
        assert_eq!(check("gradle", "9.1"), Compat::Supported);
        assert_eq!(check("gradle", "9.2"), Compat::Newer);
        assert_eq!(check("bazel", "8.3.0"), Compat::Supported);
        assert_eq!(check("bazel", "8.4.0"), Compat::Newer);
    }

    #[test]
    fn dotnet_target_frameworks() {
        assert_eq!(check("dotnet-tfm", "net8.0"), Compat::Supported);
        assert_eq!(check("dotnet-tfm", "net10.0"), Compat::Supported);
        assert_eq!(check("dotnet-tfm", "net11.0"), Compat::Newer);
        assert_eq!(check("dotnet-tfm", "net48"), Compat::Unclear);
        assert_eq!(check("dotnet-tfm", "netstandard2.1"), Compat::Unclear);
    }

    #[test]
    fn range_constraints_prefer_unclear() {
        assert_eq!(check("python", ">=3.14"), Compat::Supported);
        assert_eq!(check("python", ">=3.15"), Compat::Newer);
        assert_eq!(check("python", ">=3.9,<4"), Compat::Unclear);
        assert_eq!(check("node", ">=18"), Compat::Supported);
        assert_eq!(check("node", ">=18 <21 || >=22"), Compat::Unclear);
        assert_eq!(check("terraform", "~> 1.9"), Compat::Supported);
        assert_eq!(check("terraform", ">= 1.6, < 2.0"), Compat::Unclear);
    }

    #[test]
    fn unknown_tool_id_is_unclear_not_a_panic() {
        let f = finding(
            ".",
            &ToolVersion {
                tool: "made-up",
                value: "9".into(),
                source: "x".into(),
            },
        );
        assert_eq!(f.compat, Compat::Unclear);
        assert_eq!(f.name, "made-up");
    }

    /// README.md carries the matrix as a table between two marker comments. It must be the
    /// exact rendering of `support.toml`, so nobody has to read Rust to see what is supported.
    #[test]
    fn readme_table_matches_support_toml() {
        let readme = include_str!("../../README.md");
        let start = "<!-- support-matrix:start -->\n";
        let end = "<!-- support-matrix:end -->";
        let (_, rest) = readme
            .split_once(start)
            .expect("README.md has <!-- support-matrix:start -->");
        let (table, _) = rest
            .split_once(end)
            .expect("README.md has <!-- support-matrix:end -->");
        let expected = markdown_table();
        assert!(
            table == expected,
            "README.md's support table is out of date. Replace the block between the markers with:\n\n{expected}"
        );
    }
}
