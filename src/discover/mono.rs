//! Monorepo orchestrators as project graphs: Nx, Turborepo, Rush, moon.
//!
//! Package managers already expose per-package scripts; these adapters expose the orchestrator's
//! own targets so `rhow` shows what the repository actually runs.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;
use serde_json::Value;

pub struct Mono;

fn strip_jsonc(text: &str) -> String {
    // Remove // and /* */ comments outside strings; tolerate trailing commas by leaving them
    // (serde_json rejects them, so also strip `,` before `}` / `]`).
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut in_str = false;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                out.push(c);
                i += 1;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if chars.get(i + 1) == Some(&'*') => {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    let mut cleaned = String::with_capacity(out.len());
    let oc: Vec<char> = out.chars().collect();
    for (i, c) in oc.iter().enumerate() {
        if *c == ',' {
            let next = oc[i + 1..].iter().find(|x| !x.is_whitespace());
            if matches!(next, Some('}') | Some(']')) {
                continue;
            }
        }
        cleaned.push(*c);
    }
    cleaned
}

pub fn read_jsonc(dir: &DirInfo, file: &str) -> Option<Value> {
    serde_json::from_str(&strip_jsonc(&dir.read(file)?)).ok()
}

/// One Nx target after merging the project's definition over `nx.json` `targetDefaults`.
struct NxTarget {
    name: String,
    description: Option<String>,
    /// `Some("nx-executor:<executor>")` or the run-commands text; `None` when nothing is known.
    cmd: Option<String>,
    /// The project defined nothing of its own (`"lint": {}` or options only): the target is
    /// the workspace default applied to this project.
    inherited: bool,
}

fn command_text(cfg: &Value) -> Option<String> {
    let opts = cfg.get("options")?;
    if let Some(c) = opts.get("command").and_then(|c| c.as_str()) {
        return Some(c.to_string());
    }
    // `nx:run-commands` runs its `commands` in sequence by default; take all of them so the
    // description and risk reflect the whole target. Comment-only entries are skipped, since
    // a leading `#` would otherwise end the shell tokenizer's view of everything after it.
    let arr = opts.get("commands")?.as_array()?;
    let parts: Vec<String> = arr
        .iter()
        .filter_map(|item| {
            item.as_str().map(|s| s.to_string()).or_else(|| {
                item.get("command")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
            })
        })
        .filter(|c| !c.trim_start().starts_with('#') && !c.trim().is_empty())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" && "))
    }
}

fn nx_targets(dir: &DirInfo, defaults: Option<&Value>) -> Vec<NxTarget> {
    let mut out = Vec::new();
    let Some(v) = read_jsonc(dir, "project.json") else {
        return out;
    };
    let Some(t) = v.get("targets").and_then(|t| t.as_object()) else {
        return out;
    };
    for (name, cfg) in t {
        let default = defaults.and_then(|d| d.get(name));
        let description = cfg
            .get("metadata")
            .and_then(|m| m.get("description"))
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());
        let own_executor = cfg.get("executor").and_then(|e| e.as_str());
        let own_cmd = command_text(cfg);
        let inherited = own_executor.is_none() && own_cmd.is_none();
        let executor = own_executor.or_else(|| {
            default
                .and_then(|d| d.get("executor"))
                .and_then(|e| e.as_str())
        });
        let cmd = own_cmd
            .or_else(|| default.and_then(command_text))
            .or_else(|| executor.map(|e| format!("nx-executor:{e}")));
        out.push(NxTarget {
            name: name.clone(),
            description,
            cmd,
            inherited,
        });
    }
    out
}

/// Targets a developer reaches for by hand. Anything else that a project merely inherits
/// from the workspace defaults is pipeline plumbing, shown once at the root and hidden here.
const CORE_TARGETS: &[&str] = &[
    "build",
    "test",
    "lint",
    "typecheck",
    "type-check",
    "format",
    "e2e",
    "serve",
    "start",
    "dev",
    "clean",
    "check",
];

