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
    ruby_rails,
    mobile_maestro,
    gradle,
    maven,
    php_laravel,
    php_composer,
    nx,
    turborepo,
    terraform,
    pulumi,
    ansible,
    bazel,
    deno,
    expo,
    flutter,
    ios_app,
    swiftpm,
    elixir,
    zig,
    cmake,
    haskell,
    scala,
    clojure,
    noise,
    many,
    ci,
    ruby_gem,
    go_nested,
    terraform_modules,
    terragrunt,
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
        assert_eq!(v["schema"], 1);
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
    // Walk the fixture tree before and after discovery: the same entries, with the same
    // sizes and modification times. A file created, removed, rewritten or touched by
    // discovery would show up in one of the three.
    #[derive(Debug, PartialEq)]
    struct Entry {
        path: PathBuf,
        is_dir: bool,
        len: u64,
        modified: std::time::SystemTime,
    }
    fn stamp(dir: &std::path::Path, out: &mut Vec<Entry>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let m = e.metadata().unwrap();
            out.push(Entry {
                path: p.clone(),
                is_dir: m.is_dir(),
                len: m.len(),
                modified: m.modified().unwrap(),
            });
            if m.is_dir() {
                stamp(&p, out);
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
    }
    for name in ["polyglot", "ci"] {
        let root = fixture(name);
        let mut before = Vec::new();
        stamp(&root, &mut before);
        assert!(!before.is_empty());
        let _ = discover(&root, &Options::default());
        let mut after = Vec::new();
        stamp(&root, &mut after);
        let files = |v: &[Entry]| v.iter().map(|e| e.path.clone()).collect::<Vec<_>>();
        assert_eq!(files(&before), files(&after), "{name}: entries changed");
        assert_eq!(before, after, "{name}: sizes or mtimes changed");
    }
}

#[test]
fn declared_versions_are_extracted_and_compared() {
    use rhow::support::{self, Compat};

    // The existing fixtures declare versions at or below what `support::REGISTRY` has been
    // verified against, so nothing here should read as "newer".
    let checks: &[(&str, &[(&str, &str)])] = &[
        ("cargo", &[("cargo-edition", "2021")]),
        ("go", &[("go", "1.22")]),
        ("dotnet", &[("dotnet-tfm", "net8.0")]),
        ("taskfile", &[("taskfile", "3")]),
        ("terraform", &[("terraform", ">= 1.6")]),
        ("pnpm-monorepo", &[("pnpm", "9.12.0")]),
        ("php-composer", &[("php", "^8.1")]),
        ("ruby-rails", &[("ruby", "3.2.0")]),
        ("npm-basic", &[("npm", "10.5.0")]),
        ("yarn-workspace", &[("yarn", "4.1.0")]),
        ("bun", &[("bun", "1.1.0")]),
        ("gradle", &[("gradle", "8.9")]),
        ("bazel", &[("bazel", "7.1.0")]),
    ];
    for (fixture_name, expected) in checks {
        let repo = discover(
            &fixture(fixture_name),
            &Options {
                probe_runtime: false,
                host_os: "linux",
            },
        );
        let findings = support::check(&repo);
        for (tool, value) in *expected {
            let f = findings
                .iter()
                .find(|f| f.tool == *tool)
                .unwrap_or_else(|| {
                    panic!("{fixture_name}: no `{tool}` finding (got {findings:?})")
                });
            assert_eq!(f.value, *value, "{fixture_name}: {tool} value");
            assert_eq!(
                f.compat,
                Compat::Supported,
                "{fixture_name}: {tool} = {} should be Supported",
                f.value
            );
        }
    }
}

#[test]
fn newer_than_verified_versions_are_flagged() {
    use rhow::support::{self, Compat};

    let repo = discover(
        &fixture("version-drift"),
        &Options {
            probe_runtime: false,
            host_os: "linux",
        },
    );
    let findings = support::check(&repo);
    let expect: &[(&str, &str)] = &[
        ("cargo-edition", "2027"),
        ("go", "1.28"),
        ("dotnet-tfm", "net11.0"),
        ("taskfile", "4"),
        ("python", ">=3.16"),
        ("node", ">=26"),
        ("pnpm", "11.0.0"),
        ("php", "^8.6"),
        ("ruby", "4.1.0"),
        ("bazel", "9.0.0"),
        ("gradle", "10.0"),
        ("terraform", ">= 2.0"),
    ];
    for (tool, value) in expect {
        let f = findings
            .iter()
            .find(|f| f.tool == *tool)
            .unwrap_or_else(|| panic!("version-drift: no `{tool}` finding (got {findings:?})"));
        assert_eq!(f.value, *value, "version-drift: {tool} value");
        assert_eq!(
            f.compat,
            Compat::Newer,
            "version-drift: {tool} = {} should read Newer",
            f.value
        );
    }
    // Every declared version in this fixture is deliberately past the baseline: nothing should
    // slip through as Supported, and none of these forms should be ambiguous either.
    assert!(
        findings.iter().all(|f| f.compat == Compat::Newer),
        "unexpected non-Newer finding: {findings:?}"
    );
}
