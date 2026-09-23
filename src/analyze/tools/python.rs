//! Python: one slice of the tool knowledge table behind `analyze::tools::summarize`.

#[allow(unused_imports)]
use super::Kind::*;
#[allow(unused_imports)]
use super::{clean_path, compose, is_build_output, list};
use super::{s, Args, Summary};
#[allow(unused_imports)]
use crate::model::Risk::*;

#[allow(unused_variables, clippy::needless_return)]
pub(super) fn summarize(
    program: &str,
    args: &[String],
    a: &Args,
    sub: Option<&str>,
) -> Option<Summary> {
    match program {
        "pytest" | "py.test" => {
            let target = a.positionals().first().map(|p| clean_path(p));
            match target {
                Some(t) if !t.is_empty() && !t.starts_with('-') => {
                    s("pytest", format!("Run Python tests in {t}"), Test, Safe)
                }
                _ => s(
                    "pytest",
                    if a.has("--cov") {
                        "Run Python tests with coverage"
                    } else {
                        "Run Python tests"
                    },
                    Test,
                    Safe,
                ),
            }
        }
        "coverage" => match sub {
            Some("run") => s("coverage", "Run Python tests with coverage", Test, Safe),
            Some("report") | Some("html") | Some("xml") => {
                s("coverage", "Produce the coverage report", Test, Safe)
            }
            Some(x) => s("coverage", format!("Run coverage {x}"), Test, Safe),
            None => s("coverage", "Run coverage", Test, Safe),
        },
        "ruff" => match sub {
            None | Some("check") => s(
                "ruff",
                if a.has("--fix") {
                    "Fix lint issues with Ruff"
                } else {
                    "Check Python source code with Ruff"
                },
                Lint,
                Safe,
            ),
            Some("format") => s(
                "ruff",
                if a.has("--check") {
                    "Check Python formatting with Ruff"
                } else {
                    "Format Python code with Ruff"
                },
                Format,
                Safe,
            ),
            Some(x) => s("ruff", format!("Run ruff {x}"), Lint, Safe),
        },
        "mypy" => s(
            "mypy",
            "Run static type checking with mypy",
            TypeCheck,
            Safe,
        ),
        "pyright" | "basedpyright" => s(
            "pyright",
            "Run static type checking with Pyright",
            TypeCheck,
            Safe,
        ),
        "pyre" => s(
            "pyre",
            "Run static type checking with Pyre",
            TypeCheck,
            Safe,
        ),
        "black" => s(
            "black",
            if a.has("--check") {
                "Check Python formatting with Black"
            } else {
                "Format Python code with Black"
            },
            Format,
            Safe,
        ),
        "isort" => s(
            "isort",
            if a.has("--check") || a.has("--check-only") {
                "Check import ordering with isort"
            } else {
                "Sort Python imports with isort"
            },
            Format,
            Safe,
        ),
        "flake8" => s("flake8", "Check Python source code with Flake8", Lint, Safe),
        "pylint" => s("pylint", "Check Python source code with Pylint", Lint, Safe),
        "bandit" => s("bandit", "Scan Python code for security issues", Lint, Safe),
        "pyflakes" => s(
            "pyflakes",
            "Check Python source code with Pyflakes",
            Lint,
            Safe,
        ),
        "pip" | "pip3" => match sub {
            Some("install") => s("pip", "Install Python dependencies with pip", Install, Safe),
            Some("freeze") => s("pip", "List installed Python packages", Other, Safe),
            Some("uninstall") => s("pip", "Uninstall Python packages", Install, Safe),
            Some("compile") => s("pip", "Compile Python requirements", Install, Safe),
            Some(x) => s("pip", format!("Run pip {x}"), Install, Safe),
            None => s("pip", "Run pip", Install, Safe),
        },
        "pip-compile" => s(
            "pip-tools",
            "Compile pinned Python requirements",
            Install,
            Safe,
        ),
        "pip-sync" => s(
            "pip-tools",
            "Sync the environment to pinned requirements",
            Install,
            Safe,
        ),
        "uv" => match sub {
            Some("sync") => s("uv", "Sync the Python environment with uv", Install, Safe),
            Some("lock") => s("uv", "Update the uv lockfile", Install, Safe),
            Some("build") => s("uv", "Build the Python package", Build, Safe),
            Some("publish") => s("uv", "Publish the package to PyPI", Publish, External),
            Some("pip") => s("uv", "Manage packages with uv pip", Install, Safe),
            Some("venv") => s("uv", "Create a virtual environment", Install, Safe),
            Some("add") | Some("remove") => {
                s("uv", "Change project dependencies with uv", Install, Safe)
            }
            Some("tool") => s("uv", "Run a tool with uv", Other, Safe),
            Some(x) => s("uv", format!("Run uv {x}"), Other, Safe),
            None => s("uv", "Run uv", Other, Safe),
        },
        "poetry" => match sub {
            Some("install") => s("poetry", "Install dependencies with Poetry", Install, Safe),
            Some("lock") => s("poetry", "Update the Poetry lockfile", Install, Safe),
            Some("build") => s("poetry", "Build the Python package", Build, Safe),
            Some("publish") => s("poetry", "Publish the package to PyPI", Publish, External),
            Some("update") => s("poetry", "Update dependencies with Poetry", Install, Safe),
            Some("check") => s("poetry", "Validate pyproject.toml", Lint, Safe),
            Some(x) => s("poetry", format!("Run poetry {x}"), Other, Safe),
            None => s("poetry", "Run Poetry", Other, Safe),
        },
        "pdm" | "hatch" | "rye" | "pipenv" => match sub {
            Some("install") | Some("sync") => s(
                program,
                format!("Install dependencies with {program}"),
                Install,
                Safe,
            ),
            Some("build") => s(program, "Build the Python package", Build, Safe),
            Some("publish") => s(program, "Publish the package to PyPI", Publish, External),
            Some("test") => s(program, format!("Run tests with {program}"), Test, Safe),
            Some("fmt") => s(program, format!("Format code with {program}"), Format, Safe),
            Some("lint") => s(program, format!("Lint code with {program}"), Lint, Safe),
            Some(x) => s(program, format!("Run {program} {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "twine" => s(
            "twine",
            if sub == Some("upload") {
                "Upload the package to PyPI"
            } else {
                "Check the package with twine"
            },
            if sub == Some("upload") { Publish } else { Lint },
            if sub == Some("upload") {
                External
            } else {
                Safe
            },
        ),
        "pyproject-build" => s("build", "Build the Python package", Build, Safe),
        "tox" => match a.value("-e") {
            Some(e) => s("tox", format!("Run the {e} tox environment"), Test, Safe),
            None => s("tox", "Run tox test environments", Test, Safe),
        },
        "nox" => match a.value("-s").or_else(|| a.value("--session")) {
            Some(e) => s("nox", format!("Run the {e} nox session"), Test, Safe),
            None => s("nox", "Run nox sessions", Test, Safe),
        },
        "pre-commit" => match sub {
            Some("run") => s(
                "pre-commit",
                if a.has("--all-files") || a.has("-a") {
                    "Run pre-commit hooks on all files"
                } else {
                    "Run pre-commit hooks"
                },
                Lint,
                Safe,
            ),
            Some("install") => s("pre-commit", "Install pre-commit Git hooks", Install, Safe),
            Some("autoupdate") => s(
                "pre-commit",
                "Update pre-commit hook versions",
                Install,
                Safe,
            ),
            Some(x) => s("pre-commit", format!("Run pre-commit {x}"), Lint, Safe),
            None => s("pre-commit", "Run pre-commit", Lint, Safe),
        },
        "uvicorn" => {
            let app = sub.unwrap_or("the app");
            if a.has("--reload") {
                s(
                    "uvicorn",
                    format!("Start {app} with Uvicorn and auto-reload"),
                    Dev,
                    Safe,
                )
            } else {
                s("uvicorn", format!("Start {app} with Uvicorn"), Dev, Safe)
            }
        }
        "gunicorn" => s(
            "gunicorn",
            format!("Start {} with Gunicorn", sub.unwrap_or("the app")),
            Dev,
            Safe,
        ),
        "hypercorn" | "daphne" | "waitress-serve" => s(
            program,
            format!("Start {} with {program}", sub.unwrap_or("the app")),
            Dev,
            Safe,
        ),
        "flask" => match a.find(&["run", "shell", "routes", "db"]) {
            Some("run") => s("flask", "Start the Flask development server", Dev, Safe),
            Some("shell") => s("flask", "Open the Flask shell", Dev, Safe),
            Some("routes") => s("flask", "List Flask routes", Other, Safe),
            Some("db") => {
                let p = a.after("db");
                match p.first().copied() {
                    Some("upgrade") => s(
                        "flask",
                        "Apply pending Flask-Migrate migrations",
                        Migrate,
                        Safe,
                    ),
                    Some("downgrade") => s(
                        "flask",
                        "Roll back Flask-Migrate migrations",
                        Migrate,
                        Destructive,
                    ),
                    Some("migrate") => {
                        s("flask", "Generate a Flask-Migrate migration", Migrate, Safe)
                    }
                    _ => s("flask", "Manage Flask-Migrate migrations", Migrate, Safe),
                }
            }
            _ => s("flask", "Run Flask", Other, Safe),
        },
        "fastapi" => match sub {
            Some("dev") => s("fastapi", "Start FastAPI with auto-reload", Dev, Safe),
            Some("run") => s("fastapi", "Start the FastAPI app", Dev, Safe),
            Some(x) => s("fastapi", format!("Run fastapi {x}"), Other, Safe),
            None => s("fastapi", "Run FastAPI", Other, Safe),
        },
        "streamlit" => s(
            "streamlit",
            format!(
                "Start the Streamlit app {}",
                a.positionals()
                    .get(1)
                    .map(|p| clean_path(p))
                    .unwrap_or_default()
            )
            .trim()
            .to_string(),
            Dev,
            Safe,
        ),
        "celery" => match a.find(&["worker", "beat", "flower", "purge"]) {
            Some("worker") => s("celery", "Start a Celery worker", Dev, Safe),
            Some("beat") => s("celery", "Start the Celery beat scheduler", Dev, Safe),
            Some("flower") => s("celery", "Start the Flower monitoring UI", Dev, Safe),
            Some("purge") => s("celery", "Purge all Celery task queues", Other, Destructive),
            _ => s("celery", "Run Celery", Other, Safe),
        },
        "jupyter" | "jupyter-lab" | "jupyter-notebook" => s("jupyter", "Start Jupyter", Dev, Safe),
        "mkdocs" => match sub {
            Some("serve") => s("mkdocs", "Serve the MkDocs site locally", Dev, Safe),
            Some("build") => s("mkdocs", "Build the MkDocs site", Docs, Safe),
            Some("gh-deploy") => s(
                "mkdocs",
                "Deploy the MkDocs site to GitHub Pages",
                Deploy,
                External,
            ),
            Some(x) => s("mkdocs", format!("Run mkdocs {x}"), Docs, Safe),
            None => s("mkdocs", "Run MkDocs", Docs, Safe),
        },
        "sphinx-build" | "sphinx-autobuild" => s(
            "sphinx",
            if program == "sphinx-autobuild" {
                "Serve the Sphinx docs with auto-rebuild"
            } else {
                "Build the Sphinx docs"
            },
            Docs,
            Safe,
        ),
        "manage.py" | "python-module:django" | "django-admin" => {
            let p = a.positionals();
            match p.first().copied() {
                Some("runserver") | Some("runserver_plus") => {
                    s("django", "Start the Django development server", Dev, Safe)
                }
                Some("migrate") => s("django", "Apply Django database migrations", Migrate, Safe),
                Some("makemigrations") => s(
                    "django",
                    "Create Django migrations from model changes",
                    Migrate,
                    Safe,
                ),
                Some("test") => s("django", "Run Django tests", Test, Safe),
                Some("shell") | Some("shell_plus") => {
                    s("django", "Open the Django shell", Dev, Safe)
                }
                Some("collectstatic") => s("django", "Collect Django static files", Build, Safe),
                Some("createsuperuser") => s("django", "Create a Django admin user", Db, Safe),
                Some("flush") => s(
                    "django",
                    "Delete all data from the Django database",
                    Db,
                    Destructive,
                ),
                Some("loaddata") => s("django", "Load Django fixtures into the database", Db, Safe),
                Some("dumpdata") => s("django", "Dump Django data as fixtures", Db, Safe),
                Some("check") => s("django", "Run Django system checks", Lint, Safe),
                Some("showmigrations") => s("django", "Show Django migration status", Db, Safe),
                Some("sqlmigrate") => s("django", "Print SQL for a Django migration", Db, Safe),
                Some(x) => s("django", format!("Run the Django {x} command"), Other, Safe),
                None => s("django", "Run Django management commands", Other, Safe),
            }
        }

        _ => None,
    }
}
