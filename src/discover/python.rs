//! Python projects: uv / Poetry / PDM / Pipenv runners, pytest, Ruff, mypy, tox, nox,
//! Django `manage.py`, `[project.scripts]`, PDM scripts and Poe tasks.
//!
//! `pip` is dependency management, not a project task, so it is never exposed.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;
use toml::Value;

pub struct Python;

const MARKERS: &[&str] = &[
    "pyproject.toml",
    "uv.lock",
    "requirements.txt",
    "requirements-dev.txt",
    "Pipfile",
    "poetry.lock",
    "pdm.lock",
    "tox.ini",
    "noxfile.py",
    "pytest.ini",
    "ruff.toml",
    ".ruff.toml",
    "mypy.ini",
    ".mypy.ini",
    "manage.py",
    "setup.py",
    "setup.cfg",
    "pyrightconfig.json",
    ".flake8",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Runner {
    Uv,
    Poetry,
    Pdm,
    Pipenv,
    Hatch,
    None,
}

impl Runner {
    fn prefix(self) -> &'static str {
        match self {
            Runner::Uv => "uv run ",
            Runner::Poetry => "poetry run ",
            Runner::Pdm => "pdm run ",
            Runner::Pipenv => "pipenv run ",
            Runner::Hatch => "hatch run ",
            Runner::None => "",
        }
    }
    fn tool(self) -> &'static str {
        match self {
            Runner::Uv => "uv",
            Runner::Poetry => "poetry",
            Runner::Pdm => "pdm",
            Runner::Pipenv => "pipenv",
            Runner::Hatch => "hatch",
            Runner::None => "python",
        }
    }
}

fn runner(dir: &DirInfo, py: Option<&Value>) -> Runner {
    let tool = py.and_then(|p| p.get("tool"));
    if dir.has("uv.lock") || tool.and_then(|t| t.get("uv")).is_some() {
        Runner::Uv
    } else if dir.has("poetry.lock") || tool.and_then(|t| t.get("poetry")).is_some() {
        Runner::Poetry
    } else if dir.has("pdm.lock") || tool.and_then(|t| t.get("pdm")).is_some() {
        Runner::Pdm
    } else if dir.has("Pipfile") {
        Runner::Pipenv
    } else if tool
        .and_then(|t| t.get("hatch"))
        .and_then(|h| h.get("envs"))
        .is_some()
    {
        Runner::Hatch
    } else {
        Runner::None
    }
}

/// All dependency names mentioned in pyproject/requirements, lowercased, for cheap `contains`.
fn dependency_text(dir: &DirInfo, py: Option<&Value>) -> String {
    let mut s = String::new();
    fn push_arr(s: &mut String, v: Option<&Value>) {
        if let Some(Value::Array(a)) = v {
            for x in a {
                if let Some(t) = x.as_str() {
                    s.push_str(&t.to_ascii_lowercase());
                    s.push('\n');
                }
            }
        }
    }
    fn push_table_keys(s: &mut String, v: Option<&Value>) {
        if let Some(Value::Table(t)) = v {
            for k in t.keys() {
                s.push_str(&k.to_ascii_lowercase());
                s.push('\n');
            }
        }
    }
    if let Some(p) = py {
        let project = p.get("project");
        push_arr(&mut s, project.and_then(|x| x.get("dependencies")));
        if let Some(Value::Table(opt)) = project.and_then(|x| x.get("optional-dependencies")) {
            for v in opt.values() {
                push_arr(&mut s, Some(v));
            }
        }
        if let Some(Value::Table(groups)) = p.get("dependency-groups") {
            for v in groups.values() {
                push_arr(&mut s, Some(v));
            }
        }
        let poetry = p.get("tool").and_then(|t| t.get("poetry"));
        push_table_keys(&mut s, poetry.and_then(|x| x.get("dependencies")));
        push_table_keys(&mut s, poetry.and_then(|x| x.get("dev-dependencies")));
        if let Some(Value::Table(groups)) = poetry.and_then(|x| x.get("group")) {
            for g in groups.values() {
                push_table_keys(&mut s, g.get("dependencies"));
            }
        }
        if let Some(Value::Table(pdm)) = p
            .get("tool")
            .and_then(|t| t.get("pdm"))
            .and_then(|x| x.get("dev-dependencies"))
        {
            for v in pdm.values() {
                push_arr(&mut s, Some(v));
            }
        }
        push_arr(
            &mut s,
            p.get("tool")
                .and_then(|t| t.get("uv"))
                .and_then(|u| u.get("dev-dependencies")),
        );
    }
    for f in dir
        .files
        .iter()
        .filter(|f| f.starts_with("requirements") && f.ends_with(".txt"))
    {
        if let Some(t) = dir.read(f) {
            s.push_str(&t.to_ascii_lowercase());
        }
    }
    if let Some(t) = dir.read("Pipfile") {
        s.push_str(&t.to_ascii_lowercase());
    }
    s
}

