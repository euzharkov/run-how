//! Terminal rendering: plain, aligned, category-grouped output with compact risk markers.
//! ANSI colour only when stdout is a TTY, `NO_COLOR` is unset and `--color` allows it.

pub mod json;
pub mod support;

use crate::model::*;
use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

pub struct Style {
    pub color: bool,
    pub tty: bool,
}

impl Style {
    pub fn detect(mode: ColorMode) -> Self {
        let tty = std::io::stdout().is_terminal();
        let color = match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                tty && std::env::var_os("NO_COLOR")
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                    && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
            }
        };
        Style { color, tty }
    }
    fn paint(&self, code: &str, s: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
    fn bold(&self, s: &str) -> String {
        self.paint("1", s)
    }
    fn dim(&self, s: &str) -> String {
        self.paint("2", s)
    }
    fn yellow(&self, s: &str) -> String {
        self.paint("33", s)
    }
    fn red(&self, s: &str) -> String {
        self.paint("31", s)
    }
    fn cyan(&self, s: &str) -> String {
        self.paint("36", s)
    }
}

pub struct RenderOptions {
    pub all: bool,
}

fn width(s: &str) -> usize {
    s.chars().count()
}

fn pad(s: &str, w: usize) -> String {
    let n = width(s);
    if n >= w {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(w - n))
    }
}

