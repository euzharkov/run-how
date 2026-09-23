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

/// Global flags go first: after `why <command>` the rest of the line is the command.
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
            && err.contains("(use -C to pick a directory)"),
        "{err:?}"
    );
    assert_eq!(err.lines().count(), 1, "{err:?}");
    assert!(stdout(&o).contains("npm run test"), "{}", stdout(&o));

    // The hint is for the listing only: JSON consumers and `why` get a clean stderr.
    assert_eq!(stderr(&run(&["--json"])), "");
    assert_eq!(stderr(&run(&["test"])), "");
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
    assert_eq!(v["schema"], 1);
    assert!(v["root"].as_str().unwrap().ends_with("polyglot"));
    let actions: Vec<&serde_json::Value> = v["projects"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["actions"].as_array().unwrap())
        .collect();
    // Hidden actions are in the document, flagged, so `--all` changes nothing here.
    assert!(actions.iter().any(|a| a["hidden"] == true));
    assert!(actions.iter().all(|a| a["opaque"].is_boolean()));
    assert!(actions
        .iter()
        .all(|a| !a["working_directory"].as_str().unwrap().contains('\\')));
    let all = rhow(&[
        "-C",
        fixture("polyglot").to_str().unwrap(),
        "--json",
        "--all",
    ]);
    assert_eq!(stdout(&all), stdout(&o));
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
fn one_action_as_json() {
    let dir = fixture("polyglot");
    let o = rhow(&["-C", dir.to_str().unwrap(), "dev", "--json"]);
    assert!(o.status.success(), "{}", stderr(&o));
    let v = json(&o);
    assert_eq!(v["schema"], 1);
    assert_eq!(v["project"], ".");
    assert_eq!(v["action"]["id"], "dev");
    assert_eq!(v["action"]["command"], "pnpm run dev");
    // The script body the command resolves to, as the terminal view's `runs` line.
    assert!(v["runs"].as_str().is_some(), "{v}");
    let why = rhow(&["-C", dir.to_str().unwrap(), "why", "dev", "--json"]);
    assert_eq!(stdout(&why), stdout(&o));
    // The exact command text works as a key too, as it does without --json.
    let by_cmd = rhow(&["-C", dir.to_str().unwrap(), "why", "pnpm run dev", "--json"]);
    assert_eq!(stdout(&by_cmd), stdout(&o));
    // An unknown action is an error in both modes; nothing half-formed goes to stdout.
    let missing = rhow(&["-C", dir.to_str().unwrap(), "nope", "--json"]);
    assert_eq!(missing.status.code(), Some(2));
    assert_eq!(stdout(&missing), "");
    assert!(stderr(&missing).contains("no action named `nope`"));
}
