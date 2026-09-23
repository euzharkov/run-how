//! Rust: one slice of the tool knowledge table behind `analyze::tools::summarize`.

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

        _ => None,
    }
}
