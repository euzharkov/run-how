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
    /// Order each project's commands by type (run, build, deploy, test, …) instead of the
    /// order the project declares them in.
    pub group: bool,
}

/// The order types are shown in with `--group`: what you run first, then what you ship,
/// then what checks it.
pub const GROUP_ORDER: [Category; 8] = [
    Category::Development,
    Category::Build,
    Category::Release,
    Category::Testing,
    Category::Quality,
    Category::Database,
    Category::Infrastructure,
    Category::Other,
];

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
///
/// A repository with one project lists its actions with no headers, ordered by type with a
/// blank line between types. A repository with several projects shows one block per project
/// (root first), each ordered by type inside, so "what can I do here" reads top to bottom.
pub fn render(repo: &Repo, style: &Style, opts: &RenderOptions) -> String {
    let mut out = String::new();
    let shown: Vec<(&Project, &Action)> = repo
        .all_actions()
        .filter(|(_, a)| opts.all || a.is_primary())
        .collect();
    let hidden = repo.all_actions().count() - shown.len();

    out.push_str(&style.bold(&repo.name));
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

    // The command is what a developer types, so it leads the line; the explanation follows.
    // Column width is per block, so one long command does not push every block's text right.
    let block_width = |actions: &[&Action]| -> usize {
        actions
            .iter()
            .map(|a| width(&a.command))
            .max()
            .unwrap_or(4)
            .clamp(4, 52)
    };
    let line = |a: &Action, cmd_w: usize| -> String {
        let cmd = if width(&a.command) > cmd_w {
            a.command.clone()
        } else {
            pad(&a.command, cmd_w)
        };
        let mut l = format!("  {}  ", style.cyan(&cmd));
        if !a.opaque && !restates(&a.command, &a.description) {
            l.push_str(&a.description);
        }
        l.push_str(&trailer(style, a.risk, &a.notes));
        l.trim_end().to_string()
    };

    // Projects that have something to show, root first (discovery order is depth-first).
    let projects: Vec<&Project> = repo
        .projects
        .iter()
        .filter(|p| p.actions.iter().any(|a| opts.all || a.is_primary()))
        .collect();
    let single = projects.len() == 1;

    for p in &projects {
        let visible = |a: &&Action| opts.all || a.is_primary();
        let block: Vec<&Action> = p.actions.iter().filter(visible).collect();
        let cmd_w = block_width(&block);
        if single && !opts.group {
            out.push('\n');
            for a in &block {
                out.push_str(&line(a, cmd_w));
                out.push('\n');
            }
        } else if single {
            // No headers: type blocks separated by a blank line.
            for cat in GROUP_ORDER {
                let items: Vec<&Action> = block
                    .iter()
                    .copied()
                    .filter(|a| a.category == cat)
                    .collect();
                if items.is_empty() {
                    continue;
                }
                out.push('\n');
                for a in items {
                    out.push_str(&line(a, cmd_w));
                    out.push('\n');
                }
            }
        } else if !opts.group {
            out.push('\n');
            if !p.is_root() {
                out.push_str(&style.bold(&p.path));
                out.push('\n');
            }
            for a in &block {
                out.push_str(&line(a, cmd_w));
                out.push('\n');
            }
        } else {
            out.push('\n');
            // The root's actions sit directly under the repository name; nested projects
            // get a title with their path.
            if !p.is_root() {
                out.push_str(&style.bold(&p.path));
                out.push('\n');
            }
            for cat in GROUP_ORDER {
                for a in block.iter().filter(|a| a.category == cat) {
                    out.push_str(&line(a, cmd_w));
                    out.push('\n');
                }
            }
        }
    }

    if !repo.suggestions.is_empty() {
        out.push('\n');
        out.push_str(&style.bold("Runtime"));
        out.push('\n');
        let cmd_w = block_width(&repo.suggestions.iter().collect::<Vec<_>>());
        for a in &repo.suggestions {
            out.push_str(&format!(
                "  {}  {}\n",
                style.cyan(&pad(&a.command, cmd_w)),
                a.description
            ));
        }
    }

    if style.tty {
        out.push('\n');
        let mut foot = String::from(
            "`rhow why <command>` shows where a command comes from and why it is flagged.",
        );
        if hidden > 0 && !opts.all {
            foot.push_str(&format!(" {hidden} more with --all."));
        }
        out.push_str(&style.dim(&foot));
        out.push('\n');
    }
    out
}

