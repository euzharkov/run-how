//! bundlers/compilers: one slice of the tool knowledge table behind `analyze::tools::summarize`.

#[allow(unused_imports)]
use super::Kind::*;
#[allow(unused_imports)]
use super::{clean_path, compose, is_build_output, list};
use super::{s, Args, Summary};
#[allow(unused_imports)]
use crate::model::Risk::*;

#[allow(unused_variables, clippy::needless_return)]
pub(super) fn summarize(
    program: &str,
    args: &[String],
    a: &Args,
    sub: Option<&str>,
) -> Option<Summary> {
    match program {
        "tsc" | "tsgo" => {
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
            // `-t a b c` / `--targets=a,b`: every target named, not just the first.
            let targets: Vec<&str> = {
                let mut v: Vec<&str> = Vec::new();
                let mut it = args.iter().map(String::as_str).peekable();
                while let Some(x) = it.next() {
                    if x == "-t" || x == "--target" || x == "--targets" {
                        while let Some(n) = it.peek() {
                            if n.starts_with('-') {
                                break;
                            }
                            v.extend(it.next().unwrap().split(','));
                        }
                    } else if let Some(rest) = x
                        .strip_prefix("--target=")
                        .or_else(|| x.strip_prefix("--targets="))
                        .or_else(|| x.strip_prefix("-t="))
                    {
                        v.extend(rest.split(','));
                    }
                }
                v
            };
            let target = targets.first().copied();
            let listed = list(&targets);
            // Old-style `nx affected:test`, `nx format:write`.
            if let Some(s2) = sub {
                if let Some(t) = s2.strip_prefix("affected:") {
                    let what = match t {
                        "apps" | "libs" => {
                            return s("nx", format!("List affected Nx {t}"), Other, Safe)
                        }
                        "dep-graph" | "graph" => {
                            return s("nx", "Show the affected Nx project graph", Other, Safe)
                        }
                        _ => t,
                    };
                    return s(
                        "nx",
                        format!("Run {what} for affected Nx projects"),
                        crate::explain::name_hint(what)
                            .map(|h| h.kind)
                            .unwrap_or(Other),
                        Safe,
                    );
                }
                match s2 {
                    "format:write" => {
                        return s(
                            "nx",
                            "Format source files with Prettier via Nx",
                            Format,
                            Safe,
                        )
                    }
                    "format:check" => {
                        return s("nx", "Check formatting with Prettier via Nx", Format, Safe)
                    }
                    "migrate" => return s("nx", "Update Nx and its plugins", Install, Safe),
                    "graph" | "dep-graph" => {
                        return s("nx", "Open the Nx project graph", Other, Safe)
                    }
                    "workspace-lint" => {
                        return s("nx", "Lint the Nx workspace configuration", Lint, Safe)
                    }
                    "reset" => return s("nx", "Clear the Nx cache and daemon", Clean, Safe),
                    "release" => return s("nx", "Cut a release with Nx", Publish, External),
                    _ => {}
                }
            }
            match (sub, target) {
                (Some("run-many"), Some(t)) => s(
                    "nx",
                    format!("Run {listed} across Nx projects"),
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
                (Some("run"), _) => {
                    // `nx run owner:lint`; a `{projectName}` placeholder means a template
                    // inside targetDefaults, which says nothing on its own.
                    let spec = a.positionals().get(1).copied().unwrap_or("");
                    if spec.is_empty() || spec.contains('{') {
                        return None;
                    }
                    let (project, tgt) = spec.split_once(':').unwrap_or(("", spec));
                    s(
                        "nx",
                        if project.is_empty() {
                            format!("Run the {tgt} target with Nx")
                        } else {
                            format!("Run the {tgt} target of {project} with Nx")
                        },
                        crate::explain::name_hint(tgt)
                            .map(|h| h.kind)
                            .unwrap_or(Other),
                        Safe,
                    )
                }
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

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_sum as sum;
    use super::*;

    #[test]
    fn typescript_and_bundlers() {
        assert_eq!(sum("tsc --noEmit").kind, TypeCheck);
        assert_eq!(sum("tsc -b").text, "Build TypeScript project references");
        assert_eq!(sum("tsc -w").kind, Dev);
        assert_eq!(sum("tsup src/index.ts").text, "Build the package with tsup");
        assert_eq!(sum("tsup --watch").text, "Build with tsup in watch mode");
        assert_eq!(sum("esbuild src/index.ts --bundle").kind, Build);
    }

    #[test]
    fn monorepo_runners_and_release_tools() {
        assert_eq!(
            sum("turbo run build").text,
            "Run build across packages with Turborepo"
        );
        assert_eq!(sum("turbo test").kind, Test);
        assert_eq!(sum("np").risk, External);
        assert_eq!(sum("semantic-release").risk, External);
        assert_eq!(sum("changeset version").risk, Safe);
    }
}
