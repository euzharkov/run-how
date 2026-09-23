//! Knowledge of common developer tools: what an invocation observably does.
//!
//! Every entry describes behaviour that can be read off the command line, never guessed intent.

use crate::model::Risk;

mod bundlers;
mod db;
mod docker;
mod dotnet;
mod go;
mod infra;
mod js_frameworks;
mod js_quality;
mod js_tests;
mod jvm;
mod package_managers;
mod python;
mod rust;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Dev,
    Test,
    E2e,
    Bench,
    Lint,
    TypeCheck,
    Format,
    Build,
    Clean,
    Generate,
    Docs,
    Migrate,
    Db,
    Container,
    Infra,
    Deploy,
    Publish,
    Install,
    Run,
    Other,
}

impl Kind {
    /// Noun used when several steps are combined: "Run linting, type checks, and tests".
    pub fn noun(self) -> Option<&'static str> {
        Some(match self {
            Kind::Lint => "linting",
            Kind::TypeCheck => "type checks",
            Kind::Test => "tests",
            Kind::E2e => "end-to-end tests",
            Kind::Bench => "benchmarks",
            Kind::Format => "formatting",
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub text: String,
    pub kind: Kind,
    pub risk: Risk,
    pub tool: String,
}

pub const CARGO_BUILTINS: &[&str] = &[
    "build",
    "b",
    "run",
    "r",
    "test",
    "t",
    "check",
    "c",
    "clippy",
    "fmt",
    "bench",
    "doc",
    "d",
    "publish",
    "install",
    "uninstall",
    "update",
    "add",
    "remove",
    "rm",
    "new",
    "init",
    "clean",
    "fetch",
    "generate-lockfile",
    "metadata",
    "package",
    "search",
    "tree",
    "vendor",
    "version",
    "login",
    "logout",
    "owner",
    "yank",
    "nextest",
    "watch",
    "audit",
    "deny",
    "outdated",
    "expand",
    "llvm-cov",
    "tarpaulin",
    "insta",
    "machete",
    "udeps",
    "hack",
    "semver-checks",
    "sqlx",
    "make",
    "xtask",
    "component",
    "binstall",
    "dist",
    "release",
    "workspaces",
    "set-version",
    "miri",
    "fuzz",
    "flamegraph",
    "bloat",
    "criterion",
    "lambda",
    "shuttle",
    "leptos",
    "dioxus",
    "tauri",
    "ndk",
    "apk",
    "zigbuild",
    "cross",
    "chef",
    "about",
    "sort",
    "readme",
    "generate",
    "wasi",
    "espflash",
    "embed",
    "objdump",
    "size",
    "asm",
    "modules",
    "rustc",
    "rustdoc",
    "report",
    "locate-project",
    "pkgid",
    "verify-project",
    "read-manifest",
    "help",
];

struct Args<'a>(&'a [String]);

impl<'a> Args<'a> {
    fn has(&self, flag: &str) -> bool {
        self.0
            .iter()
            .any(|a| a == flag || a.starts_with(&format!("{flag}=")))
    }
    fn has_any(&self, flags: &[&str]) -> bool {
        flags.iter().any(|f| self.has(f))
    }
    /// First argument matching one of `subs`.
    fn find(&self, subs: &[&str]) -> Option<&'a str> {
        self.0.iter().map(String::as_str).find(|a| subs.contains(a))
    }
    fn positionals(&self) -> Vec<&'a str> {
        self.0
            .iter()
            .map(String::as_str)
            .filter(|a| !a.starts_with('-'))
            .collect()
    }
    fn sub(&self) -> Option<&'a str> {
        self.positionals().first().copied()
    }
    fn after(&self, word: &str) -> Vec<&'a str> {
        match self.0.iter().position(|a| a == word) {
            Some(i) => self.0[i + 1..]
                .iter()
                .map(String::as_str)
                .filter(|a| !a.starts_with('-'))
                .collect(),
            None => vec![],
        }
    }
    fn value(&self, flag: &str) -> Option<&'a str> {
        let mut it = self.0.iter();
        while let Some(a) = it.next() {
            if a == flag {
                return it.next().map(String::as_str);
            }
            if let Some(v) = a.strip_prefix(&format!("{flag}=")) {
                return Some(v);
            }
        }
        None
    }
}

