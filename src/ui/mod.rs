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

/// Widest command column in a listing. Longer commands are shortened with [`elide`]; the
/// full text is one `rhow <row>` away.
const MAX_COMMAND_WIDTH: usize = 40;

/// Shorten `s` to at most `max` columns by cutting out its middle: `docker compose -f
/// docker/compose.yml up -d postgres` becomes `docker compose -f … up -d postgres`. The head
/// (the program) and the tail (the target, which is what tells similar commands apart) both
/// survive, and the cuts land on word boundaries when there are any.
pub(crate) fn elide(s: &str, max: usize) -> String {
    if width(s) <= max {
        return s.to_string();
    }
    let budget = max.saturating_sub(3); // room for " … "
    let tail_max = budget * 2 / 5;

    // Tail: the last `tail_max` columns, then forward to a word boundary if it starts inside
    // a word.
    let mut tail_start = s.len();
    let mut w = 0;
    for (i, c) in s.char_indices().rev() {
        let cw = char_width(c);
        if w + cw > tail_max {
            break;
        }
        w += cw;
        tail_start = i;
    }
    let starts_mid_word = tail_start > 0
        && !s[..tail_start].ends_with(char::is_whitespace)
        && !s[tail_start..].starts_with(char::is_whitespace);
    if starts_mid_word {
        if let Some(sp) = s[tail_start..].find(char::is_whitespace) {
            tail_start += sp;
        }
    }
    let tail = s[tail_start..].trim();

    // Head: whatever budget is left, cut back to a word boundary.
    let head_max = budget.saturating_sub(width(tail));
    let mut head_end = 0;
    let mut w = 0;
    for (i, c) in s[..tail_start].char_indices() {
        let cw = char_width(c);
        if w + cw > head_max {
            break;
        }
        w += cw;
        head_end = i + c.len_utf8();
    }
    let head_slice = &s[..head_end];
    let head = if head_end < tail_start && !s[head_end..].starts_with(char::is_whitespace) {
        match head_slice.rfind(char::is_whitespace) {
            Some(sp) => &head_slice[..sp],
            None => head_slice,
        }
    } else {
        head_slice
    }
    .trim();

    match (head.is_empty(), tail.is_empty()) {
        (true, true) => "…".to_string(),
        (true, false) => format!("… {tail}"),
        (false, true) => format!("{head} …"),
        (false, false) => format!("{head} … {tail}"),
    }
}

/// Render the repository overview.
///
/// A repository with one project lists its actions with no headers, ordered by type with a
/// blank line between types. A repository with several projects shows one block per project
/// (root first), each ordered by type inside, so "what can I do here" reads top to bottom.
pub fn render(repo: &Repo, style: &Style, opts: &RenderOptions) -> String {
    let mut out = String::new();
    out.push_str(&style.bold(&repo.name));
    if let Some(root) = repo.projects.first().filter(|p| p.is_root()) {
        out.push_str(&techs_tag(root, style));
    }
    out.push('\n');

    if repo.all_actions().next().is_none() {
        out.push('\n');
        out.push_str("No project actions found.\n");
        out.push_str("Looked for package.json, Makefile, justfile, Taskfile, pyproject.toml, go.mod, Cargo.toml,\n.NET solutions/projects, Dockerfiles, Compose files and Kubernetes manifests.\n");
        return out;
    }

    // The command is what a developer types, so it leads the line; the explanation follows.
    // Column width is per block, so one long command does not push every block's text right,
    // and a command wider than the column is elided in the middle so the rows stay aligned
    // and the explanation stays in view.
    let block_width = |actions: &[&Action]| -> usize {
        actions
            .iter()
            .map(|a| width(&elide(&a.command, MAX_COMMAND_WIDTH)))
            .max()
            .unwrap_or(4)
            .clamp(4, MAX_COMMAND_WIDTH)
    };
    let blocks = listed(repo, opts);
    let line = |a: &Action, cmd_w: usize| -> String {
        let cmd = pad(&elide(&a.command, MAX_COMMAND_WIDTH), cmd_w);
        let mut l = format!("  {}  ", style.cyan(&cmd));
        if !a.opaque && !restates(&a.command, &a.description) {
            l.push_str(&a.description);
        }
        l.push_str(&trailer(style, a.risk, &a.notes));
        l.trim_end().to_string()
    };

    let single = blocks.len() == 1;
    for (p, block) in &blocks {
        let cmd_w = block_width(block);
        out.push('\n');
        // The root's actions sit directly under the repository name; nested projects get a
        // title with their path. A single grouped project shows its types as blocks
        // separated by a blank line instead.
        if !single && !p.is_root() {
            out.push_str(&title(p, style));
            out.push('\n');
        }
        let mut prev: Option<Category> = None;
        for a in block {
            if single && opts.group && prev.is_some_and(|c| c != a.category) {
                out.push('\n');
            }
            prev = Some(a.category);
            out.push_str(&line(a, cmd_w));
            out.push('\n');
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
                style.cyan(&pad(&elide(&a.command, MAX_COMMAND_WIDTH), cmd_w)),
                a.description
            ));
        }
    }
    out
}