fn nx_project_name(dir: &DirInfo) -> Option<String> {
    read_jsonc(dir, "project.json")?
        .get("name")?
        .as_str()
        .map(|s| s.to_string())
}

impl Discoverer for Mono {
    fn id(&self) -> &'static str {
        "mono"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Workspace
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has("nx.json")
            || dir.has("project.json")
            || dir.has("turbo.json")
            || dir.has("rush.json")
            || dir.has("moon.yml")
            || dir.has_dir(".moon")
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let scripts_here: Vec<String> = base
            .read("package.json")
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|p| {
                p.get("scripts")
                    .and_then(|s| s.as_object())
                    .map(|o| o.keys().cloned().collect())
            })
            .unwrap_or_default();
        let free = |n: &str| !scripts_here.iter().any(|s| s == n);

        // ---- Nx -------------------------------------------------------------------------
        if base.has("project.json") {
            let name = nx_project_name(base).unwrap_or_else(|| base.name().to_string());
            let root = ctx
                .ancestors(base)
                .into_iter()
                .find(|a| a.has("nx.json"))
                .unwrap_or(base);
            let defaults =
                read_jsonc(root, "nx.json").and_then(|v| v.get("targetDefaults").cloned());
            for t in nx_targets(base, defaults.as_ref()) {
                let target = t.name;
                let mut a = Action::new(&target, format!("nx run {name}:{target}"))
                    .tool("nx")
                    .cwd(root.rel.clone());
                if let Some(d) = t.description {
                    a = a.desc(d);
                }
                match t.cmd {
                    Some(c) if c.starts_with("nx-executor:") => {
                        let ex = c.trim_start_matches("nx-executor:");
                        if ex == "nx:noop" {
                            // A dependency-graph anchor with no work of its own.
                            a = a.hidden();
                        }
                        let opts = nx_options(
                            read_jsonc(base, "project.json").as_ref(),
                            defaults.as_ref(),
                            &target,
                        );
                        let (text, cat, risk) = executor_desc(ex, &target, &opts);
                        a = a.inferred_desc(text).cat(cat).risk(risk);
                    }
                    Some(c) => a = a.raw(c),
                    // Nothing known about it anywhere: keep it, but not in the default view.
                    None => {
                        a = a
                            .inferred_desc(format!("Run the {target} target"))
                            .confidence(Confidence::Low)
                    }
                }
                if t.inherited && !CORE_TARGETS.contains(&target.as_str()) {
                    a = a.hidden();
                }
                out.actions.push(a);
            }
        } else if base.has("nx.json") {
            let v = read_jsonc(base, "nx.json").unwrap_or(Value::Null);
            let mut targets: Vec<String> = v
                .get("targetDefaults")
                .and_then(|t| t.as_object())
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();
            for t in ["build", "test", "lint"] {
                if !targets.iter().any(|x| x == t) {
                    targets.push(t.to_string());
                }
            }
            for t in targets
                .iter()
                .filter(|t| !t.starts_with('@') && !t.contains(':'))
            {
                if free(t) {
                    let conf = if CORE_TARGETS.contains(&t.as_str()) {
                        Confidence::Medium
                    } else {
                        Confidence::Low
                    };
                    // Explain it from what `targetDefaults` says the target does; when Nx
                    // says nothing, the command already says all there is.
                    let default = v.get("targetDefaults").and_then(|d| d.get(t));
                    let executor = default
                        .and_then(|d| d.get("executor"))
                        .and_then(|e| e.as_str());
                    let mut a = Action::new(t, format!("nx run-many -t {t}"))
                        .tool("nx")
                        .inferred(conf);
                    if let Some(cmd) = default.and_then(command_text) {
                        a = a.raw(cmd);
                    } else if let Some(ex) = executor.filter(|e| *e != "nx:noop") {
                        let opts = nx_options(None, v.get("targetDefaults"), t);
                        let (text, cat, risk) = executor_desc(ex, t, &opts);
                        a = a.inferred_desc(text).cat(cat).risk(risk);
                    } else if executor == Some("nx:noop") {
                        a = a.hidden();
                    }
                    out.actions.push(a);
                }
            }
            out.actions.push(
                Action::new("affected", "nx affected -t build test lint")
                    .tool("nx")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Build, test and lint affected Nx projects")
                    .cat(Category::Quality),
            );
            out.actions.push(
                Action::new("graph", "nx graph")
                    .tool("nx")
                    .inferred(Confidence::Low)
                    .inferred_desc("Open the Nx project graph")
                    .cat(Category::Other),
            );
        }

        // ---- Turborepo --------------------------------------------------------------------
        if let Some(v) = read_jsonc(base, "turbo.json") {
            let tasks = v
                .get("tasks")
                .or_else(|| v.get("pipeline"))
                .and_then(|t| t.as_object());
            if let Some(tasks) = tasks {
                for (t, _) in tasks {
                    if t.starts_with("//") || t.contains('#') {
                        continue;
                    }
                    if free(t) {
                        out.actions.push(
                            Action::new(t, format!("turbo run {t}"))
                                .tool("turbo")
                                .inferred(Confidence::Medium)
                                .inferred_desc(format!("Run {t} across packages with Turborepo")),
                        );
                    }
                }
            }
        }

        // ---- Rush ---------------------------------------------------------------------------
        if base.has("rush.json") {
            for (n, cmd, desc, cat) in [
                (
                    "install",
                    "rush install",
                    "Install dependencies with Rush",
                    Category::Other,
                ),
                (
                    "build",
                    "rush build",
                    "Build all projects with Rush",
                    Category::Build,
                ),
                (
                    "rebuild",
                    "rush rebuild",
                    "Rebuild all projects with Rush",
                    Category::Build,
                ),
                (
                    "test",
                    "rush test",
                    "Test all projects with Rush",
                    Category::Testing,
                ),
            ] {
                let conf = if n == "install" || n == "rebuild" {
                    Confidence::Low
                } else {
                    Confidence::Medium
                };
                out.actions.push(
                    Action::new(n, cmd)
                        .tool("rush")
                        .inferred(conf)
                        .inferred_desc(desc)
                        .cat(cat),
                );
            }
            if let Some(v) = read_jsonc(base, "common/config/rush/command-line.json") {
                if let Some(cmds) = v.get("commands").and_then(|c| c.as_array()) {
                    for c in cmds {
                        let Some(n) = c.get("name").and_then(|n| n.as_str()) else {
                            continue;
                        };
                        let mut a = Action::new(n, format!("rush {n}")).tool("rush");
                        if let Some(d) = c.get("description").and_then(|d| d.as_str()) {
                            a = a.desc(d);
                        }
                        if let Some(sc) = c.get("shellCommand").and_then(|d| d.as_str()) {
                            a = a.raw(sc);
                        }
                        out.actions.push(a);
                    }
                }
            }
        }

        // ---- moon ---------------------------------------------------------------------------
        if base.has("moon.yml") {
            let text = base.read("moon.yml").unwrap_or_default();
            if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(&text) {
                let pname = v
                    .get("id")
                    .and_then(|i| i.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| base.name().to_string());
                let root = ctx
                    .ancestors(base)
                    .into_iter()
                    .find(|a| a.has_dir(".moon"))
                    .unwrap_or(base);
                if let Some(tasks) = v.get("tasks").and_then(|t| t.as_mapping()) {
                    for (k, t) in tasks {
                        let Some(name) = k.as_str() else { continue };
                        let cmd = t
                            .get("command")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                t.get("command").and_then(|c| c.as_sequence()).map(|s| {
                                    s.iter()
                                        .filter_map(|x| x.as_str())
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                })
                            });
                        let args = t
                            .get("args")
                            .and_then(|a| a.as_str())
                            .map(|s| format!(" {s}"))
                            .unwrap_or_default();
                        let mut a = Action::new(name, format!("moon run {pname}:{name}"))
                            .tool("moon")
                            .cwd(root.rel.clone());
                        if let Some(c) = cmd {
                            a = a.raw(format!("{c}{args}"));
                        }
                        if let Some(d) = t.get("description").and_then(|d| d.as_str()) {
                            a = a.desc(d);
                        }
                        out.actions.push(a);
                    }
                }
            }
        } else if base.has_dir(".moon") && !base.has("moon.yml") {
            out.actions.push(
                Action::new("check", "moon check --all")
                    .tool("moon")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Run build and test tasks for all moon projects")
                    .cat(Category::Quality),
            );
            out.actions.push(
                Action::new("ci", "moon ci")
                    .tool("moon")
                    .inferred(Confidence::Low)
                    .inferred_desc("Run affected moon tasks as in CI")
                    .cat(Category::Quality),
            );
        }
        out
    }
}