fn s(tool: &str, text: impl Into<String>, kind: Kind, risk: Risk) -> Option<Summary> {
    Some(Summary {
        text: text.into(),
        kind,
        risk,
        tool: tool.to_string(),
    })
}

fn clean_path(p: &str) -> String {
    p.trim_start_matches("./")
        .trim_start_matches(".\\")
        .trim_end_matches('/')
        .to_string()
}

/// Names that are build outputs / caches: deleting them is cleanup, not destruction.
pub fn is_build_output(path: &str) -> bool {
    const OUT: &[&str] = &[
        "dist",
        "build",
        "out",
        ".out",
        "output",
        ".output",
        "node_modules",
        ".next",
        ".nuxt",
        ".svelte-kit",
        ".turbo",
        ".cache",
        "coverage",
        "target",
        "tmp",
        ".tmp",
        "temp",
        ".temp",
        ".parcel-cache",
        "storybook-static",
        ".eslintcache",
        "out-tsc",
        "bin",
        "obj",
        ".angular",
        ".astro",
        ".vite",
        ".vercel",
        ".netlify",
        "__pycache__",
        ".pytest_cache",
        ".mypy_cache",
        ".ruff_cache",
        ".tox",
        ".nox",
        "htmlcov",
        "site",
        ".docusaurus",
        "public/build",
        "lib",
        "esm",
        "cjs",
        "types",
        ".tsbuildinfo",
        "generated",
        "__generated__",
        ".gen",
        "gen",
        "artifacts",
        ".artifacts",
        "release",
        "packages",
        ".wrangler",
        ".serverless",
        ".terraform",
        "TestResults",
        "*.tsbuildinfo",
        "*.log",
        "*.tgz",
        "*.map",
        ".DS_Store",
        "_build",
        ".expo",
        ".tamagui",
        ".gradle",
        "DerivedData",
        "Pods",
    ];
    let p = clean_path(path);
    if p.is_empty()
        || p == "/"
        || p == "*"
        || p == "."
        || p == ".."
        || p == "~"
        || p.starts_with('~')
        || p.starts_with('/')
        || p.starts_with("..")
        || p.starts_with('$')
    {
        return false;
    }
    let segs: Vec<&str> = p
        .split(['/', '\\'])
        .filter(|s| *s != "*" && *s != "**" && !s.is_empty())
        .collect();
    if segs.is_empty() {
        return false;
    }
    let last = segs[segs.len() - 1];
    let ext_glob = |pat: &str, name: &str| {
        pat.strip_prefix('*')
            .map(|suf| name.ends_with(suf))
            .unwrap_or(false)
    };
    OUT.iter()
        .any(|o| segs.iter().any(|sg| sg == o || ext_glob(o, sg)) || ext_glob(o, last))
        || last.ends_with(".tsbuildinfo")
        || last.ends_with(".egg-info")
}