fn has_dep(deps: &str, name: &str) -> bool {
    deps.lines().any(|l| {
        let l = l.trim().trim_start_matches('"').trim_start_matches('\'');
        let head: String = l
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .collect();
        head.eq_ignore_ascii_case(name)
    })
}

fn nox_sessions(text: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i].trim();
        if l.starts_with("@nox.session") || l.starts_with("@session") {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().starts_with('@') {
                j += 1;
            }
            if j < lines.len() {
                let d = lines[j].trim();
                if let Some(rest) = d.strip_prefix("def ") {
                    let name = rest.split('(').next().unwrap_or("").trim().to_string();
                    let mut doc = None;
                    if let Some(next) = lines.get(j + 1) {
                        let n = next.trim();
                        if let Some(q) = n.strip_prefix("\"\"\"").or_else(|| n.strip_prefix("'''"))
                        {
                            let q = q.trim_end_matches("\"\"\"").trim_end_matches("'''").trim();
                            if !q.is_empty() {
                                doc = Some(q.to_string());
                            }
                        }
                    }
                    if !name.is_empty() {
                        let name = if l.contains("name=") {
                            l.split("name=")
                                .nth(1)
                                .and_then(|s| {
                                    s.trim()
                                        .trim_matches(|c| {
                                            c == '"' || c == '\'' || c == ')' || c == ','
                                        })
                                        .split(['"', '\''])
                                        .next()
                                })
                                .map(|s| s.to_string())
                                .unwrap_or(name)
                        } else {
                            name
                        };
                        out.push((name, doc));
                    }
                }
            }
            i = j;
        }
        i += 1;
    }
    out
}

