//! Deterministic plain-English descriptions.
//!
//! Precedence: explicit project description → command analysis → action-name heuristics →
//! safe fallback. No guessing of intent: descriptions state observable behaviour.

use crate::analyze::{self, tools::Kind, Analysis, Resolver};
use crate::model::{Action, ActionSource, Category, Risk};
use crate::risk;

/// Target length for descriptions.
pub const MAX_LEN: usize = 64;

pub struct NameHint {
    pub category: Category,
    pub kind: Kind,
    pub description: &'static str,
}

/// Heuristics based purely on an action's name. Tokens are scanned left to right and the first
/// recognised token wins, so `test:e2e` → Testing and `db:reset` → Database.
pub fn name_hint(name: &str) -> Option<NameHint> {
    let lower = name.to_ascii_lowercase();
    let tokens = lower
        .split([':', '-', '_', '.', ' ', '/'])
        .filter(|t| !t.is_empty());
    for t in tokens {
        let hint = match t {
            "e2e" | "playwright" | "cypress" | "acceptance" => NameHint {
                category: Category::Testing,
                kind: Kind::E2e,
                description: "Run end-to-end tests",
            },
            "test" | "tests" | "spec" | "specs" | "unit" | "integration" | "coverage" | "cov"
            | "jest" | "vitest" | "pytest" => NameHint {
                category: Category::Testing,
                kind: Kind::Test,
                description: "Run tests",
            },
            "bench" | "benchmark" | "benchmarks" | "perf" => NameHint {
                category: Category::Testing,
                kind: Kind::Bench,
                description: "Run benchmarks",
            },
            "dev" | "develop" | "serve" | "server" | "start" | "watch" | "preview" | "run"
            | "up" | "web" | "app" => NameHint {
                category: Category::Development,
                kind: Kind::Dev,
                description: "Start the development server",
            },
            "lint" | "eslint" | "check" | "checks" | "audit" | "validate" | "verify" | "ci"
            | "style" | "prettier" | "ruff" | "clippy" => NameHint {
                category: Category::Quality,
                kind: Kind::Lint,
                description: "Check source code",
            },
            "typecheck" | "tsc" | "types" | "mypy" | "pyright" => NameHint {
                category: Category::Quality,
                kind: Kind::TypeCheck,
                description: "Check types",
            },
            "format" | "fmt" => NameHint {
                category: Category::Quality,
                kind: Kind::Format,
                description: "Format source code",
            },
            "build" | "compile" | "bundle" | "dist" | "package" | "pack" | "assets" => NameHint {
                category: Category::Build,
                kind: Kind::Build,
                description: "Build the project",
            },
            "clean" | "clear" | "purge" => NameHint {
                category: Category::Build,
                kind: Kind::Clean,
                description: "Delete build outputs",
            },
            "generate" | "gen" | "codegen" | "protos" | "proto" => NameHint {
                category: Category::Build,
                kind: Kind::Generate,
                description: "Generate code",
            },
            "docs" | "doc" | "documentation" | "storybook" => NameHint {
                category: Category::Build,
                kind: Kind::Docs,
                description: "Build documentation",
            },
            "migrate" | "migration" | "migrations" => NameHint {
                category: Category::Database,
                kind: Kind::Migrate,
                description: "Apply database migrations",
            },
            "db" | "database" | "seed" | "schema" | "prisma" | "sql" | "postgres" | "mysql"
            | "redis" | "mongo" => NameHint {
                category: Category::Database,
                kind: Kind::Db,
                description: "Manage the database",
            },
            "deploy" | "ship" | "rollout" => NameHint {
                category: Category::Release,
                kind: Kind::Deploy,
                description: "Deploy the application",
            },
            "publish" | "release" | "version" | "changeset" | "changesets" | "tag" => NameHint {
                category: Category::Release,
                kind: Kind::Publish,
                description: "Publish a release",
            },
            "docker" | "compose" | "container" | "containers" | "image" | "images" | "k8s"
            | "kube" | "kubernetes" | "helm" | "infra" | "infrastructure" | "terraform"
            | "services" | "stack" | "cluster" | "kafka" | "queue" => NameHint {
                category: Category::Infrastructure,
                kind: Kind::Infra,
                description: "Manage infrastructure",
            },
            "install" | "setup" | "bootstrap" | "prepare" | "init" | "deps" | "dependencies"
            | "sync" | "update" | "upgrade" => NameHint {
                category: Category::Other,
                kind: Kind::Install,
                description: "Set up the project",
            },
            _ => continue,
        };
        return Some(hint);
    }
    None
}

