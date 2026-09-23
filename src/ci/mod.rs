//! CI pipelines as a structure: GitHub Actions workflows (`.github/workflows/*.yml`) and
//! GitLab CI (`.gitlab-ci.yml`), each as jobs with their `run` steps explained like actions.
//! Read-only, like everything else: the YAML is parsed, nothing is evaluated.

use crate::analyze::{self, Resolver};
use crate::model::{CiJob, CiPipeline, CiStep};
use crate::{explain, notes, risk};
use serde_yaml::Value;
use std::path::Path;

/// Every pipeline definition under `root`. `resolver` maps `npm test`-style references to
/// the root project's scripts, so steps read like the actions they run.
pub fn discover(root: &Path, resolver: Resolver) -> Vec<CiPipeline> {
    let mut out = Vec::new();
    let wf = root.join(".github").join("workflows");
    if let Ok(rd) = std::fs::read_dir(&wf) {
        let mut files: Vec<String> = rd
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
            .filter(|f| f.ends_with(".yml") || f.ends_with(".yaml"))
            .collect();
        files.sort();
        for f in files {
            if let Some(text) = crate::repo::read_text(&wf.join(&f)) {
                if let Some(p) = github(&format!(".github/workflows/{f}"), &text, resolver) {
                    out.push(p);
                }
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
                other => str_of(other),
            });
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
                        steps.push(step(sname, &run, resolver));
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