/// The target's `options`, the project's own over the workspace `targetDefaults`.
fn nx_options(project: Option<&Value>, defaults: Option<&Value>, target: &str) -> Value {
    let mut merged = serde_json::Map::new();
    for src in [defaults, project.and_then(|p| p.get("targets"))] {
        if let Some(o) = src
            .and_then(|t| t.get(target))
            .and_then(|t| t.get("options"))
            .and_then(|o| o.as_object())
        {
            for (k, v) in o {
                merged.insert(k.clone(), v.clone());
            }
        }
    }
    Value::Object(merged)
}

fn executor_desc(ex: &str, target: &str, options: &Value) -> (String, Category, Risk) {
    let options = Some(options);
    let platform = options
        .and_then(|o| o.get("platform"))
        .and_then(|p| p.as_str())
        .unwrap_or("");
    let local = options
        .and_then(|o| o.get("local"))
        .and_then(|l| l.as_bool())
        .unwrap_or(false);
    let fix = options
        .and_then(|o| o.get("fix"))
        .and_then(|l| l.as_bool())
        .unwrap_or(false);
    let (text, cat, risk) = executor_desc_inner(ex, target, platform, local);
    if fix && text == "Check source code with ESLint" {
        return ("Fix lint issues with ESLint".to_string(), cat, risk);
    }
    (text, cat, risk)
}

