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
//! edition, a new Taskfile schema, …), bump that tool's baseline in [`REGISTRY`] below and note
//! it in the changelog. That one-line change is the entire "support matrix".

use crate::model::{Repo, ToolVersion};
use serde::Serialize;

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

impl Compat {
    pub fn label(self) -> &'static str {
        match self {
            Compat::Supported => "supported",
            Compat::Newer => "newer than verified",
            Compat::Unclear => "unclear",
        }
    }
}

pub struct Entry {
    /// Matches a [`ToolVersion::tool`].
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    /// Config file(s)/field(s) this reads to find the declared version.
    pub config: &'static str,
    /// One-line human description of what has been verified.
    pub verified: &'static str,
    /// Compares a declared value against `verified`. `None` when the entry is listed for
    /// visibility but nothing has been wired up to read a declared value for it yet.
    pub compare: Option<fn(&str) -> Compat>,
}

/// Ecosystems `rhow` discovers but that have no single "version" field worth comparing (no
/// config format to go stale against in the way a numbered schema or edition can). Listed for
/// completeness in `rhow support`, not compared.
pub struct Other {
    pub name: &'static str,
    pub category: &'static str,
    pub config: &'static str,
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

fn cargo_edition(v: &str) -> Compat {
    at_most(v, &[2021])
}
fn go_version(v: &str) -> Compat {
    at_most(v, &[1, 23])
}
fn dotnet_tfm(v: &str) -> Compat {
    // "net8.0" / "net9.0" / "netstandard2.1" / "net48" — only the modern "netN.N" shape has a
    // meaningful ordering here; classic .NET Framework (`net48`) and `netstandard*` are Unclear.
    let v = v.trim().to_ascii_lowercase();
    let Some(rest) = v.strip_prefix("net") else {
        return Compat::Unclear;
    };
    if rest.starts_with("standard") || rest.starts_with("coreapp") || !rest.contains('.') {
        return Compat::Unclear;
    }
    at_most(rest, &[9, 0])
}
fn python_requires(v: &str) -> Compat {
    range_at_most(v, &[3, 13])
}
fn taskfile_schema(v: &str) -> Compat {
    at_most(v, &[3])
}
fn node_engines(v: &str) -> Compat {
    range_at_most(v, &[22])
}
fn npm_version(v: &str) -> Compat {
    at_most(v, &[10])
}
fn pnpm_version(v: &str) -> Compat {
    at_most(v, &[9])
}
fn yarn_version(v: &str) -> Compat {
    at_most(v, &[4])
}
fn bun_version(v: &str) -> Compat {
    at_most(v, &[1])
}
fn ruby_version(v: &str) -> Compat {
    at_most(v, &[3, 3])
}
fn php_requires(v: &str) -> Compat {
    range_at_most(v, &[8, 3])
}
fn terraform_required(v: &str) -> Compat {
    range_at_most(v, &[1, 9])
}
fn tofu_required(v: &str) -> Compat {
    range_at_most(v, &[1, 8])
}
fn gradle_version(v: &str) -> Compat {
    at_most(v, &[8, 10])
}
fn bazel_version(v: &str) -> Compat {
    at_most(v, &[7, 4])
}

// ---------------------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------------------

pub const REGISTRY: &[Entry] = &[
    Entry {
        id: "cargo-edition",
        name: "Rust / Cargo",
        category: "Rust",
        config: "Cargo.toml `edition` (own or inherited from `[workspace.package]`)",
        verified: "editions 2015-2021; 2024 not yet verified",
        compare: Some(cargo_edition),
    },
    Entry {
        id: "go",
        name: "Go",
        category: "Go",
        config: "the `go` directive in go.mod / go.work",
        verified: "up to Go 1.23",
        compare: Some(go_version),
    },
    Entry {
        id: "dotnet-tfm",
        name: ".NET",
        category: ".NET",
        config: "`<TargetFramework(s)>` in .csproj/.fsproj/.vbproj",
        verified: "net5.0-net9.0 (net48 and netstandard* recognised but not version-checked)",
        compare: Some(dotnet_tfm),
    },
    Entry {
        id: "python",
        name: "Python",
        category: "Python",
        config: "`project.requires-python` in pyproject.toml",
        verified: "up to Python 3.13 (compound constraints like `>=3.9,<4` are left unclear)",
        compare: Some(python_requires),
    },
    Entry {
        id: "taskfile",
        name: "Taskfile",
        category: "Taskfile",
        config: "`version:` in Taskfile.yml",
        verified: "schema version 3",
        compare: Some(taskfile_schema),
    },
    Entry {
        id: "node",
        name: "Node.js",
        category: "JavaScript",
        config: "`engines.node` in package.json",
        verified: "up to Node 22",
        compare: Some(node_engines),
    },
    Entry {
        id: "npm",
        name: "npm",
        category: "JavaScript",
        config: "the `packageManager` field in package.json",
        verified: "up to npm 10",
        compare: Some(npm_version),
    },
    Entry {
        id: "pnpm",
        name: "pnpm",
        category: "JavaScript",
        config: "the `packageManager` field in package.json, pnpm-workspace.yaml",
        verified: "up to pnpm 9",
        compare: Some(pnpm_version),
    },
    Entry {
        id: "yarn",
        name: "Yarn",
        category: "JavaScript",
        config: "the `packageManager` field in package.json",
        verified: "up to Yarn 4 (Berry)",
        compare: Some(yarn_version),
    },
    Entry {
        id: "bun",
        name: "Bun",
        category: "JavaScript",
        config: "the `packageManager` field in package.json, bun.lock(b)",
        verified: "up to Bun 1",
        compare: Some(bun_version),
    },
    Entry {
        id: "ruby",
        name: "Ruby",
        category: "Ruby",
        config: ".ruby-version",
        verified: "up to Ruby 3.3",
        compare: Some(ruby_version),
    },
    Entry {
        id: "php",
        name: "PHP",
        category: "PHP",
        config: "`require.php` in composer.json",
        verified: "up to PHP 8.3",
        compare: Some(php_requires),
    },
    Entry {
        id: "terraform",
        name: "Terraform",
        category: "Infrastructure",
        config: "`required_version` in a `terraform {}` block",
        verified: "up to Terraform 1.9",
        compare: Some(terraform_required),
    },
    Entry {
        id: "tofu",
        name: "OpenTofu",
        category: "Infrastructure",
        config: "`required_version` in a `terraform {}` block",
        verified: "up to OpenTofu 1.8",
        compare: Some(tofu_required),
    },
    Entry {
        id: "gradle",
        name: "Gradle",
        category: "JVM",
        config: "gradle/wrapper/gradle-wrapper.properties `distributionUrl`",
        verified: "up to Gradle 8.10",
        compare: Some(gradle_version),
    },
    Entry {
        id: "bazel",
        name: "Bazel",
        category: "Bazel",
        config: ".bazelversion",
        verified: "up to Bazel 7.4",
        compare: Some(bazel_version),
    },
];

/// Ecosystems with no single version worth comparing, listed in `rhow support` for completeness.
pub const OTHER: &[Other] = &[
    Other {
        name: "Make",
        category: "Make",
        config: "Makefile / GNUmakefile",
    },
    Other {
        name: "Just",
        category: "Just",
        config: "justfile",
    },
    Other {
        name: "Docker / Compose",
        category: "Infrastructure",
        config: "Dockerfile, compose.yml",
    },
    Other {
        name: "Kubernetes",
        category: "Infrastructure",
        config: "Helm charts, Kustomize, plain manifests",
    },
    Other {
        name: "Pulumi",
        category: "Infrastructure",
        config: "Pulumi.yaml",
    },
    Other {
        name: "Ansible",
        category: "Infrastructure",
        config: "ansible.cfg, playbooks",
    },
    Other {
        name: "Nx / Turborepo / Rush / moon",
        category: "JavaScript",
        config: "nx.json, turbo.json, rush.json, moon.yml",
    },
    Other {
        name: "Deno",
        category: "Deno",
        config: "deno.json(c)",
    },
    Other {
        name: "Maven",
        category: "JVM",
        config: "pom.xml",
    },
    Other {
        name: "Swift / Xcode",
        category: "Mobile",
        config: "Package.swift, .xcodeproj/.xcworkspace",
    },
    Other {
        name: "Fastlane",
        category: "Mobile",
        config: "fastlane/Fastfile",
    },
    Other {
        name: "Flutter / Dart",
        category: "Mobile",
        config: "pubspec.yaml",
    },
    Other {
        name: "Expo / React Native",
        category: "Mobile",
        config: "package.json dependencies",
    },
    Other {
        name: "Elixir / Mix",
        category: "Other",
        config: "mix.exs",
    },
    Other {
        name: "Zig",
        category: "Other",
        config: "build.zig",
    },
    Other {
        name: "CMake",
        category: "Other",
        config: "CMakeLists.txt",
    },
    Other {
        name: "Haskell",
        category: "Other",
        config: "stack.yaml, cabal.project",
    },
    Other {
        name: "Scala / sbt",
        category: "Other",
        config: "build.sbt",
    },
    Other {
        name: "Clojure",
        category: "Other",
        config: "deps.edn, project.clj",
    },
    Other {
        name: "Pants",
        category: "Bazel",
        config: "pants.toml",
    },
];

// ---------------------------------------------------------------------------------------
// Checking a discovered repository
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Project path, `.` for the root.
    pub project: String,
    pub tool: &'static str,
    pub name: &'static str,
    pub value: String,
    pub source: String,
    pub compat: Compat,
}