/// Normalise an explicit description: single line, trimmed, capitalised, no trailing period.
pub fn tidy(d: &str) -> String {
    let first = d
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let first = first.trim_start_matches('#').trim();
    let mut s: String = first.chars().take(120).collect();
    while s.ends_with('.') {
        s.pop();
    }
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn join_and(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let (last, rest) = items.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Build a description from a command analysis.
pub fn describe(analysis: &Analysis, name: &str) -> String {
    let steps = &analysis.steps;
    match steps.len() {
        0 => name_hint(name)
            .map(|h| h.description.to_string())
            .unwrap_or_else(|| format!("Run {name}")),
        1 => steps[0].text.clone(),
        _ => {
            // Drop noise steps (echo, sleep) when something meaningful remains.
            let meaningful: Vec<_> = steps
                .iter()
                .filter(|s| {
                    !matches!(
                        s.tool.as_deref(),
                        Some("echo") | Some("sleep") | Some("cat")
                    )
                })
                .collect();
            let steps: Vec<_> = if meaningful.is_empty() {
                steps.iter().collect()
            } else {
                meaningful
            };
            if steps.len() == 1 {
                return steps[0].text.clone();
            }
            // All steps summarisable as nouns → "Run linting, type checks, and tests".
            let mut nouns: Vec<String> = Vec::new();
            let all_nouns = steps.iter().all(|s| s.kind.noun().is_some());
            if all_nouns {
                for s in &steps {
                    let n = s.kind.noun().unwrap().to_string();
                    if !nouns.contains(&n) {
                        nouns.push(n);
                    }
                }
                if nouns.len() >= 2 && nouns.len() <= 4 {
                    return format!("Run {}", join_and(&nouns));
                }
                if nouns.len() == 1 {
                    // e.g. eslint && stylelint → keep the more specific texts
                    let texts: Vec<String> = steps.iter().map(|s| s.text.clone()).collect();
                    let joined = join_and(
                        &texts
                            .iter()
                            .enumerate()
                            .map(|(i, t)| if i == 0 { t.clone() } else { lower_first(t) })
                            .collect::<Vec<_>>(),
                    );
                    if joined.len() <= MAX_LEN + 10 {
                        return joined;
                    }
                    return format!("Run {} ({} steps)", nouns[0], steps.len());
                }
            }
            // Mixed: "Delete dist, then build with tsup".
            let texts: Vec<String> = steps
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    if i == 0 {
                        s.text.clone()
                    } else {
                        lower_first(&s.text)
                    }
                })
                .collect();
            let mut dedup: Vec<String> = Vec::new();
            for t in texts {
                if !dedup.iter().any(|d| d.eq_ignore_ascii_case(&t)) {
                    dedup.push(t);
                }
            }
            let joined = dedup.join(", then ");
            if joined.len() <= MAX_LEN + 12 {
                joined
            } else {
                let first = &dedup[0];
                let rest = dedup.len() - 1;
                // Prefer the most significant step (highest kind priority) as the headline.
                let headline = steps
                    .iter()
                    .max_by_key(|s| (s.risk, kind_weight(s.kind)))
                    .map(|s| s.text.clone())
                    .unwrap_or_else(|| first.clone());
                format!(
                    "{headline} (+{rest} more step{})",
                    if rest == 1 { "" } else { "s" }
                )
            }
        }
    }
}

fn kind_weight(k: Kind) -> u8 {
    match k {
        Kind::Deploy | Kind::Publish => 10,
        Kind::Migrate | Kind::Db => 9,
        Kind::Dev => 8,
        Kind::E2e | Kind::Test | Kind::Bench => 7,
        Kind::Build => 6,
        Kind::Lint | Kind::TypeCheck | Kind::Format => 5,
        Kind::Infra | Kind::Container => 5,
        Kind::Generate | Kind::Docs => 4,
        Kind::Run => 3,
        Kind::Install => 2,
        Kind::Clean => 1,
        Kind::Other => 0,
    }
}

