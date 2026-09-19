//! JavaScript / TypeScript projects: `package.json` scripts with npm, pnpm, Yarn and Bun,
//! including workspaces.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::{glob_match, DirInfo};
use serde_json::Value;

pub struct Js;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pm {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl Pm {
    fn name(self) -> &'static str {
        match self {
            Pm::Npm => "npm",
            Pm::Pnpm => "pnpm",
            Pm::Yarn => "yarn",
            Pm::Bun => "bun",
        }
    }
}

const LIFECYCLE: &[&str] = &[
    "prepare",
    "prepublish",
    "prepublishOnly",
    "prepack",
    "postpack",
    "publish",
    "postpublish",
    "preinstall",
    "install",
    "postinstall",
    "preuninstall",
    "uninstall",
    "postuninstall",
    "preversion",
    "version",
    "postversion",
    "dependencies",
    "prerestart",
    "postrestart",
];

fn read_pkg(dir: &DirInfo) -> Option<Value> {
    serde_json::from_str(&dir.read("package.json")?).ok()
}

fn pm_from_field(pkg: &Value) -> Option<Pm> {
    let f = pkg.get("packageManager")?.as_str()?;
    let name = f.split('@').next()?;
    Some(match name {
        "npm" => Pm::Npm,
        "pnpm" => Pm::Pnpm,
        "yarn" => Pm::Yarn,
        "bun" => Pm::Bun,
        _ => return None,
    })
}

fn pm_from_lock(dir: &DirInfo) -> Option<Pm> {
    if dir.has_any(&["bun.lock", "bun.lockb"]) {
        Some(Pm::Bun)
    } else if dir.has("pnpm-lock.yaml") || dir.has("pnpm-workspace.yaml") {
        Some(Pm::Pnpm)
    } else if dir.has("yarn.lock") || dir.has(".yarnrc.yml") || dir.has(".yarnrc") {
        Some(Pm::Yarn)
    } else if dir.has_any(&["package-lock.json", "npm-shrinkwrap.json"]) {
        Some(Pm::Npm)
    } else {
        None
    }
}

/// Determine the package manager for `dir`: `packageManager` field, then lockfiles here and in
/// ancestors, then npm.
pub fn package_manager(ctx: &Context, dir: &DirInfo, pkg: &Value) -> Pm {
    if let Some(pm) = pm_from_field(pkg) {
        return pm;
    }
    for a in ctx.ancestors(dir) {
        if let Some(pm) = pm_from_lock(a) {
            return pm;
        }
        if a.rel != dir.rel {
            if let Some(p) = read_pkg(a) {
                if let Some(pm) = pm_from_field(&p) {
                    return pm;
                }
            }
        }
    }
    Pm::Npm
}

fn workspace_globs(dir: &DirInfo) -> Vec<String> {
    let mut globs = Vec::new();
    if let Some(text) = dir.read("pnpm-workspace.yaml") {
        if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(&text) {
            if let Some(seq) = v.get("packages").and_then(|p| p.as_sequence()) {
                globs.extend(seq.iter().filter_map(|s| s.as_str()).map(|s| s.to_string()));
            }
        }
    }
    if let Some(pkg) = read_pkg(dir) {
        let ws = pkg.get("workspaces");
        let arr = match ws {
            Some(Value::Array(a)) => Some(a.clone()),
            Some(Value::Object(o)) => o.get("packages").and_then(|p| p.as_array()).cloned(),
            _ => None,
        };
        if let Some(a) = arr {
            globs.extend(a.iter().filter_map(|s| s.as_str()).map(|s| s.to_string()));
        }
    }
    globs
}

pub struct Workspace<'a> {
    pub root: &'a DirInfo,
}

/// If `dir` is a member of an ancestor workspace, return that workspace root.
pub fn workspace_of<'a>(ctx: &Context<'a>, dir: &DirInfo) -> Option<Workspace<'a>> {
    for a in ctx.ancestors(dir).into_iter().skip(1) {
        let globs = workspace_globs(a);
        if globs.is_empty() {
            continue;
        }
        let rel = dir.rel_to(a, "");
        let negated: Vec<&str> = globs.iter().filter_map(|g| g.strip_prefix('!')).collect();
        if negated.iter().any(|g| glob_match(g, &rel)) {
            continue;
        }
        if globs
            .iter()
            .filter(|g| !g.starts_with('!'))
            .any(|g| glob_match(g, &rel))
        {
            return Some(Workspace { root: a });
        }
    }
    None
}

