//! End-to-end tests of the `rhow` binary: flag combinations, JSON envelopes and the
//! stderr hints. Everything runs against `fixtures/` or a throwaway temporary directory;
//! nothing is written into the repository.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn rhow(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rhow"))
        .args(["--no-runtime", "--color", "never"])
        .args(args)
        .output()
        .expect("run rhow")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn json(o: &Output) -> serde_json::Value {
    serde_json::from_str(&stdout(o)).unwrap_or_else(|e| panic!("{e}: {}", stdout(o)))
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("rhow-cli-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn hint_when_no_git_repository_above_cwd() {
    let dir = TempDir::new("nogit");
    std::fs::write(
        dir.0.join("package.json"),
        r#"{"name":"t","scripts":{"test":"vitest run"}}"#,
    )
    .unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rhow"))
            .args(["--no-runtime", "--color", "never"])
            .args(args)
            .current_dir(&dir.0)
            .output()
            .unwrap()
    };

    let o = run(&[]);
    assert!(o.status.success());
    let err = stderr(&o);
    assert!(
        err.starts_with("rhow: no git repository above ")
            && err.contains("(use `rhow <dir>` to pick a directory)"),
        "{err:?}"
    );
    assert_eq!(err.lines().count(), 1, "{err:?}");
    assert!(stdout(&o).contains("npm run test"), "{}", stdout(&o));

    // The hint is for the listing only: JSON consumers and `support` get a clean stderr.
    assert_eq!(stderr(&run(&["--json"])), "");
    assert_eq!(stderr(&run(&["support"])), "");

    // With a repository the walk finds one and stays quiet.
    std::fs::create_dir(dir.0.join(".git")).unwrap();
    assert_eq!(stderr(&run(&[])), "");

    // `-C` asks for exactly that directory: no hint even without a `.git`.
    std::fs::remove_dir(dir.0.join(".git")).unwrap();
    let o = rhow(&["-C", dir.0.to_str().unwrap()]);
    assert!(o.status.success());
    assert_eq!(stderr(&o), "");
}

#[test]
fn group_with_json_is_refused() {
    let o = rhow(&[
        "-C",
        fixture("polyglot").to_str().unwrap(),
        "--group",
        "--json",
    ]);
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(stdout(&o), "");
    assert!(stderr(&o).contains("--group"), "{}", stderr(&o));
}

#[test]
fn json_envelope_has_a_schema_and_every_action() {
    let o = rhow(&["-C", fixture("polyglot").to_str().unwrap(), "--json"]);
    assert!(o.status.success());
    let v = json(&o);
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys[0], "schema");
    assert_eq!(v["schema"], 2);
    assert!(v["root"].as_str().unwrap().ends_with("polyglot"));
    let actions: Vec<&serde_json::Value> = v["projects"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["actions"].as_array().unwrap())
        .collect();
    // Nothing is hidden: the document holds exactly the actions the listing shows, one row
    // each, and there is no flag to reveal more.
    assert!(actions.iter().all(|a| a.get("hidden").is_none()));
    assert!(actions.iter().all(|a| a["opaque"].is_boolean()));
    assert!(actions
        .iter()
        .all(|a| !a["working_directory"].as_str().unwrap().contains('\\')));
    let listing = stdout(&rhow(&[fixture("polyglot").to_str().unwrap()]));
    let rows = listing
        .lines()
        .filter(|l| l.starts_with("  ") && !l.starts_with("   "))
        .count();
    assert_eq!(rows, actions.len(), "{listing}");
    let all = rhow(&[fixture("polyglot").to_str().unwrap(), "--all"]);
    assert_eq!(all.status.code(), Some(2));
}

#[test]
fn ci_json_is_only_the_pipelines() {
    let o = rhow(&["-C", fixture("ci").to_str().unwrap(), "--ci", "--json"]);
    assert!(o.status.success());
    let v = json(&o);
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["schema", "version", "ci"]);
    let files: Vec<&str> = v["ci"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["file"].as_str().unwrap())
        .collect();
    assert_eq!(files, [".github/workflows/ci.yml", ".gitlab-ci.yml"]);
}

#[test]
fn the_argument_is_a_directory_or_support() {
    let dir = fixture("polyglot");
    let d = dir.to_str().unwrap();
    // `rhow DIR` inspects exactly that directory; `-C DIR` is the same, kept for muscle memory.
    let by_arg = rhow(&[d]);
    assert!(by_arg.status.success(), "{}", stderr(&by_arg));
    assert!(
        stdout(&by_arg).starts_with("polyglot"),
        "{}",
        stdout(&by_arg)
    );
    assert_eq!(stdout(&rhow(&["-C", d])), stdout(&by_arg));
    assert_eq!(
        stdout(&rhow(&["--json", d])),
        stdout(&rhow(&["--json", "-C", d]))
    );
    // `support` takes the directory too, flags before or after.
    let support = rhow(&["support", d]);
    assert!(support.status.success(), "{}", stderr(&support));
    assert_eq!(stdout(&rhow(&["support", "-C", d])), stdout(&support));
    assert!(rhow(&["support", d, "--json"]).status.success());
    // There is no per-action view: an id or a row number is just a directory that is not
    // there, and anything that is not one directory is refused. Nothing reaches stdout.
    for args in [
        &["dev"][..],
        &["1"],
        &["why", "dev"],
        &[d, "support"],
        &[d, "-C", d],
    ] {
        let o = rhow(args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert_eq!(stdout(&o), "", "{args:?}");
    }
    assert!(stderr(&rhow(&["dev"])).contains("dev is not a directory"));
}