/// A description whose every meaningful word is already in the command ("Run test across
/// all Nx projects" for `nx run-many -t test`, "Run scripts/foo.sh" for `./scripts/foo.sh`)
/// adds nothing; the line is shorter without it.
pub fn restates(command: &str, description: &str) -> bool {
    const FILLER: &[&str] = &[
        "run",
        "runs",
        "the",
        "a",
        "an",
        "across",
        "all",
        "every",
        "nx",
        "project",
        "projects",
        "script",
        "scripts",
        "target",
        "task",
        "in",
        "with",
        "for",
        "workspace",
        "package",
        "packages",
        "of",
        "to",
        "and",
        "then",
        "make",
        "rake",
        "gradle",
        "just",
        "composer",
        "dependencies",
        "dependency",
        "default",
        "targets",
    ];
    let cmd = command.to_ascii_lowercase();
    // `nx run owner:lint` offers both `owner:lint` and `lint` as words.
    let cmd_words: Vec<&str> = cmd
        .split(|c: char| {
            !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':' && c != '/'
        })
        .flat_map(|w| std::iter::once(w).chain(w.split(':')))
        .filter(|w| !w.is_empty())
        .collect();
    let mut meaningful = 0;
    for w in description
        .to_ascii_lowercase()
        .split(|c: char| {
            !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':' && c != '/'
        })
        .filter(|w| !w.is_empty())
    {
        if FILLER.contains(&w) {
            continue;
        }
        meaningful += 1;
        let w = w.trim_start_matches("./");
        if !cmd_words
            .iter()
            .any(|c| c.trim_start_matches("./") == w || c.ends_with(&format!("/{w}")))
        {
            return false;
        }
    }
    meaningful > 0
}

/// Risk and notes as a trailing aside: `  ● external  · long-running`.
fn trailer(style: &Style, risk: Risk, notes: &[Note]) -> String {
    let mut t = String::new();
    // A coloured dot (U+25CF: single width, in every default terminal font on macOS, Linux
    // and Windows) carries the severity; without colour the bracketed word does.
    let mark = |word: &str, paint: &dyn Fn(&str) -> String| {
        if style.color {
            paint(&format!("● {word}"))
        } else {
            format!("[{word}]")
        }
    };
    match risk {
        Risk::Safe => {}
        Risk::External => t.push_str(&format!("  {}", mark("external", &|s| style.yellow(s)))),
        Risk::Destructive => t.push_str(&format!("  {}", mark("destructive", &|s| style.red(s)))),
    }
    for n in notes {
        t.push_str(&style.dim(&format!("  {} {}", n.icon(), n.label())));
    }
    t
}

