//! CI pipelines as a structure: GitHub Actions workflows (`.github/workflows/*.yml`) and
//! GitLab CI (`.gitlab-ci.yml`), each as jobs with their `run` steps explained like actions.
//! Read-only, like everything else: the YAML is parsed, nothing is evaluated.

use crate::analyze::{self, Resolver};
use crate::model::{CiJob, CiPipeline, CiStep, Risk};
use crate::{explain, notes, risk};
use serde_yaml::Value;
use std::path::Path;

/// Every pipeline definition under `root`. `resolver` maps `npm test`-style references to
/// the root project's scripts, so steps read like the actions they run.
pub fn discover(root: &Path, resolver: Resolver) -> Vec<CiPipeline> {
    let mut out = Vec::new();
    let wf = root.join(".github").join("workflows");
    for f in list_files(&wf) {
        if !(f.ends_with(".yml") || f.ends_with(".yaml")) {
            continue;
        }
        if let Some(text) = crate::repo::read_text(&wf.join(&f)) {
            if let Some(p) = github(&format!(".github/workflows/{f}"), &text, resolver) {
                out.push(p);
            }
        }
    }
    if let Some(text) = crate::repo::read_text(&root.join(".gitlab-ci.yml")) {
        if let Some(p) = gitlab(".gitlab-ci.yml", &text, resolver) {
            out.push(p);
        }
    }
    out
}

/// Sorted names of the regular files directly inside `dir`; empty when it does not exist.
///
/// This is the one directory listing outside `repo::scan`. The scanner never descends into
/// dot-directories (`.git`, `.github`, `.venv`, …), so `.github/workflows` is not in its
/// `DirInfo` list, and extending the walk for one path would cost every other repository a
/// pass over its `.git`. It follows the walker's rules: metadata only, no symlinks, no
/// recursion; the files themselves go through `repo::read_text` like everything else.
fn list_files(dir: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<String> = rd
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();
    files.sort();
    files
}

fn step(name: Option<String>, command: &str, resolver: Resolver) -> CiStep {
    let analysis = analyze::analyze(command, resolver);
    let description = explain::describe(&analysis, name.as_deref().unwrap_or(""));
    CiStep {
        name,
        risk: risk::classify(command, &analysis),
        notes: notes::classify(command, &analysis),
        description,
        command: command.trim().to_string(),
    }
}

/// A `run` step whose `shell:` is not a POSIX shell (`python`, `pwsh`, `powershell`, `cmd`,
/// `node`, …) is a script in that language: tokenising it as shell would read `rm -rf build`
/// inside a Python string literal as a destructive command. Such a step is shown as what it
/// is, with no verdict on its content.
fn foreign_step(name: Option<String>, command: &str, shell: &str) -> CiStep {
    CiStep {
        name,
        command: command.trim().to_string(),
        description: format!("Run a {shell} script"),
        risk: Risk::Safe,
        notes: Vec::new(),
    }
}

/// The program named by a `shell:` value (`bash -e {0}` → `bash`, `/bin/sh` → `sh`), or
/// `None` when it is a POSIX shell the tokenizer understands.
fn foreign_shell(shell: &str) -> Option<String> {
    let program = shell.split_whitespace().next()?;
    let name = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(program)
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    match name.as_str() {
        "sh" | "bash" | "zsh" | "dash" | "" => None,
        _ => Some(name),
    }
}

/// `defaults.run.shell` of a workflow or a job.
fn default_shell(m: &serde_yaml::Mapping) -> Option<String> {
    get(m, "defaults")
        .and_then(Value::as_mapping)
        .and_then(|d| get(d, "run"))
        .and_then(Value::as_mapping)
        .and_then(|r| get(r, "shell"))
        .and_then(str_of)
}

fn str_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// `on` parses as the YAML 1.1 boolean `true` in some parsers; accept both spellings.
fn get<'a>(m: &'a serde_yaml::Mapping, key: &str) -> Option<&'a Value> {
    m.get(Value::String(key.to_string())).or_else(|| {
        if key == "on" {
            m.get(Value::Bool(true))
        } else {
            None
        }
    })
}

fn seq_strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(|b| b.as_sequence())
        .map(|b| b.iter().filter_map(str_of).collect())
        .unwrap_or_default()
}

