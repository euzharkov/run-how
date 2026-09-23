//! Terminal rendering: plain, aligned, category-grouped output with compact risk markers.
//! ANSI colour only when `--color` allows it: `--color` wins, then `NO_COLOR` (never), then
//! `CLICOLOR_FORCE` / `FORCE_COLOR` (always), then a TTY whose `TERM` is not `dumb`.

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
        let env = |k: &str| std::env::var(k).ok();
        let color = match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => auto_color(tty, &env),
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

/// The `--color auto` decision, with the environment injected so it can be tested:
/// `NO_COLOR` (set, even empty) turns colour off; `CLICOLOR_FORCE` / `FORCE_COLOR` (set to
/// anything but empty or `0`) turn it on even when piped; otherwise colour needs a TTY whose
/// `TERM` is not `dumb`.
pub fn auto_color(tty: bool, env: &dyn Fn(&str) -> Option<String>) -> bool {
    if env("NO_COLOR").is_some() {
        return false;
    }
    let forced = |k: &str| env(k).map(|v| !v.is_empty() && v != "0").unwrap_or(false);
    if forced("CLICOLOR_FORCE") || forced("FORCE_COLOR") {
        return true;
    }
    tty && env("TERM").map(|t| t != "dumb").unwrap_or(true)
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

/// Terminal columns a string occupies. Combining marks and zero-width characters take no
/// column; East Asian wide and fullwidth forms (and the emoji blocks a project description
/// may carry) take two; everything else takes one. Close enough to `wcwidth` for aligning
/// columns without a dependency; ANSI sequences are not expected here (strings are padded
/// before they are painted).
pub fn width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

fn char_width(c: char) -> usize {
    match c as u32 {
        // Combining marks and zero-width characters.
        0x0300..=0x036F | 0x20D0..=0x20FF | 0xFE20..=0xFE2F | 0x200B..=0x200D | 0xFEFF => 0,
        // East Asian wide and fullwidth ranges, plus the common emoji blocks.
        0x1100..=0x115F
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1F64F
        | 0x1F900..=0x1F9FF
        | 0x20000..=0x3FFFD => 2,
        _ => 1,
    }
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

/// Action ids that look like `id`, for "did you mean" hints: the six closest, closest first
/// (same last segment, then prefix, then substring, then ids the query contains), ties by
/// length and then name. An empty query matches nothing.
pub fn similar_ids<'a>(repo: &'a Repo, id: &str) -> Vec<&'a str> {
    let id = id.trim();
    if id.is_empty() {
        return Vec::new();
    }
    let rank = |c: &str| -> Option<usize> {
        if c.split(':').next_back() == Some(id) {
            Some(0)
        } else if c.starts_with(id) {
            Some(1)
        } else if c.contains(id) {
            Some(2)
        } else if id.contains(c) {
            Some(3)
        } else {
            None
        }
    };
    let mut close: Vec<(usize, usize, &str)> = repo
        .all_actions()
        .map(|(_, a)| a.id.as_str())
        .filter_map(|c| rank(c).map(|r| (r, c.len().abs_diff(id.len()), c)))
        .collect();
    close.sort();
    close.dedup();
    close.into_iter().map(|(_, _, c)| c).take(6).collect()
}

/// The action behind an id: the action id, the exact command text as printed in the
/// listing, or a runtime suggestion's id.
pub fn find_action<'a>(repo: &'a Repo, id: &str) -> Option<(&'a Project, &'a Action)> {
    repo.all_actions()
        .find(|(_, a)| a.id == id)
        .or_else(|| repo.all_actions().find(|(_, a)| a.command == id.trim()))
        .or_else(|| {
            repo.suggestions
                .iter()
                .find(|a| a.id == id)
                .map(|a| (&repo.projects[0], a))
        })
}

