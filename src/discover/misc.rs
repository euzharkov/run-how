//! Smaller stacks that fit one adapter each: Elixir Mix, Zig, CMake, Haskell (Stack / Cabal),
//! Scala sbt, Clojure (deps.edn / Leiningen), Nim, OCaml dune.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Misc;

const MARKERS: &[&str] = &[
    "mix.exs",
    "build.zig",
    "CMakeLists.txt",
    "stack.yaml",
    "cabal.project",
    "build.sbt",
    "deps.edn",
    "project.clj",
    "dune-project",
];

impl Discoverer for Misc {
    fn id(&self) -> &'static str {
        "misc"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Other
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has_any(MARKERS)
            || !dir.with_ext("cabal").is_empty()
            || !dir.with_ext("nimble").is_empty()
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let push = |out: &mut Discovery, a: Action| out.actions.push(a);

        // ---- Elixir -----------------------------------------------------------------------
        if let Some(mix) = base.read("mix.exs") {
            let phoenix = mix.contains(":phoenix");
            let ecto = mix.contains(":ecto");
            if phoenix {
                push(
                    &mut out,
                    Action::new("dev", "mix phx.server")
                        .tool("mix")
                        .inferred(Confidence::High),
                );
            } else if mix.contains("mod:") {
                push(
                    &mut out,
                    Action::new("run", "mix run --no-halt")
                        .tool("mix")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Run the Elixir application")
                        .cat(Category::Development),
                );
            }
            push(
                &mut out,
                Action::new("test", "mix test")
                    .tool("mix")
                    .inferred(Confidence::High),
            );
            push(
                &mut out,
                Action::new("format", "mix format")
                    .tool("mix")
                    .inferred(Confidence::High),
            );
            push(
                &mut out,
                Action::new("compile", "mix compile")
                    .tool("mix")
                    .inferred(Confidence::Medium),
            );
            if mix.contains(":credo") {
                push(
                    &mut out,
                    Action::new("lint", "mix credo --strict")
                        .tool("mix")
                        .inferred(Confidence::High),
                );
            }
            if mix.contains(":dialyxir") {
                push(
                    &mut out,
                    Action::new("typecheck", "mix dialyzer")
                        .tool("mix")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Run Dialyzer type analysis")
                        .cat(Category::Quality),
                );
            }
            if ecto {
                push(
                    &mut out,
                    Action::new("migrate", "mix ecto.migrate")
                        .tool("mix")
                        .inferred(Confidence::High),
                );
                push(
                    &mut out,
                    Action::new("db:setup", "mix ecto.setup")
                        .tool("mix")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Create, migrate and seed the database")
                        .cat(Category::Database),
                );
                push(
                    &mut out,
                    Action::new("db:reset", "mix ecto.reset")
                        .tool("mix")
                        .inferred(Confidence::Low),
                );
            }
            push(
                &mut out,
                Action::new("deps", "mix deps.get")
                    .tool("mix")
                    .inferred(Confidence::Low)
                    .inferred_desc("Fetch Elixir dependencies")
                    .cat(Category::Other),
            );
            push(
                &mut out,
                Action::new("shell", "iex -S mix")
                    .tool("mix")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Open an IEx shell with the app loaded")
                    .cat(Category::Development),
            );
        }

        // ---- Zig ----------------------------------------------------------------------------
        if let Some(bz) = base.read("build.zig") {
            push(
                &mut out,
                Action::new("build", "zig build")
                    .tool("zig")
                    .inferred(Confidence::High)
                    .inferred_desc("Build with Zig")
                    .cat(Category::Build),
            );
            if bz.contains("\"run\"") || bz.contains("addRunArtifact") {
                push(
                    &mut out,
                    Action::new("run", "zig build run")
                        .tool("zig")
                        .inferred(Confidence::High)
                        .inferred_desc("Build and run the Zig program")
                        .cat(Category::Development),
                );
            }
            if bz.contains("\"test\"") || bz.contains("addTest") {
                push(
                    &mut out,
                    Action::new("test", "zig build test")
                        .tool("zig")
                        .inferred(Confidence::High)
                        .inferred_desc("Run Zig tests")
                        .cat(Category::Testing),
                );
            }
            push(
                &mut out,
                Action::new("fmt", "zig fmt .")
                    .tool("zig")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Format Zig source")
                    .cat(Category::Quality),
            );
        }

