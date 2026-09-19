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

fn nx_targets(dir: &DirInfo) -> Vec<(String, Option<String>, Option<String>)> {
    // (target, description, executor/command)
    let mut out = Vec::new();
    let Some(v) = read_jsonc(dir, "project.json") else {
        return out;
    };
    let Some(t) = v.get("targets").and_then(|t| t.as_object()) else {
        return out;
    };
    for (name, cfg) in t {
        let desc = cfg
            .get("metadata")
            .and_then(|m| m.get("description"))
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());
        let opts = cfg.get("options");
        // `nx:run-commands` most often runs several commands in sequence (its default is not
        // parallel); take every one of them, not just the first, so the description and risk
        // reflect the whole target instead of only its first step.
        let cmd = opts
            .and_then(|o| o.get("command"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                opts.and_then(|o| o.get("commands"))
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|item| {
                                item.as_str().map(|s| s.to_string()).or_else(|| {
                                    item.get("command")
                                        .and_then(|c| c.as_str())
                                        .map(|s| s.to_string())
                                })
                            })
                            .collect::<Vec<_>>()
                            .join(" && ")
                    })
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| {
                cfg.get("executor")
                    .and_then(|e| e.as_str())
                    .map(|e| format!("nx-executor:{e}"))
            });
        out.push((name.clone(), desc, cmd));
    }
    out
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
    fn project_name(&self, dir: &DirInfo) -> Option<String> {
        read_jsonc(dir, "project.json")?
            .get("name")?
            .as_str()
            .map(|s| s.to_string())
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
            let name = self
                .project_name(base)
                .unwrap_or_else(|| base.name().to_string());
            let root = ctx
                .ancestors(base)
                .into_iter()
                .find(|a| a.has("nx.json"))
                .unwrap_or(base);
            for (target, desc, cmd) in nx_targets(base) {
                let mut a = Action::new(&target, format!("nx run {name}:{target}"))
                    .tool("nx")
                    .cwd(root.rel.clone());
                if let Some(d) = desc {
                    a = a.desc(d);
                }
                match cmd {
                    Some(c) if c.starts_with("nx-executor:") => {
                        let ex = c.trim_start_matches("nx-executor:");
                        let (text, cat) = executor_desc(ex, &target);
                        a = a.inferred_desc(text).cat(cat);
                    }
                    Some(c) => a = a.raw(c),
                    None => {}
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
                    out.actions.push(
                        Action::new(t, format!("nx run-many -t {t}"))
                            .tool("nx")
                            .inferred(Confidence::Medium)
                            .inferred_desc(format!("Run {t} across all Nx projects")),
                    );
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

fn executor_desc(ex: &str, target: &str) -> (String, Category) {
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
    match e {
        "dev-server" | "serve" | "server" | "start" | "node" | "run-ios" | "run-android"
        | "storybook" => (format!("Serve {subject}{with}"), Category::Development),
        "build" | "package" | "bundle" | "export" | "compile" | "tsc" | "rollup" | "esbuild"
        | "webpack" | "vite" | "tsup" => (format!("Build{with}"), Category::Build),
        "test" | "jest" | "vitest" | "mocha" => (
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
            ("Run end-to-end tests".to_string(), Category::Testing)
        }
        "lint" | "eslint" | "stylelint" => (
            "Check source code with ESLint".to_string(),
            Category::Quality,
        ),
        "typecheck" => ("Check TypeScript types".to_string(), Category::Quality),
        // Reached only when the executor is run-commands/run-script but no command text
        // could be extracted at all (e.g. it is defined solely under a per-environment
        // `configurations` override rather than the target's base `options`).
        "run-commands" | "run-script" => (format!("Run the {target} task"), Category::Other),
        "docker-build" => ("Build the Docker image".to_string(), Category::Build),
        "publish" | "npm-publish" | "release" => {
            ("Publish the package".to_string(), Category::Release)
        }
        _ => (
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
