//! Snapshot tests over the fixture repositories in `fixtures/`.
//!
//! Each fixture is discovered with runtime probing disabled and a fixed host OS so the
//! normalised model is identical on every platform.

use rhow::ui::debug_table;
use rhow::{discover, Options};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn snapshot(name: &str) {
    let root = fixture(name);
    let repo = discover(
        &root,
        &Options {
            probe_runtime: false,
            host_os: "linux",
        },
    );
    let table = debug_table(&repo);
    insta::with_settings!({ snapshot_path => "snapshots", prepend_module_to_snapshot => false }, {
        insta::assert_snapshot!(name, table);
    });
}

macro_rules! fixture_tests {
    ($($name:ident),* $(,)?) => {
        $( #[test] fn $name() { snapshot(stringify!($name).replace('_', "-").as_str()); } )*
    };
}

fixture_tests!(
    npm_basic,
    npm_workspaces,
    pnpm_monorepo,
    yarn_workspace,
    bun,
    make,
    just,
    taskfile,
    python_uv,
    python_django,
    python_poetry,
    go,
    cargo,
    cargo_workspace,
    dotnet,
    docker_compose,
    kubernetes,
    helm,
    polyglot,
    tricky,
    empty,
);

#[test]
fn ids_are_unique_and_json_roundtrips() {
    for name in ["polyglot", "tricky", "dotnet", "pnpm-monorepo"] {
        let repo = discover(
            &fixture(name),
            &Options {
                probe_runtime: false,
                host_os: "linux",
            },
        );
        let mut ids: Vec<&str> = repo
            .projects
            .iter()
            .flat_map(|p| p.actions.iter())
            .map(|a| a.id.as_str())
            .collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(n, ids.len(), "duplicate ids in {name}");
        let json = rhow::ui::json::render(&repo);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["projects"].is_array());
    }
}

#[test]
fn windows_host_prefers_windows_scripts() {
    let repo = discover(
        &fixture("dotnet"),
        &Options {
            probe_runtime: false,
            host_os: "windows",
        },
    );
    let build_cmd = repo.projects[0]
        .actions
        .iter()
        .find(|a| a.id == "cmd:build")
        .expect("build.cmd");
    assert_eq!(build_cmd.confidence, rhow::Confidence::Medium);
    let ps = repo.projects[0]
        .actions
        .iter()
        .find(|a| a.tool == "pwsh" && a.name == "build")
        .unwrap();
    assert!(ps.command.starts_with("powershell"));
}

#[test]
fn discovery_is_read_only() {
    // Walk the fixture tree before and after discovery and compare modification times.
    fn stamp(dir: &std::path::Path, out: &mut Vec<(PathBuf, std::time::SystemTime)>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            out.push((p.clone(), e.metadata().unwrap().modified().unwrap()));
            if p.is_dir() {
                stamp(&p, out);
            }
        }
    }
    let root = fixture("polyglot");
    let mut before = Vec::new();
    stamp(&root, &mut before);
    let _ = discover(&root, &Options::default());
    let mut after = Vec::new();
    stamp(&root, &mut after);
    assert_eq!(before, after);
}