fn github(file: &str, text: &str, resolver: Resolver) -> Option<CiPipeline> {
    let doc: Value = serde_yaml::from_str(text).ok()?;
    let m = doc.as_mapping()?;
    let name = get(m, "name")
        .and_then(str_of)
        .unwrap_or_else(|| file.rsplit('/').next().unwrap_or(file).to_string());
    let mut triggers = Vec::new();
    match get(m, "on") {
        Some(Value::String(s)) => triggers.push(s.clone()),
        Some(Value::Sequence(seq)) => triggers.extend(seq.iter().filter_map(str_of)),
        Some(Value::Mapping(on)) => {
            for (k, v) in on {
                let Some(k) = str_of(k) else { continue };
                let vm = v.as_mapping();
                let branches = seq_strings(vm.and_then(|vm| get(vm, "branches")));
                let tags = seq_strings(vm.and_then(|vm| get(vm, "tags")));
                if !branches.is_empty() {
                    triggers.push(format!("{k} {}", branches.join(", ")));
                } else if !tags.is_empty() {
                    triggers.push(format!("{k} tags {}", tags.join(", ")));
                } else {
                    triggers.push(k);
                }
            }
        }
        _ => {}
    }
    let workflow_shell = default_shell(m);
    let mut jobs = Vec::new();
    if let Some(Value::Mapping(js)) = get(m, "jobs") {
        for (id, job) in js {
            let Some(id) = str_of(id) else { continue };
            let Some(jm) = job.as_mapping() else { continue };
            let name = get(jm, "name").and_then(str_of).unwrap_or(id);
            let runs_on = get(jm, "runs-on").and_then(|r| match r {
                Value::Sequence(seq) => {
                    Some(seq.iter().filter_map(str_of).collect::<Vec<_>>().join(", "))
                }
                // `runs-on: { group: ..., labels: [...] }` names a runner group.
                Value::Mapping(rm) => {
                    let labels = seq_strings(get(rm, "labels"));
                    if !labels.is_empty() {
                        Some(labels.join(", "))
                    } else {
                        get(rm, "group").and_then(str_of)
                    }
                }
                other => str_of(other),
            });
            let job_shell = default_shell(jm).or_else(|| workflow_shell.clone());
            let mut steps = Vec::new();
            // A reusable workflow call has no steps of its own; show what it calls.
            if let Some(uses) = get(jm, "uses").and_then(str_of) {
                let mut s = step(None, "", resolver);
                s.description = format!("Run the reusable workflow {uses}");
                s.command = format!("uses: {uses}");
                steps.push(s);
            }
            if let Some(Value::Sequence(ss)) = get(jm, "steps") {
                for s in ss {
                    let Some(sm) = s.as_mapping() else { continue };
                    let sname = get(sm, "name").and_then(str_of);
                    if let Some(run) = get(sm, "run").and_then(str_of) {
                        let shell = get(sm, "shell")
                            .and_then(str_of)
                            .or_else(|| job_shell.clone());
                        match shell.as_deref().and_then(foreign_shell) {
                            Some(lang) => steps.push(foreign_step(sname, &run, &lang)),
                            None => steps.push(step(sname, &run, resolver)),
                        }
                    }
                }
            }
            jobs.push(CiJob {
                name,
                runs_on,
                steps,
            });
        }
    }
    Some(CiPipeline {
        system: "github-actions",
        file: file.to_string(),
        name,
        triggers,
        jobs,
    })
}

const GITLAB_KEYWORDS: &[&str] = &[
    "stages",
    "variables",
    "default",
    "include",
    "workflow",
    "image",
    "services",
    "cache",
    "before_script",
    "after_script",
];