impl Discoverer for Python {
    fn id(&self) -> &'static str {
        "python"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Python
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has_any(MARKERS)
    }
    fn discover(&self, _ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let py: Option<Value> = base
            .read("pyproject.toml")
            .and_then(|t| toml::from_str(&t).ok());
        if let Some(rp) = py
            .as_ref()
            .and_then(|p| p.get("project"))
            .and_then(|p| p.get("requires-python"))
            .and_then(|v| v.as_str())
        {
            out.version("python", rp, "pyproject.toml");
        }
        let r = runner(base, py.as_ref());
        let pre = r.prefix();
        let tool = r.tool();
        let deps = dependency_text(base, py.as_ref());
        let tool_cfg = |name: &str| -> bool {
            py.as_ref()
                .and_then(|p| p.get("tool"))
                .and_then(|t| t.get(name))
                .is_some()
        };
        let python = |cmd: &str| format!("{pre}{cmd}");

        // ---- declared entrypoints -----------------------------------------------------
        if let Some(scripts) = py
            .as_ref()
            .and_then(|p| p.get("project"))
            .and_then(|p| p.get("scripts"))
            .and_then(|s| s.as_table())
        {
            for (name, target) in scripts {
                let t = target.as_str().unwrap_or("");
                let module = t.split(':').next().unwrap_or(t);
                let a = Action::new(name, python(name))
                    .tool(tool)
                    .inferred_desc(format!("Run the {name} command ({module})"))
                    .cat(Category::Development);
                out.actions.push(a);
            }
        }
        // ---- PDM scripts / Poe tasks (declared) ---------------------------------------
        if let Some(scripts) = py
            .as_ref()
            .and_then(|p| p.get("tool"))
            .and_then(|t| t.get("pdm"))
            .and_then(|p| p.get("scripts"))
            .and_then(|s| s.as_table())
        {
            for (name, v) in scripts {
                if name == "_" {
                    continue;
                }
                let (body, help) = script_body(v);
                out.script("pdm", name, &body);
                let mut a = Action::new(name, format!("pdm run {name}"))
                    .tool("pdm")
                    .raw(body);
                if let Some(h) = help {
                    a = a.desc(h);
                }
                out.actions.push(a);
            }
        }
        if let Some(tasks) = py
            .as_ref()
            .and_then(|p| p.get("tool"))
            .and_then(|t| t.get("poe"))
            .and_then(|p| p.get("tasks"))
            .and_then(|s| s.as_table())
        {
            for (name, v) in tasks {
                let (body, help) = script_body(v);
                out.script("poe", name, &body);
                let mut a = Action::new(name, python(&format!("poe {name}")))
                    .tool("poe")
                    .raw(body);
                if let Some(h) = help {
                    a = a.desc(h);
                }
                out.actions.push(a);
            }
        }
        if let Some(envs) = py
            .as_ref()
            .and_then(|p| p.get("tool"))
            .and_then(|t| t.get("hatch"))
            .and_then(|h| h.get("envs"))
            .and_then(|e| e.as_table())
        {
            for (env, cfg) in envs {
                let Some(scripts) = cfg.get("scripts").and_then(|s| s.as_table()) else {
                    continue;
                };
                for (name, v) in scripts {
                    let (body, _) = script_body(v);
                    let id = if env == "default" {
                        name.clone()
                    } else {
                        format!("{env}:{name}")
                    };
                    out.script("hatch", &id, &body);
                    let cmd = if env == "default" {
                        format!("hatch run {name}")
                    } else {
                        format!("hatch run {env}:{name}")
                    };
                    out.actions
                        .push(Action::new(id, cmd).tool("hatch").raw(body));
                }
            }
        }
        if let Some(tasks) = base.read("tasks.py") {
            // invoke: `@task` followed by `def name(c, ...)` with a docstring.
            let lines: Vec<&str> = tasks.lines().collect();
            for (i, l) in lines.iter().enumerate() {
                if !l.trim().starts_with("@task") {
                    continue;
                }
                let Some(def) = lines.get(i + 1).map(|d| d.trim()) else {
                    continue;
                };
                let Some(rest) = def.strip_prefix("def ") else {
                    continue;
                };
                let name = rest
                    .split('(')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .replace('_', "-");
                if name.is_empty() || name.starts_with('-') {
                    continue;
                }
                let doc = lines
                    .get(i + 2)
                    .map(|d| d.trim())
                    .and_then(|d| d.strip_prefix("\"\"\"").or_else(|| d.strip_prefix("'''")))
                    .map(|d| {
                        d.trim_end_matches("\"\"\"")
                            .trim_end_matches("'''")
                            .trim()
                            .to_string()
                    })
                    .filter(|d| !d.is_empty());
                let mut a = Action::new(&name, python(&format!("invoke {name}"))).tool("invoke");
                match doc {
                    Some(d) => a = a.desc(d),
                    None => a = a.inferred_desc(format!("Run the {name} invoke task")),
                }
                out.actions.push(a);
            }
        }
        let declared: Vec<String> = out.actions.iter().map(|a| a.name.clone()).collect();
        let free = |n: &str| !declared.iter().any(|d| d == n);

        // ---- Django ---------------------------------------------------------------------
        let django = base.has("manage.py");
        if django {
            let m = |c: &str| python(&format!("python manage.py {c}"));
            out.actions.push(
                Action::new("dev", m("runserver"))
                    .tool("django")
                    .inferred(Confidence::High)
                    .cat(Category::Development),
            );
            out.actions.push(
                Action::new("migrate", m("migrate"))
                    .tool("django")
                    .inferred(Confidence::High),
            );
            out.actions.push(
                Action::new("makemigrations", m("makemigrations"))
                    .tool("django")
                    .inferred(Confidence::High),
            );
            out.actions.push(
                Action::new("shell", m("shell"))
                    .tool("django")
                    .inferred(Confidence::Medium),
            );
            out.actions.push(
                Action::new("collectstatic", m("collectstatic --noinput"))
                    .tool("django")
                    .inferred(Confidence::Low),
            );
            out.actions.push(
                Action::new("createsuperuser", m("createsuperuser"))
                    .tool("django")
                    .inferred(Confidence::Low),
            );
        }

        // ---- tests ----------------------------------------------------------------------
        let pytest_cfg = base.has("pytest.ini")
            || tool_cfg("pytest")
            || base.has("conftest.py")
            || base
                .read("tox.ini")
                .map(|t| t.contains("[pytest]"))
                .unwrap_or(false)
            || base
                .read("setup.cfg")
                .map(|t| t.contains("[tool:pytest]"))
                .unwrap_or(false);
        let pytest_dep = has_dep(&deps, "pytest");
        if (pytest_cfg || pytest_dep || base.has_dir("tests") || base.has_dir("test"))
            && free("test")
        {
            let conf = if pytest_cfg || pytest_dep {
                Confidence::High
            } else {
                Confidence::Medium
            };
            out.actions.push(
                Action::new("test", python("pytest"))
                    .tool(tool)
                    .inferred(conf)
                    .cat(Category::Testing),
            );
        } else if django && free("test") {
            out.actions.push(
                Action::new("test", python("python manage.py test"))
                    .tool("django")
                    .inferred(Confidence::High)
                    .cat(Category::Testing),
            );
        }
        if django && (pytest_cfg || pytest_dep) {
            out.actions.push(
                Action::new("django:test", python("python manage.py test"))
                    .tool("django")
                    .inferred(Confidence::Low)
                    .cat(Category::Testing),
            );
        }
        if base.has("tox.ini") || tool_cfg("tox") {
            out.actions.push(
                Action::new("tox", python("tox"))
                    .tool("tox")
                    .inferred(Confidence::High)
                    .cat(Category::Testing),
            );
        }
        if let Some(nf) = base.read("noxfile.py") {
            for (name, doc) in nox_sessions(&nf) {
                let mut a = Action::new(format!("nox:{name}"), python(&format!("nox -s {name}")))
                    .tool("nox")
                    .inferred(Confidence::High);
                if let Some(d) = doc {
                    a = a.desc(d);
                } else {
                    a = a.inferred_desc(format!("Run the {name} nox session"));
                }
                out.actions.push(a);
            }
        }

        // ---- quality --------------------------------------------------------------------
        let ruff = base.has_any(&["ruff.toml", ".ruff.toml"])
            || tool_cfg("ruff")
            || has_dep(&deps, "ruff");
        let black = tool_cfg("black") || has_dep(&deps, "black");
        let flake8 = base.has(".flake8") || has_dep(&deps, "flake8");
        let mypy =
            base.has_any(&["mypy.ini", ".mypy.ini"]) || tool_cfg("mypy") || has_dep(&deps, "mypy");
        let pyright =
            base.has("pyrightconfig.json") || tool_cfg("pyright") || has_dep(&deps, "pyright");
        if ruff {
            if free("lint") {
                out.actions.push(
                    Action::new("lint", python("ruff check ."))
                        .tool(tool)
                        .inferred(Confidence::High),
                );
            }
            if free("format") {
                out.actions.push(
                    Action::new("format", python("ruff format ."))
                        .tool(tool)
                        .inferred(Confidence::High),
                );
            }
        } else {
            if flake8 && free("lint") {
                out.actions.push(
                    Action::new("lint", python("flake8"))
                        .tool(tool)
                        .inferred(Confidence::High),
                );
            }
            if black && free("format") {
                out.actions.push(
                    Action::new("format", python("black ."))
                        .tool(tool)
                        .inferred(Confidence::High),
                );
            }
        }
        if mypy && free("typecheck") {
            out.actions.push(
                Action::new("typecheck", python("mypy ."))
                    .tool(tool)
                    .inferred(Confidence::High),
            );
        } else if pyright && free("typecheck") {
            out.actions.push(
                Action::new("typecheck", python("pyright"))
                    .tool(tool)
                    .inferred(Confidence::High),
            );
        }
        if base.has(".pre-commit-config.yaml") && free("pre-commit") {
            out.actions.push(
                Action::new("pre-commit", python("pre-commit run --all-files"))
                    .tool("pre-commit")
                    .inferred(Confidence::Medium),
            );
        }

        // ---- web servers (only with a visible app object) -------------------------------
        if !django && free("dev") {
            for f in [
                "main.py",
                "app.py",
                "app/main.py",
                "src/main.py",
                "server.py",
            ] {
                let Some(text) = crate::repo::read_text(&base.path.join(f)) else {
                    continue;
                };
                let module = f.trim_end_matches(".py").replace('/', ".");
                if text.contains("FastAPI(") {
                    let var = "app";
                    out.actions.push(
                        Action::new("dev", python(&format!("uvicorn {module}:{var} --reload")))
                            .tool(tool)
                            .inferred(Confidence::Medium)
                            .cat(Category::Development),
                    );
                    break;
                }
                if text.contains("Flask(") {
                    out.actions.push(
                        Action::new("dev", python(&format!("flask --app {module} run --debug")))
                            .tool(tool)
                            .inferred(Confidence::Medium)
                            .cat(Category::Development),
                    );
                    break;
                }
            }
        }
        // ---- docs -----------------------------------------------------------------------
        if base.has("mkdocs.yml") && free("docs") {
            out.actions.push(
                Action::new("docs", python("mkdocs serve"))
                    .tool(tool)
                    .inferred(Confidence::High),
            );
        }
        out
    }
}

fn script_body(v: &Value) -> (String, Option<String>) {
    match v {
        Value::String(s) => (s.clone(), None),
        Value::Array(a) => (
            a.iter()
                .filter_map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            None,
        ),
        Value::Table(t) => {
            let help = t
                .get("help")
                .and_then(|h| h.as_str())
                .map(|s| s.to_string());
            let body = t
                .get("cmd")
                .or_else(|| t.get("shell"))
                .or_else(|| t.get("script"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    t.get("composite").and_then(|c| c.as_array()).map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .map(|x| format!("pdm run {x}"))
                            .collect::<Vec<_>>()
                            .join(" && ")
                    })
                })
                .or_else(|| {
                    t.get("sequence").and_then(|c| c.as_array()).map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .map(|x| format!("poe {x}"))
                            .collect::<Vec<_>>()
                            .join(" && ")
                    })
                })
                .unwrap_or_default();
            (body, help)
        }
        _ => (String::new(), None),
    }
}