/// A description whose every meaningful word is already in the command ("Run test across
/// all Nx projects" for `nx run-many -t test`, "Run scripts/foo.sh" for `./scripts/foo.sh`)
/// adds nothing; the line is shorter without it. Scope ("in all workspace packages") only
/// counts as said when the command shows it (`run-many`, `-r`): `pnpm run test` does not
/// tell that its script is `pnpm -r test`, so there the description is the information.
pub fn restates(command: &str, description: &str) -> bool {
    // The quantifier is the information; "workspace" or "packages" alone ("Run yarn
    // workspace") says nothing the command does not.
    const SCOPE: &[&str] = &["all", "every", "across"];
    const SCOPED: &[&str] = &[
        "run-many",
        "-r",
        "--recursive",
        "--workspaces",
        "-ws",
        "foreach",
        "--all",
        "--workspace",
        "./...",
    ];
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
    let scoped = cmd_words.iter().any(|w| SCOPED.contains(w));
    let mut meaningful = 0;
    for w in description
        .to_ascii_lowercase()
        .split(|c: char| {
            !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':' && c != '/'
        })
        .filter(|w| !w.is_empty())
    {
        if FILLER.contains(&w) && (scoped || !SCOPE.contains(&w)) {
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

/// The command cell of a CI step: its first line, elided to the column, with a trailing mark
/// when the step has more lines.
fn step_command(command: &str) -> String {
    let mut lines = command.lines();
    let first = lines.next().unwrap_or("");
    if lines.next().is_some() {
        format!("{} …", elide(first, MAX_COMMAND_WIDTH - 2))
    } else {
        elide(first, MAX_COMMAND_WIDTH)
    }
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
    // No repository title: unlike the listing, where it carries the technology tag, here it
    // would only repeat the directory the developer is standing in.
    let mut out = String::new();
    if repo.ci.is_empty() {
        out.push_str(
            "No CI pipelines found (looked for .github/workflows/*.yml and .gitlab-ci.yml).\n",
        );
        return out;
    }
    for (i, p) in repo.ci.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
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
                .map(|s| width(&step_command(&s.command)))
                .max()
                .unwrap_or(4)
                .clamp(4, MAX_COMMAND_WIDTH);
            for s in &j.steps {
                let cmd = pad(&step_command(&s.command), w);
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

/// The listing as rows: one block per project that has something to show (root first,
/// discovery order is depth-first), each action in the order the project declares it or,
/// with `--group`, by type.
pub fn listed<'a>(repo: &'a Repo, opts: &RenderOptions) -> Vec<(&'a Project, Vec<&'a Action>)> {
    repo.projects
        .iter()
        .filter_map(|p| {
            let mut block: Vec<&Action> = p.actions.iter().collect();
            if block.is_empty() {
                return None;
            }
            if opts.group {
                block = GROUP_ORDER
                    .iter()
                    .flat_map(|cat| block.iter().copied().filter(move |a| a.category == *cat))
                    .collect();
            }
            Some((p, block))
        })
        .collect()
}

/// A project's title line: its path, then the technologies it visibly uses
/// (`apps/api  [.NET, Kafka, Docker]`).
fn title(p: &Project, style: &Style) -> String {
    format!("{}{}", style.bold(&p.path), techs_tag(p, style))
}

/// `  [.NET, Kafka, Docker]`, or nothing when the project names no technology.
fn techs_tag(p: &Project, style: &Style) -> String {
    if p.techs.is_empty() {
        String::new()
    } else {
        format!("  {}", style.dim(&format!("[{}]", p.techs.join(", "))))
    }
}

/// Deterministic tabular dump used by snapshot tests.
pub fn debug_table(repo: &Repo) -> String {
    let mut o = String::new();
    for p in &repo.projects {
        o.push_str(&format!(
            "## {} [{}] {:?} tools={} techs={}\n",
            p.path,
            p.name,
            p.kind,
            p.tools.join(","),
            p.techs.join(",")
        ));
        for a in &p.actions {
            let notes: Vec<&str> = a.notes.iter().map(|n| n.label()).collect();
            o.push_str(&format!(
                "{:<24} {:<14} {:<8} {:<6} {:<11} {:<10} {} :: {}{}\n",
                a.id,
                a.category.label(),
                format!("{:?}", a.source).to_lowercase(),
                format!("{:?}", a.confidence).to_lowercase(),
                a.risk.label(),
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
                techs: vec![],
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
    fn long_commands_are_elided_in_the_middle_keeping_the_tail() {
        let e = |s: &str| elide(s, 40);
        assert_eq!(e("pnpm run dev"), "pnpm run dev");
        assert_eq!(
            e("docker compose -f docker/compose.yml up -d postgres"),
            "docker compose -f … up -d postgres"
        );
        assert_eq!(
            e("docker compose -f docker/compose.yml up -d kafka"),
            "docker compose -f … up -d kafka"
        );
        assert_eq!(
            e("npm run compile --workspaces --if-present"),
            "npm run compile … --if-present"
        );
        // No word boundaries: a plain cut, still within the limit.
        let long = "a".repeat(60);
        let out = e(&long);
        assert!(width(&out) <= 40, "{out}");
        assert!(out.contains('…'));
        // Wide characters count by column, never by char.
        let wide = "日本語 ".repeat(20);
        assert!(width(&e(&wide)) <= 40);
        // The listing keeps the column aligned and the explanation visible.
        let r = repo(vec![
            action(
                "pg",
                "docker compose -f docker/compose.yml up -d postgres",
                "Start PostgreSQL",
            ),
            action(
                "kafka",
                "docker compose -f docker/compose.yml up -d kafka",
                "Start Kafka",
            ),
        ]);
        let style = Style {
            color: false,
            tty: false,
        };
        let out = render(&r, &style, &RenderOptions { group: false });
        assert!(
            out.contains("  docker compose -f … up -d postgres  Start PostgreSQL"),
            "{out}"
        );
        assert!(
            out.contains("  docker compose -f … up -d kafka     Start Kafka"),
            "{out}"
        );
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
        let out = render(&r, &style, &RenderOptions { group: false });
        let starts: Vec<usize> = out
            .lines()
            .filter(|l| l.contains("  npm"))
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
        assert!(restates(
            "pnpm -r run test",
            "Run test in every workspace package"
        ));
        assert!(!restates("npm run test", "Run Vitest tests"));
        // The script is `pnpm -r test`: the command alone does not say "all packages".
        assert!(!restates(
            "pnpm run test",
            "Run test in all workspace packages"
        ));
        assert!(restates("yarn app", "Run yarn workspace"));
        assert!(!restates("nx run-many -t build", "Build the Angular app"));
        assert!(!restates("make docs", "Build the HTML documentation"));
    }
}