/// `rhow --ci`: every pipeline as workflow → job → run steps, each step explained.
pub fn render_ci(repo: &Repo, style: &Style) -> String {
    let mut out = String::new();
    out.push_str(&style.bold(&repo.name));
    out.push('\n');
    if repo.ci.is_empty() {
        out.push_str(
            "\nNo CI pipelines found (looked for .github/workflows/*.yml and .gitlab-ci.yml).\n",
        );
        return out;
    }
    for p in &repo.ci {
        out.push('\n');
        let mut head = format!("{}  {}", style.bold(&p.name), style.dim(&p.file));
        if !p.triggers.is_empty() {
            head.push_str(&style.dim(&format!("  on {}", p.triggers.join(", "))));
        }
        out.push_str(&head);
        out.push('\n');
        for j in &p.jobs {
            let mut title = format!("  {}", style.cyan(&j.name));
            if let Some(r) = &j.runs_on {
                title.push_str(&style.dim(&format!("  {r}")));
            }
            out.push_str(&title);
            out.push('\n');
            let w = j
                .steps
                .iter()
                .map(|s| width(s.command.lines().next().unwrap_or("")))
                .max()
                .unwrap_or(4)
                .clamp(4, 52);
            for s in &j.steps {
                let mut lines = s.command.lines();
                let first = lines.next().unwrap_or("");
                let more = lines.count();
                let shown = if more > 0 {
                    format!("{first} …")
                } else {
                    first.to_string()
                };
                let cmd = if width(&shown) > w {
                    shown
                } else {
                    pad(&shown, w)
                };
                let mut l = format!("    {}  ", style.cyan(&cmd));
                if !restates(&s.command, &s.description) {
                    l.push_str(&s.description);
                }
                l.push_str(&trailer(style, s.risk, &s.notes));
                out.push_str(l.trim_end());
                out.push('\n');
            }
        }
    }
    out
}

/// Action ids that look like `id`, for "did you mean" hints. At most six, sorted.
pub fn similar_ids<'a>(repo: &'a Repo, id: &str) -> Vec<&'a str> {
    let mut close: Vec<&str> = repo
        .all_actions()
        .map(|(_, a)| a.id.as_str())
        .filter(|c| c.contains(id) || id.contains(*c) || c.split(':').next_back() == Some(id))
        .take(6)
        .collect();
    close.sort();
    close
}

/// Detailed view of one action (`rhow <id>` / `rhow why <id>`).
pub fn render_why(repo: &Repo, id: &str, style: &Style) -> Option<String> {
    // Accept the action id or the exact command text as printed in the listing.
    let (project, action) = repo
        .all_actions()
        .find(|(_, a)| a.id == id)
        .or_else(|| repo.all_actions().find(|(_, a)| a.command == id.trim()))
        .or_else(|| {
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
            let notes: Vec<&str> = a.notes.iter().map(|n| n.label()).collect();
            o.push_str(&format!(
                "{:<24} {:<14} {:<8} {:<6} {:<11} {}{:<10} {} :: {}{}\n",
                a.id,
                a.category.label(),
                format!("{:?}", a.source).to_lowercase(),
                format!("{:?}", a.confidence).to_lowercase(),
                a.risk.label(),
                if a.hidden { "hidden " } else { "" },
                a.working_directory,
                a.command,
                a.description,
                if notes.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", notes.join(","))
                }
            ));
        }
    }
    for p in &repo.ci {
        o.push_str(&format!(
            "## ci {} [{}] {} on={}\n",
            p.file,
            p.name,
            p.system,
            p.triggers.join("|")
        ));
        for j in &p.jobs {
            o.push_str(&format!(
                "job {} ({})\n",
                j.name,
                j.runs_on.as_deref().unwrap_or("-")
            ));
            for s in &j.steps {
                let notes: Vec<&str> = s.notes.iter().map(|n| n.label()).collect();
                o.push_str(&format!(
                    "  {:<11} {} :: {}{}\n",
                    s.risk.label(),
                    s.command.lines().next().unwrap_or(""),
                    s.description,
                    if notes.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", notes.join(","))
                    }
                ));
            }
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::restates;

    #[test]
    fn tautological_descriptions_are_dropped() {
        assert!(restates(
            "nx run-many -t eas-pre-ota",
            "Run eas-pre-ota across all Nx projects"
        ));
        assert!(restates("./scripts/foo.sh", "Run scripts/foo.sh"));
        assert!(restates("npm run test", "Run the test script"));
        assert!(restates(
            "nx run owner:eas-ota-deploy",
            "Run the eas-ota-deploy target"
        ));
        assert!(restates(
            "nx run owner:quality",
            "Run the quality dependencies"
        ));
        assert!(!restates("npm run test", "Run Vitest tests"));
        assert!(!restates("nx run-many -t build", "Build the Angular app"));
        assert!(!restates("make docs", "Build the HTML documentation"));
    }
}
