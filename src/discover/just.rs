//! Just recipes with comments, `[doc]`, `[private]` and `[group]` attributes.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Just;

const FILES: &[&str] = &["justfile", "Justfile", ".justfile", "JUSTFILE"];

#[derive(Debug, Default, Clone)]
pub struct Recipe {
    pub name: String,
    pub doc: Option<String>,
    pub private: bool,
    pub group: Option<String>,
    pub deps: Vec<String>,
    pub body: Vec<String>,
    pub aliases: Vec<String>,
}

fn parse_attr_value(attr: &str, key: &str) -> Option<String> {
    let rest = attr.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('(')?;
    let rest = rest.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &rest[1..];
    let end = inner.find(quote)?;
    Some(inner[..end].to_string())
}

pub fn parse(text: &str) -> Vec<Recipe> {
    let mut out: Vec<Recipe> = Vec::new();
    let mut pending_doc: Option<String> = None;
    let mut pending_private = false;
    let mut pending_group: Option<String> = None;
    let mut pending_attr_doc: Option<String> = None;
    let mut current: Option<usize> = None;
    let mut aliases: Vec<(String, String)> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(i) = current {
                let mut b = line.trim().to_string();
                while b.starts_with('@') || b.starts_with('-') {
                    b.remove(0);
                }
                if !b.is_empty() && !b.starts_with("#!") && !b.starts_with('#') {
                    out[i].body.push(b);
                }
            }
            continue;
        }
        let t = line.trim();
        if t.is_empty() {
            pending_doc = None;
            current = None;
            continue;
        }
        if let Some(c) = t.strip_prefix('#') {
            if !c.starts_with('!') {
                pending_doc = Some(c.trim().to_string());
            }
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            let inner = &t[1..t.len() - 1];
            for attr in inner.split(',').map(str::trim) {
                if attr == "private" {
                    pending_private = true;
                } else if let Some(v) = parse_attr_value(attr, "doc") {
                    pending_attr_doc = Some(v);
                } else if let Some(v) = parse_attr_value(attr, "group") {
                    pending_group = Some(v);
                }
            }
            continue;
        }
        if let Some(rest) = t.strip_prefix("alias ") {
            if let Some((a, b)) = rest.split_once(":=") {
                aliases.push((a.trim().to_string(), b.trim().to_string()));
            }
            pending_doc = None;
            continue;
        }
        let first = t.split_whitespace().next().unwrap_or("");
        if matches!(
            first,
            "set" | "mod" | "mod?" | "import" | "import?" | "export" | "unexport"
        ) || t.contains(":=")
        {
            pending_doc = None;
            pending_private = false;
            pending_group = None;
            pending_attr_doc = None;
            continue;
        }
        let Some(colon) = t.find(':') else { continue };
        if t[colon + 1..].starts_with('=') {
            continue;
        }
        let head = t[..colon].trim();
        let mut name = head
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_start_matches('@')
            .to_string();
        if name.is_empty() {
            continue;
        }
        if let Some(n) = name.strip_prefix('_') {
            let _ = n;
        }
        let deps_str = t[colon + 1..].trim();
        let deps: Vec<String> = deps_str
            .split_whitespace()
            .filter(|d| *d != "&&")
            .map(|d| {
                d.trim_start_matches('(')
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(')')
                    .to_string()
            })
            .filter(|d| !d.is_empty())
            .collect();
        if name.ends_with(':') {
            name.pop();
        }
        let private = pending_private || name.starts_with('_');
        out.push(Recipe {
            name,
            doc: pending_attr_doc.take().or(pending_doc.take()),
            private,
            group: pending_group.take(),
            deps,
            body: vec![],
            aliases: vec![],
        });
        pending_private = false;
        current = Some(out.len() - 1);
    }
    for (alias, target) in aliases {
        if let Some(r) = out.iter_mut().find(|r| r.name == target) {
            r.aliases.push(alias);
        }
    }
    out
}

impl Discoverer for Just {
    fn id(&self) -> &'static str {
        "just"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Just
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
        for r in parse(&text) {
            let mut raw: Vec<String> = r.body.iter().take(8).cloned().collect();
            if raw.is_empty() {
                raw = r.deps.iter().map(|d| format!("just {d}")).collect();
            }
            let raw = raw.join(" && ");
            out.script("just", &r.name, &raw);
            let mut a = Action::new(&r.name, format!("just {}", r.name))
                .tool("just")
                .raw(raw);
            if let Some(d) = &r.doc {
                a = a.desc(d.clone());
            }
            if r.private {
                a = a.hidden();
            }
            if a.category == Category::Other {
                if let Some(g) = &r.group {
                    if let Some(h) = crate::explain::name_hint(g) {
                        a = a.cat(h.category);
                    }
                }
            }
            out.actions.push(a);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recipes() {
        let jf = "\
set dotenv-load

alias t := test

# Run the test suite
test *ARGS:
    cargo test {{ARGS}}

[private]
_helper:
    echo hi

[doc(\"Build release binary\")]
[group('build')]
release: test
    cargo build --release

default:
    @just --list
";
        let r = parse(jf);
        let names: Vec<&str> = r.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["test", "_helper", "release", "default"]);
        assert_eq!(r[0].doc.as_deref(), Some("Run the test suite"));
        assert_eq!(r[0].aliases, ["t"]);
        assert!(r[1].private);
        assert_eq!(r[2].doc.as_deref(), Some("Build release binary"));
        assert_eq!(r[2].group.as_deref(), Some("build"));
        assert_eq!(r[2].deps, ["test"]);
        assert_eq!(r[2].body, ["cargo build --release"]);
    }
}
