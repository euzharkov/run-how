//! Bazel and Pants build systems.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Bazel;

impl Discoverer for Bazel {
    fn id(&self) -> &'static str {
        "bazel"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Bazel
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has_any(&[
            "MODULE.bazel",
            "WORKSPACE",
            "WORKSPACE.bazel",
            "WORKSPACE.bzlmod",
            "pants.toml",
        ])
    }
    fn discover(&self, _ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        if base.has_any(&[
            "MODULE.bazel",
            "WORKSPACE",
            "WORKSPACE.bazel",
            "WORKSPACE.bzlmod",
        ]) {
            let bin = if base.has(".bazelversion") {
                "bazelisk"
            } else {
                "bazel"
            };
            let b = |s: &str| format!("{bin} {s}");
            out.actions.push(
                Action::new("build", b("build //..."))
                    .tool("bazel")
                    .inferred(Confidence::High)
                    .inferred_desc("Build all Bazel targets")
                    .cat(Category::Build),
            );
            out.actions.push(
                Action::new("test", b("test //..."))
                    .tool("bazel")
                    .inferred(Confidence::High)
                    .inferred_desc("Run all Bazel tests")
                    .cat(Category::Testing),
            );
            out.actions.push(
                Action::new("query", b("query //..."))
                    .tool("bazel")
                    .inferred(Confidence::Low)
                    .inferred_desc("List all Bazel targets")
                    .cat(Category::Other),
            );
            out.actions.push(
                Action::new("clean", b("clean"))
                    .tool("bazel")
                    .inferred(Confidence::Low)
                    .inferred_desc("Delete Bazel outputs")
                    .cat(Category::Build),
            );
            if base.has(".buildifier.json") || base.has("BUILD.bazel") || base.has("BUILD") {
                out.actions.push(
                    Action::new("format:build", "buildifier -r .".to_string())
                        .tool("bazel")
                        .inferred(Confidence::Low)
                        .inferred_desc("Format BUILD and .bzl files")
                        .cat(Category::Quality),
                );
            }
        }
        if base.has("pants.toml") {
            let p = |s: &str| format!("pants {s}");
            out.actions.push(
                Action::new("test", p("test ::"))
                    .tool("pants")
                    .inferred(Confidence::High)
                    .inferred_desc("Run all tests with Pants")
                    .cat(Category::Testing),
            );
            out.actions.push(
                Action::new("lint", p("lint ::"))
                    .tool("pants")
                    .inferred(Confidence::High)
                    .inferred_desc("Lint all targets with Pants")
                    .cat(Category::Quality),
            );
            out.actions.push(
                Action::new("fmt", p("fmt ::"))
                    .tool("pants")
                    .inferred(Confidence::High)
                    .inferred_desc("Format all targets with Pants")
                    .cat(Category::Quality),
            );
            out.actions.push(
                Action::new("check", p("check ::"))
                    .tool("pants")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Type-check all targets with Pants")
                    .cat(Category::Quality),
            );
            out.actions.push(
                Action::new("package", p("package ::"))
                    .tool("pants")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Build all packages with Pants")
                    .cat(Category::Build),
            );
        }
        out
    }
}
