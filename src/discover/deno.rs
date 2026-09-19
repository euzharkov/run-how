//! Deno: `deno.json(c)` tasks plus conventional test/lint/fmt/check.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Deno;

impl Discoverer for Deno {
    fn id(&self) -> &'static str {
        "deno"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Deno
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has_any(&["deno.json", "deno.jsonc", "deno.lock"])
    }
    fn project_name(&self, dir: &DirInfo) -> Option<String> {
        let f = dir.first_of(&["deno.json", "deno.jsonc"])?;
        super::mono::read_jsonc(dir, f)?
            .get("name")?
            .as_str()
            .map(|s| s.rsplit('/').next().unwrap_or(s).to_string())
    }
    fn discover(&self, _ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let cfg = base
            .first_of(&["deno.json", "deno.jsonc"])
            .and_then(|f| super::mono::read_jsonc(base, f));
        let mut declared: Vec<String> = Vec::new();
        if let Some(tasks) = cfg
            .as_ref()
            .and_then(|c| c.get("tasks"))
            .and_then(|t| t.as_object())
        {
            for (name, v) in tasks {
                let (body, desc) = match v {
                    serde_json::Value::String(s) => (s.clone(), None),
                    serde_json::Value::Object(o) => (
                        o.get("command")
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string(),
                        o.get("description")
                            .and_then(|d| d.as_str())
                            .map(|s| s.to_string()),
                    ),
                    _ => continue,
                };
                out.script("deno", name, &body);
                declared.push(name.clone());
                let mut a = Action::new(name, format!("deno task {name}"))
                    .tool("deno")
                    .raw(body);
                if let Some(d) = desc {
                    a = a.desc(d);
                }
                out.actions.push(a);
            }
        }
        let free = |n: &str| !declared.iter().any(|d| d == n);
        if free("test") {
            out.actions.push(
                Action::new("test", "deno test")
                    .tool("deno")
                    .inferred(Confidence::High),
            );
        }
        if free("lint") {
            out.actions.push(
                Action::new("lint", "deno lint")
                    .tool("deno")
                    .inferred(Confidence::High),
            );
        }
        if free("fmt") && free("format") {
            out.actions.push(
                Action::new("fmt", "deno fmt")
                    .tool("deno")
                    .inferred(Confidence::High),
            );
        }
        if free("check") {
            let entry = ["main.ts", "mod.ts", "src/main.ts"]
                .iter()
                .find(|f| base.path.join(f).is_file())
                .copied();
            if let Some(e) = entry {
                out.actions.push(
                    Action::new("check", format!("deno check {e}"))
                        .tool("deno")
                        .inferred(Confidence::Medium),
                );
                if free("dev") && free("start") && e != "mod.ts" {
                    out.actions.push(
                        Action::new("dev", format!("deno run -A --watch {e}"))
                            .tool("deno")
                            .inferred(Confidence::Medium)
                            .inferred_desc(format!("Run {e} with Deno and auto-reload"))
                            .cat(Category::Development),
                    );
                }
            }
        }
        out
    }
}
