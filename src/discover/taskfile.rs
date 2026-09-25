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

fn parse_tasks(doc: &Value, prefix: &str) -> Vec<Task> {
    let mut out = Vec::new();
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
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let Some(file) = base.first_of(FILES) else {
            return out;
        };
        let Some(doc) = ctx.yaml(base, file) else {
            return out;
        };
        if let Some(v) = doc.get("version").and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_f64().map(|f| f.to_string()))
        }) {
            out.version("taskfile", v, file);
        }
        let mut tasks = parse_tasks(&doc, "");
        // One level of includes.
        {
            if let Some(inc) = doc.get("includes").and_then(|i| i.as_mapping()) {
                for (k, v) in inc {
                    let Some(ns) = k.as_str() else { continue };
                    let (path, internal, flatten) = match v {
                        Value::String(p) => (p.clone(), false, false),
                        Value::Mapping(m) => (
                            m.get("taskfile")
                                .and_then(|t| t.as_str())
                                .unwrap_or("")
                                .to_string(),
                            m.get("internal").and_then(|b| b.as_bool()).unwrap_or(false),
                            m.get("flatten").and_then(|b| b.as_bool()).unwrap_or(false),
                        ),
                        _ => continue,
                    };
                    // `flatten: true` merges the included tasks into the root namespace.
                    let ns = if flatten { "" } else { ns };
                    if path.is_empty() {
                        continue;
                    }
                    // `taskfile:` names a directory (holding a Taskfile) or a file.
                    let p = path.trim_start_matches("./").trim_end_matches('/');
                    let candidates: Vec<(&DirInfo, &str)> = if ctx.has_dir(base, p) {
                        ctx.child(base, p)
                            .into_iter()
                            .flat_map(|d| FILES.iter().map(move |f| (d, *f)))
                            .collect()
                    } else {
                        match p.rsplit_once('/') {
                            Some((sub, f)) => {
                                ctx.child(base, sub).map(|d| (d, f)).into_iter().collect()
                            }
                            None => vec![(base, p)],
                        }
                    };
                    for (d, f) in candidates {
                        if let Some(inc_doc) = ctx.yaml(d, f) {
                            for mut task in parse_tasks(&inc_doc, ns) {
                                task.internal |= internal;
                                // A flattened task with the name of a root task is the
                                // root's (root wins at runtime).
                                if tasks.iter().any(|t| t.name == task.name) {
                                    continue;
                                }
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
            // `task` refuses to run an internal task from the command line.
            if t.internal {
                a = a.redundant();
            }
            out.actions.push(a);
        }
        out
    }
}