fn gitlab(file: &str, text: &str, resolver: Resolver) -> Option<CiPipeline> {
    let doc: Value = serde_yaml::from_str(text).ok()?;
    let m = doc.as_mapping()?;
    let mut jobs = Vec::new();
    for (k, v) in m {
        let Some(name) = str_of(k) else { continue };
        if name.starts_with('.') || GITLAB_KEYWORDS.contains(&name.as_str()) {
            continue;
        }
        let Some(jm) = v.as_mapping() else { continue };
        let Some(script) = get(jm, "script") else {
            continue;
        };
        let runs_on = get(jm, "stage").and_then(str_of).or_else(|| {
            get(jm, "image").and_then(|i| match i {
                Value::Mapping(im) => get(im, "name").and_then(str_of),
                other => str_of(other),
            })
        });
        let mut steps = Vec::new();
        match script {
            Value::Sequence(seq) => {
                for line in seq.iter().filter_map(str_of) {
                    steps.push(step(None, &line, resolver));
                }
            }
            other => {
                if let Some(s) = str_of(other) {
                    steps.push(step(None, &s, resolver));
                }
            }
        }
        jobs.push(CiJob {
            name,
            runs_on,
            steps,
        });
    }
    let triggers = if get(m, "workflow")
        .and_then(|w| w.as_mapping())
        .and_then(|wm| get(wm, "rules"))
        .is_some()
    {
        vec!["workflow rules".to_string()]
    } else {
        Vec::new()
    };
    Some(CiPipeline {
        system: "gitlab-ci",
        file: file.to_string(),
        name: "GitLab CI".to_string(),
        triggers,
        jobs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Note, Risk};

    #[test]
    fn github_workflow_structure() {
        let yml = r#"
name: CI
on:
  push:
    branches: [main]
  pull_request:
jobs:
  test:
    name: Test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - run: cargo fmt --all -- --check
      - name: Tests
        run: cargo test
  publish:
    runs-on: ubuntu-latest
    steps:
      - run: npm publish
"#;
        let p = github(".github/workflows/ci.yml", yml, &analyze::no_resolver).unwrap();
        assert_eq!(p.name, "CI");
        assert_eq!(p.triggers, ["push main", "pull_request"]);
        assert_eq!(p.jobs.len(), 2);
        assert_eq!(p.jobs[0].name, "Test (${{ matrix.os }})");
        assert_eq!(p.jobs[0].steps.len(), 2);
        assert_eq!(p.jobs[0].steps[1].name.as_deref(), Some("Tests"));
        assert_eq!(p.jobs[0].steps[1].description, "Run Rust tests");
        assert_eq!(p.jobs[1].steps[0].risk, Risk::External);
    }

    #[test]
    fn non_shell_steps_are_not_tokenised_as_shell() {
        let yml = r#"
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: |
          import shutil
          shutil.rmtree("build")  # rm -rf build
        shell: python
      - run: Remove-Item -Recurse build
        shell: pwsh
      - run: rmdir /s /q build
        shell: cmd
      - run: git clean -fdx
        shell: bash -e {0}
      - run: git clean -fdx
        shell: /bin/sh
  scripted:
    runs-on: ubuntu-latest
    defaults:
      run:
        shell: node {0}
    steps:
      - run: 'require("child_process").execSync("git clean -fdx")'
      - run: git clean -fdx
        shell: bash
"#;
        let p = github(".github/workflows/ci.yml", yml, &analyze::no_resolver).unwrap();
        let s = &p.jobs[0].steps;
        assert_eq!(s[0].description, "Run a python script");
        assert_eq!(s[0].risk, Risk::Safe);
        assert!(s[0].notes.is_empty());
        assert_eq!(s[1].description, "Run a pwsh script");
        assert_eq!(s[2].description, "Run a cmd script");
        // A POSIX shell, however spelled, is analysed as before.
        assert_eq!(s[3].risk, Risk::Destructive);
        assert_eq!(s[4].risk, Risk::Destructive);
        // A job-level default shell applies to steps without their own; a step's own wins.
        let s = &p.jobs[1].steps;
        assert_eq!(s[0].description, "Run a node script");
        assert_eq!(s[0].risk, Risk::Safe);
        assert_eq!(s[1].risk, Risk::Destructive);
    }

    #[test]
    fn workflow_default_shell_applies_to_every_job() {
        let yml = r#"
on: push
defaults:
  run:
    shell: pwsh
jobs:
  a:
    runs-on: windows-latest
    steps:
      - run: Remove-Item -Recurse -Force build
"#;
        let p = github("w.yml", yml, &analyze::no_resolver).unwrap();
        assert_eq!(p.jobs[0].steps[0].description, "Run a pwsh script");
    }

    #[test]
    fn triggers_as_string_and_sequence() {
        let one = github("w.yml", "on: push\njobs: {}", &analyze::no_resolver).unwrap();
        assert_eq!(one.triggers, ["push"]);
        assert_eq!(one.name, "w.yml");
        let many = github(
            ".github/workflows/w.yml",
            "on: [push, pull_request]\njobs: {}",
            &analyze::no_resolver,
        )
        .unwrap();
        assert_eq!(many.triggers, ["push", "pull_request"]);
        assert_eq!(many.name, "w.yml");
        let tagged = github(
            "w.yml",
            "on:\n  push:\n    tags: ['v*']\njobs: {}",
            &analyze::no_resolver,
        )
        .unwrap();
        assert_eq!(tagged.triggers, ["push tags v*"]);
    }

    #[test]
    fn runs_on_shapes() {
        let yml = r#"
on: push
jobs:
  a:
    runs-on: [self-hosted, linux]
    steps: []
  b:
    runs-on:
      group: big-runners
      labels: [gpu, linux]
    steps: []
  c:
    runs-on:
      group: big-runners
    steps: []
  d:
    steps: []
"#;
        let p = github("w.yml", yml, &analyze::no_resolver).unwrap();
        let runs: Vec<Option<&str>> = p.jobs.iter().map(|j| j.runs_on.as_deref()).collect();
        assert_eq!(
            runs,
            [
                Some("self-hosted, linux"),
                Some("gpu, linux"),
                Some("big-runners"),
                None
            ]
        );
    }

    #[test]
    fn reusable_workflow_job_has_one_step_and_no_run_steps() {
        let yml = r#"
on: push
jobs:
  call:
    uses: org/repo/.github/workflows/build.yml@main
    with:
      x: 1
"#;
        let p = github("w.yml", yml, &analyze::no_resolver).unwrap();
        assert_eq!(p.jobs.len(), 1);
        assert_eq!(p.jobs[0].name, "call");
        assert_eq!(p.jobs[0].runs_on, None);
        assert_eq!(p.jobs[0].steps.len(), 1);
        let s = &p.jobs[0].steps[0];
        assert_eq!(s.command, "uses: org/repo/.github/workflows/build.yml@main");
        assert_eq!(
            s.description,
            "Run the reusable workflow org/repo/.github/workflows/build.yml@main"
        );
        assert_eq!(s.risk, Risk::Safe);
        assert!(s.notes.is_empty());
    }

    #[test]
    fn gitlab_script_as_string_or_list() {
        let yml = r#"
one:
  script: npm test
many:
  script:
    - npm ci
    - npm test
    - 42
none:
  image: node:22
"#;
        let p = gitlab(".gitlab-ci.yml", yml, &analyze::no_resolver).unwrap();
        let names: Vec<&str> = p.jobs.iter().map(|j| j.name.as_str()).collect();
        assert_eq!(names, ["one", "many"]);
        assert_eq!(p.jobs[0].steps.len(), 1);
        assert_eq!(p.jobs[0].steps[0].command, "npm test");
        assert_eq!(p.jobs[1].steps.len(), 3);
        assert_eq!(p.jobs[1].steps[2].command, "42");
        assert!(p.triggers.is_empty());
    }

    #[test]
    fn unparsable_yaml_is_dropped_silently() {
        for bad in [
            "jobs: [unclosed",
            "- just\n- a list",
            "plain text",
            "",
            "on: push\n  bad: indent",
        ] {
            assert!(
                github("w.yml", bad, &analyze::no_resolver).is_none(),
                "{bad:?}"
            );
            assert!(
                gitlab(".gitlab-ci.yml", bad, &analyze::no_resolver).is_none(),
                "{bad:?}"
            );
        }
        // A mapping whose jobs are malformed still yields the pipeline, minus those jobs.
        let p = github(
            "w.yml",
            "on: push\njobs:\n  a: 3\n  b: [x]",
            &analyze::no_resolver,
        )
        .unwrap();
        assert!(p.jobs.is_empty());
    }

    #[test]
    fn list_files_skips_directories_and_missing_dirs() {
        let dir = std::env::temp_dir().join(format!("rhow-ci-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("b.yml"), "on: push").unwrap();
        std::fs::write(dir.join("a.yaml"), "on: push").unwrap();
        assert_eq!(list_files(&dir), ["a.yaml", "b.yml"]);
        assert!(list_files(&dir.join("nope")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gitlab_jobs_skip_templates_and_keywords() {
        let yml = r#"
stages: [test, deploy]
.base:
  image: node:22
test:
  stage: test
  script:
    - npm ci
    - npm test
deploy:
  stage: deploy
  script: kubectl apply -f k8s/
"#;
        let p = gitlab(".gitlab-ci.yml", yml, &analyze::no_resolver).unwrap();
        let names: Vec<&str> = p.jobs.iter().map(|j| j.name.as_str()).collect();
        assert_eq!(names, ["test", "deploy"]);
        assert_eq!(p.jobs[0].runs_on.as_deref(), Some("test"));
        assert_eq!(p.jobs[0].steps.len(), 2);
        assert!(p.jobs[0].steps[0].notes.contains(&Note::Download));
        assert_eq!(p.jobs[1].steps[0].risk, Risk::External);
    }
}
