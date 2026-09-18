//! GNU Make: public-looking targets, `.PHONY`, and `##` comment descriptions.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Make;

const FILES: &[&str] = &["GNUmakefile", "makefile", "Makefile"];

#[derive(Debug, Default, Clone)]
pub struct Target {
    pub name: String,
    pub deps: Vec<String>,
    pub recipe: Vec<String>,
    pub comment: Option<String>,
    pub phony: bool,
}

fn join_continuations(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cont = false;
    for line in text.lines() {
        if cont {
            cur.push(' ');
            cur.push_str(line.trim_start());
        } else {
            cur = line.to_string();
        }
        if cur.ends_with('\\') {
            cur.pop();
            cont = true;
        } else {
            out.push(std::mem::take(&mut cur));
            cont = false;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn is_assignment(line: &str) -> bool {
    let colon = line.find(':');
    let eq = line.find('=');
    match (colon, eq) {
        (Some(c), Some(e)) => {
            e < c || line[c + 1..].starts_with('=') || line[c..].starts_with("::=")
        }
        (None, Some(_)) => true,
        _ => false,
    }
}

pub fn parse(text: &str) -> Vec<Target> {
    let lines = join_continuations(text);
    let mut targets: Vec<Target> = Vec::new();
    let mut phony: Vec<String> = Vec::new();
    let mut pending_comment: Option<String> = None;
    let mut current: Option<usize> = None;
    let mut in_define = false;
    let mut cond_depth = 0usize;
    for raw in &lines {
        let line = raw.trim_end();
        if in_define {
            if line.trim_start().starts_with("endef") {
                in_define = false;
            }
            continue;
        }
        if line.starts_with('\t') {
            if let Some(i) = current {
                let mut r = line.trim().to_string();
                while r.starts_with('@') || r.starts_with('-') || r.starts_with('+') {
                    r.remove(0);
                }
                if !r.is_empty() {
                    targets[i].recipe.push(r);
                }
            }
            continue;
        }
        let t = line.trim();
        if t.is_empty() {
            pending_comment = None;
            current = None;
            continue;
        }
        if let Some(c) = t.strip_prefix('#') {
            if !c.starts_with('!') {
                let c = c.trim_start_matches('#').trim();
                if !c.is_empty() {
                    pending_comment = Some(c.to_string());
                }
            }
            continue;
        }
        let first = t.split_whitespace().next().unwrap_or("");
        match first {
            "define" => {
                in_define = true;
                continue;
            }
            "ifeq" | "ifneq" | "ifdef" | "ifndef" => {
                cond_depth += 1;
                continue;
            }
            "else" => continue,
            "endif" => {
                cond_depth = cond_depth.saturating_sub(1);
                continue;
            }
            "include" | "-include" | "sinclude" | "export" | "unexport" | "override" | "vpath" => {
                continue
            }
            _ => {}
        }
        if is_assignment(t) {
            pending_comment = None;
            continue;
        }
        let Some(colon) = t.find(':') else { continue };
        let lhs = t[..colon].trim();
        let mut rhs = t[colon + 1..].trim_start_matches(':').trim();
        let mut inline_comment = None;
        if let Some(h) = rhs.find('#') {
            let c = rhs[h..].trim_start_matches('#').trim();
            if !c.is_empty() {
                inline_comment = Some(c.to_string());
            }
            rhs = rhs[..h].trim();
        }
        if lhs == ".PHONY" {
            phony.extend(rhs.split_whitespace().map(|s| s.to_string()));
            current = None;
            continue;
        }
        if lhs.starts_with('.') || lhs.is_empty() {
            current = None;
            continue;
        }
        let deps: Vec<String> = rhs
            .split_whitespace()
            .map(|s| s.to_string())
            .filter(|d| !d.starts_with('|'))
            .collect();
        let names: Vec<&str> = lhs.split_whitespace().collect();
        let comment = inline_comment.or(pending_comment.take());
        current = None;
        for n in names {
            if n.contains('%') || n.contains('$') || n.contains('(') || n.contains('=') {
                continue;
            }
            let idx = match targets.iter().position(|x| x.name == n) {
                Some(i) => {
                    targets[i].deps.extend(deps.clone());
                    if targets[i].comment.is_none() {
                        targets[i].comment = comment.clone();
                    }
                    i
                }
                None => {
                    targets.push(Target {
                        name: n.to_string(),
                        deps: deps.clone(),
                        recipe: vec![],
                        comment: comment.clone(),
                        phony: false,
                    });
                    targets.len() - 1
                }
            };
            current = Some(idx);
        }
    }
    for t in &mut targets {
        t.phony = phony.iter().any(|p| p == &t.name);
    }
    targets
}

fn looks_like_file(name: &str) -> bool {
    name.contains('/') || (name.contains('.') && !name.starts_with('.')) || name.ends_with(".o")
}

impl Discoverer for Make {
    fn id(&self) -> &'static str {
        "make"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Make
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
        let targets = parse(&text);
        let has_phony = targets.iter().any(|t| t.phony);
        let all_deps: Vec<&str> = targets
            .iter()
            .flat_map(|t| t.deps.iter().map(String::as_str))
            .collect();
        for t in &targets {
            let mut raw: Vec<String> = t
                .recipe
                .iter()
                .map(|r| r.replace("$(MAKE)", "make").replace("${MAKE}", "make"))
                .collect();
            if raw.is_empty() {
                raw = t.deps.iter().map(|d| format!("make {d}")).collect();
            }
            let raw = raw.iter().take(8).cloned().collect::<Vec<_>>().join(" && ");
            out.script("make", &t.name, &raw);

            let mut a = Action::new(&t.name, format!("make {}", t.name))
                .tool("make")
                .raw(raw);
            if let Some(c) = &t.comment {
                a = a.desc(c.clone());
            }
            let internal = t.name.starts_with('_') || t.name.starts_with('.');
            let filey = looks_like_file(&t.name) && !t.phony;
            if internal || filey {
                a = a.hidden();
            } else if has_phony && !t.phony && t.comment.is_none() {
                // Undeclared, undocumented target while the file does use .PHONY: probably internal.
                a = a.confidence(Confidence::Low);
            } else if !t.phony
                && t.comment.is_none()
                && all_deps.contains(&t.name.as_str())
                && t.recipe.is_empty()
            {
                a = a.confidence(Confidence::Low);
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
    fn parses_targets_and_comments() {
        let mk = "\
.PHONY: build test clean

## Build the binary
build: deps
\tgo build ./...

test: ## Run the tests
\tgo test ./...

deps:
\tgo mod download

clean:
\trm -rf bin

VAR := x
%.o: %.c
\tcc -c $<
";
        let t = parse(mk);
        let names: Vec<&str> = t.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["build", "test", "deps", "clean"]);
        assert_eq!(t[0].comment.as_deref(), Some("Build the binary"));
        assert_eq!(t[1].comment.as_deref(), Some("Run the tests"));
        assert!(t[0].phony && t[1].phony && !t[2].phony);
        assert_eq!(t[0].recipe, ["go build ./..."]);
    }
}
