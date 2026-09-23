//! JavaScript / TypeScript projects: `package.json` scripts with npm, pnpm, Yarn and Bun,
//! including workspaces.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;
use serde_json::Value;
use std::rc::Rc;

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

fn read_pkg(ctx: &Context, dir: &DirInfo) -> Option<Rc<Value>> {
    ctx.json(dir, "package.json")
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
            if let Some(p) = read_pkg(ctx, a) {
                if let Some(pm) = pm_from_field(&p) {
                    return pm;
                }
            }
        }
    }
    Pm::Npm
}

fn script_command(pm: Pm, script: &str) -> String {
    let needs_run = |builtins: &[&str]| builtins.contains(&script);
    match pm {
        Pm::Npm => format!("npm run {script}"),
        Pm::Pnpm => format!("pnpm run {script}"),
        Pm::Yarn => {
            if needs_run(&YARN_BUILTINS_FOR_RUN) {
                format!("yarn run {script}")
            } else {
                format!("yarn {script}")
            }
        }
        Pm::Bun => format!("bun run {script}"),
    }
}

/// Yarn subcommands a script name would collide with, so `yarn run <script>` is needed.
const YARN_BUILTINS_FOR_RUN: [&str; 28] = [
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
];

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
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let Some(pkg) = read_pkg(ctx, base) else {
            return out;
        };
        let pm = package_manager(ctx, base, &pkg);
        let tool: &'static str = pm.name();
        // Only this project's own `packageManager` pin counts as something IT declares
        // (an inherited one belongs to the workspace root project, which reads it itself).
        if let Some(v) = pkg.get("packageManager").and_then(|v| v.as_str()) {
            if let Some((_, ver)) = v.split_once('@') {
                out.version(tool, ver.split('+').next().unwrap_or(ver), "package.json");
            }
        }
        if let Some(node) = pkg
            .get("engines")
            .and_then(|e| e.get("node"))
            .and_then(|v| v.as_str())
        {
            out.version("node", node, "package.json");
        }
        // A workspace member's scripts are shown the way they are run inside that package
        // (`pnpm dev` in apps/web), which is how a per-project listing reads. The
        // root-with-filter form (`pnpm --filter web dev`) is equivalent and longer.

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

        let dep_names: Vec<String> = ["dependencies", "devDependencies"]
            .iter()
            .filter_map(|k| pkg.get(k).and_then(|d| d.as_object()))
            .flat_map(|o| o.keys().cloned())
            .collect();
        let dep = |n: &str| dep_names.iter().any(|d| d == n);
        let script_names: Vec<String> = pkg
            .get("scripts")
            .and_then(|s| s.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        let free = |n: &str| !script_names.iter().any(|s| s == n);
        let runner = match pm {
            Pm::Npm => "npx",
            Pm::Pnpm => "pnpm exec",
            Pm::Yarn => "yarn",
            Pm::Bun => "bunx",
        };
        // A hoisted `expo` dependency in a workspace root is not an app; an Expo app has an
        // app config next to its package.json.
        let app_config =
            base.has("app.json") || base.files.iter().any(|f| f.starts_with("app.config."));
        if dep("expo") && app_config {
            let x = |c: &str| format!("{runner} expo {c}");
            if free("start") && free("dev") {
                out.actions.push(
                    Action::new("start", x("start"))
                        .tool("expo")
                        .inferred(Confidence::High)
                        .cat(Category::Development),
                );
            }
            if free("ios") {
                out.actions.push(
                    Action::new("ios", x("run:ios"))
                        .tool("expo")
                        .inferred(Confidence::High)
                        .cat(Category::Development),
                );
            }
            if free("android") {
                out.actions.push(
                    Action::new("android", x("run:android"))
                        .tool("expo")
                        .inferred(Confidence::High)
                        .cat(Category::Development),
                );
            }
            if free("web") && dep("react-native-web") {
                out.actions.push(
                    Action::new("web", x("start --web"))
                        .tool("expo")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Start the Expo app in the browser")
                        .cat(Category::Development),
                );
            }
            out.actions.push(
                Action::new("prebuild", x("prebuild"))
                    .tool("expo")
                    .inferred(Confidence::Low)
                    .cat(Category::Build),
            );
            out.actions.push(
                Action::new("doctor", format!("{runner} expo-doctor"))
                    .tool("expo")
                    .inferred(Confidence::Low)
                    .inferred_desc("Check the Expo project for common issues")
                    .cat(Category::Quality),
            );
            if base.has("eas.json") {
                let e = |c: &str| format!("{runner} eas {c}");
                out.actions.push(
                    Action::new("eas:build", e("build"))
                        .tool("eas")
                        .inferred(Confidence::High)
                        .inferred_desc("Build the app with EAS Build")
                        .cat(Category::Build)
                        .risk(Risk::External),
                );
                out.actions.push(
                    Action::new("eas:build:local", e("build --local"))
                        .tool("eas")
                        .inferred(Confidence::Low)
                        .inferred_desc("Build the app locally with EAS")
                        .cat(Category::Build),
                );
                out.actions.push(
                    Action::new("eas:update", e("update"))
                        .tool("eas")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Publish an over-the-air update with EAS")
                        .cat(Category::Release)
                        .risk(Risk::External),
                );
                out.actions.push(
                    Action::new("eas:submit", e("submit"))
                        .tool("eas")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Submit the app to the app stores")
                        .cat(Category::Release)
                        .risk(Risk::External),
                );
            }
        } else if dep("react-native")
            && (app_config || base.has_dir("ios") || base.has_dir("android"))
        {
            let rn = |c: &str| format!("{runner} react-native {c}");
            if free("start") {
                out.actions.push(
                    Action::new("start", rn("start"))
                        .tool("react-native")
                        .inferred(Confidence::High)
                        .cat(Category::Development),
                );
            }
            if free("ios") && base.has_dir("ios") {
                out.actions.push(
                    Action::new("ios", rn("run-ios"))
                        .tool("react-native")
                        .inferred(Confidence::High)
                        .cat(Category::Development),
                );
            }
            if free("android") && base.has_dir("android") {
                out.actions.push(
                    Action::new("android", rn("run-android"))
                        .tool("react-native")
                        .inferred(Confidence::High)
                        .cat(Category::Development),
                );
            }
            if base.has_dir("ios") && base.path.join("ios/Podfile").is_file() {
                out.actions.push(
                    Action::new("pods", "pod install --project-directory=ios")
                        .tool("cocoapods")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Install iOS CocoaPods dependencies")
                        .cat(Category::Other),
                );
            }
        }
        if (dep("detox")
            || base.has_any(&[
                ".detoxrc.js",
                ".detoxrc.json",
                ".detoxrc.ts",
                "detox.config.js",
            ]))
            && free("test:e2e")
            && free("e2e")
        {
            out.actions.push(
                Action::new("test:e2e", format!("{runner} detox test"))
                    .tool("detox")
                    .inferred(Confidence::High)
                    .cat(Category::Testing),
            );
        }
        if dep("@playwright/test")
            && free("test:e2e")
            && free("e2e")
            && base
                .files
                .iter()
                .any(|f| f.starts_with("playwright.config"))
        {
            out.actions.push(
                Action::new("test:e2e", format!("{runner} playwright test"))
                    .tool("playwright")
                    .inferred(Confidence::High)
                    .cat(Category::Testing),
            );
        }
        if dep("cypress")
            && free("test:e2e")
            && free("e2e")
            && free("cypress")
            && base.files.iter().any(|f| f.starts_with("cypress.config"))
        {
            out.actions.push(
                Action::new("test:e2e", format!("{runner} cypress run"))
                    .tool("cypress")
                    .inferred(Confidence::High)
                    .cat(Category::Testing),
            );
        }
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
            let wire: Option<String> = if body.trim() == "wireit" {
                pkg.get("wireit")
                    .and_then(|w| w.get(name))
                    .and_then(|w| w.get("command"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            };
            let body: &str = wire.as_deref().unwrap_or(body);
            out.script("js", name, body);
            let command = script_command(pm, name);
            let mut a = Action::new(name, command).tool(tool).raw(body);
            if let Some(d) = descriptions(name) {
                a = a.desc(d);
            }
            // Lifecycle hooks and `_private` / `.private` scripts are not meant to be typed.
            if is_lifecycle(name, &names) || name.starts_with('_') || name.starts_with('.') {
                a = a.hidden();
            }
            out.actions.push(a);
        }
        out
    }
}