        // ---- CMake --------------------------------------------------------------------------
        if let Some(cm) = base.read("CMakeLists.txt") {
            // `--preset <name>` only for a preset the project's `CMakePresets.json` names;
            // a build or test preset is used only when one of that name exists too.
            let presets = ctx.json(base, "CMakePresets.json");
            let names = |kind: &str| -> Vec<String> {
                presets
                    .as_ref()
                    .and_then(|p| p.get(kind))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter(|p| !p.get("hidden").and_then(|h| h.as_bool()).unwrap_or(false))
                            .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let configure = names("configurePresets");
            let preset = configure
                .iter()
                .find(|n| *n == "default")
                .or_else(|| configure.first())
                .cloned();
            if let Some(preset) = preset {
                push(
                    &mut out,
                    Action::new("configure", format!("cmake --preset {preset}"))
                        .tool("cmake")
                        .inferred(Confidence::High)
                        .inferred_desc(format!("Configure with the {preset} CMake preset"))
                        .cat(Category::Build),
                );
                if names("buildPresets").contains(&preset) {
                    push(
                        &mut out,
                        Action::new("build", format!("cmake --build --preset {preset}"))
                            .tool("cmake")
                            .inferred(Confidence::High)
                            .inferred_desc(format!("Build with the {preset} CMake preset"))
                            .cat(Category::Build),
                    );
                } else {
                    push(
                        &mut out,
                        Action::new("build", "cmake --build build")
                            .tool("cmake")
                            .inferred(Confidence::Medium)
                            .inferred_desc("Build the CMake project")
                            .cat(Category::Build),
                    );
                }
                if names("testPresets").contains(&preset) {
                    push(
                        &mut out,
                        Action::new("test", format!("ctest --preset {preset}"))
                            .tool("cmake")
                            .inferred(Confidence::High)
                            .inferred_desc(format!("Run CTest with the {preset} preset"))
                            .cat(Category::Testing),
                    );
                }
            } else {
                push(
                    &mut out,
                    Action::new("configure", "cmake -S . -B build")
                        .tool("cmake")
                        .inferred(Confidence::High)
                        .inferred_desc("Configure the CMake project into build/")
                        .cat(Category::Build),
                );
                push(
                    &mut out,
                    Action::new("build", "cmake --build build")
                        .tool("cmake")
                        .inferred(Confidence::High)
                        .inferred_desc("Build the CMake project")
                        .cat(Category::Build),
                );
                if cm.contains("enable_testing")
                    || cm.contains("add_test")
                    || cm.contains("include(CTest)")
                {
                    push(
                        &mut out,
                        Action::new("test", "ctest --test-dir build")
                            .tool("cmake")
                            .inferred(Confidence::High)
                            .inferred_desc("Run CTest tests")
                            .cat(Category::Testing),
                    );
                }
            }
            if base.has(".clang-format") {
                push(
                    &mut out,
                    Action::new(
                        "format",
                        "find . -name '*.cpp' -o -name '*.h' | xargs clang-format -i",
                    )
                    .tool("cmake")
                    .inferred(Confidence::Low)
                    .inferred_desc("Format C/C++ sources with clang-format")
                    .cat(Category::Quality),
                );
            }
        }

        // ---- Haskell ------------------------------------------------------------------------
        if base.has("stack.yaml") {
            push(
                &mut out,
                Action::new("build", "stack build")
                    .tool("stack")
                    .inferred(Confidence::High)
                    .inferred_desc("Build with Stack")
                    .cat(Category::Build),
            );
            push(
                &mut out,
                Action::new("test", "stack test")
                    .tool("stack")
                    .inferred(Confidence::High)
                    .inferred_desc("Run tests with Stack")
                    .cat(Category::Testing),
            );
            push(
                &mut out,
                Action::new("run", "stack run")
                    .tool("stack")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Run the executable with Stack")
                    .cat(Category::Development),
            );
            push(
                &mut out,
                Action::new("repl", "stack ghci")
                    .tool("stack")
                    .inferred(Confidence::Low)
                    .inferred_desc("Open GHCi with the project loaded")
                    .cat(Category::Development),
            );
        } else if base.has("cabal.project") || !base.with_ext("cabal").is_empty() {
            push(
                &mut out,
                Action::new("build", "cabal build")
                    .tool("cabal")
                    .inferred(Confidence::High)
                    .inferred_desc("Build with Cabal")
                    .cat(Category::Build),
            );
            push(
                &mut out,
                Action::new("test", "cabal test")
                    .tool("cabal")
                    .inferred(Confidence::High)
                    .inferred_desc("Run tests with Cabal")
                    .cat(Category::Testing),
            );
            push(
                &mut out,
                Action::new("run", "cabal run")
                    .tool("cabal")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Run the executable with Cabal")
                    .cat(Category::Development),
            );
        }
        if base.has(".hlint.yaml") {
            push(
                &mut out,
                Action::new("lint", "hlint .")
                    .tool("haskell")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Check Haskell source with HLint")
                    .cat(Category::Quality),
            );
        }

        // ---- Scala ----------------------------------------------------------------------------
        if base.has("build.sbt") {
            push(
                &mut out,
                Action::new("compile", "sbt compile")
                    .tool("sbt")
                    .inferred(Confidence::High)
                    .inferred_desc("Compile with sbt")
                    .cat(Category::Build),
            );
            push(
                &mut out,
                Action::new("test", "sbt test")
                    .tool("sbt")
                    .inferred(Confidence::High)
                    .inferred_desc("Run tests with sbt")
                    .cat(Category::Testing),
            );
            push(
                &mut out,
                Action::new("run", "sbt run")
                    .tool("sbt")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Run the main class with sbt")
                    .cat(Category::Development),
            );
            if base.has(".scalafmt.conf") {
                push(
                    &mut out,
                    Action::new("format", "sbt scalafmtAll")
                        .tool("sbt")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Format Scala source with scalafmt")
                        .cat(Category::Quality),
                );
            }
            push(
                &mut out,
                Action::new("package", "sbt package")
                    .tool("sbt")
                    .inferred(Confidence::Low)
                    .inferred_desc("Package the JAR with sbt")
                    .cat(Category::Build),
            );
        }

        // ---- Clojure --------------------------------------------------------------------------
        if let Some(edn) = base.read("deps.edn") {
            // :aliases {:test {...} :dev {...}}
            if let Some(idx) = edn.find(":aliases") {
                let tail = &edn[idx + 8..];
                let mut depth = 0i32;
                let mut names: Vec<String> = Vec::new();
                let mut cur = String::new();
                let mut in_key = false;
                for c in tail.chars() {
                    match c {
                        '{' => {
                            depth += 1;
                            if depth == 1 {
                                continue;
                            }
                        }
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    if depth == 1 {
                        if c == ':' {
                            in_key = true;
                            cur.clear();
                            continue;
                        }
                        if in_key {
                            if c.is_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.' {
                                cur.push(c);
                            } else {
                                if !cur.is_empty() {
                                    names.push(cur.clone());
                                }
                                in_key = false;
                            }
                        }
                    }
                }
                for n in names {
                    let hint = crate::explain::name_hint(&n);
                    let flag = if edn.contains(&format!(":{n} {{:exec-fn"))
                        || edn.contains(&format!(":{n}\n")) && edn.contains(":exec-fn")
                    {
                        "-X"
                    } else {
                        "-M"
                    };
                    let mut a = Action::new(&n, format!("clojure {flag}:{n}"))
                        .tool("clojure")
                        .inferred(Confidence::Medium)
                        .inferred_desc(format!("Run the {n} deps.edn alias"));
                    if let Some(h) = hint {
                        a = a.cat(h.category);
                    }
                    push(&mut out, a);
                }
            }
            push(
                &mut out,
                Action::new("repl", "clojure -M -r")
                    .tool("clojure")
                    .inferred(Confidence::Low)
                    .inferred_desc("Start a Clojure REPL")
                    .cat(Category::Development),
            );
        }
        if base.has("project.clj") {
            push(
                &mut out,
                Action::new("test", "lein test")
                    .tool("lein")
                    .inferred(Confidence::High)
                    .inferred_desc("Run tests with Leiningen")
                    .cat(Category::Testing),
            );
            push(
                &mut out,
                Action::new("run", "lein run")
                    .tool("lein")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Run the app with Leiningen")
                    .cat(Category::Development),
            );
            push(
                &mut out,
                Action::new("repl", "lein repl")
                    .tool("lein")
                    .inferred(Confidence::Low)
                    .inferred_desc("Start a REPL with Leiningen")
                    .cat(Category::Development),
            );
            push(
                &mut out,
                Action::new("uberjar", "lein uberjar")
                    .tool("lein")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Build the standalone JAR")
                    .cat(Category::Build),
            );
        }

        // ---- Nim / OCaml ---------------------------------------------------------------------
        if !base.with_ext("nimble").is_empty() {
            push(
                &mut out,
                Action::new("build", "nimble build")
                    .tool("nimble")
                    .inferred(Confidence::High)
                    .inferred_desc("Build with Nimble")
                    .cat(Category::Build),
            );
            push(
                &mut out,
                Action::new("test", "nimble test")
                    .tool("nimble")
                    .inferred(Confidence::High)
                    .inferred_desc("Run tests with Nimble")
                    .cat(Category::Testing),
            );
        }
        if base.has("dune-project") {
            push(
                &mut out,
                Action::new("build", "dune build")
                    .tool("dune")
                    .inferred(Confidence::High)
                    .inferred_desc("Build with dune")
                    .cat(Category::Build),
            );
            push(
                &mut out,
                Action::new("test", "dune test")
                    .tool("dune")
                    .inferred(Confidence::High)
                    .inferred_desc("Run tests with dune")
                    .cat(Category::Testing),
            );
            push(
                &mut out,
                Action::new("format", "dune fmt")
                    .tool("dune")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Format OCaml source with dune")
                    .cat(Category::Quality),
            );
        }
        out
    }
}
