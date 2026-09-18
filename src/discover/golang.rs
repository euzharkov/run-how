//! Go modules and workspaces: conventional inferred actions, `go run` targets, and `generate`
//! only when `//go:generate` directives exist.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Go;

fn module_name(dir: &DirInfo) -> Option<String> {
    let text = dir.read("go.mod")?;
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("module "))?;
    let m = line
        .trim()
        .trim_start_matches("module ")
        .trim()
        .trim_matches('"');
    Some(m.rsplit('/').next().unwrap_or(m).to_string())
}

fn is_main_package(path: &std::path::Path, files: &[String]) -> bool {
    for f in files
        .iter()
        .filter(|f| f.ends_with(".go") && !f.ends_with("_test.go"))
    {
        if let Some(text) = crate::repo::read_text(&path.join(f)) {
            if text.lines().any(|l| l.trim() == "package main") {
                return true;
            }
        }
    }
    false
}

impl Discoverer for Go {
    fn id(&self) -> &'static str {
        "go"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Go
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has("go.mod") || dir.has("go.work")
    }
    fn project_name(&self, dir: &DirInfo) -> Option<String> {
        module_name(dir)
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let workspace = base.has("go.work") && !base.has("go.mod");
        let name = module_name(base).unwrap_or_else(|| base.name().to_string());
        let scope = if workspace {
            "the Go workspace"
        } else {
            "Go packages"
        };

        out.actions.push(
            Action::new("build", "go build ./...")
                .tool("go")
                .inferred(Confidence::High)
                .inferred_desc(format!("Build {scope}"))
                .cat(Category::Build),
        );
        out.actions.push(
            Action::new("test", "go test ./...")
                .tool("go")
                .inferred(Confidence::High),
        );
        out.actions.push(
            Action::new("vet", "go vet ./...")
                .tool("go")
                .inferred(Confidence::High),
        );
        out.actions.push(
            Action::new("fmt", "go fmt ./...")
                .tool("go")
                .inferred(Confidence::High),
        );
        if base.has_any(&[
            ".golangci.yml",
            ".golangci.yaml",
            ".golangci.toml",
            ".golangci.json",
        ]) {
            out.actions.push(
                Action::new("lint", "golangci-lint run")
                    .tool("go")
                    .inferred(Confidence::High),
            );
        }
        out.actions.push(
            Action::new("tidy", "go mod tidy")
                .tool("go")
                .inferred(Confidence::Low),
        );

        if workspace {
            return out;
        }
        // run targets
        let subtree = ctx.descendants(base);
        if is_main_package(&base.path, &base.files) {
            out.actions.push(
                Action::new("run", "go run .")
                    .tool("go")
                    .inferred(Confidence::High)
                    .inferred_desc(format!("Run the Go {name} program"))
                    .cat(Category::Development),
            );
        } else {
            let mut cmds: Vec<&DirInfo> = subtree
                .iter()
                .copied()
                .filter(|d| {
                    d.rel_to(base, "").starts_with("cmd/")
                        && d.rel_to(base, "").matches('/').count() == 1
                })
                .filter(|d| is_main_package(&d.path, &d.files))
                .collect();
            cmds.sort_by(|a, b| a.rel.cmp(&b.rel));
            let single = cmds.len() == 1;
            for d in cmds {
                let rel = d.rel_to(base, "");
                let bin = d.name();
                let id = if single {
                    "run".to_string()
                } else {
                    format!("run:{bin}")
                };
                out.actions.push(
                    Action::new(id, format!("go run ./{rel}"))
                        .tool("go")
                        .inferred(Confidence::High)
                        .inferred_desc(format!("Run the {bin} command"))
                        .cat(Category::Development),
                );
            }
        }
        // generate only when directives exist (nested modules are skipped: they have their own go.mod)
        let mut has_generate = false;
        let mut scanned = 0usize;
        'outer: for d in std::iter::once(base).chain(subtree.iter().copied()) {
            if d.rel != base.rel && d.has("go.mod") {
                continue;
            }
            for f in d.files.iter().filter(|f| f.ends_with(".go")) {
                scanned += 1;
                if scanned > 4000 {
                    break 'outer;
                }
                if let Some(text) = crate::repo::read_text(&d.path.join(f)) {
                    if text.contains("//go:generate") {
                        has_generate = true;
                        break 'outer;
                    }
                }
            }
        }
        if has_generate {
            out.actions.push(
                Action::new("generate", "go generate ./...")
                    .tool("go")
                    .inferred(Confidence::High),
            );
        }
        if base.has(".goreleaser.yaml") || base.has(".goreleaser.yml") {
            out.actions.push(
                Action::new("release:snapshot", "goreleaser release --snapshot --clean")
                    .tool("go")
                    .inferred(Confidence::Medium)
                    .cat(Category::Release),
            );
        }
        out
    }
}