fn executor_desc_inner(
    ex: &str,
    target: &str,
    platform: &str,
    local: bool,
) -> (String, Category, Risk) {
    let e = ex.rsplit(':').next().unwrap_or(ex);
    let pkg = ex.split(':').next().unwrap_or("");
    let fw = if pkg.contains("next") {
        "Next.js"
    } else if pkg.contains("vite") {
        "Vite"
    } else if pkg.contains("webpack") {
        "webpack"
    } else if pkg.contains("esbuild") {
        "esbuild"
    } else if pkg.contains("rollup") {
        "Rollup"
    } else if pkg.contains("angular") {
        "Angular"
    } else if pkg.contains("react-native") {
        "React Native"
    } else if pkg.contains("expo") {
        "Expo"
    } else if pkg.contains("storybook") {
        "Storybook"
    } else if pkg.contains("docker") {
        "Docker"
    } else {
        ""
    };
    let with = if fw.is_empty() {
        String::new()
    } else {
        format!(" with {fw}")
    };
    // The nx target name (e.g. "serve") is often just the generic action, not something
    // worth repeating in the sentence; use "the app" in that case instead of echoing it back.
    let subject = if matches!(target, "serve" | "start" | "dev" | "run" | "node") {
        "the app".to_string()
    } else {
        target.to_string()
    };
    let safe = |t: String, c: Category| (t, c, Risk::Safe);
    if pkg.contains("angular") {
        match e {
            "application" | "browser" | "browser-esbuild" | "ng-packagr" => {
                return safe("Build the Angular app".into(), Category::Build)
            }
            "dev-server" => {
                return safe(
                    "Start the Angular development server".into(),
                    Category::Development,
                )
            }
            "karma" | "jest" | "web-test-runner" => {
                return safe("Run Angular unit tests".into(), Category::Testing)
            }
            "extract-i18n" => {
                return safe(
                    "Extract i18n messages from the Angular app".into(),
                    Category::Build,
                )
            }
            "server" | "prerender" | "app-shell" => {
                return safe("Build the Angular server bundle".into(), Category::Build)
            }
            _ => {}
        }
    }
    if pkg.contains("expo") {
        let os = match platform {
            "ios" => "iOS",
            "android" => "Android",
            _ => "",
        };
        match e {
            "start" => {
                return safe(
                    "Start the Expo development server".into(),
                    Category::Development,
                )
            }
            "run" if !os.is_empty() => {
                return safe(
                    format!("Build and run the app on {os}"),
                    Category::Development,
                )
            }
            "run" => {
                return safe(
                    "Build and run the app on a device".into(),
                    Category::Development,
                )
            }
            "build" if local => {
                return safe("Build the app locally with EAS".into(), Category::Build)
            }
            "build" => {
                return (
                    "Build the app with EAS Build".into(),
                    Category::Build,
                    Risk::External,
                )
            }
            "update" => {
                return (
                    "Publish an over-the-air update with EAS".into(),
                    Category::Release,
                    Risk::External,
                )
            }
            "submit" => {
                return (
                    "Submit the app to the app stores".into(),
                    Category::Release,
                    Risk::External,
                )
            }
            "prebuild" => {
                return safe(
                    "Generate the native projects with Expo prebuild".into(),
                    Category::Build,
                )
            }
            "export" => return safe("Export the app bundle with Expo".into(), Category::Build),
            "install" => return safe("Install dependencies with Expo".into(), Category::Other),
            "ensure-symlink" => {
                return safe("Ensure the Expo workspace symlink".into(), Category::Other)
            }
            _ => {}
        }
    }
    match e {
        "dev-server" | "serve" | "server" | "start" | "node" | "run-ios" | "run-android"
        | "storybook" => safe(format!("Serve {subject}{with}"), Category::Development),
        "build" | "package" | "bundle" | "export" | "compile" | "tsc" | "rollup" | "esbuild"
        | "webpack" | "vite" | "tsup" => safe(format!("Build{with}"), Category::Build),
        "test" | "jest" | "vitest" | "mocha" => safe(
            format!(
                "Run {} tests",
                if pkg.contains("jest") {
                    "Jest"
                } else if pkg.contains("vitest") {
                    "Vitest"
                } else if pkg.contains("playwright") {
                    "Playwright"
                } else if pkg.contains("cypress") {
                    "Cypress"
                } else {
                    "unit"
                }
            ),
            Category::Testing,
        ),
        "playwright" | "cypress" | "e2e" | "detox" => {
            safe("Run end-to-end tests".to_string(), Category::Testing)
        }
        "lint" | "eslint" | "stylelint" => safe(
            "Check source code with ESLint".to_string(),
            Category::Quality,
        ),
        "typecheck" => safe("Check TypeScript types".to_string(), Category::Quality),
        // Reached only when the executor is run-commands/run-script but no command text
        // could be extracted at all (e.g. it is defined solely under a per-environment
        // `configurations` override rather than the target's base `options`).
        "run-commands" | "run-script" => safe(format!("Run the {target} task"), Category::Other),
        "docker-build" => safe("Build the Docker image".to_string(), Category::Build),
        "publish" | "npm-publish" | "release" => (
            "Publish the package".to_string(),
            Category::Release,
            Risk::External,
        ),
        "noop" => safe(format!("Run the {target} dependencies"), Category::Other),
        _ => safe(
            format!("Run the {target} target ({e})"),
            crate::explain::name_hint(target)
                .map(|h| h.category)
                .unwrap_or(Category::Other),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonc() {
        let v: Value = serde_json::from_str(&strip_jsonc(
            "{\n // c\n \"a\": [1, 2,], /* x */ \"b\": \"//no\",\n}",
        ))
        .unwrap();
        assert_eq!(v["a"].as_array().unwrap().len(), 2);
        assert_eq!(v["b"], "//no");
    }
}
