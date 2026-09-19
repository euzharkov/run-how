//! Taskfile (go-task): tasks with `desc`/`summary`, `internal`, and one level of `includes`.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;
use serde_yaml::Value;

pub struct Taskfile;

const FILES: &[&str] = &[
    "Taskfile.yml",
    "Taskfile.yaml",
    "taskfile.yml",
    "taskfile.yaml",
    "Taskfile.dist.yml",
    "Taskfile.dist.yaml",
];

struct Task {
    name: String,
    desc: Option<String>,
    internal: bool,
    raw: String,
}

fn cmd_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Mapping(m) => {
            if let Some(Value::String(c)) = m.get("cmd") {
                Some(c.clone())
            } else if let Some(Value::String(t)) = m.get("task") {
                Some(format!("task {t}"))
            } else if let Some(d) = m.get("defer") {
                cmd_text(d)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn parse_tasks(text: &str, prefix: &str) -> Vec<Task> {
    let mut out = Vec::new();
    let Ok(doc) = serde_yaml::from_str::<Value>(text) else {
        return out;
    };
    let Some(tasks) = doc.get("tasks").and_then(|t| t.as_mapping()) else {
        return out;
    };
    for (k, v) in tasks {
        let Some(name) = k.as_str() else { continue };
        let full = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}:{name}")
        };
        let mut desc = None;
        let mut internal = false;
        let mut cmds: Vec<String> = Vec::new();
        match v {
            Value::String(s) => cmds.push(s.clone()),
            Value::Sequence(seq) => cmds.extend(seq.iter().filter_map(cmd_text)),
            Value::Mapping(m) => {
                desc = m
                    .get("desc")
                    .and_then(|d| d.as_str())
                    .or_else(|| m.get("summary").and_then(|d| d.as_str()))
                    .map(|s| s.to_string());
                internal = m.get("internal").and_then(|b| b.as_bool()).unwrap_or(false);
                if let Some(Value::Sequence(deps)) = m.get("deps") {
                    for d in deps {
                        match d {
                            Value::String(s) => cmds.push(format!("task {s}")),
                            Value::Mapping(dm) => {
                                if let Some(Value::String(t)) = dm.get("task") {
                                    cmds.push(format!("task {t}"));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                match m.get("cmds") {
                    Some(Value::Sequence(seq)) => cmds.extend(seq.iter().filter_map(cmd_text)),
                    Some(Value::String(s)) => cmds.push(s.clone()),
                    _ => {}
                }
                if let Some(Value::String(c)) = m.get("cmd") {
                    cmds.push(c.clone());
                }
            }
            _ => {}
        }
        let raw = cmds
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join(" && ");
        out.push(Task {
            name: full,
            desc,
            internal,
            raw,
        });
    }
    out
}

impl Discoverer for Taskfile {
    fn id(&self) -> &'static str {
        "task"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Taskfile
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has_any(FILES)
    }
    fn discover(&self, _ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let Some(file) = base.first_of(FILES) else {
            return out;
        };
        let Some(text) = base.read(file) else {
            return out;
        };
        let root_doc = serde_yaml::from_str::<Value>(&text).ok();
        if let Some(v) = root_doc.as_ref().and_then(|doc| {
            doc.get("version").and_then(|v| {
                v.as_str()
                    .map(|s| s.to_string())
                    .or_else(|| v.as_f64().map(|f| f.to_string()))
            })
        }) {
            out.version("taskfile", v, file);
        }
        let mut tasks = parse_tasks(&text, "");
        // One level of includes.
        if let Some(doc) = &root_doc {
            if let Some(inc) = doc.get("includes").and_then(|i| i.as_mapping()) {
                for (k, v) in inc {
                    let Some(ns) = k.as_str() else { continue };
                    let (path, internal) = match v {
                        Value::String(p) => (p.clone(), false),
                        Value::Mapping(m) => (
                            m.get("taskfile")
                                .and_then(|t| t.as_str())
                                .unwrap_or("")
                                .to_string(),
                            m.get("internal").and_then(|b| b.as_bool()).unwrap_or(false),
                        ),
                        _ => continue,
                    };
                    if path.is_empty() {
                        continue;
                    }
                    let p = base.path.join(path.trim_start_matches("./"));
                    let candidates = if p.is_dir() {
                        FILES.iter().map(|f| p.join(f)).collect::<Vec<_>>()
                    } else {
                        vec![p]
                    };
                    for c in candidates {
                        if let Some(t) = crate::repo::read_text(&c) {
                            for mut task in parse_tasks(&t, ns) {
                                task.internal |= internal;
                                tasks.push(task);
                            }
                            break;
                        }
                    }
                }
            }
        }
        for t in tasks {
            out.script("task", &t.name, &t.raw);
            let mut a = Action::new(&t.name, format!("task {}", t.name))
                .tool("task")
                .raw(t.raw.clone());
            if let Some(d) = &t.desc {
                a = a.desc(d.clone());
            }
            if t.internal || t.name.starts_with('_') || t.name.contains(":_") {
                a = a.hidden();
            }
            out.actions.push(a);
        }
        out
    }
}
