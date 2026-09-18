//! Knowledge of common developer tools: what an invocation observably does.
//!
//! Every entry describes behaviour that can be read off the command line, never guessed intent.

use crate::model::Risk;

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
    use Kind::*;
    use Risk::*;
    match program {
        // ----------------------------------------------------------------- JS frameworks
        "vite" => match sub {
            None | Some("dev") | Some("serve") => {
                s("vite", "Start the Vite development server", Dev, Safe)
            }
            Some("build") => s("vite", "Build the app with Vite", Build, Safe),
            Some("preview") => s("vite", "Preview the production build with Vite", Dev, Safe),
            Some(x) => s("vite", format!("Run vite {x}"), Other, Safe),
        },
        "next" => match sub {
            None | Some("dev") => s("next", "Start the Next.js development server", Dev, Safe),
            Some("build") => s("next", "Build the Next.js app", Build, Safe),
            Some("start") => s("next", "Start the Next.js production server", Dev, Safe),
            Some("lint") => s("next", "Lint the Next.js app", Lint, Safe),
            Some("export") => s("next", "Export the Next.js app as static HTML", Build, Safe),
            Some("telemetry") => s("next", "Configure Next.js telemetry", Other, Safe),
            Some(x) => s("next", format!("Run next {x}"), Other, Safe),
        },
        "nuxt" | "nuxi" => match sub {
            None | Some("dev") => s("nuxt", "Start the Nuxt development server", Dev, Safe),
            Some("build") => s("nuxt", "Build the Nuxt app", Build, Safe),
            Some("generate") => s("nuxt", "Generate the static Nuxt site", Build, Safe),
            Some("preview") | Some("start") => {
                s("nuxt", "Preview the Nuxt production build", Dev, Safe)
            }
            Some("typecheck") => s("nuxt", "Check TypeScript types with Nuxt", TypeCheck, Safe),
            Some("prepare") => s("nuxt", "Generate Nuxt type stubs", Generate, Safe),
            Some(x) => s("nuxt", format!("Run nuxt {x}"), Other, Safe),
        },
        "astro" => match sub {
            None | Some("dev") => s("astro", "Start the Astro development server", Dev, Safe),
            Some("build") => s("astro", "Build the Astro site", Build, Safe),
            Some("preview") => s("astro", "Preview the Astro production build", Dev, Safe),
            Some("check") => s(
                "astro",
                "Check the Astro project for errors",
                TypeCheck,
                Safe,
            ),
            Some("sync") => s("astro", "Generate Astro content types", Generate, Safe),
            Some(x) => s("astro", format!("Run astro {x}"), Other, Safe),
        },
        "remix" => match sub {
            Some("dev") | None => s("remix", "Start the Remix development server", Dev, Safe),
            Some("build") => s("remix", "Build the Remix app", Build, Safe),
            Some(x) => s("remix", format!("Run remix {x}"), Other, Safe),
        },
        "react-router" => match sub {
            Some("dev") | None => s(
                "react-router",
                "Start the React Router development server",
                Dev,
                Safe,
            ),
            Some("build") => s("react-router", "Build the React Router app", Build, Safe),
            Some("typegen") => s(
                "react-router",
                "Generate React Router route types",
                Generate,
                Safe,
            ),
            Some(x) => s("react-router", format!("Run react-router {x}"), Other, Safe),
        },
        "svelte-kit" | "vite-plugin-svelte" => s("svelte-kit", "Run SvelteKit", Other, Safe),
        "svelte-check" => s(
            "svelte-check",
            "Check Svelte components and types",
            TypeCheck,
            Safe,
        ),
        "vue-tsc" => {
            if a.has("--noEmit") || a.has("--noemit") {
                s("vue-tsc", "Check Vue and TypeScript types", TypeCheck, Safe)
            } else if a.has_any(&["-b", "--build"]) {
                s(
                    "vue-tsc",
                    "Build TypeScript project references for Vue",
                    Build,
                    Safe,
                )
            } else {
                s("vue-tsc", "Compile Vue and TypeScript sources", Build, Safe)
            }
        }
        "vue-cli-service" => match sub {
            Some("serve") => s("vue-cli", "Start the Vue CLI development server", Dev, Safe),
            Some("build") => s("vue-cli", "Build the Vue app", Build, Safe),
            Some("lint") => s("vue-cli", "Lint the Vue app", Lint, Safe),
            Some("test:unit") => s("vue-cli", "Run Vue unit tests", Test, Safe),
            Some("test:e2e") => s("vue-cli", "Run Vue end-to-end tests", E2e, Safe),
            Some(x) => s("vue-cli", format!("Run vue-cli-service {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "ng" => match sub {
            Some("serve") | Some("s") => {
                s("angular", "Start the Angular development server", Dev, Safe)
            }
            Some("build") | Some("b") => s("angular", "Build the Angular app", Build, Safe),
            Some("test") | Some("t") => s("angular", "Run Angular unit tests", Test, Safe),
            Some("e2e") | Some("e") => s("angular", "Run Angular end-to-end tests", E2e, Safe),
            Some("lint") => s("angular", "Lint the Angular app", Lint, Safe),
            Some(x) => s("angular", format!("Run ng {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "react-scripts" | "craco" | "react-app-rewired" => match sub {
            Some("start") => s(
                "react-scripts",
                "Start the Create React App dev server",
                Dev,
                Safe,
            ),
            Some("build") => s("react-scripts", "Build the React app", Build, Safe),
            Some("test") => s("react-scripts", "Run React app tests", Test, Safe),
            Some("eject") => s(
                "react-scripts",
                "Eject from Create React App",
                Other,
                Destructive,
            ),
            Some(x) => s(
                "react-scripts",
                format!("Run react-scripts {x}"),
                Other,
                Safe,
            ),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "expo" => match sub {
            None | Some("start") => s("expo", "Start the Expo development server", Dev, Safe),
            Some("run:ios") => s("expo", "Build and run the app on iOS", Dev, Safe),
            Some("run:android") => s("expo", "Build and run the app on Android", Dev, Safe),
            Some("export") => s("expo", "Export the Expo app bundle", Build, Safe),
            Some("prebuild") => s(
                "expo",
                "Generate native iOS and Android projects",
                Generate,
                Safe,
            ),
            Some(x) => s("expo", format!("Run expo {x}"), Other, Safe),
        },
        "eas" => match sub {
            Some("build") => s("eas", "Build the app with EAS Build", Build, External),
            Some("submit") => s("eas", "Submit the app to the app stores", Publish, External),
            Some("update") => s(
                "eas",
                "Publish an EAS over-the-air update",
                Publish,
                External,
            ),
            Some(x) => s("eas", format!("Run eas {x}"), Other, External),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "react-native" => match sub {
            Some("start") | None => s("react-native", "Start the Metro bundler", Dev, Safe),
            Some("run-ios") => s("react-native", "Build and run the app on iOS", Dev, Safe),
            Some("run-android") => s(
                "react-native",
                "Build and run the app on Android",
                Dev,
                Safe,
            ),
            Some(x) => s("react-native", format!("Run react-native {x}"), Other, Safe),
        },
        "electron" => s("electron", "Start the Electron app", Dev, Safe),
        "electron-builder" => s("electron-builder", "Package the Electron app", Build, Safe),
        "electron-forge" => match sub {
            Some("start") => s("electron-forge", "Start the Electron app", Dev, Safe),
            Some("make") => s(
                "electron-forge",
                "Build Electron distributables",
                Build,
                Safe,
            ),
            Some("package") => s("electron-forge", "Package the Electron app", Build, Safe),
            Some("publish") => s(
                "electron-forge",
                "Publish the Electron app",
                Publish,
                External,
            ),
            Some(x) => s(
                "electron-forge",
                format!("Run electron-forge {x}"),
                Other,
                Safe,
            ),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "tauri" => match sub {
            Some("dev") => s(
                "tauri",
                "Start the Tauri app in development mode",
                Dev,
                Safe,
            ),
            Some("build") => s("tauri", "Build the Tauri app", Build, Safe),
            Some(x) => s("tauri", format!("Run tauri {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "storybook" | "start-storybook" | "build-storybook" => match (program, sub) {
            ("build-storybook", _) | (_, Some("build")) => {
                s("storybook", "Build the static Storybook", Build, Safe)
            }
            _ => s("storybook", "Start Storybook", Dev, Safe),
        },
        "docusaurus" => match sub {
            Some("start") => s("docusaurus", "Start the Docusaurus site locally", Dev, Safe),
            Some("build") => s("docusaurus", "Build the Docusaurus site", Docs, Safe),
            Some("deploy") => s("docusaurus", "Deploy the Docusaurus site", Deploy, External),
            Some("serve") => s("docusaurus", "Serve the built Docusaurus site", Dev, Safe),
            Some(x) => s("docusaurus", format!("Run docusaurus {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "vitepress" => match sub {
            Some("dev") | None => s(
                "vitepress",
                "Start the VitePress docs site locally",
                Dev,
                Safe,
            ),
            Some("build") => s("vitepress", "Build the VitePress docs site", Docs, Safe),
            Some("preview") => s("vitepress", "Preview the built VitePress site", Dev, Safe),
            Some(x) => s("vitepress", format!("Run vitepress {x}"), Other, Safe),
        },
        "typedoc" => s("typedoc", "Generate API docs with TypeDoc", Docs, Safe),
        "mdbook" => match sub {
            Some("serve") => s("mdbook", "Serve the mdBook locally", Dev, Safe),
            Some("build") => s("mdbook", "Build the mdBook", Docs, Safe),
            Some("test") => s("mdbook", "Test the mdBook code samples", Test, Safe),
            Some(x) => s("mdbook", format!("Run mdbook {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },

        // ----------------------------------------------------------------- bundlers/compilers
        "tsc" => {
            if a.has("--noEmit") || a.has("--noemit") {
                s("tsc", "Check TypeScript types", TypeCheck, Safe)
            } else if a.has_any(&["-w", "--watch"]) {
                s("tsc", "Compile TypeScript in watch mode", Dev, Safe)
            } else if a.has_any(&["-b", "--build"]) {
                s("tsc", "Build TypeScript project references", Build, Safe)
            } else {
                s("tsc", "Compile TypeScript sources", Build, Safe)
            }
        }
        "tsup" => s(
            "tsup",
            if a.has("--watch") {
                "Build with tsup in watch mode"
            } else {
                "Build the package with tsup"
            },
            if a.has("--watch") { Dev } else { Build },
            Safe,
        ),
        "tsdown" => s("tsdown", "Build the package with tsdown", Build, Safe),
        "unbuild" => s("unbuild", "Build the package with unbuild", Build, Safe),
        "microbundle" => s(
            "microbundle",
            "Build the package with microbundle",
            Build,
            Safe,
        ),
        "rollup" => s(
            "rollup",
            if a.has_any(&["-w", "--watch"]) {
                "Bundle with Rollup in watch mode"
            } else {
                "Bundle with Rollup"
            },
            if a.has_any(&["-w", "--watch"]) {
                Dev
            } else {
                Build
            },
            Safe,
        ),
        "esbuild" => s(
            "esbuild",
            if a.has("--watch") || a.has("--serve") {
                "Bundle with esbuild in watch mode"
            } else {
                "Bundle with esbuild"
            },
            if a.has("--watch") || a.has("--serve") {
                Dev
            } else {
                Build
            },
            Safe,
        ),
        "webpack" | "webpack-cli" => {
            if a.find(&["serve", "s", "server"]).is_some() || a.has("--watch") || a.has("-w") {
                s("webpack", "Start the webpack dev server", Dev, Safe)
            } else {
                s("webpack", "Bundle with webpack", Build, Safe)
            }
        }
        "webpack-dev-server" => s("webpack", "Start the webpack dev server", Dev, Safe),
        "parcel" => match sub {
            Some("build") => s("parcel", "Bundle with Parcel", Build, Safe),
            _ => s("parcel", "Start the Parcel development server", Dev, Safe),
        },
        "rspack" | "rsbuild" => match sub {
            Some("build") => s(
                program,
                format!(
                    "Build with {}",
                    if program == "rspack" {
                        "Rspack"
                    } else {
                        "Rsbuild"
                    }
                ),
                Build,
                Safe,
            ),
            _ => s(
                program,
                format!(
                    "Start the {} dev server",
                    if program == "rspack" {
                        "Rspack"
                    } else {
                        "Rsbuild"
                    }
                ),
                Dev,
                Safe,
            ),
        },
        "swc" => s("swc", "Compile sources with SWC", Build, Safe),
        "babel" => s("babel", "Compile sources with Babel", Build, Safe),
        "gulp" => s(
            "gulp",
            match sub {
                Some(t) => format!("Run the {t} gulp task"),
                None => "Run the default gulp task".into(),
            },
            Other,
            Safe,
        ),
        "grunt" => s(
            "grunt",
            match sub {
                Some(t) => format!("Run the {t} grunt task"),
                None => "Run the default grunt task".into(),
            },
            Other,
            Safe,
        ),
        "turbo" => {
            let target = match sub {
                Some("run") => a.positionals().get(1).copied(),
                other => other,
            };
            match target {
                Some(t) => s(
                    "turbo",
                    format!("Run {t} across packages with Turborepo"),
                    crate::explain::name_hint(t)
                        .map(|h| h.kind)
                        .unwrap_or(Other),
                    Safe,
                ),
                None => s("turbo", "Run Turborepo", Other, Safe),
            }
        }
        "nx" => {
            let target = a.value("-t").or_else(|| a.value("--target"));
            match (sub, target) {
                (Some("run-many"), Some(t)) => s(
                    "nx",
                    format!("Run {t} across Nx projects"),
                    crate::explain::name_hint(t)
                        .map(|h| h.kind)
                        .unwrap_or(Other),
                    Safe,
                ),
                (Some("affected"), Some(t)) => s(
                    "nx",
                    format!("Run {t} for affected Nx projects"),
                    crate::explain::name_hint(t)
                        .map(|h| h.kind)
                        .unwrap_or(Other),
                    Safe,
                ),
                (Some("serve"), _) => s(
                    "nx",
                    format!(
                        "Serve {} with Nx",
                        a.positionals().get(1).copied().unwrap_or("the app")
                    ),
                    Dev,
                    Safe,
                ),
                (Some("build"), _) => s(
                    "nx",
                    format!(
                        "Build {} with Nx",
                        a.positionals().get(1).copied().unwrap_or("the project")
                    ),
                    Build,
                    Safe,
                ),
                (Some("test"), _) => s(
                    "nx",
                    format!(
                        "Test {} with Nx",
                        a.positionals().get(1).copied().unwrap_or("the project")
                    ),
                    Test,
                    Safe,
                ),
                (Some("lint"), _) => s(
                    "nx",
                    format!(
                        "Lint {} with Nx",
                        a.positionals().get(1).copied().unwrap_or("the project")
                    ),
                    Lint,
                    Safe,
                ),
                (Some("graph"), _) => s("nx", "Show the Nx project graph", Other, Safe),
                (Some("reset"), _) => s("nx", "Clear the Nx cache", Clean, Safe),
                (Some(x), _) => s("nx", format!("Run nx {x}"), Other, Safe),
                (None, _) => s("nx", "Run Nx", Other, Safe),
            }
        }
        "lerna" => match sub {
            Some("run") => s(
                "lerna",
                format!(
                    "Run {} in all packages with Lerna",
                    a.positionals().get(1).copied().unwrap_or("a script")
                ),
                Other,
                Safe,
            ),
            Some("publish") => s("lerna", "Publish packages with Lerna", Publish, External),
            Some("version") => s("lerna", "Bump package versions with Lerna", Other, Safe),
            Some("bootstrap") => s(
                "lerna",
                "Install and link packages with Lerna",
                Install,
                Safe,
            ),
            Some(x) => s("lerna", format!("Run lerna {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "rush" => match sub {
            Some("build") | Some("rebuild") => {
                s("rush", "Build all projects with Rush", Build, Safe)
            }
            Some("test") => s("rush", "Test all projects with Rush", Test, Safe),
            Some("install") | Some("update") => {
                s("rush", "Install dependencies with Rush", Install, Safe)
            }
            Some("publish") => s("rush", "Publish packages with Rush", Publish, External),
            Some(x) => s("rush", format!("Run rush {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "moon" => s(
            "moon",
            format!("Run {} with moon", sub.unwrap_or("a task")),
            Other,
            Safe,
        ),
        "changeset" | "changesets" => match sub {
            Some("version") => s(
                "changeset",
                "Bump versions from pending changesets",
                Other,
                Safe,
            ),
            Some("publish") => s(
                "changeset",
                "Publish packages with Changesets",
                Publish,
                External,
            ),
            _ => s("changeset", "Create a changeset entry", Other, Safe),
        },
        "semantic-release" => s(
            "semantic-release",
            "Publish a release with semantic-release",
            Publish,
            External,
        ),
        "release-it" => s(
            "release-it",
            "Cut a release with release-it",
            Publish,
            External,
        ),
        "np" => s("np", "Publish the package with np", Publish, External),
        "standard-version" => s(
            "standard-version",
            "Bump version and update the changelog",
            Other,
            Safe,
        ),
        "conventional-changelog" => s(
            "conventional-changelog",
            "Generate the changelog",
            Docs,
            Safe,
        ),

        // ----------------------------------------------------------------- JS tests
        "vitest" => {
            let cov = a.has("--coverage");
            match sub {
                Some("run") | None | Some("watch") | Some("dev") => {
                    let watch = sub == Some("watch")
                        || sub == Some("dev")
                        || a.has("--watch")
                        || a.has("-w");
                    let t = if cov {
                        "Run Vitest tests with coverage"
                    } else if watch {
                        "Run Vitest tests in watch mode"
                    } else {
                        "Run Vitest tests"
                    };
                    s("vitest", t, Test, Safe)
                }
                Some("bench") => s("vitest", "Run Vitest benchmarks", Bench, Safe),
                Some("ui") => s("vitest", "Open the Vitest UI", Test, Safe),
                Some("typecheck") => s("vitest", "Run Vitest type checks", TypeCheck, Safe),
                Some("list") => s("vitest", "List Vitest tests", Test, Safe),
                Some(x) => s("vitest", format!("Run Vitest tests in {}", x), Test, Safe),
            }
        }
        "jest" => {
            let t = if a.has("--coverage") {
                "Run Jest tests with coverage"
            } else if a.has("--watch") || a.has("--watchAll") {
                "Run Jest tests in watch mode"
            } else {
                "Run Jest tests"
            };
            s("jest", t, Test, Safe)
        }
        "mocha" => s("mocha", "Run Mocha tests", Test, Safe),
        "ava" => s("ava", "Run AVA tests", Test, Safe),
        "tap" | "node-tap" => s("tap", "Run node-tap tests", Test, Safe),
        "uvu" => s("uvu", "Run uvu tests", Test, Safe),
        "karma" => s("karma", "Run Karma browser tests", Test, Safe),
        "jasmine" => s("jasmine", "Run Jasmine tests", Test, Safe),
        "playwright" => match sub {
            Some("test") | None => {
                if a.has("--ui") {
                    s("playwright", "Open the Playwright test UI", E2e, Safe)
                } else {
                    s("playwright", "Run Playwright end-to-end tests", E2e, Safe)
                }
            }
            Some("install") => s("playwright", "Download Playwright browsers", Install, Safe),
            Some("codegen") => s(
                "playwright",
                "Record a Playwright test interactively",
                E2e,
                Safe,
            ),
            Some("show-report") => s("playwright", "Open the Playwright HTML report", E2e, Safe),
            Some(x) => s("playwright", format!("Run playwright {x}"), E2e, Safe),
        },
        "cypress" => match sub {
            Some("open") => s("cypress", "Open the Cypress test runner", E2e, Safe),
            Some("run") | None => s("cypress", "Run Cypress end-to-end tests", E2e, Safe),
            Some(x) => s("cypress", format!("Run cypress {x}"), E2e, Safe),
        },
        "wdio" | "webdriverio" => s("wdio", "Run WebdriverIO tests", E2e, Safe),
        "detox" => s("detox", "Run Detox end-to-end tests", E2e, Safe),
        "c8" | "nyc" => s(program, "Run tests with coverage", Test, Safe),
        "codecov" => s("codecov", "Upload coverage to Codecov", Publish, External),
        "lighthouse" => s("lighthouse", "Audit a page with Lighthouse", Test, Safe),
        "tsd" => s("tsd", "Test TypeScript type definitions", TypeCheck, Safe),
        "expect-type" | "attw" | "are-the-types-wrong" => {
            s("attw", "Check published type definitions", TypeCheck, Safe)
        }
        "publint" => s("publint", "Lint the package for publishing", Lint, Safe),
        "size-limit" => s("size-limit", "Check the bundle size", Test, Safe),

        // ----------------------------------------------------------------- JS quality
        "eslint" => {
            if a.has("--fix") {
                s("eslint", "Fix lint issues with ESLint", Lint, Safe)
            } else {
                s("eslint", "Check source code with ESLint", Lint, Safe)
            }
        }
        "eslint_d" => s("eslint", "Check source code with ESLint", Lint, Safe),
        "oxlint" => s("oxlint", "Check source code with Oxlint", Lint, Safe),
        "xo" => s("xo", "Check source code with XO", Lint, Safe),
        "standard" => s("standard", "Check source code with StandardJS", Lint, Safe),
        "prettier" => {
            if a.has("--check") || a.has("-c") || a.has("--list-different") || a.has("-l") {
                s("prettier", "Check formatting with Prettier", Format, Safe)
            } else {
                s("prettier", "Format source code with Prettier", Format, Safe)
            }
        }
        "biome" => match sub {
            Some("check") => s(
                "biome",
                if a.has("--write") || a.has("--apply") {
                    "Fix lint and format issues with Biome"
                } else {
                    "Check lint and formatting with Biome"
                },
                Lint,
                Safe,
            ),
            Some("lint") => s("biome", "Check source code with Biome", Lint, Safe),
            Some("format") => s(
                "biome",
                if a.has("--write") {
                    "Format source code with Biome"
                } else {
                    "Check formatting with Biome"
                },
                Format,
                Safe,
            ),
            Some("ci") => s("biome", "Run Biome checks in CI mode", Lint, Safe),
            Some(x) => s("biome", format!("Run biome {x}"), Lint, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "dprint" => s(
            "dprint",
            if sub == Some("check") {
                "Check formatting with dprint"
            } else {
                "Format source code with dprint"
            },
            Format,
            Safe,
        ),
        "stylelint" => s("stylelint", "Check stylesheets with Stylelint", Lint, Safe),
        "markdownlint" | "markdownlint-cli2" => {
            s("markdownlint", "Check Markdown files", Lint, Safe)
        }
        "cspell" => s("cspell", "Spell-check source files", Lint, Safe),
        "knip" => s(
            "knip",
            "Find unused files and dependencies with Knip",
            Lint,
            Safe,
        ),
        "depcheck" => s("depcheck", "Find unused dependencies", Lint, Safe),
        "madge" => s("madge", "Check for circular dependencies", Lint, Safe),
        "commitlint" => s("commitlint", "Check commit messages", Lint, Safe),
        "lint-staged" => s("lint-staged", "Run linters on staged files", Lint, Safe),
        "husky" => s("husky", "Install Git hooks with Husky", Install, Safe),
        "simple-git-hooks" => s("simple-git-hooks", "Install Git hooks", Install, Safe),
        "lefthook" => s(
            "lefthook",
            if sub == Some("install") {
                "Install Git hooks with Lefthook"
            } else {
                "Run Lefthook hooks"
            },
            Lint,
            Safe,
        ),
        "syncpack" => s(
            "syncpack",
            "Check dependency versions across packages",
            Lint,
            Safe,
        ),
        "sort-package-json" => s(
            "sort-package-json",
            "Sort package.json fields",
            Format,
            Safe,
        ),
        "npm-check-updates" | "ncu" => s("ncu", "Check for dependency updates", Other, Safe),
        "patch-package" => s(
            "patch-package",
            "Apply local dependency patches",
            Install,
            Safe,
        ),
        "tsx" | "ts-node" | "ts-node-dev" | "tsnd" | "ts-node-esm" | "node-dev" | "babel-node"
        | "vite-node" | "jiti" | "esno" => {
            let file = a
                .positionals()
                .iter()
                .find(|p| !matches!(**p, "watch"))
                .copied();
            let watch = a.has("watch")
                || a.has("--watch")
                || matches!(program, "ts-node-dev" | "tsnd" | "node-dev");
            match file {
                Some(f) => s(
                    program,
                    if watch {
                        format!("Run {} with auto-restart", clean_path(f))
                    } else {
                        format!("Run {}", clean_path(f))
                    },
                    if watch { Dev } else { Run },
                    Safe,
                ),
                None => s(program, format!("Run {program}"), Run, Safe),
            }
        }
        "bun-file" => match sub {
            Some(f) => s(
                "bun",
                if a.has("--watch") || a.has("--hot") {
                    format!("Run {} with Bun and auto-reload", clean_path(f))
                } else {
                    format!("Run {} with Bun", clean_path(f))
                },
                if a.has("--watch") || a.has("--hot") {
                    Dev
                } else {
                    Run
                },
                Safe,
            ),
            None => s("bun", "Run a file with Bun", Run, Safe),
        },
        "nodemon" => {
            let file = a.positionals().first().copied();
            match file {
                Some(f) => s(
                    "nodemon",
                    format!("Run {} with auto-restart", clean_path(f)),
                    Dev,
                    Safe,
                ),
                None => s("nodemon", "Run the app with auto-restart", Dev, Safe),
            }
        }
        "node" | "deno" => {
            if program == "deno" {
                return match sub {
                    Some("test") => s("deno", "Run Deno tests", Test, Safe),
                    Some("lint") => s("deno", "Check source code with Deno lint", Lint, Safe),
                    Some("fmt") => s("deno", "Format source code with Deno", Format, Safe),
                    Some("check") => s("deno", "Check TypeScript types with Deno", TypeCheck, Safe),
                    Some("task") => s(
                        "deno",
                        format!(
                            "Run the {} Deno task",
                            a.positionals().get(1).copied().unwrap_or("default")
                        ),
                        Other,
                        Safe,
                    ),
                    Some("run") | Some("serve") => s(
                        "deno",
                        format!(
                            "Run {} with Deno",
                            a.positionals()
                                .get(1)
                                .map(|p| clean_path(p))
                                .unwrap_or_else(|| "the app".into())
                        ),
                        Run,
                        Safe,
                    ),
                    Some("compile") => s("deno", "Compile the app with Deno", Build, Safe),
                    Some("publish") => s("deno", "Publish the package to JSR", Publish, External),
                    Some(x) => s("deno", format!("Run deno {x}"), Other, Safe),
                    None => s("deno", "Start the Deno REPL", Other, Safe),
                };
            }
            if a.has("--test") {
                return s("node", "Run Node.js tests", Test, Safe);
            }
            if a.has("-e") || a.has("--eval") || a.has("-p") || a.has("--print") {
                return s("node", "Run inline Node.js code", Run, Safe);
            }
            let value_flags = [
                "--import",
                "-r",
                "--require",
                "--loader",
                "--experimental-loader",
                "--env-file",
                "--inspect-port",
                "--max-old-space-size",
            ];
            let mut i = 0;
            let mut file = None;
            let mut watch = false;
            while i < args.len() {
                let x = args[i].as_str();
                if value_flags.contains(&x) {
                    i += 2;
                    continue;
                }
                if x == "--watch" || x.starts_with("--watch-path") {
                    watch = true;
                }
                if !x.starts_with('-') {
                    file = Some(x);
                    break;
                }
                i += 1;
            }
            match file {
                Some(f) => s(
                    "node",
                    if watch {
                        format!("Run {} with auto-reload", clean_path(f))
                    } else {
                        format!("Run {}", clean_path(f))
                    },
                    if watch { Dev } else { Run },
                    Safe,
                ),
                None => s("node", "Start the Node.js REPL", Other, Safe),
            }
        }
        "serve" | "http-server" | "sirv" | "live-server" | "browser-sync" => s(
            program,
            format!(
                "Serve {} over HTTP",
                a.positionals()
                    .first()
                    .map(|p| clean_path(p))
                    .unwrap_or_else(|| "static files".into())
            ),
            Dev,
            Safe,
        ),
        "json-server" => s("json-server", "Start a mock JSON API server", Dev, Safe),
        "wait-on" => s(
            "wait-on",
            "Wait for a resource to become available",
            Other,
            Safe,
        ),
        "kill-port" => s("kill-port", "Free a TCP port", Other, Safe),
        "open-cli" | "opener" | "open" if program != "open" || a.has("-a") => s(
            "open",
            format!(
                "Open {}",
                a.positionals().last().copied().unwrap_or("an app")
            ),
            Other,
            Safe,
        ),
        "rimraf" | "del-cli" | "del" | "trash" | "shx" | "rm" | "rmdir" => {
            let mut targets: Vec<&str> = a.positionals();
            if program == "shx" {
                if targets
                    .first()
                    .map(|t| t.starts_with("rm"))
                    .unwrap_or(false)
                {
                    targets.remove(0);
                } else {
                    return s(
                        "shx",
                        format!("Run shx {}", targets.first().copied().unwrap_or("")),
                        Other,
                        Safe,
                    );
                }
            }
            if targets.is_empty() {
                return s("rm", "Delete files", Other, Safe);
            }
            let recursive = program != "rm"
                || a.has_any(&[
                    "-r",
                    "-rf",
                    "-fr",
                    "-R",
                    "-Rf",
                    "--recursive",
                    "-rfv",
                    "-frv",
                ]);
            let all_outputs = targets.iter().all(|t| is_build_output(t));
            let shown: Vec<String> = targets.iter().map(|t| clean_path(t)).collect();
            let shown: Vec<&str> = shown.iter().map(String::as_str).collect();
            if all_outputs {
                s("rm", format!("Delete {}", list(&shown)), Clean, Safe)
            } else if recursive {
                s("rm", format!("Delete {}", list(&shown)), Clean, Destructive)
            } else {
                s("rm", format!("Delete {}", list(&shown)), Clean, Safe)
            }
        }
        "cp" | "cpx" | "copyfiles" | "ncp" | "cpy" | "cpy-cli" | "xcopy" | "robocopy" => {
            let p = a.positionals();
            if p.len() >= 2 {
                s(
                    "cp",
                    format!(
                        "Copy {} to {}",
                        clean_path(p[0]),
                        clean_path(p[p.len() - 1])
                    ),
                    Build,
                    Safe,
                )
            } else {
                s("cp", "Copy files", Build, Safe)
            }
        }
        "mv" | "move" | "rename" => {
            let p = a.positionals();
            if p.len() >= 2 {
                s(
                    "mv",
                    format!(
                        "Move {} to {}",
                        clean_path(p[0]),
                        clean_path(p[p.len() - 1])
                    ),
                    Build,
                    Safe,
                )
            } else {
                s("mv", "Move files", Build, Safe)
            }
        }
        "mkdir" | "mkdirp" | "make-dir" => s(
            "mkdir",
            format!(
                "Create {}",
                a.positionals()
                    .first()
                    .map(|p| clean_path(p))
                    .unwrap_or_else(|| "directories".into())
            ),
            Build,
            Safe,
        ),
        "touch" => s("touch", "Create empty files", Other, Safe),
        "echo" | "printf" => s("echo", "Print a message", Other, Safe),
        "cat" | "type" => s("cat", "Print a file", Other, Safe),
        "ln" => s("ln", "Create a symbolic link", Other, Safe),
        "chmod" => s("chmod", "Change file permissions", Other, Safe),
        "tar" => s(
            "tar",
            if a.has("-x") || args.first().map(|f| f.contains('x')).unwrap_or(false) {
                "Extract an archive"
            } else {
                "Create an archive"
            },
            Build,
            Safe,
        ),
        "zip" | "7z" => s("zip", "Create a zip archive", Build, Safe),
        "unzip" => s("unzip", "Extract a zip archive", Build, Safe),
        "curl" | "wget" | "http" | "httpie" | "xh" => {
            let method = a
                .value("-X")
                .or_else(|| a.value("--request"))
                .map(|m| m.to_uppercase());
            let url = a
                .positionals()
                .iter()
                .find(|p| p.starts_with("http"))
                .copied()
                .unwrap_or("a URL");
            let short = url.split('/').nth(2).unwrap_or(url);
            match method.as_deref() {
                Some("POST") | Some("PUT") | Some("DELETE") | Some("PATCH") => s(
                    "curl",
                    format!("Send a {} request to {}", method.unwrap(), short),
                    Other,
                    External,
                ),
                _ if a.has("-d") || a.has("--data") || a.has("--data-binary") => s(
                    "curl",
                    format!("Send a POST request to {short}"),
                    Other,
                    External,
                ),
                _ => s("curl", format!("Fetch {short}"), Other, Safe),
            }
        }
        "ssh" => s(
            "ssh",
            format!(
                "Run commands on {}",
                a.positionals().first().copied().unwrap_or("a remote host")
            ),
            Other,
            External,
        ),
        "scp" | "rsync" | "sftp" => s(
            program,
            "Copy files to or from a remote host",
            Other,
            External,
        ),
        "sleep" => s("sleep", "Wait", Other, Safe),
        "run-script" => s(
            "script",
            format!("Run {}", clean_path(sub.unwrap_or("a script"))),
            Run,
            Safe,
        ),
        "which" | "where" | "test" | "[" => s(program, "Check the environment", Other, Safe),
        "pkill" | "kill" | "killall" | "taskkill" => {
            s("kill", "Stop a running process", Other, Safe)
        }
        "lsof" | "netstat" => s(program, "Inspect open ports", Other, Safe),
        "xdg-open" | "start" => s("open", "Open in the default application", Other, Safe),
        "http.server" => s("python", "Serve the current directory over HTTP", Dev, Safe),

        // ----------------------------------------------------------------- package managers
        "npm" => match sub {
            Some("install") | Some("i") | Some("ci") | Some("add") | Some("clean-install") => {
                s("npm", "Install dependencies with npm", Install, Safe)
            }
            Some("publish") => s("npm", "Publish the package to npm", Publish, External),
            Some("version") => s("npm", "Bump the package version", Other, Safe),
            Some("pack") => s("npm", "Create an npm tarball", Build, Safe),
            Some("audit") => s(
                "npm",
                if a.has("fix") {
                    "Fix vulnerable dependencies with npm audit"
                } else {
                    "Audit dependencies for vulnerabilities"
                },
                Lint,
                Safe,
            ),
            Some("outdated") => s("npm", "List outdated dependencies", Other, Safe),
            Some("update") | Some("up") | Some("upgrade") => {
                s("npm", "Update dependencies", Install, Safe)
            }
            Some("link") => s("npm", "Link the package locally", Install, Safe),
            Some("cache") => s("npm", "Manage the npm cache", Clean, Safe),
            Some("prune") => s("npm", "Remove extraneous packages", Clean, Safe),
            Some("dedupe") | Some("ddp") => s("npm", "Deduplicate dependencies", Install, Safe),
            Some("rebuild") => s("npm", "Rebuild native dependencies", Install, Safe),
            Some("login") => s("npm", "Log in to the npm registry", Other, External),
            Some("deprecate") | Some("unpublish") => s(
                "npm",
                "Change the package on the npm registry",
                Publish,
                External,
            ),
            Some(x) => s("npm", format!("Run npm {x}"), Other, Safe),
            None => s("npm", "Run npm", Other, Safe),
        },
        "pnpm" => match sub {
            Some("install") | Some("i") | Some("add") | None => {
                s("pnpm", "Install dependencies with pnpm", Install, Safe)
            }
            Some("publish") => s("pnpm", "Publish the package to npm", Publish, External),
            Some("pack") => s("pnpm", "Create an npm tarball", Build, Safe),
            Some("audit") => s("pnpm", "Audit dependencies for vulnerabilities", Lint, Safe),
            Some("outdated") => s("pnpm", "List outdated dependencies", Other, Safe),
            Some("update") | Some("up") => s("pnpm", "Update dependencies", Install, Safe),
            Some("prune") => s("pnpm", "Remove unreferenced packages", Clean, Safe),
            Some("store") => s("pnpm", "Manage the pnpm store", Clean, Safe),
            Some("deploy") => s(
                "pnpm",
                "Copy a workspace package with its dependencies",
                Build,
                Safe,
            ),
            Some("dedupe") => s("pnpm", "Deduplicate dependencies", Install, Safe),
            Some("patch") | Some("patch-commit") => s("pnpm", "Patch a dependency", Install, Safe),
            Some("approve-builds") => s("pnpm", "Approve dependency build scripts", Install, Safe),
            Some(x) => s("pnpm", format!("Run pnpm {x}"), Other, Safe),
        },
        "yarn" => match sub {
            None | Some("install") | Some("add") => {
                s("yarn", "Install dependencies with Yarn", Install, Safe)
            }
            Some("publish") => s("yarn", "Publish the package to npm", Publish, External),
            Some("npm") if a.positionals().get(1) == Some(&"publish") => {
                s("yarn", "Publish the package to npm", Publish, External)
            }
            Some("pack") => s("yarn", "Create an npm tarball", Build, Safe),
            Some("audit") => s("yarn", "Audit dependencies for vulnerabilities", Lint, Safe),
            Some("upgrade") | Some("up") | Some("upgrade-interactive") => {
                s("yarn", "Update dependencies", Install, Safe)
            }
            Some("dedupe") => s("yarn", "Deduplicate dependencies", Install, Safe),
            Some("version") => s("yarn", "Bump the package version", Other, Safe),
            Some("constraints") => s("yarn", "Check workspace constraints", Lint, Safe),
            Some("dlx") => s("yarn", "Run a package with yarn dlx", Other, Safe),
            Some(x) => s("yarn", format!("Run yarn {x}"), Other, Safe),
        },
        "bun" => match sub {
            None | Some("install") | Some("i") | Some("add") => {
                s("bun", "Install dependencies with Bun", Install, Safe)
            }
            Some("test") => s("bun", "Run tests with Bun", Test, Safe),
            Some("build") => s("bun", "Bundle with Bun", Build, Safe),
            Some("publish") => s("bun", "Publish the package to npm", Publish, External),
            Some("update") => s("bun", "Update dependencies", Install, Safe),
            Some("outdated") => s("bun", "List outdated dependencies", Other, Safe),
            Some("pm") => s("bun", "Manage packages with Bun", Other, Safe),
            Some(x) => s("bun", format!("Run bun {x}"), Other, Safe),
        },
        "corepack" => s(
            "corepack",
            "Set up the package manager with Corepack",
            Install,
            Safe,
        ),

        // ----------------------------------------------------------------- DB / ORM
        "prisma" => match (sub, a.positionals().get(1).copied()) {
            (Some("generate"), _) => s("prisma", "Generate the Prisma client", Generate, Safe),
            (Some("migrate"), Some("dev")) => s(
                "prisma",
                "Create and apply Prisma migrations",
                Migrate,
                Safe,
            ),
            (Some("migrate"), Some("deploy")) => {
                s("prisma", "Apply pending Prisma migrations", Migrate, Safe)
            }
            (Some("migrate"), Some("reset")) => s(
                "prisma",
                "Reset the database and re-apply migrations",
                Migrate,
                Destructive,
            ),
            (Some("migrate"), Some("status")) => {
                s("prisma", "Show Prisma migration status", Db, Safe)
            }
            (Some("migrate"), Some("diff")) => s(
                "prisma",
                "Diff the Prisma schema against the database",
                Db,
                Safe,
            ),
            (Some("migrate"), Some("resolve")) => s(
                "prisma",
                "Mark a Prisma migration as applied",
                Migrate,
                Safe,
            ),
            (Some("db"), Some("push")) => s(
                "prisma",
                if a.has("--force-reset") {
                    "Reset the database from the Prisma schema"
                } else {
                    "Push the Prisma schema to the database"
                },
                Db,
                if a.has("--force-reset") {
                    Destructive
                } else {
                    Safe
                },
            ),
            (Some("db"), Some("pull")) => {
                s("prisma", "Pull the database schema into Prisma", Db, Safe)
            }
            (Some("db"), Some("seed")) => s("prisma", "Seed the database", Db, Safe),
            (Some("db"), Some("execute")) => {
                s("prisma", "Execute SQL against the database", Db, Safe)
            }
            (Some("studio"), _) => s("prisma", "Open Prisma Studio", Db, Safe),
            (Some("format"), _) => s("prisma", "Format the Prisma schema", Format, Safe),
            (Some("validate"), _) => s("prisma", "Validate the Prisma schema", Lint, Safe),
            (Some(x), _) => s("prisma", format!("Run prisma {x}"), Db, Safe),
            (None, _) => s("prisma", "Run Prisma", Db, Safe),
        },
        "drizzle-kit" => match sub {
            Some("generate") => s("drizzle-kit", "Generate Drizzle migrations", Migrate, Safe),
            Some("migrate") => s("drizzle-kit", "Apply Drizzle migrations", Migrate, Safe),
            Some("push") => s(
                "drizzle-kit",
                "Push the Drizzle schema to the database",
                Db,
                Safe,
            ),
            Some("pull") | Some("introspect") => s(
                "drizzle-kit",
                "Pull the database schema into Drizzle",
                Db,
                Safe,
            ),
            Some("studio") => s("drizzle-kit", "Open Drizzle Studio", Db, Safe),
            Some("drop") => s(
                "drizzle-kit",
                "Delete a Drizzle migration",
                Migrate,
                Destructive,
            ),
            Some("check") => s(
                "drizzle-kit",
                "Check Drizzle migrations for conflicts",
                Lint,
                Safe,
            ),
            Some(x) => s("drizzle-kit", format!("Run drizzle-kit {x}"), Db, Safe),
            None => s("drizzle-kit", "Run Drizzle Kit", Db, Safe),
        },
        "knex" => match sub {
            Some("migrate:latest") => s("knex", "Apply pending Knex migrations", Migrate, Safe),
            Some("migrate:rollback") => {
                s("knex", "Roll back Knex migrations", Migrate, Destructive)
            }
            Some("migrate:make") => s("knex", "Create a Knex migration", Migrate, Safe),
            Some("seed:run") => s("knex", "Seed the database with Knex", Db, Safe),
            Some(x) => s("knex", format!("Run knex {x}"), Db, Safe),
            None => s("knex", "Run Knex", Db, Safe),
        },
        "sequelize" | "sequelize-cli" => match sub {
            Some("db:migrate") => s(
                "sequelize",
                "Apply pending Sequelize migrations",
                Migrate,
                Safe,
            ),
            Some("db:migrate:undo") | Some("db:migrate:undo:all") => s(
                "sequelize",
                "Roll back Sequelize migrations",
                Migrate,
                Destructive,
            ),
            Some("db:seed:all") | Some("db:seed") => {
                s("sequelize", "Seed the database with Sequelize", Db, Safe)
            }
            Some("db:drop") => s("sequelize", "Drop the database", Db, Destructive),
            Some("db:create") => s("sequelize", "Create the database", Db, Safe),
            Some(x) => s("sequelize", format!("Run sequelize {x}"), Db, Safe),
            None => s("sequelize", "Run Sequelize", Db, Safe),
        },
        "typeorm" | "typeorm-ts-node-commonjs" | "typeorm-ts-node-esm" => match sub {
            Some("migration:run") => {
                s("typeorm", "Apply pending TypeORM migrations", Migrate, Safe)
            }
            Some("migration:revert") => s(
                "typeorm",
                "Revert the last TypeORM migration",
                Migrate,
                Destructive,
            ),
            Some("migration:generate") => {
                s("typeorm", "Generate a TypeORM migration", Migrate, Safe)
            }
            Some("schema:drop") => s("typeorm", "Drop the database schema", Db, Destructive),
            Some("schema:sync") => s("typeorm", "Sync the database schema", Db, Safe),
            Some(x) => s("typeorm", format!("Run typeorm {x}"), Db, Safe),
            None => s("typeorm", "Run TypeORM", Db, Safe),
        },
        "mikro-orm" => match sub {
            Some("migration:up") => s(
                "mikro-orm",
                "Apply pending MikroORM migrations",
                Migrate,
                Safe,
            ),
            Some("migration:down") => s(
                "mikro-orm",
                "Revert MikroORM migrations",
                Migrate,
                Destructive,
            ),
            Some("migration:create") => {
                s("mikro-orm", "Create a MikroORM migration", Migrate, Safe)
            }
            Some("schema:fresh") | Some("schema:drop") => s(
                "mikro-orm",
                "Drop and recreate the database schema",
                Db,
                Destructive,
            ),
            Some("seeder:run") => s("mikro-orm", "Seed the database", Db, Safe),
            Some(x) => s("mikro-orm", format!("Run mikro-orm {x}"), Db, Safe),
            None => s("mikro-orm", "Run MikroORM", Db, Safe),
        },
        "dbmate" | "goose" | "migrate" | "sql-migrate" | "atlas" | "dbmigrate" | "flyway"
        | "liquibase" | "sqlx" | "diesel" | "sea-orm-cli" | "alembic" | "dotnet-ef" => {
            let name = program;
            let pos = a.positionals();
            let joined = pos.join(" ");
            let up = [
                "up",
                "migrate",
                "upgrade",
                "update",
                "run",
                "migration run",
                "database update",
                "apply",
            ];
            let down = [
                "down",
                "rollback",
                "downgrade",
                "reset",
                "drop",
                "database drop",
                "revert",
                "redo",
            ];
            if down
                .iter()
                .any(|d| joined.starts_with(d) || pos.contains(d))
            {
                s(
                    name,
                    format!("Roll back database migrations with {name}"),
                    Migrate,
                    Destructive,
                )
            } else if up
                .iter()
                .any(|u| joined.starts_with(u) || pos.first() == Some(u))
            {
                s(
                    name,
                    format!("Apply pending migrations with {name}"),
                    Migrate,
                    Safe,
                )
            } else if pos.iter().any(|p| {
                p.contains("create")
                    || p.contains("new")
                    || p.contains("generate")
                    || p.contains("revision")
            }) {
                s(
                    name,
                    format!("Create a migration with {name}"),
                    Migrate,
                    Safe,
                )
            } else if pos
                .iter()
                .any(|p| p.contains("status") || p.contains("current") || p.contains("history"))
            {
                s(name, format!("Show migration status with {name}"), Db, Safe)
            } else {
                s(
                    name,
                    format!("Run {name} {}", pos.first().copied().unwrap_or("")),
                    Db,
                    Safe,
                )
            }
        }
        "psql" | "mysql" | "sqlite3" | "sqlcmd" | "mongosh" | "redis-cli" | "clickhouse-client" => {
            let joined = args.join(" ").to_lowercase();
            if joined.contains("drop database")
                || joined.contains("drop table")
                || joined.contains("flushall")
                || joined.contains("flushdb")
                || joined.contains("dropdatabase")
            {
                s(
                    program,
                    format!("Drop data with {program}"),
                    Db,
                    Destructive,
                )
            } else {
                s(program, format!("Open a {program} session"), Db, Safe)
            }
        }
        "createdb" => s("createdb", "Create the PostgreSQL database", Db, Safe),
        "dropdb" => s("dropdb", "Drop the PostgreSQL database", Db, Destructive),
        "pg_dump" | "mysqldump" | "mongodump" => s(program, "Dump the database", Db, Safe),
        "pg_restore" | "mongorestore" => {
            s(program, "Restore the database from a dump", Db, Destructive)
        }

        // ----------------------------------------------------------------- Python
        "pytest" | "py.test" => {
            let target = a.positionals().first().map(|p| clean_path(p));
            match target {
                Some(t) if !t.is_empty() && !t.starts_with('-') => {
                    s("pytest", format!("Run Python tests in {t}"), Test, Safe)
                }
                _ => s(
                    "pytest",
                    if a.has("--cov") {
                        "Run Python tests with coverage"
                    } else {
                        "Run Python tests"
                    },
                    Test,
                    Safe,
                ),
            }
        }
        "coverage" => match sub {
            Some("run") => s("coverage", "Run Python tests with coverage", Test, Safe),
            Some("report") | Some("html") | Some("xml") => {
                s("coverage", "Produce the coverage report", Test, Safe)
            }
            Some(x) => s("coverage", format!("Run coverage {x}"), Test, Safe),
            None => s("coverage", "Run coverage", Test, Safe),
        },
        "ruff" => match sub {
            None | Some("check") => s(
                "ruff",
                if a.has("--fix") {
                    "Fix lint issues with Ruff"
                } else {
                    "Check Python source code with Ruff"
                },
                Lint,
                Safe,
            ),
            Some("format") => s(
                "ruff",
                if a.has("--check") {
                    "Check Python formatting with Ruff"
                } else {
                    "Format Python code with Ruff"
                },
                Format,
                Safe,
            ),
            Some(x) => s("ruff", format!("Run ruff {x}"), Lint, Safe),
        },
        "mypy" => s(
            "mypy",
            "Run static type checking with mypy",
            TypeCheck,
            Safe,
        ),
        "pyright" | "basedpyright" => s(
            "pyright",
            "Run static type checking with Pyright",
            TypeCheck,
            Safe,
        ),
        "pyre" => s(
            "pyre",
            "Run static type checking with Pyre",
            TypeCheck,
            Safe,
        ),
        "black" => s(
            "black",
            if a.has("--check") {
                "Check Python formatting with Black"
            } else {
                "Format Python code with Black"
            },
            Format,
            Safe,
        ),
        "isort" => s(
            "isort",
            if a.has("--check") || a.has("--check-only") {
                "Check import ordering with isort"
            } else {
                "Sort Python imports with isort"
            },
            Format,
            Safe,
        ),
        "flake8" => s("flake8", "Check Python source code with Flake8", Lint, Safe),
        "pylint" => s("pylint", "Check Python source code with Pylint", Lint, Safe),
        "bandit" => s("bandit", "Scan Python code for security issues", Lint, Safe),
        "pyflakes" => s(
            "pyflakes",
            "Check Python source code with Pyflakes",
            Lint,
            Safe,
        ),
        "pip" | "pip3" => match sub {
            Some("install") => s("pip", "Install Python dependencies with pip", Install, Safe),
            Some("freeze") => s("pip", "List installed Python packages", Other, Safe),
            Some("uninstall") => s("pip", "Uninstall Python packages", Install, Safe),
            Some("compile") => s("pip", "Compile Python requirements", Install, Safe),
            Some(x) => s("pip", format!("Run pip {x}"), Install, Safe),
            None => s("pip", "Run pip", Install, Safe),
        },
        "pip-compile" => s(
            "pip-tools",
            "Compile pinned Python requirements",
            Install,
            Safe,
        ),
        "pip-sync" => s(
            "pip-tools",
            "Sync the environment to pinned requirements",
            Install,
            Safe,
        ),
        "uv" => match sub {
            Some("sync") => s("uv", "Sync the Python environment with uv", Install, Safe),
            Some("lock") => s("uv", "Update the uv lockfile", Install, Safe),
            Some("build") => s("uv", "Build the Python package", Build, Safe),
            Some("publish") => s("uv", "Publish the package to PyPI", Publish, External),
            Some("pip") => s("uv", "Manage packages with uv pip", Install, Safe),
            Some("venv") => s("uv", "Create a virtual environment", Install, Safe),
            Some("add") | Some("remove") => {
                s("uv", "Change project dependencies with uv", Install, Safe)
            }
            Some("tool") => s("uv", "Run a tool with uv", Other, Safe),
            Some(x) => s("uv", format!("Run uv {x}"), Other, Safe),
            None => s("uv", "Run uv", Other, Safe),
        },
        "poetry" => match sub {
            Some("install") => s("poetry", "Install dependencies with Poetry", Install, Safe),
            Some("lock") => s("poetry", "Update the Poetry lockfile", Install, Safe),
            Some("build") => s("poetry", "Build the Python package", Build, Safe),
            Some("publish") => s("poetry", "Publish the package to PyPI", Publish, External),
            Some("update") => s("poetry", "Update dependencies with Poetry", Install, Safe),
            Some("check") => s("poetry", "Validate pyproject.toml", Lint, Safe),
            Some(x) => s("poetry", format!("Run poetry {x}"), Other, Safe),
            None => s("poetry", "Run Poetry", Other, Safe),
        },
        "pdm" | "hatch" | "rye" | "pipenv" => match sub {
            Some("install") | Some("sync") => s(
                program,
                format!("Install dependencies with {program}"),
                Install,
                Safe,
            ),
            Some("build") => s(program, "Build the Python package", Build, Safe),
            Some("publish") => s(program, "Publish the package to PyPI", Publish, External),
            Some("test") => s(program, format!("Run tests with {program}"), Test, Safe),
            Some("fmt") => s(program, format!("Format code with {program}"), Format, Safe),
            Some("lint") => s(program, format!("Lint code with {program}"), Lint, Safe),
            Some(x) => s(program, format!("Run {program} {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "twine" => s(
            "twine",
            if sub == Some("upload") {
                "Upload the package to PyPI"
            } else {
                "Check the package with twine"
            },
            if sub == Some("upload") { Publish } else { Lint },
            if sub == Some("upload") {
                External
            } else {
                Safe
            },
        ),
        "pyproject-build" => s("build", "Build the Python package", Build, Safe),
        "tox" => match a.value("-e") {
            Some(e) => s("tox", format!("Run the {e} tox environment"), Test, Safe),
            None => s("tox", "Run tox test environments", Test, Safe),
        },
        "nox" => match a.value("-s").or_else(|| a.value("--session")) {
            Some(e) => s("nox", format!("Run the {e} nox session"), Test, Safe),
            None => s("nox", "Run nox sessions", Test, Safe),
        },
        "pre-commit" => match sub {
            Some("run") => s(
                "pre-commit",
                if a.has("--all-files") || a.has("-a") {
                    "Run pre-commit hooks on all files"
                } else {
                    "Run pre-commit hooks"
                },
                Lint,
                Safe,
            ),
            Some("install") => s("pre-commit", "Install pre-commit Git hooks", Install, Safe),
            Some("autoupdate") => s(
                "pre-commit",
                "Update pre-commit hook versions",
                Install,
                Safe,
            ),
            Some(x) => s("pre-commit", format!("Run pre-commit {x}"), Lint, Safe),
            None => s("pre-commit", "Run pre-commit", Lint, Safe),
        },
        "uvicorn" => {
            let app = sub.unwrap_or("the app");
            if a.has("--reload") {
                s(
                    "uvicorn",
                    format!("Start {app} with Uvicorn and auto-reload"),
                    Dev,
                    Safe,
                )
            } else {
                s("uvicorn", format!("Start {app} with Uvicorn"), Dev, Safe)
            }
        }
        "gunicorn" => s(
            "gunicorn",
            format!("Start {} with Gunicorn", sub.unwrap_or("the app")),
            Dev,
            Safe,
        ),
        "hypercorn" | "daphne" | "waitress-serve" => s(
            program,
            format!("Start {} with {program}", sub.unwrap_or("the app")),
            Dev,
            Safe,
        ),
        "flask" => match a.find(&["run", "shell", "routes", "db"]) {
            Some("run") => s("flask", "Start the Flask development server", Dev, Safe),
            Some("shell") => s("flask", "Open the Flask shell", Dev, Safe),
            Some("routes") => s("flask", "List Flask routes", Other, Safe),
            Some("db") => {
                let p = a.after("db");
                match p.first().copied() {
                    Some("upgrade") => s(
                        "flask",
                        "Apply pending Flask-Migrate migrations",
                        Migrate,
                        Safe,
                    ),
                    Some("downgrade") => s(
                        "flask",
                        "Roll back Flask-Migrate migrations",
                        Migrate,
                        Destructive,
                    ),
                    Some("migrate") => {
                        s("flask", "Generate a Flask-Migrate migration", Migrate, Safe)
                    }
                    _ => s("flask", "Manage Flask-Migrate migrations", Migrate, Safe),
                }
            }
            _ => s("flask", "Run Flask", Other, Safe),
        },
        "fastapi" => match sub {
            Some("dev") => s("fastapi", "Start FastAPI with auto-reload", Dev, Safe),
            Some("run") => s("fastapi", "Start the FastAPI app", Dev, Safe),
            Some(x) => s("fastapi", format!("Run fastapi {x}"), Other, Safe),
            None => s("fastapi", "Run FastAPI", Other, Safe),
        },
        "streamlit" => s(
            "streamlit",
            format!(
                "Start the Streamlit app {}",
                a.positionals()
                    .get(1)
                    .map(|p| clean_path(p))
                    .unwrap_or_default()
            )
            .trim()
            .to_string(),
            Dev,
            Safe,
        ),
        "celery" => match a.find(&["worker", "beat", "flower", "purge"]) {
            Some("worker") => s("celery", "Start a Celery worker", Dev, Safe),
            Some("beat") => s("celery", "Start the Celery beat scheduler", Dev, Safe),
            Some("flower") => s("celery", "Start the Flower monitoring UI", Dev, Safe),
            Some("purge") => s("celery", "Purge all Celery task queues", Other, Destructive),
            _ => s("celery", "Run Celery", Other, Safe),
        },
        "jupyter" | "jupyter-lab" | "jupyter-notebook" => s("jupyter", "Start Jupyter", Dev, Safe),
        "mkdocs" => match sub {
            Some("serve") => s("mkdocs", "Serve the MkDocs site locally", Dev, Safe),
            Some("build") => s("mkdocs", "Build the MkDocs site", Docs, Safe),
            Some("gh-deploy") => s(
                "mkdocs",
                "Deploy the MkDocs site to GitHub Pages",
                Deploy,
                External,
            ),
            Some(x) => s("mkdocs", format!("Run mkdocs {x}"), Docs, Safe),
            None => s("mkdocs", "Run MkDocs", Docs, Safe),
        },
        "sphinx-build" | "sphinx-autobuild" => s(
            "sphinx",
            if program == "sphinx-autobuild" {
                "Serve the Sphinx docs with auto-rebuild"
            } else {
                "Build the Sphinx docs"
            },
            Docs,
            Safe,
        ),
        "manage.py" | "python-module:django" | "django-admin" => {
            let p = a.positionals();
            match p.first().copied() {
                Some("runserver") | Some("runserver_plus") => {
                    s("django", "Start the Django development server", Dev, Safe)
                }
                Some("migrate") => s("django", "Apply Django database migrations", Migrate, Safe),
                Some("makemigrations") => s(
                    "django",
                    "Create Django migrations from model changes",
                    Migrate,
                    Safe,
                ),
                Some("test") => s("django", "Run Django tests", Test, Safe),
                Some("shell") | Some("shell_plus") => {
                    s("django", "Open the Django shell", Dev, Safe)
                }
                Some("collectstatic") => s("django", "Collect Django static files", Build, Safe),
                Some("createsuperuser") => s("django", "Create a Django admin user", Db, Safe),
                Some("flush") => s(
                    "django",
                    "Delete all data from the Django database",
                    Db,
                    Destructive,
                ),
                Some("loaddata") => s("django", "Load Django fixtures into the database", Db, Safe),
                Some("dumpdata") => s("django", "Dump Django data as fixtures", Db, Safe),
                Some("check") => s("django", "Run Django system checks", Lint, Safe),
                Some("showmigrations") => s("django", "Show Django migration status", Db, Safe),
                Some("sqlmigrate") => s("django", "Print SQL for a Django migration", Db, Safe),
                Some(x) => s("django", format!("Run the Django {x} command"), Other, Safe),
                None => s("django", "Run Django management commands", Other, Safe),
            }
        }

        // ----------------------------------------------------------------- Go
        "go" => {
            let pkg = a.positionals().get(1).copied();
            let scope = |default: &str| -> String {
                match pkg {
                    Some("./...") | None => default.to_string(),
                    Some(".") => default.to_string(),
                    Some(p) => clean_path(p),
                }
            };
            match sub {
                Some("build") => s(
                    "go",
                    if pkg.is_none() || pkg == Some("./...") {
                        "Build Go packages".into()
                    } else {
                        format!("Build {}", scope("Go packages"))
                    },
                    Build,
                    Safe,
                ),
                Some("test") => s(
                    "go",
                    if a.has("-race") {
                        "Run Go tests with the race detector".into()
                    } else if a.has("-bench") {
                        "Run Go benchmarks".into()
                    } else if a.has("-cover") || a.has("-coverprofile") {
                        "Run Go tests with coverage".into()
                    } else if pkg.is_none() || pkg == Some("./...") {
                        "Run Go tests".into()
                    } else {
                        format!("Run Go tests in {}", scope(""))
                    },
                    if a.has("-bench") { Bench } else { Test },
                    Safe,
                ),
                Some("vet") => s("go", "Check Go source with go vet", Lint, Safe),
                Some("fmt") => s("go", "Format Go source", Format, Safe),
                Some("run") => s(
                    "go",
                    format!(
                        "Run {}",
                        match pkg {
                            Some(".") | None => "the Go program".to_string(),
                            Some(p) => clean_path(p),
                        }
                    ),
                    Run,
                    Safe,
                ),
                Some("generate") => s("go", "Run go:generate directives", Generate, Safe),
                Some("install") => s("go", "Install Go binaries", Install, Safe),
                Some("mod") => match pkg {
                    Some("tidy") => s("go", "Tidy Go module dependencies", Install, Safe),
                    Some("download") => s("go", "Download Go module dependencies", Install, Safe),
                    Some("vendor") => s("go", "Vendor Go module dependencies", Install, Safe),
                    Some("verify") => s("go", "Verify Go module dependencies", Lint, Safe),
                    _ => s("go", "Manage Go modules", Install, Safe),
                },
                Some("work") => s("go", "Manage the Go workspace", Install, Safe),
                Some("tool") => s(
                    "go",
                    format!("Run the Go tool {}", pkg.unwrap_or("")),
                    Other,
                    Safe,
                ),
                Some("clean") => s("go", "Clean Go build cache and outputs", Clean, Safe),
                Some("get") => s("go", "Add or update Go dependencies", Install, Safe),
                Some("doc") => s("go", "Show Go documentation", Docs, Safe),
                Some(x) => s("go", format!("Run go {x}"), Other, Safe),
                None => s("go", "Run go", Other, Safe),
            }
        }
        "gofmt" => s(
            "gofmt",
            if a.has("-l") {
                "Check Go formatting"
            } else {
                "Format Go source"
            },
            Format,
            Safe,
        ),
        "goimports" => s("goimports", "Format Go imports", Format, Safe),
        "gofumpt" => s("gofumpt", "Format Go source with gofumpt", Format, Safe),
        "golangci-lint" => s(
            "golangci-lint",
            if sub == Some("fmt") {
                "Format Go source with golangci-lint"
            } else {
                "Lint Go source with golangci-lint"
            },
            Lint,
            Safe,
        ),
        "staticcheck" => s(
            "staticcheck",
            "Check Go source with staticcheck",
            Lint,
            Safe,
        ),
        "govulncheck" => s(
            "govulncheck",
            "Scan Go dependencies for vulnerabilities",
            Lint,
            Safe,
        ),
        "gotestsum" => s("gotestsum", "Run Go tests with gotestsum", Test, Safe),
        "air" => s("air", "Run the Go app with live reload", Dev, Safe),
        "goreleaser" => match sub {
            Some("release") => s(
                "goreleaser",
                if a.has("--snapshot") {
                    "Build a GoReleaser snapshot"
                } else {
                    "Publish a release with GoReleaser"
                },
                Publish,
                if a.has("--snapshot") { Safe } else { External },
            ),
            Some("build") => s(
                "goreleaser",
                "Build release binaries with GoReleaser",
                Build,
                Safe,
            ),
            Some("check") => s("goreleaser", "Validate the GoReleaser config", Lint, Safe),
            Some(x) => s("goreleaser", format!("Run goreleaser {x}"), Other, Safe),
            None => s("goreleaser", "Run GoReleaser", Other, Safe),
        },
        "mockgen" | "mockery" => s(program, "Generate Go mocks", Generate, Safe),
        "swag" => s(
            "swag",
            "Generate Swagger docs from Go annotations",
            Generate,
            Safe,
        ),
        "sqlc" => s(
            "sqlc",
            "Generate Go code from SQL with sqlc",
            Generate,
            Safe,
        ),
        "buf" => match sub {
            Some("generate") => s(
                "buf",
                "Generate code from Protobuf with buf",
                Generate,
                Safe,
            ),
            Some("lint") => s("buf", "Lint Protobuf definitions", Lint, Safe),
            Some("breaking") => s("buf", "Check Protobuf for breaking changes", Lint, Safe),
            Some("push") => s(
                "buf",
                "Push Protobuf schemas to the registry",
                Publish,
                External,
            ),
            Some(x) => s("buf", format!("Run buf {x}"), Other, Safe),
            None => s("buf", "Run buf", Other, Safe),
        },
        "protoc" | "protoc-gen-go" => s("protoc", "Compile Protobuf definitions", Generate, Safe),
        "graphql-codegen" | "gql-gen" | "graphql-code-generator" => {
            s("graphql-codegen", "Generate GraphQL types", Generate, Safe)
        }
        "openapi-generator"
        | "openapi-generator-cli"
        | "openapi-typescript"
        | "orval"
        | "kubb"
        | "swagger-codegen" => s(program, "Generate API client code", Generate, Safe),
        "wasm-pack" => s(
            "wasm-pack",
            if sub == Some("build") {
                "Build the WebAssembly package"
            } else {
                "Run wasm-pack"
            },
            Build,
            Safe,
        ),
        "trunk" => match sub {
            Some("serve") => s("trunk", "Serve the WebAssembly app with Trunk", Dev, Safe),
            Some("build") => s("trunk", "Build the WebAssembly app with Trunk", Build, Safe),
            Some(x) => s("trunk", format!("Run trunk {x}"), Other, Safe),
            None => s("trunk", "Run Trunk", Other, Safe),
        },

        // ----------------------------------------------------------------- Rust
        "cargo" => {
            let pkg = a.value("-p").or_else(|| a.value("--package"));
            let ws = a.has("--workspace") || a.has("--all");
            let suffix = match (pkg, ws) {
                (Some(p), _) => format!(" for {p}"),
                (None, true) => " for the whole workspace".to_string(),
                _ => String::new(),
            };
            match sub {
                Some("build") | Some("b") => s(
                    "cargo",
                    format!(
                        "Build the Rust project{}{}",
                        if a.has("--release") {
                            " in release mode"
                        } else {
                            ""
                        },
                        suffix
                    ),
                    Build,
                    Safe,
                ),
                Some("run") | Some("r") => s(
                    "cargo",
                    match a.value("--bin") {
                        Some(b) => format!("Run the {b} binary"),
                        None => format!("Run the Rust program{suffix}"),
                    },
                    Run,
                    Safe,
                ),
                Some("test") | Some("t") => {
                    s("cargo", format!("Run Rust tests{suffix}"), Test, Safe)
                }
                Some("nextest") => s(
                    "cargo",
                    format!("Run Rust tests with nextest{suffix}"),
                    Test,
                    Safe,
                ),
                Some("check") | Some("c") => s(
                    "cargo",
                    format!("Type-check the Rust project{suffix}"),
                    TypeCheck,
                    Safe,
                ),
                Some("clippy") => s(
                    "cargo",
                    format!("Lint Rust source with Clippy{suffix}"),
                    Lint,
                    Safe,
                ),
                Some("fmt") => s(
                    "cargo",
                    if a.has("--check") {
                        "Check Rust formatting"
                    } else {
                        "Format Rust source"
                    },
                    Format,
                    Safe,
                ),
                Some("bench") => s("cargo", "Run Rust benchmarks", Bench, Safe),
                Some("doc") | Some("d") => s(
                    "cargo",
                    if a.has("--open") {
                        "Build and open the Rust docs"
                    } else {
                        "Build the Rust docs"
                    },
                    Docs,
                    Safe,
                ),
                Some("publish") => s("cargo", "Publish the crate to crates.io", Publish, External),
                Some("install") => s("cargo", "Install a Rust binary", Install, Safe),
                Some("clean") => s("cargo", "Delete the Cargo target directory", Clean, Safe),
                Some("update") => s("cargo", "Update Cargo dependencies", Install, Safe),
                Some("fetch") => s("cargo", "Fetch Cargo dependencies", Install, Safe),
                Some("audit") => s(
                    "cargo",
                    "Audit Rust dependencies for vulnerabilities",
                    Lint,
                    Safe,
                ),
                Some("deny") => s("cargo", "Check dependencies with cargo-deny", Lint, Safe),
                Some("outdated") => s("cargo", "List outdated Rust dependencies", Other, Safe),
                Some("watch") => s("cargo", "Rebuild on change with cargo-watch", Dev, Safe),
                Some("llvm-cov") | Some("tarpaulin") => {
                    s("cargo", "Run Rust tests with coverage", Test, Safe)
                }
                Some("insta") => s(
                    "cargo",
                    "Review snapshot tests with cargo-insta",
                    Test,
                    Safe,
                ),
                Some("machete") | Some("udeps") => {
                    s("cargo", "Find unused Rust dependencies", Lint, Safe)
                }
                Some("hack") => s(
                    "cargo",
                    "Check feature combinations with cargo-hack",
                    Lint,
                    Safe,
                ),
                Some("semver-checks") => s("cargo", "Check for semver violations", Lint, Safe),
                Some("miri") => s("cargo", "Run tests under Miri", Test, Safe),
                Some("fuzz") => s("cargo", "Run fuzz targets", Test, Safe),
                Some("package") => s("cargo", "Package the crate", Build, Safe),
                Some("dist") => s(
                    "cargo",
                    "Build distributable archives with cargo-dist",
                    Build,
                    Safe,
                ),
                Some("release") => s(
                    "cargo",
                    "Release the crate with cargo-release",
                    Publish,
                    External,
                ),
                Some("sqlx") => match a.positionals().get(1).copied() {
                    Some("migrate") => match a.positionals().get(2).copied() {
                        Some("run") => s("cargo", "Apply pending sqlx migrations", Migrate, Safe),
                        Some("revert") => s(
                            "cargo",
                            "Revert the last sqlx migration",
                            Migrate,
                            Destructive,
                        ),
                        Some("add") => s("cargo", "Create a sqlx migration", Migrate, Safe),
                        _ => s("cargo", "Manage sqlx migrations", Migrate, Safe),
                    },
                    Some("database") => match a.positionals().get(2).copied() {
                        Some("drop") => s("cargo", "Drop the database with sqlx", Db, Destructive),
                        Some("reset") => {
                            s("cargo", "Recreate the database with sqlx", Db, Destructive)
                        }
                        Some("create") => s("cargo", "Create the database with sqlx", Db, Safe),
                        _ => s("cargo", "Manage the database with sqlx", Db, Safe),
                    },
                    Some("prepare") => {
                        s("cargo", "Prepare sqlx offline query data", Generate, Safe)
                    }
                    _ => s("cargo", "Run sqlx", Db, Safe),
                },
                Some("tauri") => match a.positionals().get(1).copied() {
                    Some("dev") => s(
                        "cargo",
                        "Start the Tauri app in development mode",
                        Dev,
                        Safe,
                    ),
                    Some("build") => s("cargo", "Build the Tauri app", Build, Safe),
                    _ => s("cargo", "Run cargo tauri", Other, Safe),
                },
                Some("leptos") | Some("dioxus") => match a.positionals().get(1).copied() {
                    Some("watch") | Some("serve") => s(
                        "cargo",
                        format!("Start the {} app with live reload", sub.unwrap()),
                        Dev,
                        Safe,
                    ),
                    Some("build") => s(
                        "cargo",
                        format!("Build the {} app", sub.unwrap()),
                        Build,
                        Safe,
                    ),
                    _ => s("cargo", format!("Run cargo {}", sub.unwrap()), Other, Safe),
                },
                Some("xtask") => s(
                    "cargo",
                    format!(
                        "Run the {} xtask",
                        a.positionals().get(1).copied().unwrap_or("default")
                    ),
                    Other,
                    Safe,
                ),
                Some(x) => s("cargo", format!("Run cargo {x}"), Other, Safe),
                None => s("cargo", "Run cargo", Other, Safe),
            }
        }
        "rustup" => s("rustup", "Manage the Rust toolchain", Install, Safe),
        "rustfmt" => s("rustfmt", "Format Rust source", Format, Safe),
        "cross" => s(
            "cross",
            format!("Cross-compile with cross ({})", sub.unwrap_or("build")),
            Build,
            Safe,
        ),

        // ----------------------------------------------------------------- .NET
        "dotnet" => {
            let p = a.positionals();
            let target = p
                .get(1)
                .copied()
                .filter(|t| t.ends_with("proj") || t.ends_with(".sln") || t.ends_with(".slnx"));
            let named = |verb: &str, default: &str| -> String {
                match target {
                    Some(t) => format!(
                        "{verb} {}",
                        std::path::Path::new(t)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(t)
                    ),
                    None => default.to_string(),
                }
            };
            match sub {
                Some("build") => s(
                    "dotnet",
                    named("Build", "Build the .NET project"),
                    Build,
                    Safe,
                ),
                Some("run") => s(
                    "dotnet",
                    match a.value("--project") {
                        Some(pr) => format!(
                            "Run the {} project",
                            std::path::Path::new(pr.trim_end_matches('/'))
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or(pr)
                        ),
                        None => "Run the .NET project".into(),
                    },
                    Run,
                    Safe,
                ),
                Some("test") => s(
                    "dotnet",
                    named("Run tests for", "Run .NET tests"),
                    Test,
                    Safe,
                ),
                Some("restore") => s("dotnet", "Restore NuGet packages", Install, Safe),
                Some("clean") => s("dotnet", "Clean .NET build outputs", Clean, Safe),
                Some("pack") => s("dotnet", "Create NuGet packages", Build, Safe),
                Some("publish") => s(
                    "dotnet",
                    named("Publish", "Publish the .NET app for deployment"),
                    Build,
                    Safe,
                ),
                Some("watch") => s(
                    "dotnet",
                    match p.get(1).copied() {
                        Some("test") => "Run .NET tests on change".to_string(),
                        _ => "Run the .NET project with hot reload".to_string(),
                    },
                    Dev,
                    Safe,
                ),
                Some("format") => s(
                    "dotnet",
                    if a.has("--verify-no-changes") {
                        "Check .NET formatting"
                    } else {
                        "Format .NET source code"
                    },
                    Format,
                    Safe,
                ),
                Some("nuget") => match p.get(1).copied() {
                    Some("push") => s("dotnet", "Push NuGet packages to a feed", Publish, External),
                    _ => s("dotnet", "Run dotnet nuget", Other, Safe),
                },
                Some("tool") => match p.get(1).copied() {
                    Some("restore") => s("dotnet", "Restore .NET local tools", Install, Safe),
                    Some("install") | Some("update") => {
                        s("dotnet", "Install .NET tools", Install, Safe)
                    }
                    Some("run") => s(
                        "dotnet",
                        format!("Run the {} tool", p.get(2).copied().unwrap_or("")),
                        Other,
                        Safe,
                    ),
                    _ => s("dotnet", "Manage .NET tools", Other, Safe),
                },
                Some("ef") => match (p.get(1).copied(), p.get(2).copied()) {
                    (Some("database"), Some("update")) => s(
                        "dotnet-ef",
                        "Apply EF Core migrations to the database",
                        Migrate,
                        Safe,
                    ),
                    (Some("database"), Some("drop")) => s(
                        "dotnet-ef",
                        "Drop the database with EF Core",
                        Db,
                        Destructive,
                    ),
                    (Some("migrations"), Some("add")) => {
                        s("dotnet-ef", "Create an EF Core migration", Migrate, Safe)
                    }
                    (Some("migrations"), Some("remove")) => s(
                        "dotnet-ef",
                        "Remove the last EF Core migration",
                        Migrate,
                        Safe,
                    ),
                    (Some("migrations"), Some("script")) => {
                        s("dotnet-ef", "Generate an EF Core SQL script", Migrate, Safe)
                    }
                    (Some("migrations"), Some("list")) => {
                        s("dotnet-ef", "List EF Core migrations", Db, Safe)
                    }
                    _ => s("dotnet-ef", "Run dotnet ef", Db, Safe),
                },
                Some("cake") => s("cake", "Run the Cake build script", Build, Safe),
                Some("nuke") => s(
                    "nuke",
                    format!(
                        "Run the {} NUKE target",
                        p.get(1).copied().unwrap_or("default")
                    ),
                    Build,
                    Safe,
                ),
                Some("fantomas") => s("fantomas", "Format F# source with Fantomas", Format, Safe),
                Some("fsi") => s("dotnet", "Run an F# script", Run, Safe),
                Some("new") => s("dotnet", "Create a new .NET project", Other, Safe),
                Some("sln") => s("dotnet", "Modify the solution file", Other, Safe),
                Some("workload") => s("dotnet", "Manage .NET workloads", Install, Safe),
                Some("dev-certs") => s(
                    "dotnet",
                    "Manage HTTPS development certificates",
                    Other,
                    Safe,
                ),
                Some("user-secrets") => s("dotnet", "Manage user secrets", Other, Safe),
                Some("outdated") => s("dotnet", "List outdated NuGet packages", Other, Safe),
                Some("csharpier") => s("dotnet", "Format C# source with CSharpier", Format, Safe),
                Some(x) => s("dotnet", format!("Run dotnet {x}"), Other, Safe),
                None => s("dotnet", "Run dotnet", Other, Safe),
            }
        }
        "nuke" => s(
            "nuke",
            format!("Run the {} NUKE target", sub.unwrap_or("default")),
            Build,
            Safe,
        ),
        "msbuild" => s("msbuild", "Build with MSBuild", Build, Safe),
        "nuget" => s(
            "nuget",
            if sub == Some("push") {
                "Push NuGet packages to a feed"
            } else {
                "Run NuGet"
            },
            if sub == Some("push") { Publish } else { Other },
            if sub == Some("push") { External } else { Safe },
        ),

        // ----------------------------------------------------------------- JVM / others
        "gradle" | "gradlew" => match sub {
            Some("build") => s("gradle", "Build with Gradle", Build, Safe),
            Some("test") => s("gradle", "Run Gradle tests", Test, Safe),
            Some("run") | Some("bootRun") => s("gradle", "Run the app with Gradle", Run, Safe),
            Some("clean") => s("gradle", "Clean Gradle build outputs", Clean, Safe),
            Some("publish") | Some("publishToMavenCentral") => {
                s("gradle", "Publish artifacts with Gradle", Publish, External)
            }
            Some("check") => s("gradle", "Run Gradle checks", Lint, Safe),
            Some(x) => s("gradle", format!("Run the {x} Gradle task"), Other, Safe),
            None => s("gradle", "Run Gradle", Other, Safe),
        },
        "mvn" | "mvnw" => match sub {
            Some("package") => s("maven", "Package with Maven", Build, Safe),
            Some("test") => s("maven", "Run Maven tests", Test, Safe),
            Some("verify") => s("maven", "Run Maven verification", Test, Safe),
            Some("install") => s(
                "maven",
                "Install artifacts to the local Maven repo",
                Build,
                Safe,
            ),
            Some("deploy") => s("maven", "Deploy artifacts with Maven", Publish, External),
            Some("clean") => s("maven", "Clean Maven build outputs", Clean, Safe),
            Some("compile") => s("maven", "Compile with Maven", Build, Safe),
            Some(x) => s("maven", format!("Run Maven {x}"), Other, Safe),
            None => s("maven", "Run Maven", Other, Safe),
        },
        "bundle" => match sub {
            Some("exec") => s(
                "bundler",
                format!(
                    "Run {} with Bundler",
                    a.positionals().get(1).copied().unwrap_or("a command")
                ),
                Other,
                Safe,
            ),
            Some("install") | None => s("bundler", "Install Ruby gems", Install, Safe),
            Some(x) => s("bundler", format!("Run bundle {x}"), Other, Safe),
        },
        "rake" => match sub {
            Some("db:migrate")
            | Some("db:reset")
            | Some("db:drop")
            | Some("db:seed")
            | Some("db:rollback")
            | Some("db:schema:load")
            | Some("db:prepare")
            | Some("db:create")
            | Some("test")
            | Some("test:system")
            | Some("routes")
            | Some("assets:precompile") => summarize("rails", args),
            Some(t) => s("rake", format!("Run the {t} Rake task"), Other, Safe),
            None => s("rake", "Run the default Rake task", Other, Safe),
        },
        "rails" => match sub {
            Some("server") | Some("s") => s("rails", "Start the Rails server", Dev, Safe),
            Some("console") | Some("c") => s("rails", "Open the Rails console", Dev, Safe),
            Some("db:migrate") => s("rails", "Apply Rails migrations", Migrate, Safe),
            Some("db:reset") | Some("db:drop") => {
                s("rails", "Drop and recreate the database", Db, Destructive)
            }
            Some("db:seed") => s("rails", "Seed the database", Db, Safe),
            Some("db:rollback") => s(
                "rails",
                "Roll back the last Rails migration",
                Migrate,
                Destructive,
            ),
            Some("db:schema:load") => s(
                "rails",
                "Load the schema into the database",
                Db,
                Destructive,
            ),
            Some("db:prepare") | Some("db:setup") => s(
                "rails",
                "Create the database and apply migrations",
                Db,
                Safe,
            ),
            Some("db:create") => s("rails", "Create the database", Db, Safe),
            Some("test") => s("rails", "Run Rails tests", Test, Safe),
            Some("test:system") => s("rails", "Run Rails system (browser) tests", E2e, Safe),
            Some("routes") => s("rails", "List Rails routes", Other, Safe),
            Some("assets:precompile") => s("rails", "Precompile Rails assets", Build, Safe),
            Some("tailwindcss:build") | Some("css:build") | Some("javascript:build") => {
                s("rails", "Build Rails front-end assets", Build, Safe)
            }
            Some("generate") | Some("g") => s("rails", "Run a Rails generator", Generate, Safe),
            Some(x) => s("rails", format!("Run rails {x}"), Other, Safe),
            None => s("rails", "Run Rails", Other, Safe),
        },
        "rubocop" => s("rubocop", "Check Ruby source with RuboCop", Lint, Safe),
        "composer" => match sub {
            Some("install") | None => s("composer", "Install PHP dependencies", Install, Safe),
            Some("test") => s("composer", "Run PHP tests", Test, Safe),
            Some(x) => s("composer", format!("Run composer {x}"), Other, Safe),
        },
        "phpunit" | "pest" => s(program, "Run PHP tests", Test, Safe),
        "php" => match sub {
            Some("artisan") => match a.positionals().get(1).copied() {
                Some("serve") => s("artisan", "Start the Laravel development server", Dev, Safe),
                Some("migrate") => s("artisan", "Apply Laravel migrations", Migrate, Safe),
                Some("migrate:fresh") | Some("migrate:reset") | Some("db:wipe") => s(
                    "artisan",
                    "Drop all tables and re-run migrations",
                    Db,
                    Destructive,
                ),
                Some("test") => s("artisan", "Run Laravel tests", Test, Safe),
                Some(x) => s("artisan", format!("Run artisan {x}"), Other, Safe),
                None => s("artisan", "Run artisan", Other, Safe),
            },
            Some(f) => s("php", format!("Run {}", clean_path(f)), Run, Safe),
            None => s("php", "Run PHP", Run, Safe),
        },
        "zig" => s(
            "zig",
            format!("Run zig {}", sub.unwrap_or("build")),
            Build,
            Safe,
        ),
        "cmake" => s(
            "cmake",
            if a.has("--build") {
                "Build with CMake"
            } else {
                "Configure the CMake project"
            },
            Build,
            Safe,
        ),
        "ninja" => s("ninja", "Build with Ninja", Build, Safe),
        "bazel" | "bazelisk" => match sub {
            Some("build") => s("bazel", "Build with Bazel", Build, Safe),
            Some("test") => s("bazel", "Run Bazel tests", Test, Safe),
            Some("run") => s(
                "bazel",
                format!(
                    "Run {} with Bazel",
                    a.positionals().get(1).copied().unwrap_or("a target")
                ),
                Run,
                Safe,
            ),
            Some("clean") => s("bazel", "Clean Bazel outputs", Clean, Safe),
            Some(x) => s("bazel", format!("Run bazel {x}"), Other, Safe),
            None => s("bazel", "Run Bazel", Other, Safe),
        },
        "pants" => s(
            "pants",
            format!("Run pants {}", sub.unwrap_or("")),
            Other,
            Safe,
        ),
        "swift" => match sub {
            Some("build") => s("swift", "Build the Swift package", Build, Safe),
            Some("test") => s("swift", "Run Swift tests", Test, Safe),
            Some("run") => s("swift", "Run the Swift executable", Run, Safe),
            Some(x) => s("swift", format!("Run swift {x}"), Other, Safe),
            None => s("swift", "Run Swift", Other, Safe),
        },
        "xcodebuild" => s(
            "xcodebuild",
            if a.has("test") {
                "Run Xcode tests"
            } else {
                "Build with Xcode"
            },
            if a.has("test") { Test } else { Build },
            Safe,
        ),
        "flutter" => match sub {
            Some("run") => s("flutter", "Run the Flutter app", Dev, Safe),
            Some("build") => s("flutter", "Build the Flutter app", Build, Safe),
            Some("test") => s("flutter", "Run Flutter tests", Test, Safe),
            Some("analyze") => s("flutter", "Analyze Dart source", Lint, Safe),
            Some("pub") => s("flutter", "Manage Dart packages", Install, Safe),
            Some(x) => s("flutter", format!("Run flutter {x}"), Other, Safe),
            None => s("flutter", "Run Flutter", Other, Safe),
        },
        "dart" => s(
            "dart",
            format!("Run dart {}", sub.unwrap_or("")),
            Other,
            Safe,
        ),
        "mix" => match sub {
            Some("test") => s("mix", "Run Elixir tests", Test, Safe),
            Some("phx.server") => s("mix", "Start the Phoenix server", Dev, Safe),
            Some("ecto.migrate") => s("mix", "Apply Ecto migrations", Migrate, Safe),
            Some("ecto.reset") | Some("ecto.drop") => {
                s("mix", "Drop and recreate the database", Db, Destructive)
            }
            Some("format") => s("mix", "Format Elixir source", Format, Safe),
            Some("credo") => s("mix", "Check Elixir source with Credo", Lint, Safe),
            Some("compile") => s("mix", "Compile the Elixir project", Build, Safe),
            Some(x) => s("mix", format!("Run mix {x}"), Other, Safe),
            None => s("mix", "Run mix", Other, Safe),
        },

        // ----------------------------------------------------------------- Docker
        "docker" | "podman" | "nerdctl" => {
            let name = program;
            if sub == Some("compose") {
                let rest: Vec<String> = a.after("compose").iter().map(|s| s.to_string()).collect();
                let flags: Vec<String> = args
                    .iter()
                    .filter(|x| x.starts_with('-') || x.ends_with(".yml") || x.ends_with(".yaml"))
                    .cloned()
                    .collect();
                let mut all = rest.clone();
                all.extend(flags);
                return compose(&all);
            }
            match sub {
                Some("build") | Some("buildx") => {
                    s(name, "Build the Docker image", Container, Safe)
                }
                Some("push") => s(
                    name,
                    "Push the Docker image to a registry",
                    Publish,
                    External,
                ),
                Some("pull") => s(name, "Pull a Docker image", Container, Safe),
                Some("run") => {
                    let pos = a.positionals();
                    s(
                        name,
                        format!(
                            "Run a {} container",
                            pos.get(1).copied().unwrap_or("Docker")
                        ),
                        Container,
                        Safe,
                    )
                }
                Some("exec") => s(
                    name,
                    format!(
                        "Run a command in the {} container",
                        a.positionals().get(1).copied().unwrap_or("running")
                    ),
                    Container,
                    Safe,
                ),
                Some("logs") => s(name, "Show container logs", Container, Safe),
                Some("ps") => s(name, "List containers", Container, Safe),
                Some("stop") => s(name, "Stop containers", Container, Safe),
                Some("start") => s(name, "Start containers", Container, Safe),
                Some("rm") => s(
                    name,
                    "Remove containers",
                    Container,
                    if a.has("-f") || a.has("--force") {
                        Destructive
                    } else {
                        Safe
                    },
                ),
                Some("rmi") => s(name, "Remove Docker images", Clean, Safe),
                Some("system") => match a.positionals().get(1).copied() {
                    Some("prune") => s(
                        name,
                        if a.has("-a") || a.has("--all") || a.has("--volumes") {
                            "Delete all unused Docker data"
                        } else {
                            "Delete unused Docker data"
                        },
                        Clean,
                        Destructive,
                    ),
                    Some("df") | Some("info") => {
                        s(name, "Show Docker system information", Container, Safe)
                    }
                    _ => s(name, "Manage the Docker system", Container, Safe),
                },
                Some("volume") => match a.positionals().get(1).copied() {
                    Some("rm") | Some("prune") => s(name, "Delete Docker volumes", Db, Destructive),
                    _ => s(name, "Manage Docker volumes", Container, Safe),
                },
                Some("image") => match a.positionals().get(1).copied() {
                    Some("prune") => s(name, "Delete unused Docker images", Clean, Safe),
                    Some("build") => s(name, "Build the Docker image", Container, Safe),
                    _ => s(name, "Manage Docker images", Container, Safe),
                },
                Some("container") => match a.positionals().get(1).copied() {
                    Some("prune") => s(name, "Delete stopped containers", Clean, Safe),
                    _ => s(name, "Manage Docker containers", Container, Safe),
                },
                Some("network") => s(name, "Manage Docker networks", Container, Safe),
                Some("login") => s(name, "Log in to a container registry", Other, External),
                Some("tag") => s(name, "Tag the Docker image", Container, Safe),
                Some("save") | Some("load") => {
                    s(name, "Transfer a Docker image archive", Container, Safe)
                }
                Some("context") => s(name, "Switch the Docker context", Container, Safe),
                Some("machine") if name == "podman" => s(
                    name,
                    format!(
                        "{} the Podman machine",
                        match a.positionals().get(1).copied() {
                            Some("start") => "Start",
                            Some("stop") => "Stop",
                            Some("init") => "Create",
                            _ => "Manage",
                        }
                    ),
                    Infra,
                    Safe,
                ),
                Some("info") | Some("version") => {
                    s(name, "Show Docker information", Container, Safe)
                }
                Some(x) => s(name, format!("Run {name} {x}"), Container, Safe),
                None => s(name, format!("Run {name}"), Container, Safe),
            }
        }
        "docker-compose" | "podman-compose" => compose(args),
        "colima" => s(
            "colima",
            format!(
                "{} the Colima Docker runtime",
                match sub {
                    Some("start") => "Start",
                    Some("stop") => "Stop",
                    Some("delete") => "Delete",
                    Some("status") => "Show status of",
                    _ => "Manage",
                }
            ),
            Infra,
            if sub == Some("delete") {
                Destructive
            } else {
                Safe
            },
        ),
        "orb" | "orbctl" => s(
            "orbstack",
            format!(
                "{} OrbStack",
                match sub {
                    Some("start") => "Start",
                    Some("stop") => "Stop",
                    _ => "Manage",
                }
            ),
            Infra,
            Safe,
        ),
        "lima" | "limactl" => s("lima", "Manage the Lima VM", Infra, Safe),
        "dive" => s("dive", "Inspect Docker image layers", Container, Safe),
        "hadolint" => s("hadolint", "Lint the Dockerfile", Lint, Safe),
        "trivy" | "grype" => s(program, "Scan for vulnerabilities", Lint, Safe),
        "act" => s("act", "Run GitHub Actions workflows locally", Test, Safe),

        // ----------------------------------------------------------------- Kubernetes / IaC
        "kubectl" | "oc" => {
            let p = a.positionals();
            let dry = a.has("--dry-run");
            let target = p.iter().skip(1).find(|x| !x.starts_with('-')).copied();
            let ns = a
                .value("-n")
                .or_else(|| a.value("--namespace"))
                .map(|n| format!(" in namespace {n}"))
                .unwrap_or_default();
            match sub {
                Some("apply") => {
                    if dry {
                        s(
                            "kubectl",
                            "Validate Kubernetes manifests (dry run)",
                            Infra,
                            Safe,
                        )
                    } else {
                        s(
                            "kubectl",
                            format!("Apply Kubernetes resources{ns}"),
                            Deploy,
                            External,
                        )
                    }
                }
                Some("delete") => s(
                    "kubectl",
                    format!("Delete Kubernetes resources{ns}"),
                    Infra,
                    Destructive,
                ),
                Some("create") => s(
                    "kubectl",
                    if dry {
                        "Generate a Kubernetes manifest (dry run)".into()
                    } else {
                        format!("Create Kubernetes resources{ns}")
                    },
                    Infra,
                    if dry { Safe } else { External },
                ),
                Some("get") | Some("describe") | Some("top") | Some("events") => s(
                    "kubectl",
                    format!("Query {} from the cluster", target.unwrap_or("resources")),
                    Infra,
                    External,
                ),
                Some("logs") => s(
                    "kubectl",
                    format!("Stream logs from {}", target.unwrap_or("a pod")),
                    Infra,
                    External,
                ),
                Some("exec") => s(
                    "kubectl",
                    format!("Run a command in {}", target.unwrap_or("a pod")),
                    Infra,
                    External,
                ),
                Some("port-forward") => s(
                    "kubectl",
                    format!("Forward a local port to {}", target.unwrap_or("a pod")),
                    Infra,
                    External,
                ),
                Some("rollout") => s(
                    "kubectl",
                    format!(
                        "{} a rollout{ns}",
                        match p.get(1).copied() {
                            Some("restart") => "Restart",
                            Some("status") => "Check",
                            Some("undo") => "Roll back",
                            Some("history") => "Show history of",
                            _ => "Manage",
                        }
                    ),
                    Deploy,
                    External,
                ),
                Some("scale") => s(
                    "kubectl",
                    format!("Scale {}{ns}", target.unwrap_or("a deployment")),
                    Infra,
                    External,
                ),
                Some("kustomize") => s(
                    "kubectl",
                    format!(
                        "Render Kustomize manifests from {}",
                        target.map(clean_path).unwrap_or_else(|| ".".into())
                    ),
                    Infra,
                    Safe,
                ),
                Some("diff") => s(
                    "kubectl",
                    "Diff manifests against the cluster",
                    Infra,
                    External,
                ),
                Some("config") => s("kubectl", "Change the kubectl context", Infra, Safe),
                Some("wait") => s(
                    "kubectl",
                    "Wait for a Kubernetes condition",
                    Infra,
                    External,
                ),
                Some("patch") | Some("edit") | Some("set") | Some("label") | Some("annotate")
                | Some("replace") => s(
                    "kubectl",
                    format!("Modify {} in the cluster", target.unwrap_or("resources")),
                    Infra,
                    External,
                ),
                Some("drain") | Some("cordon") | Some("taint") => {
                    s("kubectl", "Change cluster node scheduling", Infra, External)
                }
                Some("cp") => s("kubectl", "Copy files to or from a pod", Infra, External),
                Some("run") => s("kubectl", "Run a pod in the cluster", Infra, External),
                Some("version") | Some("api-resources") | Some("explain") => {
                    s("kubectl", "Show kubectl information", Infra, Safe)
                }
                Some(x) => s("kubectl", format!("Run kubectl {x}"), Infra, External),
                None => s("kubectl", "Run kubectl", Infra, Safe),
            }
        }
        "kustomize" => match sub {
            Some("build") => s(
                "kustomize",
                format!(
                    "Render Kustomize manifests from {}",
                    a.positionals()
                        .get(1)
                        .map(|p| clean_path(p))
                        .unwrap_or_else(|| ".".into())
                ),
                Infra,
                Safe,
            ),
            Some("edit") => s("kustomize", "Edit the kustomization", Infra, Safe),
            Some(x) => s("kustomize", format!("Run kustomize {x}"), Infra, Safe),
            None => s("kustomize", "Run Kustomize", Infra, Safe),
        },
        "helm" => {
            let p = a.positionals();
            match sub {
                Some("template") => s("helm", "Render the Helm chart templates", Infra, Safe),
                Some("lint") => s("helm", "Validate the Helm chart", Lint, Safe),
                Some("install") => s(
                    "helm",
                    format!(
                        "Install the {} Helm release",
                        p.get(1).copied().unwrap_or("")
                    )
                    .trim()
                    .to_string(),
                    Deploy,
                    External,
                ),
                Some("upgrade") => s(
                    "helm",
                    if a.has("--install") || a.has("-i") {
                        format!(
                            "Install or upgrade the {} Helm release",
                            p.get(1).copied().unwrap_or("")
                        )
                    } else {
                        format!(
                            "Upgrade the {} Helm release",
                            p.get(1).copied().unwrap_or("")
                        )
                    }
                    .trim()
                    .to_string(),
                    Deploy,
                    External,
                ),
                Some("uninstall") | Some("delete") | Some("del") => s(
                    "helm",
                    format!(
                        "Uninstall the {} Helm release",
                        p.get(1).copied().unwrap_or("")
                    )
                    .trim()
                    .to_string(),
                    Infra,
                    Destructive,
                ),
                Some("rollback") => s("helm", "Roll back a Helm release", Deploy, External),
                Some("dependency") | Some("dep") => {
                    s("helm", "Update Helm chart dependencies", Install, Safe)
                }
                Some("package") => s("helm", "Package the Helm chart", Build, Safe),
                Some("push") => s(
                    "helm",
                    "Push the Helm chart to a registry",
                    Publish,
                    External,
                ),
                Some("repo") => s("helm", "Manage Helm repositories", Install, Safe),
                Some("list") | Some("ls") | Some("status") | Some("get") | Some("history") => s(
                    "helm",
                    "Query Helm releases in the cluster",
                    Infra,
                    External,
                ),
                Some("test") => s(
                    "helm",
                    "Run Helm release tests in the cluster",
                    Test,
                    External,
                ),
                Some("diff") => s(
                    "helm",
                    "Diff the Helm release against the cluster",
                    Infra,
                    External,
                ),
                Some(x) => s("helm", format!("Run helm {x}"), Infra, Safe),
                None => s("helm", "Run Helm", Infra, Safe),
            }
        }
        "helmfile" => match sub {
            Some("apply") | Some("sync") => s(
                "helmfile",
                "Apply Helm releases with Helmfile",
                Deploy,
                External,
            ),
            Some("destroy") | Some("delete") => s(
                "helmfile",
                "Destroy Helm releases with Helmfile",
                Infra,
                Destructive,
            ),
            Some("diff") => s(
                "helmfile",
                "Diff Helm releases with Helmfile",
                Infra,
                External,
            ),
            Some("template") => s(
                "helmfile",
                "Render Helm releases with Helmfile",
                Infra,
                Safe,
            ),
            Some("lint") => s("helmfile", "Lint Helm releases with Helmfile", Lint, Safe),
            Some(x) => s("helmfile", format!("Run helmfile {x}"), Infra, Safe),
            None => s("helmfile", "Run Helmfile", Infra, Safe),
        },
        "skaffold" => match sub {
            Some("dev") => s(
                "skaffold",
                "Build and deploy to the cluster on change",
                Deploy,
                External,
            ),
            Some("run") | Some("deploy") => s(
                "skaffold",
                "Build and deploy to the cluster with Skaffold",
                Deploy,
                External,
            ),
            Some("build") => s("skaffold", "Build images with Skaffold", Build, Safe),
            Some("render") => s("skaffold", "Render manifests with Skaffold", Infra, Safe),
            Some("delete") => s(
                "skaffold",
                "Delete Skaffold-deployed resources",
                Infra,
                Destructive,
            ),
            Some(x) => s("skaffold", format!("Run skaffold {x}"), Infra, Safe),
            None => s("skaffold", "Run Skaffold", Infra, Safe),
        },
        "tilt" => match sub {
            Some("up") => s(
                "tilt",
                "Start the Tilt development environment",
                Dev,
                External,
            ),
            Some("down") => s("tilt", "Tear down Tilt resources", Infra, Destructive),
            Some("ci") => s("tilt", "Run Tilt in CI mode", Deploy, External),
            Some(x) => s("tilt", format!("Run tilt {x}"), Infra, Safe),
            None => s("tilt", "Run Tilt", Infra, Safe),
        },
        "minikube" | "kind" | "k3d" => match sub {
            Some("start") | Some("create") => s(
                program,
                format!("Create a local Kubernetes cluster with {program}"),
                Infra,
                Safe,
            ),
            Some("stop") => s(program, format!("Stop the {program} cluster"), Infra, Safe),
            Some("delete") => s(
                program,
                format!("Delete the {program} cluster"),
                Infra,
                Destructive,
            ),
            Some("load") | Some("image") => s(
                program,
                format!("Load an image into the {program} cluster"),
                Infra,
                Safe,
            ),
            Some(x) => s(program, format!("Run {program} {x}"), Infra, Safe),
            None => s(program, format!("Run {program}"), Infra, Safe),
        },
        "kubeconform" | "kubeval" => s(
            program,
            "Validate Kubernetes manifests against schemas",
            Lint,
            Safe,
        ),
        "kube-linter" | "kube-score" | "polaris" => {
            s(program, "Lint Kubernetes manifests", Lint, Safe)
        }
        "argocd" => match sub {
            Some("app") => match a.positionals().get(1).copied() {
                Some("sync") => s("argocd", "Sync the Argo CD application", Deploy, External),
                Some("delete") => s(
                    "argocd",
                    "Delete the Argo CD application",
                    Infra,
                    Destructive,
                ),
                _ => s("argocd", "Manage Argo CD applications", Infra, External),
            },
            _ => s("argocd", "Run Argo CD", Infra, External),
        },
        "terraform" | "tofu" | "terragrunt" => {
            let name = if program == "tofu" {
                "OpenTofu"
            } else if program == "terragrunt" {
                "Terragrunt"
            } else {
                "Terraform"
            };
            match sub {
                Some("init") => s(
                    program,
                    format!("Initialise the {name} working directory"),
                    Infra,
                    Safe,
                ),
                Some("plan") => s(
                    program,
                    if a.has("-destroy") {
                        format!("Plan a {name} destroy")
                    } else {
                        format!("Plan {name} changes")
                    },
                    Infra,
                    External,
                ),
                Some("apply") => s(
                    program,
                    format!("Apply {name} changes to infrastructure"),
                    Deploy,
                    External,
                ),
                Some("destroy") => s(
                    program,
                    format!("Destroy {name}-managed infrastructure"),
                    Infra,
                    Destructive,
                ),
                Some("fmt") => s(
                    program,
                    if a.has("-check") {
                        format!("Check {name} formatting")
                    } else {
                        format!("Format {name} files")
                    },
                    Format,
                    Safe,
                ),
                Some("validate") => s(
                    program,
                    format!("Validate the {name} configuration"),
                    Lint,
                    Safe,
                ),
                Some("output") | Some("show") | Some("state") => {
                    s(program, format!("Inspect {name} state"), Infra, External)
                }
                Some("import") => s(
                    program,
                    format!("Import a resource into {name} state"),
                    Infra,
                    External,
                ),
                Some("taint") | Some("untaint") => s(
                    program,
                    format!("Mark a {name} resource for recreation"),
                    Infra,
                    External,
                ),
                Some("workspace") => {
                    s(program, format!("Switch the {name} workspace"), Infra, Safe)
                }
                Some("run-all") => s(
                    program,
                    format!("Run {name} across all modules"),
                    Infra,
                    External,
                ),
                Some("test") => s(program, format!("Run {name} tests"), Test, External),
                Some(x) => s(program, format!("Run {} {x}", program), Infra, Safe),
                None => s(program, format!("Run {name}"), Infra, Safe),
            }
        }
        "tflint" => s("tflint", "Lint Terraform files", Lint, Safe),
        "tfsec" | "checkov" => s(
            program,
            "Scan infrastructure code for security issues",
            Lint,
            Safe,
        ),
        "pulumi" => match sub {
            Some("up") | Some("update") => s("pulumi", "Deploy the Pulumi stack", Deploy, External),
            Some("preview") | Some("pre") => s("pulumi", "Preview Pulumi changes", Infra, External),
            Some("destroy") | Some("down") | Some("dn") => {
                s("pulumi", "Destroy the Pulumi stack", Infra, Destructive)
            }
            Some("refresh") => s("pulumi", "Refresh Pulumi state", Infra, External),
            Some("stack") => s("pulumi", "Manage Pulumi stacks", Infra, External),
            Some(x) => s("pulumi", format!("Run pulumi {x}"), Infra, External),
            None => s("pulumi", "Run Pulumi", Infra, Safe),
        },
        "cdk" | "cdktf" => match sub {
            Some("deploy") => s(program, "Deploy the CDK stack", Deploy, External),
            Some("destroy") => s(program, "Destroy the CDK stack", Infra, Destructive),
            Some("synth") | Some("synthesize") => {
                s(program, "Synthesize the CDK stack", Build, Safe)
            }
            Some("diff") => s(
                program,
                "Diff the CDK stack against deployed state",
                Infra,
                External,
            ),
            Some("bootstrap") => s(program, "Bootstrap the CDK environment", Infra, External),
            Some(x) => s(program, format!("Run {program} {x}"), Infra, Safe),
            None => s(program, "Run CDK", Infra, Safe),
        },
        "sam" => match sub {
            Some("build") => s("sam", "Build the SAM application", Build, Safe),
            Some("deploy") => s("sam", "Deploy the SAM application", Deploy, External),
            Some("local") => s("sam", "Run the SAM application locally", Dev, Safe),
            Some("delete") => s("sam", "Delete the SAM stack", Infra, Destructive),
            Some(x) => s("sam", format!("Run sam {x}"), Infra, Safe),
            None => s("sam", "Run SAM", Infra, Safe),
        },
        "serverless" | "sls" => match sub {
            Some("deploy") => s(
                "serverless",
                "Deploy with the Serverless Framework",
                Deploy,
                External,
            ),
            Some("remove") => s(
                "serverless",
                "Remove the Serverless deployment",
                Infra,
                Destructive,
            ),
            Some("offline") => s("serverless", "Run the Serverless app locally", Dev, Safe),
            Some("invoke") => s(
                "serverless",
                "Invoke a Serverless function",
                Infra,
                External,
            ),
            Some("logs") => s(
                "serverless",
                "Stream Serverless function logs",
                Infra,
                External,
            ),
            Some(x) => s("serverless", format!("Run serverless {x}"), Infra, Safe),
            None => s("serverless", "Run the Serverless Framework", Infra, Safe),
        },
        "ansible-playbook" => s(
            "ansible",
            format!(
                "Run the {} Ansible playbook",
                a.positionals()
                    .first()
                    .map(|p| clean_path(p))
                    .unwrap_or_default()
            )
            .trim()
            .to_string(),
            Deploy,
            if a.has("--check") { Safe } else { External },
        ),
        "ansible" | "ansible-galaxy" | "ansible-lint" => s(
            "ansible",
            if program == "ansible-lint" {
                "Lint Ansible playbooks"
            } else if program == "ansible-galaxy" {
                "Install Ansible roles and collections"
            } else {
                "Run Ansible ad-hoc commands"
            },
            if program == "ansible-lint" {
                Lint
            } else {
                Infra
            },
            if program == "ansible" { External } else { Safe },
        ),
        "packer" => s(
            "packer",
            if sub == Some("build") {
                "Build machine images with Packer"
            } else {
                "Run Packer"
            },
            Build,
            if sub == Some("build") { External } else { Safe },
        ),
        "vagrant" => s(
            "vagrant",
            format!(
                "{} the Vagrant VM",
                match sub {
                    Some("up") => "Start",
                    Some("halt") => "Stop",
                    Some("destroy") => "Destroy",
                    Some("ssh") => "Connect to",
                    Some("provision") => "Provision",
                    _ => "Manage",
                }
            ),
            Infra,
            if sub == Some("destroy") {
                Destructive
            } else {
                Safe
            },
        ),
        "aws" => {
            let p = a.positionals();
            let joined = p.join(" ");
            let destructive = ["delete", "terminate", "remove", "rm ", "purge", "rb "];
            let read = [
                "describe",
                "list",
                "get",
                "ls",
                "sts get-caller-identity",
                "logs tail",
            ];
            if destructive.iter().any(|d| joined.contains(d)) {
                s(
                    "aws",
                    format!(
                        "Delete AWS resources ({})",
                        p.first().copied().unwrap_or("")
                    ),
                    Infra,
                    Destructive,
                )
            } else if read.iter().any(|r| joined.contains(r)) {
                s(
                    "aws",
                    format!("Query AWS {}", p.first().copied().unwrap_or("resources")),
                    Infra,
                    External,
                )
            } else {
                s(
                    "aws",
                    format!(
                        "Call AWS {}",
                        p.iter().take(2).copied().collect::<Vec<_>>().join(" ")
                    ),
                    Infra,
                    External,
                )
            }
        }
        "gcloud" | "az" | "doctl" | "linode-cli" | "hcloud" | "oci" => {
            let p = a.positionals();
            let joined = p.join(" ");
            if joined.contains("delete") || joined.contains("destroy") || joined.contains("remove")
            {
                s(
                    program,
                    format!("Delete cloud resources with {program}"),
                    Infra,
                    Destructive,
                )
            } else if joined.starts_with("auth")
                || joined.starts_with("login")
                || joined.starts_with("config")
            {
                s(
                    program,
                    format!("Configure {program} authentication"),
                    Infra,
                    External,
                )
            } else {
                s(
                    program,
                    format!(
                        "Call {program} {}",
                        p.iter().take(2).copied().collect::<Vec<_>>().join(" ")
                    ),
                    Infra,
                    External,
                )
            }
        }
        "fly" | "flyctl" => match sub {
            Some("deploy") => s("fly", "Deploy the app to Fly.io", Deploy, External),
            Some("launch") => s("fly", "Create and deploy a Fly.io app", Deploy, External),
            Some("logs") => s("fly", "Stream Fly.io logs", Infra, External),
            Some("ssh") => s("fly", "Connect to the Fly.io machine", Infra, External),
            Some("scale") => s("fly", "Scale the Fly.io app", Infra, External),
            Some("destroy") | Some("apps") if a.has("destroy") || sub == Some("destroy") => {
                s("fly", "Destroy the Fly.io app", Infra, Destructive)
            }
            Some(x) => s("fly", format!("Run fly {x}"), Infra, External),
            None => s("fly", "Run flyctl", Infra, Safe),
        },
        "vercel" | "vc" => match sub {
            Some("dev") => s("vercel", "Run the app locally with Vercel", Dev, Safe),
            Some("build") => s("vercel", "Build the app for Vercel", Build, Safe),
            Some("deploy") | None => s(
                "vercel",
                if a.has("--prod") {
                    "Deploy to Vercel production"
                } else {
                    "Deploy a preview to Vercel"
                },
                Deploy,
                External,
            ),
            Some("pull") | Some("env") => {
                s("vercel", "Sync Vercel project settings", Infra, External)
            }
            Some("link") => s("vercel", "Link the Vercel project", Infra, External),
            Some(x) => s("vercel", format!("Run vercel {x}"), Infra, External),
        },
        "netlify" | "ntl" => match sub {
            Some("dev") => s("netlify", "Run the app locally with Netlify", Dev, Safe),
            Some("build") => s("netlify", "Build the app with Netlify", Build, Safe),
            Some("deploy") => s(
                "netlify",
                if a.has("--prod") {
                    "Deploy to Netlify production"
                } else {
                    "Deploy a preview to Netlify"
                },
                Deploy,
                External,
            ),
            Some(x) => s("netlify", format!("Run netlify {x}"), Infra, External),
            None => s("netlify", "Run Netlify", Infra, Safe),
        },
        "wrangler" => match sub {
            Some("dev") => s("wrangler", "Run the Cloudflare Worker locally", Dev, Safe),
            Some("deploy") | Some("publish") => {
                s("wrangler", "Deploy the Cloudflare Worker", Deploy, External)
            }
            Some("pages") => match a.positionals().get(1).copied() {
                Some("dev") => s("wrangler", "Run Cloudflare Pages locally", Dev, Safe),
                Some("deploy") | Some("publish") => {
                    s("wrangler", "Deploy to Cloudflare Pages", Deploy, External)
                }
                _ => s("wrangler", "Manage Cloudflare Pages", Infra, External),
            },
            Some("d1") => {
                let p = a.positionals();
                if p.contains(&"migrations") && p.contains(&"apply") {
                    s(
                        "wrangler",
                        if a.has("--local") {
                            "Apply D1 migrations locally"
                        } else {
                            "Apply D1 migrations"
                        },
                        Migrate,
                        if a.has("--local") { Safe } else { External },
                    )
                } else {
                    s(
                        "wrangler",
                        "Manage the D1 database",
                        Db,
                        if a.has("--local") { Safe } else { External },
                    )
                }
            }
            Some("tail") => s("wrangler", "Stream Cloudflare Worker logs", Infra, External),
            Some("delete") => s(
                "wrangler",
                "Delete the Cloudflare Worker",
                Infra,
                Destructive,
            ),
            Some("types") => s(
                "wrangler",
                "Generate Cloudflare Worker types",
                Generate,
                Safe,
            ),
            Some(x) => s("wrangler", format!("Run wrangler {x}"), Infra, External),
            None => s("wrangler", "Run Wrangler", Infra, Safe),
        },
        "firebase" => match sub {
            Some("deploy") => s("firebase", "Deploy to Firebase", Deploy, External),
            Some("emulators:start") => s("firebase", "Start the Firebase emulators", Dev, Safe),
            Some("serve") => s("firebase", "Serve the Firebase project locally", Dev, Safe),
            Some(x) => s("firebase", format!("Run firebase {x}"), Infra, External),
            None => s("firebase", "Run Firebase", Infra, Safe),
        },
        "supabase" => match sub {
            Some("start") => s("supabase", "Start the local Supabase stack", Dev, Safe),
            Some("stop") => s("supabase", "Stop the local Supabase stack", Infra, Safe),
            Some("db") => match a.positionals().get(1).copied() {
                Some("reset") => s(
                    "supabase",
                    "Reset the local Supabase database",
                    Db,
                    Destructive,
                ),
                Some("push") => s(
                    "supabase",
                    "Push migrations to the remote Supabase database",
                    Migrate,
                    External,
                ),
                Some("pull") => s("supabase", "Pull the remote Supabase schema", Db, External),
                Some("diff") => s("supabase", "Diff the Supabase schema", Db, Safe),
                _ => s("supabase", "Manage the Supabase database", Db, Safe),
            },
            Some("gen") => s("supabase", "Generate Supabase types", Generate, Safe),
            Some("functions") => match a.positionals().get(1).copied() {
                Some("serve") => s(
                    "supabase",
                    "Serve Supabase edge functions locally",
                    Dev,
                    Safe,
                ),
                Some("deploy") => s(
                    "supabase",
                    "Deploy Supabase edge functions",
                    Deploy,
                    External,
                ),
                _ => s("supabase", "Manage Supabase edge functions", Infra, Safe),
            },
            Some("migration") => s("supabase", "Manage Supabase migrations", Migrate, Safe),
            Some(x) => s("supabase", format!("Run supabase {x}"), Infra, Safe),
            None => s("supabase", "Run Supabase", Infra, Safe),
        },
        "heroku" => match sub {
            Some("local") => s("heroku", "Run the app locally with Heroku", Dev, Safe),
            Some("logs") => s("heroku", "Stream Heroku logs", Infra, External),
            Some("run") => s("heroku", "Run a command on Heroku", Infra, External),
            Some(x) => s("heroku", format!("Run heroku {x}"), Infra, External),
            None => s("heroku", "Run Heroku", Infra, Safe),
        },
        "railway" | "render" | "dokku" | "caprover" => s(
            program,
            format!("Run {program} {}", sub.unwrap_or(""))
                .trim()
                .to_string(),
            Infra,
            External,
        ),
        "gh" => match (sub, a.positionals().get(1).copied()) {
            (Some("release"), Some("create")) => {
                s("gh", "Create a GitHub release", Publish, External)
            }
            (Some("release"), Some("upload")) => {
                s("gh", "Upload assets to a GitHub release", Publish, External)
            }
            (Some("release"), Some("delete")) => {
                s("gh", "Delete a GitHub release", Publish, Destructive)
            }
            (Some("pr"), Some("create")) => s("gh", "Create a pull request", Other, External),
            (Some("pr"), Some("merge")) => s("gh", "Merge a pull request", Other, External),
            (Some("workflow"), Some("run")) => {
                s("gh", "Trigger a GitHub Actions workflow", Other, External)
            }
            (Some("run"), _) => s("gh", "Inspect GitHub Actions runs", Other, External),
            (Some("auth"), _) => s("gh", "Authenticate with GitHub", Other, External),
            (Some("api"), _) => s("gh", "Call the GitHub API", Other, External),
            (Some("repo"), Some("delete")) => {
                s("gh", "Delete a GitHub repository", Other, Destructive)
            }
            (Some(x), _) => s("gh", format!("Run gh {x}"), Other, External),
            (None, _) => s("gh", "Run the GitHub CLI", Other, Safe),
        },
        "git" => {
            let p = a.positionals();
            match sub {
                Some("push") => s(
                    "git",
                    if a.has("--force") || a.has("-f") || a.has("--force-with-lease") {
                        "Force-push commits to the remote"
                    } else if a.has("--tags") {
                        "Push tags to the remote"
                    } else {
                        "Push commits to the remote"
                    },
                    Publish,
                    if a.has("--force") || a.has("-f") {
                        Destructive
                    } else {
                        External
                    },
                ),
                Some("pull") | Some("fetch") => {
                    s("git", "Fetch changes from the remote", Other, External)
                }
                Some("clone") => s("git", "Clone a repository", Other, External),
                Some("clean") => s(
                    "git",
                    "Delete untracked files",
                    Clean,
                    if a.has("-x")
                        || a.has("-fdx")
                        || a.has("-xdf")
                        || a.has("-fxd")
                        || a.has("-dfx")
                        || a.has("-xfd")
                        || a.has("-dxf")
                    {
                        Destructive
                    } else {
                        Safe
                    },
                ),
                Some("reset") => s(
                    "git",
                    if a.has("--hard") {
                        "Discard all local changes"
                    } else {
                        "Reset the Git index"
                    },
                    Other,
                    if a.has("--hard") { Destructive } else { Safe },
                ),
                Some("checkout") | Some("restore") | Some("switch") => {
                    s("git", "Check out files or a branch", Other, Safe)
                }
                Some("tag") => s("git", "Create a Git tag", Other, Safe),
                Some("commit") => s("git", "Create a commit", Other, Safe),
                Some("add") => s("git", "Stage files", Other, Safe),
                Some("status") | Some("diff") | Some("log") | Some("rev-parse")
                | Some("describe") | Some("branch") | Some("show") | Some("ls-files") => {
                    s("git", "Inspect the Git repository", Other, Safe)
                }
                Some("submodule") => s(
                    "git",
                    if p.get(1) == Some(&"update") {
                        "Update Git submodules"
                    } else {
                        "Manage Git submodules"
                    },
                    Install,
                    External,
                ),
                Some("lfs") => s("git", "Manage Git LFS files", Install, Safe),
                Some("stash") => s("git", "Stash local changes", Other, Safe),
                Some("config") => s("git", "Change Git configuration", Other, Safe),
                Some("rebase") | Some("merge") | Some("cherry-pick") => {
                    s("git", format!("Run git {}", sub.unwrap()), Other, Safe)
                }
                Some("cliff") => s(
                    "git-cliff",
                    "Generate the changelog with git-cliff",
                    Docs,
                    Safe,
                ),
                Some(x) => s("git", format!("Run git {x}"), Other, Safe),
                None => s("git", "Run git", Other, Safe),
            }
        }
        "git-cliff" => s(
            "git-cliff",
            "Generate the changelog with git-cliff",
            Docs,
            Safe,
        ),
        "kafka-topics" | "kafka-topics.sh" => {
            if a.has("--delete") {
                s("kafka", "Delete Kafka topics", Infra, Destructive)
            } else if a.has("--create") {
                s(
                    "kafka",
                    format!(
                        "Create the {} Kafka topic",
                        a.value("--topic").unwrap_or("")
                    )
                    .trim()
                    .to_string(),
                    Infra,
                    External,
                )
            } else {
                s("kafka", "List Kafka topics", Infra, External)
            }
        }
        "kafka-console-producer" | "kafka-console-producer.sh" => s(
            "kafka",
            "Produce messages to a Kafka topic",
            Infra,
            External,
        ),
        "kafka-console-consumer" | "kafka-console-consumer.sh" => s(
            "kafka",
            "Consume messages from a Kafka topic",
            Infra,
            External,
        ),
        "kafka-consumer-groups" | "kafka-consumer-groups.sh" => s(
            "kafka",
            if a.has("--reset-offsets") {
                "Reset Kafka consumer group offsets"
            } else if a.has("--delete") {
                "Delete Kafka consumer groups"
            } else {
                "Inspect Kafka consumer groups"
            },
            Infra,
            if a.has("--reset-offsets") || a.has("--delete") {
                Destructive
            } else {
                External
            },
        ),
        "kcat" | "kafkacat" => s("kafka", "Interact with Kafka via kcat", Infra, External),
        "rpk" => {
            let p = a.positionals();
            if p.contains(&"delete") {
                s("redpanda", "Delete Redpanda resources", Infra, Destructive)
            } else if p.first() == Some(&"container") {
                s(
                    "redpanda",
                    "Manage the local Redpanda container",
                    Infra,
                    Safe,
                )
            } else {
                s(
                    "redpanda",
                    "Interact with Redpanda via rpk",
                    Infra,
                    External,
                )
            }
        }
        "mkcert" => s("mkcert", "Create local TLS certificates", Other, Safe),
        "ngrok" | "cloudflared" => s(
            program,
            "Expose a local port through a tunnel",
            Infra,
            External,
        ),
        "stripe" => s("stripe", "Run the Stripe CLI", Infra, External),
        "sentry-cli" => s("sentry-cli", "Upload to Sentry", Publish, External),
        "python-module:http.server" => {
            s("python", "Serve the current directory over HTTP", Dev, Safe)
        }
        _ => None,
    }
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