fn list(items: &[&str]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].to_string(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let (last, rest) = items.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

/// Describe an invocation of a known tool. Returns `None` for unknown programs.
pub fn summarize(program: &str, args: &[String]) -> Option<Summary> {
    let a = Args(args);
    let sub = a.sub();
    js_frameworks::summarize(program, args, &a, sub)
        .or_else(|| bundlers::summarize(program, args, &a, sub))
        .or_else(|| js_tests::summarize(program, args, &a, sub))
        .or_else(|| js_quality::summarize(program, args, &a, sub))
        .or_else(|| package_managers::summarize(program, args, &a, sub))
        .or_else(|| db::summarize(program, args, &a, sub))
        .or_else(|| python::summarize(program, args, &a, sub))
        .or_else(|| go::summarize(program, args, &a, sub))
        .or_else(|| rust::summarize(program, args, &a, sub))
        .or_else(|| dotnet::summarize(program, args, &a, sub))
        .or_else(|| jvm::summarize(program, args, &a, sub))
        .or_else(|| docker::summarize(program, args, &a, sub))
        .or_else(|| infra::summarize(program, args, &a, sub))
}

fn compose(args: &[String]) -> Option<Summary> {
    let a = Args(args);
    use Kind::*;
    use Risk::*;
    let subs = [
        "up", "down", "logs", "build", "ps", "pull", "push", "exec", "run", "restart", "stop",
        "start", "rm", "config", "kill", "pause", "unpause", "top", "images", "create", "cp",
        "watch", "attach", "events", "ls", "port", "version", "stats", "wait", "scale",
    ];
    let sub = a.find(&subs);
    let services: Vec<&str> = match sub {
        Some(sv) => a
            .after(sv)
            .into_iter()
            .filter(|x| !x.contains('=') && !x.ends_with(".yml") && !x.ends_with(".yaml"))
            .collect(),
        None => vec![],
    };
    let svc_phrase = |default: &str| -> String {
        if services.is_empty() {
            default.to_string()
        } else if services.len() == 1 {
            format!("the {} service", services[0])
        } else {
            format!("the {} services", list(&services))
        }
    };
    match sub {
        Some("up") => {
            let detached = a.has("-d") || a.has("--detach");
            let build = a.has("--build");
            let text = if build {
                format!("Build and start {}", svc_phrase("Docker services"))
            } else if detached {
                format!("Start {} in the background", svc_phrase("Docker services"))
            } else {
                format!("Start {}", svc_phrase("Docker services"))
            };
            s("docker compose", text, Container, Safe)
        }
        Some("down") => {
            if a.has("-v") || a.has("--volumes") {
                s(
                    "docker compose",
                    "Stop Docker services and delete their volumes",
                    Container,
                    Destructive,
                )
            } else if a.has("--rmi") {
                s(
                    "docker compose",
                    "Stop Docker services and remove their images",
                    Container,
                    Safe,
                )
            } else {
                s(
                    "docker compose",
                    "Stop and remove Docker services",
                    Container,
                    Safe,
                )
            }
        }
        Some("logs") => s(
            "docker compose",
            if a.has("-f") || a.has("--follow") {
                format!("Follow logs of {}", svc_phrase("Docker services"))
            } else {
                format!("Show logs of {}", svc_phrase("Docker services"))
            },
            Container,
            Safe,
        ),
        Some("build") => s(
            "docker compose",
            format!("Build images for {}", svc_phrase("Docker services")),
            Container,
            Safe,
        ),
        Some("ps") => s(
            "docker compose",
            "List Docker service containers",
            Container,
            Safe,
        ),
        Some("pull") => s(
            "docker compose",
            format!("Pull images for {}", svc_phrase("Docker services")),
            Container,
            Safe,
        ),
        Some("push") => s(
            "docker compose",
            "Push Docker service images to a registry",
            Publish,
            External,
        ),
        Some("exec") => s(
            "docker compose",
            format!("Run a command in {}", svc_phrase("a service container")),
            Container,
            Safe,
        ),
        Some("run") => s(
            "docker compose",
            format!(
                "Run a one-off command in {}",
                svc_phrase("a service container")
            ),
            Container,
            Safe,
        ),
        Some("restart") => s(
            "docker compose",
            format!("Restart {}", svc_phrase("Docker services")),
            Container,
            Safe,
        ),
        Some("stop") => s(
            "docker compose",
            format!("Stop {}", svc_phrase("Docker services")),
            Container,
            Safe,
        ),
        Some("start") => s(
            "docker compose",
            format!("Start {}", svc_phrase("Docker services")),
            Container,
            Safe,
        ),
        Some("rm") => s(
            "docker compose",
            format!(
                "Remove stopped containers of {}",
                svc_phrase("Docker services")
            ),
            Container,
            if a.has("-v") { Destructive } else { Safe },
        ),
        Some("config") => s(
            "docker compose",
            "Validate and print the Compose configuration",
            Lint,
            Safe,
        ),
        Some("kill") => s(
            "docker compose",
            format!("Kill {}", svc_phrase("Docker services")),
            Container,
            Safe,
        ),
        Some("watch") => s(
            "docker compose",
            "Start Docker services and rebuild on change",
            Dev,
            Safe,
        ),
        Some("images") => s(
            "docker compose",
            "List Docker service images",
            Container,
            Safe,
        ),
        Some("create") => s(
            "docker compose",
            "Create Docker service containers",
            Container,
            Safe,
        ),
        Some("cp") => s(
            "docker compose",
            "Copy files to or from a service container",
            Container,
            Safe,
        ),
        Some(x) => s(
            "docker compose",
            format!("Run docker compose {x}"),
            Container,
            Safe,
        ),
        None => s("docker compose", "Run Docker Compose", Container, Safe),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sum(cmd: &str) -> Summary {
        let inv = crate::analyze::parse(cmd).remove(0);
        summarize(&inv.program, &inv.args).expect("known tool")
    }

    #[test]
    fn describes_common_tools() {
        assert_eq!(sum("vite").text, "Start the Vite development server");
        assert_eq!(sum("vitest").text, "Run Vitest tests");
        assert_eq!(
            sum("playwright test").text,
            "Run Playwright end-to-end tests"
        );
        assert_eq!(sum("eslint .").text, "Check source code with ESLint");
        assert_eq!(sum("tsc --noEmit").text, "Check TypeScript types");
        assert_eq!(sum("go test ./...").text, "Run Go tests");
        assert_eq!(
            sum("cargo test --workspace").text,
            "Run Rust tests for the whole workspace"
        );
        assert_eq!(
            sum("dotnet test Api.Tests.csproj").text,
            "Run tests for Api.Tests"
        );
        assert_eq!(
            sum("docker compose down -v").text,
            "Stop Docker services and delete their volumes"
        );
        assert_eq!(sum("docker compose down -v").risk, Risk::Destructive);
        assert_eq!(
            sum("docker compose -f docker/compose.yml up -d postgres").text,
            "Start the postgres service in the background"
        );
        assert_eq!(sum("prisma migrate reset").risk, Risk::Destructive);
        assert_eq!(sum("kubectl apply -f k8s/").risk, Risk::External);
        assert_eq!(sum("terraform destroy").risk, Risk::Destructive);
        assert_eq!(sum("npm publish").risk, Risk::External);
        assert_eq!(sum("git push origin main").risk, Risk::External);
    }

    #[test]
    fn rm_of_build_outputs_is_cleanup() {
        assert_eq!(sum("rm -rf dist").risk, Risk::Safe);
        assert_eq!(sum("rm -rf dist").text, "Delete dist");
        assert_eq!(
            sum("rimraf dist coverage .turbo").text,
            "Delete dist, coverage, and .turbo"
        );
        assert_eq!(sum("rm -rf ./src").risk, Risk::Destructive);
        assert_eq!(sum("rm -rf ~/data").risk, Risk::Destructive);
        assert_eq!(sum("rm -rf packages/*/dist").risk, Risk::Safe);
    }

    #[test]
    fn unknown_script_is_conservative() {
        assert_eq!(sum("node ./scripts/foo.mjs").text, "Run scripts/foo.mjs");
        assert_eq!(
            sum("node --import tsx src/server.ts").text,
            "Run src/server.ts"
        );
        assert_eq!(
            sum("node --watch server.js").text,
            "Run server.js with auto-reload"
        );
        assert!(summarize("some-unknown-tool", &[]).is_none());
    }
}