/// Compare every version a discovered repository declares against [`REGISTRY`].
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
    let entry = REGISTRY.iter().find(|e| e.id == v.tool);
    let (name, compat) = match entry {
        Some(e) => (
            e.name,
            e.compare.map(|f| f(&v.value)).unwrap_or(Compat::Unclear),
        ),
        None => (v.tool, Compat::Unclear),
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

    #[test]
    fn registry_ids_are_unique_and_wired() {
        let mut ids: Vec<&str> = REGISTRY.iter().map(|e| e.id).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(n, ids.len(), "duplicate ids in REGISTRY");
        assert!(
            REGISTRY.iter().all(|e| e.compare.is_some()),
            "every REGISTRY entry should be checkable; list unwired ecosystems in OTHER instead"
        );
    }

    #[test]
    fn cargo_edition_baseline() {
        assert_eq!(cargo_edition("2015"), Compat::Supported);
        assert_eq!(cargo_edition("2021"), Compat::Supported);
        assert_eq!(cargo_edition("2024"), Compat::Newer);
    }

    #[test]
    fn go_version_baseline() {
        assert_eq!(go_version("1.22"), Compat::Supported);
        assert_eq!(go_version("1.23"), Compat::Supported);
        assert_eq!(go_version("1.24"), Compat::Newer);
    }

    #[test]
    fn dotnet_tfm_baseline() {
        assert_eq!(dotnet_tfm("net8.0"), Compat::Supported);
        assert_eq!(dotnet_tfm("net9.0"), Compat::Supported);
        assert_eq!(dotnet_tfm("net10.0"), Compat::Newer);
        assert_eq!(dotnet_tfm("net48"), Compat::Unclear);
        assert_eq!(dotnet_tfm("netstandard2.1"), Compat::Unclear);
    }

    #[test]
    fn taskfile_schema_baseline() {
        assert_eq!(taskfile_schema("3"), Compat::Supported);
        assert_eq!(taskfile_schema("4"), Compat::Newer);
    }

    #[test]
    fn range_constraints_prefer_unclear() {
        assert_eq!(python_requires(">=3.13"), Compat::Supported);
        assert_eq!(python_requires(">=3.14"), Compat::Newer);
        assert_eq!(python_requires(">=3.9,<4"), Compat::Unclear);
        assert_eq!(node_engines(">=18"), Compat::Supported);
        assert_eq!(node_engines(">=18 <21 || >=22"), Compat::Unclear);
        assert_eq!(terraform_required("~> 1.9"), Compat::Supported);
        assert_eq!(terraform_required(">= 1.6, < 2.0"), Compat::Unclear);
    }

    #[test]
    fn gradle_and_bazel_baselines() {
        assert_eq!(gradle_version("8.10"), Compat::Supported);
        assert_eq!(gradle_version("8.11"), Compat::Newer);
        assert_eq!(bazel_version("7.4.0"), Compat::Supported);
        assert_eq!(bazel_version("7.5.0"), Compat::Newer);
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
}