fn script_command(pm: Pm, script: &str, ws: Option<(&str, &str)>) -> String {
    // ws = (workspace member name or path, path)
    let needs_run = |builtins: &[&str]| builtins.contains(&script);
    match (pm, ws) {
        (Pm::Npm, None) => format!("npm run {script}"),
        (Pm::Npm, Some((name, _))) => format!("npm run {script} --workspace={name}"),
        (Pm::Pnpm, None) => format!("pnpm run {script}"),
        (Pm::Pnpm, Some((name, _))) => {
            if needs_run(&[
                "install", "add", "remove", "update", "publish", "test", "start", "run", "exec",
                "dlx", "list", "why", "pack", "link", "audit", "outdated", "prune", "store",
                "deploy", "env",
            ]) {
                format!("pnpm --filter {name} run {script}")
            } else {
                format!("pnpm --filter {name} {script}")
            }
        }
        (Pm::Yarn, None) => {
            if needs_run(&[
                "add",
                "install",
                "remove",
                "upgrade",
                "up",
                "why",
                "info",
                "init",
                "link",
                "unlink",
                "pack",
                "publish",
                "version",
                "cache",
                "config",
                "set",
                "dlx",
                "exec",
                "node",
                "npm",
                "workspace",
                "workspaces",
                "run",
                "bin",
                "audit",
                "outdated",
                "list",
                "global",
                "create",
            ]) {
                format!("yarn run {script}")
            } else {
                format!("yarn {script}")
            }
        }
        (Pm::Yarn, Some((name, _))) => format!("yarn workspace {name} {script}"),
        (Pm::Bun, None) => format!("bun run {script}"),
        (Pm::Bun, Some((name, _))) => format!("bun run --filter {name} {script}"),
    }
}

fn is_lifecycle(name: &str, scripts: &[&str]) -> bool {
    if LIFECYCLE.contains(&name) {
        return true;
    }
    for pre in ["pre", "post"] {
        if let Some(rest) = name.strip_prefix(pre) {
            if scripts.contains(&rest) || LIFECYCLE.contains(&rest) {
                return true;
            }
        }
    }
    false
}

impl Discoverer for Js {
    fn id(&self) -> &'static str {
        "js"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::JavaScript
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has("package.json")
    }
    fn project_name(&self, dir: &DirInfo) -> Option<String> {
        read_pkg(dir)?.get("name")?.as_str().map(|s| s.to_string())
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let Some(pkg) = read_pkg(base) else {
            return out;
        };
        let pm = package_manager(ctx, base, &pkg);
        let tool: &'static str = pm.name();
        let ws = workspace_of(ctx, base);
        let member_name: Option<String> = ws.as_ref().map(|w| {
            pkg.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    if pm == Pm::Pnpm {
                        format!("./{}", base.rel_to(w.root, ""))
                    } else {
                        base.rel_to(w.root, "")
                    }
                })
        });
        let cwd = ws.as_ref().map(|w| w.root.rel.clone());

        let descriptions = |name: &str| -> Option<String> {
            pkg.get("scripts-info")
                .and_then(|o| o.get(name))
                .and_then(|v| v.as_str())
                .or_else(|| {
                    pkg.get("ntl")
                        .and_then(|n| n.get("descriptions"))
                        .and_then(|o| o.get(name))
                        .and_then(|v| v.as_str())
                })
                .map(|s| s.to_string())
        };

        if base.has_dir(".maestro") || base.has_dir("maestro") {
            let dir = if base.has_dir(".maestro") {
                ".maestro"
            } else {
                "maestro"
            };
            out.actions.push(
                Action::new("test:maestro", format!("maestro test {dir}"))
                    .tool("maestro")
                    .inferred(Confidence::High)
                    .inferred_desc("Run Maestro mobile end-to-end flows")
                    .cat(Category::Testing),
            );
        }
        let Some(scripts) = pkg.get("scripts").and_then(|s| s.as_object()) else {
            return out;
        };
        let names: Vec<&str> = scripts.keys().map(|k| k.as_str()).collect();
        for (name, val) in scripts {
            let Some(body) = val.as_str() else { continue };
            out.script("js", name, body);
            let command = script_command(
                pm,
                name,
                member_name.as_deref().map(|n| (n, base.rel.as_str())),
            );
            let mut a = Action::new(name, command).tool(tool).raw(body);
            if let Some(c) = &cwd {
                a = a.cwd(c.clone());
            }
            if let Some(d) = descriptions(name) {
                a = a.desc(d);
            }
            if is_lifecycle(name, &names) {
                a = a.hidden();
            }
            out.actions.push(a);
        }
        out
    }
}