/// Pick a category from the analysis, falling back to name heuristics.
pub fn categorize(analysis: &Analysis, name: &str) -> Category {
    let mut votes: Vec<(Category, usize)> = Vec::new();
    for s in &analysis.steps {
        if let Some(c) = analyze::kind_category(s.kind) {
            match votes.iter_mut().find(|(k, _)| *k == c) {
                Some(v) => v.1 += 1,
                None => votes.push((c, 1)),
            }
        }
    }
    // Name is a strong signal for the *purpose* of a compound script (e.g. `check`, `ci`).
    let hint = name_hint(name);
    if votes.is_empty() {
        return hint.map(|h| h.category).unwrap_or(Category::Other);
    }
    // Prefer the category the name points at when it also appears in the votes.
    if let Some(h) = &hint {
        if votes.iter().any(|(c, _)| *c == h.category) && h.category != Category::Other {
            return h.category;
        }
    }
    let precedence = |c: Category| match c {
        Category::Release => 0,
        Category::Database => 1,
        Category::Infrastructure => 2,
        Category::Development => 3,
        Category::Testing => 4,
        Category::Quality => 5,
        Category::Build => 6,
        Category::Other => 7,
    };
    votes.sort_by(|a, b| b.1.cmp(&a.1).then(precedence(a.0).cmp(&precedence(b.0))));
    votes[0].0
}

/// Fill in description, category and risk for an action once its command is known.
pub fn finalize(a: &mut Action, resolver: Resolver) {
    let text = a.analysis_text().to_string();
    let analysis = analyze::analyze(&text, resolver);
    if a.description.is_empty() {
        a.description = describe(&analysis, &a.name);
        a.opaque = analysis.steps.iter().all(|s| {
            s.unknown
                || matches!(
                    s.tool.as_deref(),
                    Some("echo") | Some("cat") | Some("sleep")
                )
        }) && !analysis.steps.is_empty();
    }
    if a.category == Category::Other {
        a.category = categorize(&analysis, &a.name);
    }
    let r = risk::classify(&text, &analysis);
    a.risk = a.risk.max(r);
    a.notes = crate::notes::classify(&text, &analysis);
    // Name-based external hint only for opaque commands ("deploy": "./deploy.sh").
    if a.risk == Risk::Safe && analysis.is_opaque() && a.source == ActionSource::Declared {
        let lower = a.name.to_ascii_lowercase();
        if lower
            .split([':', '-', '_', '.'])
            .any(|t| t == "deploy" || t == "publish")
        {
            a.risk = Risk::External;
        }
    }
    a.description = truncate(&a.description);
}

fn truncate(s: &str) -> String {
    const HARD: usize = 96;
    if s.chars().count() <= HARD {
        return s.to_string();
    }
    let mut out: String = s.chars().take(HARD - 1).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::{analyze, no_resolver};

    fn d(cmd: &str) -> String {
        describe(&analyze(cmd, &no_resolver), "x")
    }

    #[test]
    fn combines_known_kinds() {
        assert_eq!(
            d("eslint . && tsc --noEmit && vitest run"),
            "Run linting, type checks, and tests"
        );
        assert_eq!(
            d("prettier --check . && eslint ."),
            "Run formatting and linting"
        );
    }

    #[test]
    fn mixed_steps_are_sequenced() {
        assert_eq!(
            d("rimraf dist && tsup"),
            "Delete dist, then build the package with tsup"
        );
    }

    #[test]
    fn single_step() {
        assert_eq!(
            d("docker compose down -v"),
            "Stop Docker services and delete their volumes"
        );
        assert_eq!(d("node ./scripts/foo.mjs"), "Run scripts/foo.mjs");
    }

    #[test]
    fn categories() {
        let c = |cmd: &str, name: &str| categorize(&analyze(cmd, &no_resolver), name);
        assert_eq!(c("vite", "dev"), Category::Development);
        assert_eq!(
            c("eslint . && tsc --noEmit && vitest run", "check"),
            Category::Quality
        );
        assert_eq!(c("prisma migrate reset", "db:reset"), Category::Database);
        assert_eq!(c("node scripts/seed.js", "seed"), Category::Database);
        assert_eq!(c("kubectl apply -f k8s", "deploy"), Category::Release);
        assert_eq!(c("./scripts/thing.sh", "xyz"), Category::Other);
    }

    #[test]
    fn tidy_descriptions() {
        assert_eq!(tidy("  run the thing.  "), "Run the thing");
        assert_eq!(tidy("## Build it\nmore"), "Build it");
    }
}
