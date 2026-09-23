//! JS quality: one slice of the tool knowledge table behind `analyze::tools::summarize`.

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
            // Skip the values of flags that take one (`--project tsconfig.json`), so the
            // script file is the first real positional.
            let value_flags = ["--project", "-P", "-r", "--require", "--loader", "--import"];
            let mut file = None;
            let mut it = args.iter().map(String::as_str);
            while let Some(x) = it.next() {
                if value_flags.contains(&x) {
                    it.next();
                } else if !x.starts_with('-') && x != "watch" {
                    file = Some(x);
                    break;
                }
            }
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

        _ => None,
    }
}