/// Render the repository overview.
pub fn render(repo: &Repo, style: &Style, opts: &RenderOptions) -> String {
    let mut out = String::new();
    let shown: Vec<(&Project, &Action)> = repo
        .all_actions()
        .filter(|(_, a)| opts.all || a.is_primary())
        .collect();
    let hidden = repo.all_actions().count() - shown.len();

    // Header
    let mut header = style.bold(&repo.name);
    let nested: Vec<&str> = repo
        .projects
        .iter()
        .filter(|p| !p.is_root() && !p.actions.is_empty())
        .map(|p| p.name.as_str())
        .collect();
    if !nested.is_empty() {
        let list = if nested.len() > 8 {
            format!("{} and {} more", nested[..8].join(", "), nested.len() - 8)
        } else {
            nested.join(", ")
        };
        header.push_str(&style.dim(&format!(
            "  ·  {} project{}: {}",
            nested.len(),
            if nested.len() == 1 { "" } else { "s" },
            list
        )));
    }
    out.push_str(&header);
    out.push('\n');

    if shown.is_empty() {
        out.push('\n');
        out.push_str("No project actions found.\n");
        if hidden > 0 {
            out.push_str(&format!(
                "{hidden} low-confidence action{} available with --all.\n",
                if hidden == 1 { "" } else { "s" }
            ));
        } else {
            out.push_str("Looked for package.json, Makefile, justfile, Taskfile, pyproject.toml, go.mod, Cargo.toml,\n.NET solutions/projects, Dockerfiles, Compose files and Kubernetes manifests.\n");
        }
        return out;
    }

    let id_w = shown
        .iter()
        .map(|(_, a)| width(&a.id))
        .max()
        .unwrap_or(4)
        .clamp(4, 28);
    let any_risk = shown.iter().any(|(_, a)| a.risk != Risk::Safe);
    let desc_w = if any_risk {
        shown
            .iter()
            .map(|(_, a)| width(&a.description))
            .max()
            .unwrap_or(0)
            .min(60)
    } else {
        0
    };

    for cat in Category::ALL {
        let items: Vec<&(&Project, &Action)> =
            shown.iter().filter(|(_, a)| a.category == cat).collect();
        if items.is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str(&style.bold(cat.label()));
        out.push('\n');
        for (_, a) in items {
            let id = if width(&a.id) > id_w {
                a.id.clone()
            } else {
                pad(&a.id, id_w)
            };
            let id_s = style.cyan(&id);
            let mut line = format!("  {id_s}  ");
            let desc = a.description.clone();
            match a.risk {
                Risk::Safe => {
                    line.push_str(&desc);
                    if opts.all && !a.is_primary() {
                        line.push_str(&style.dim("  (low confidence)"));
                    }
                }
                Risk::External => {
                    line.push_str(&pad(&desc, desc_w));
                    line.push_str("  ");
                    line.push_str(&style.yellow("⚠ external"));
                }
                Risk::Destructive => {
                    line.push_str(&pad(&desc, desc_w));
                    line.push_str("  ");
                    line.push_str(&style.red("⚠ destructive"));
                }
            }
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }

    if !repo.suggestions.is_empty() {
        out.push('\n');
        out.push_str(&style.bold("Runtime"));
        out.push('\n');
        for a in &repo.suggestions {
            out.push_str(&format!(
                "  {}  {}\n",
                style.cyan(&pad(&a.id, id_w)),
                a.description
            ));
        }
    }

    if style.tty {
        out.push('\n');
        let mut foot = String::from("Run an action with `rhow <name>`.");
        if hidden > 0 && !opts.all {
            foot.push_str(&format!(" {hidden} more with --all."));
        }
        out.push_str(&style.dim(&foot));
        out.push('\n');
    }
    out
}

/// Detailed view of one action (`rhow why <id>`).
pub fn render_why(repo: &Repo, id: &str, style: &Style) -> Option<String> {
    let (project, action) = repo.all_actions().find(|(_, a)| a.id == id).or_else(|| {
        repo.suggestions
            .iter()
            .find(|a| a.id == id)
            .map(|a| (&repo.projects[0], a))
    })?;
    let mut o = String::new();
    o.push_str(&format!(
        "{}  {}\n",
        style.bold(&action.id),
        action.description
    ));
    o.push_str(&format!(
        "  {}   {}\n",
        style.dim("command "),
        action.command
    ));
    o.push_str(&format!(
        "  {}   {}\n",
        style.dim("in      "),
        if action.working_directory == "." {
            "."
        } else {
            &action.working_directory
        }
    ));
    o.push_str(&format!(
        "  {}   {} ({})\n",
        style.dim("project "),
        project.name,
        project.path
    ));
    o.push_str(&format!(
        "  {}   {}\n",
        style.dim("source  "),
        match action.source {
            ActionSource::Declared => format!("declared by the project ({})", action.tool),
            ActionSource::Inferred => format!("inferred from {} conventions", action.tool),
        }
    ));
    o.push_str(
        &format!("  {}   {:?}\n", style.dim("confidence"), action.confidence)
            .to_lowercase()
            .replacen("confidence", &style.dim("confidence"), 1),
    );
    o.push_str(&format!(
        "  {}   {}\n",
        style.dim("risk    "),
        match action.risk {
            Risk::Safe => "safe".to_string(),
            Risk::External => style.yellow("external"),
            Risk::Destructive => style.red("destructive"),
        }
    ));
    if let Some(raw) = &action.raw {
        if raw != &action.command {
            o.push_str(&format!("  {}   {}\n", style.dim("runs    "), raw));
        }
    }
    Some(o)
}

/// Deterministic tabular dump used by snapshot tests.
pub fn debug_table(repo: &Repo) -> String {
    let mut o = String::new();
    for p in &repo.projects {
        o.push_str(&format!(
            "## {} [{}] {:?} tools={}\n",
            p.path,
            p.name,
            p.kind,
            p.tools.join(",")
        ));
        for a in &p.actions {
            o.push_str(&format!(
                "{:<24} {:<14} {:<8} {:<6} {:<11} {}{:<10} {} :: {}\n",
                a.id,
                a.category.label(),
                format!("{:?}", a.source).to_lowercase(),
                format!("{:?}", a.confidence).to_lowercase(),
                a.risk.label(),
                if a.hidden { "hidden " } else { "" },
                a.working_directory,
                a.command,
                a.description
            ));
        }
    }
    o
}
