//! Rust / Cargo: conventional actions, workspace members and `.cargo/config.toml` aliases.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::{glob_match, DirInfo};
use std::rc::Rc;
use toml::Value;

pub struct Cargo;

fn manifest(ctx: &Context, dir: &DirInfo) -> Option<Rc<Value>> {
    ctx.toml(dir, "Cargo.toml")
}

fn read_aliases(ctx: &Context, dir: &DirInfo) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(cfg_dir) = ctx.child(dir, ".cargo") else {
        return out;
    };
    for f in ["config.toml", "config"] {
        let Some(v) = ctx.toml(cfg_dir, f) else {
            continue;
        };
        if let Some(Value::Table(al)) = v.get("alias") {
            for (k, v) in al {
                let body = match v {
                    Value::String(s) => s.clone(),
                    Value::Array(a) => a
                        .iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    _ => continue,
                };
                out.push((k.clone(), body));
            }
        }
        break;
    }
    out
}

/// Workspace root and whether `dir` is a member of it.
fn workspace_root<'a>(ctx: &Context<'a>, dir: &DirInfo) -> Option<&'a DirInfo> {
    for a in ctx.ancestors(dir).into_iter().skip(1) {
        let Some(m) = manifest(ctx, a) else { continue };
        let Some(ws) = m.get("workspace") else {
            continue;
        };
        let rel = dir.rel_to(a, "");
        let members: Vec<String> = ws
            .get("members")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let excluded: Vec<String> = ws
            .get("exclude")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        if excluded.iter().any(|e| glob_match(e, &rel)) {
            continue;
        }
        if members.iter().any(|g| glob_match(g, &rel)) {
            return Some(a);
        }
    }
    None
}

impl Discoverer for Cargo {
    fn id(&self) -> &'static str {
        "cargo"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Rust
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has("Cargo.toml")
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let Some(m) = manifest(ctx, base) else {
            return out;
        };
        let pkg = m.get("package");
        let name = pkg
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());
        let is_ws_root = m.get("workspace").is_some();
        let virtual_ws = is_ws_root && pkg.is_none();
        let member_of = workspace_root(ctx, base);

        // `edition` may be set directly, inherited from this manifest's own
        // `[workspace.package]`, or (`edition.workspace = true`) from the workspace root's.
        let ws_edition = |m: &Value| {
            m.get("workspace")
                .and_then(|w| w.get("package"))
                .and_then(|p| p.get("edition"))
                .and_then(|e| e.as_str())
                .map(|s| s.to_string())
        };
        let own = pkg.and_then(|p| p.get("edition"));
        let inherits = own
            .and_then(|e| e.get("workspace"))
            .and_then(|w| w.as_bool())
            .unwrap_or(false);
        let edition: Option<(String, String)> = if inherits {
            member_of.and_then(|root| {
                let e = ws_edition(manifest(ctx, root)?.as_ref())?;
                let src = if root.rel == "." {
                    "Cargo.toml".to_string()
                } else {
                    format!("{}/Cargo.toml", root.rel)
                };
                Some((e, src))
            })
        } else {
            own.and_then(|e| e.as_str())
                .map(|s| s.to_string())
                .or_else(|| ws_edition(&m))
                .map(|e| (e, "Cargo.toml".to_string()))
        };
        if let Some((e, src)) = edition {
            out.version("cargo-edition", e, src);
        }

        // How to address this crate.
        let (sel, cwd): (String, Option<String>) = match (&member_of, &name) {
            (Some(root), Some(n)) => (format!(" -p {n}"), Some(root.rel.clone())),
            _ if is_ws_root && !virtual_ws => (String::new(), None),
            _ if virtual_ws => (" --workspace".to_string(), None),
            _ => (String::new(), None),
        };
        let cmd = |sub: &str| format!("cargo {sub}{sel}");
        let mk = |a: Action| -> Action {
            match &cwd {
                Some(c) => a.cwd(c.clone()),
                None => a,
            }
        };

        // declared aliases
        for (alias, body) in read_aliases(ctx, base) {
            out.script("cargo", &alias, &format!("cargo {body}"));
            out.actions.push(
                Action::new(&alias, format!("cargo {alias}"))
                    .tool("cargo")
                    .raw(format!("cargo {body}")),
            );
        }

        let has_bin = ctx.has_file(base, "src/main.rs")
            || m.get("bin").is_some()
            || ctx.has_dir(base, "src/bin");
        let has_bench = m.get("bench").is_some() || base.has_dir("benches");
        let scope = if virtual_ws {
            "the workspace"
        } else {
            "the crate"
        };

        if has_bin && !virtual_ws {
            out.actions.push(mk(Action::new("run", cmd("run"))
                .tool("cargo")
                .inferred(Confidence::High)
                .cat(Category::Development)));
        }
        out.actions.push(mk(Action::new("build", cmd("build"))
            .tool("cargo")
            .inferred(Confidence::High)
            .inferred_desc(format!("Build {scope}"))
            .cat(Category::Build)));
        out.actions.push(mk(Action::new("test", cmd("test"))
            .tool("cargo")
            .inferred(Confidence::High)));
        out.actions.push(mk(Action::new("check", cmd("check"))
            .tool("cargo")
            .inferred(Confidence::High)));
        out.actions.push(mk(Action::new("clippy", cmd("clippy"))
            .tool("cargo")
            .inferred(Confidence::Medium)));
        out.actions.push(mk(Action::new("fmt", cmd("fmt"))
            .tool("cargo")
            .inferred(Confidence::Medium)));
        if has_bench {
            out.actions.push(mk(Action::new("bench", cmd("bench"))
                .tool("cargo")
                .inferred(Confidence::High)));
        } else {
            out.actions.push(mk(Action::new("bench", cmd("bench"))
                .tool("cargo")
                .inferred(Confidence::Low)));
        }
        out.actions
            .push(mk(Action::new("doc", format!("cargo doc --no-deps{sel}"))
                .tool("cargo")
                .inferred(Confidence::Low)));
        if let Some(Value::Table(deps)) = m.get("dependencies") {
            if deps.contains_key("sqlx") {
                out.actions.push(mk(Action::new(
                    "migrate",
                    "cargo sqlx migrate run".to_string(),
                )
                .tool("cargo")
                .inferred(Confidence::Medium)));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_inherits_workspace_edition() {
        let (root, dirs) = super::super::fixture_dirs("cargo-workspace");
        let ctx = Context::new(&root, &dirs, "linux");
        let core = ctx.dir_at("crates/core").unwrap();
        let d = Cargo.discover(&ctx, core, &[core]);
        let v = d
            .versions
            .iter()
            .find(|v| v.tool == "cargo-edition")
            .unwrap();
        assert_eq!(
            (v.value.as_str(), v.source.as_str()),
            ("2021", "Cargo.toml")
        );
    }
}