/// Detailed view of one action (`rhow <id>` / `rhow why <id>`).
pub fn render_why(repo: &Repo, id: &str, style: &Style) -> Option<String> {
    let (project, action) = find_action(repo, id)?;
    let mut o = String::new();
    o.push_str(&format!(
        "{}  {}\n",
        style.bold(&action.id),
        action.description
    ));
    // Every label is padded to the longest (`confidence`) before it is painted, so the
    // values line up whether or not colour is on. Plain text first, paint last:
    // lowercasing or padding a painted string would mangle the escape codes.
    let mut row = |label: &str, value: &str| {
        o.push_str(&format!("  {}  {}\n", style.dim(&pad(label, 10)), value));
    };
    row("command", &action.command);
    row("in", &action.working_directory);
    row("project", &format!("{} ({})", project.name, project.path));
    row(
        "source",
        &match action.source {
            ActionSource::Declared => format!("declared by the project ({})", action.tool),
            ActionSource::Inferred => format!("inferred from {} conventions", action.tool),
        },
    );
    row(
        "confidence",
        &format!("{:?}", action.confidence).to_lowercase(),
    );
    row(
        "risk",
        &match action.risk {
            Risk::Safe => "safe".to_string(),
            Risk::External => style.yellow("external"),
            Risk::Destructive => style.red("destructive"),
        },
    );
    if let Some(raw) = &action.raw {
        if raw != &action.command {
            row("runs", raw);
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
    use super::*;

    fn action(id: &str, command: &str, description: &str) -> Action {
        Action::new(id, command)
            .inferred_desc(description)
            .cwd(".")
            .tool("npm")
    }

    fn repo(actions: Vec<Action>) -> Repo {
        Repo {
            root: std::path::PathBuf::from("/r"),
            name: "r".into(),
            projects: vec![Project {
                name: "r".into(),
                path: ".".into(),
                kind: ProjectKind::JavaScript,
                tools: vec!["npm"],
                actions,
                versions: vec![],
            }],
            suggestions: vec![],
            ci: vec![],
        }
    }

    #[test]
    fn display_width_counts_columns_not_bytes_or_chars() {
        assert_eq!(width("abc"), 3);
        assert_eq!(width("日本語"), 6);
        assert_eq!(width("한글"), 4);
        assert_eq!(width("e\u{301}"), 1); // e + combining acute
        assert_eq!(width("a\u{200B}b"), 2); // zero-width space
        assert_eq!(width("ｆｕｌｌ"), 8);
        assert_eq!(width("\u{1F600}"), 2);
        assert_eq!(pad("日本", 6), "日本  ");
    }

    #[test]
    fn columns_align_by_display_width() {
        let r = repo(vec![
            action("build", "npm run build", "Build the app"),
            action("docs", "npm run 文档", "Build the docs"),
            action("check", "npm run che\u{301}ck", "Check the app"),
        ]);
        let style = Style {
            color: false,
            tty: false,
        };
        let out = render(
            &r,
            &style,
            &RenderOptions {
                all: false,
                group: false,
            },
        );
        let starts: Vec<usize> = out
            .lines()
            .filter(|l| l.starts_with("  npm"))
            .map(|l| {
                let (cmd, _) = l
                    .split_once("  Build")
                    .or_else(|| l.split_once("  Check"))
                    .unwrap();
                width(cmd)
            })
            .collect();
        assert_eq!(starts.len(), 3);
        assert!(starts.iter().all(|w| *w == starts[0]), "{out}");
    }

    #[test]
    fn similar_ids_are_ranked_and_empty_query_matches_nothing() {
        let r = repo(vec![
            action("api:test", "x", ""),
            action("test", "x", ""),
            action("test:watch", "x", ""),
            action("pretest", "x", ""),
            action("lint", "x", ""),
            action("build", "x", ""),
            action("contest", "x", ""),
            action("testing:all", "x", ""),
        ]);
        assert!(similar_ids(&r, "").is_empty());
        assert!(similar_ids(&r, "   ").is_empty());
        let close = similar_ids(&r, "test");
        assert_eq!(close.len(), 6);
        assert_eq!(close[0], "test");
        assert_eq!(close[1], "api:test");
        assert_eq!(close[2], "test:watch");
        assert!(!close.contains(&"lint"));
        assert!(!close.contains(&"build"));
        assert_eq!(similar_ids(&r, "zzz"), Vec::<&str>::new());
    }

    #[test]
    fn why_paints_each_field_once() {
        let r = repo(vec![action("build", "npm run build", "Build the app")]);
        let style = Style {
            color: true,
            tty: true,
        };
        let out = render_why(&r, "build", &style).unwrap();
        let line = out.lines().find(|l| l.contains("confidence")).unwrap();
        assert_eq!(line.matches("\x1b[2m").count(), 1, "{line:?}");
        assert!(line.ends_with("  exact"), "{line:?}");
    }

    #[test]
    fn color_detection_order() {
        let env = |vars: &'static [(&str, &str)]| {
            move |k: &str| {
                vars.iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| v.to_string())
            }
        };
        assert!(auto_color(true, &env(&[])));
        assert!(!auto_color(false, &env(&[])));
        assert!(!auto_color(true, &env(&[("TERM", "dumb")])));
        assert!(!auto_color(true, &env(&[("NO_COLOR", "")])));
        assert!(!auto_color(
            true,
            &env(&[("NO_COLOR", "1"), ("FORCE_COLOR", "1")])
        ));
        assert!(auto_color(false, &env(&[("CLICOLOR_FORCE", "1")])));
        assert!(auto_color(false, &env(&[("FORCE_COLOR", "3")])));
        assert!(!auto_color(false, &env(&[("FORCE_COLOR", "0")])));
        assert!(!auto_color(false, &env(&[("FORCE_COLOR", "")])));
        assert!(!auto_color(false, &env(&[("CLICOLOR_FORCE", "0")])));
    }

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
