use clap::{Parser, ValueEnum};
use rhow::ui::{self, ColorMode, RenderOptions, Style};
use rhow::{discover, repo, Options};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Discover how to run and operate a repository.
///
/// Run `rhow` inside any project to list its useful actions with the command behind each
/// one and a plain-English explanation. rhow only reads: it never installs, starts, runs
/// or contacts anything. Copy the command it shows and run it with the project's own tool.
#[derive(Parser, Debug)]
#[command(name = "rhow", version, about, long_about = None, after_help = "\
Examples:
  rhow                 List actions grouped by category
  rhow --all           Include low-confidence and internal actions
  rhow --group         Order each project's commands by type (run, build, deploy, test, …)
  rhow --ci            Show CI pipelines as workflows, jobs and steps
  rhow --json          Emit the normalised model as JSON (every action, hidden ones flagged)
  rhow --ci --json     Only the CI pipelines, as JSON
  rhow why 'make test'   Where that command comes from and why it is flagged
  rhow why db:reset      The same, by action id
  rhow db:reset --json   The same, as JSON
  rhow support         Show which tool versions this build has been verified against")]
struct Cli {
    /// `why <command or id>`, `support`, or an action id.
    action: Option<String>,

    /// The rest of `why <command>`; a quoted command or its bare words both work.
    #[arg(hide = true, trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,

    /// Show every discovered action, including internal and low-confidence ones.
    #[arg(long, short = 'a')]
    all: bool,

    /// Order each project's commands by type (run, build, deploy, test, …) instead of the
    /// order the project declares them in.
    #[arg(long, short = 'g')]
    group: bool,

    /// Show the CI pipelines (GitHub Actions, GitLab CI) as workflows, jobs and steps.
    #[arg(long)]
    ci: bool,

    /// Output as JSON: the normalised model (every action, hidden ones flagged), the CI
    /// pipelines with --ci, one action with `why <x>` or an id, or the support matrix.
    #[arg(long)]
    json: bool,

    /// Colour output.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,

    /// Directory to inspect, exactly as given. Without it, rhow starts from the current
    /// directory and inspects the enclosing git repository.
    #[arg(short = 'C', long = "directory", value_name = "DIR")]
    directory: Option<PathBuf>,

    /// Do not look at the local machine for runtime suggestions (Colima, Docker Desktop, …).
    #[arg(long)]
    no_runtime: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

fn main() -> ExitCode {
    let mut cli = Cli::parse();
    // `why <command>` takes the rest of the line verbatim (so `why make -j test` works),
    // which also swallows a `--json` written after the action. Honour it there too.
    if let Some(i) = cli.args.iter().position(|a| a == "--json") {
        cli.args.remove(i);
        cli.json = true;
    }
    let cli = cli;
    if cli.group && cli.json {
        eprintln!("rhow: --group orders the terminal listing; it has no meaning with --json");
        return ExitCode::from(2);
    }
    let start = cli
        .directory
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if !start.is_dir() {
        eprintln!("rhow: {} is not a directory", start.display());
        return ExitCode::from(2);
    }
    let root = match cli.directory {
        Some(_) => start.canonicalize().unwrap_or(start.clone()),
        None => repo::find_root(&start),
    };
    let listing = cli.action.is_none() && !cli.json;
    if let Some(hint) = no_git_hint(&start, &root, cli.directory.is_some(), listing) {
        eprintln!("{hint}");
    }
    let opts = Options {
        probe_runtime: !cli.no_runtime,
        ..Options::default()
    };
    let repo = discover(&root, &opts);
    let style = Style::detect(match cli.color {
        ColorArg::Auto => ColorMode::Auto,
        ColorArg::Always => ColorMode::Always,
        ColorArg::Never => ColorMode::Never,
    });

    match cli.action.as_deref() {
        None => {
            if cli.json && cli.ci {
                println!("{}", ui::json::render_ci(&repo));
            } else if cli.json {
                println!("{}", ui::json::render(&repo));
            } else if cli.ci {
                print!("{}", ui::render_ci(&repo, &style));
            } else {
                print!(
                    "{}",
                    ui::render(
                        &repo,
                        &style,
                        &RenderOptions {
                            all: cli.all,
                            group: cli.group
                        }
                    )
                );
            }
            ExitCode::SUCCESS
        }
        Some("support") => {
            if cli.json {
                println!("{}", ui::support::render_json(Some(&repo)));
            } else {
                print!("{}", ui::support::render(Some(&repo), &style));
            }
            ExitCode::SUCCESS
        }
        Some("why") if cli.args.is_empty() => {
            eprintln!("usage: rhow why <command or action id>");
            ExitCode::from(2)
        }
        Some("why") => show(&repo, &cli.args.join(" "), &style, cli.json),
        Some(id) => {
            if !cli.args.is_empty() {
                eprintln!(
                    "rhow: shows commands, it does not run them; `rhow {id}` takes no arguments"
                );
                return ExitCode::from(2);
            }
            show(&repo, id, &style, cli.json)
        }
    }
}

/// The one-line hint for a run that walked up from `cwd` and found no `.git`: rhow is then
/// inspecting `cwd` alone, which is usually not what was meant. Only for the terminal
/// listing (never for `--json`, which scripts parse) and never with `-C`, which asks for
/// exactly that directory.
fn no_git_hint(cwd: &Path, root: &Path, explicit_dir: bool, listing: bool) -> Option<String> {
    if explicit_dir || !listing || root.join(".git").exists() {
        return None;
    }
    let cwd = cwd.display();
    Some(format!(
        "rhow: no git repository above {cwd}; inspecting {cwd} only (use -C to pick a directory)"
    ))
}

/// `rhow <action>`: the command behind an action, where it comes from and why it is flagged.
fn show(repo: &rhow::Repo, id: &str, style: &Style, json: bool) -> ExitCode {
    let rendered = if json {
        ui::json::render_action(repo, id).map(|s| format!("{s}\n"))
    } else {
        ui::render_why(repo, id, style)
    };
    match rendered {
        Some(s) => {
            print!("{s}");
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("rhow: no action named `{id}`.");
            let close = ui::similar_ids(repo, id);
            if !close.is_empty() {
                eprintln!("Did you mean: {}", close.join(", "));
            }
            eprintln!("Run `rhow` to list available actions.");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::no_git_hint;
    use std::path::Path;

    #[test]
    fn hint_only_for_the_listing_without_a_git_root() {
        let dir = std::env::temp_dir().join(format!("rhow-nogit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let hint = no_git_hint(&dir, &dir, false, true).expect("hint without .git");
        assert_eq!(
            hint,
            format!(
                "rhow: no git repository above {d}; inspecting {d} only (use -C to pick a directory)",
                d = dir.display()
            )
        );
        assert_eq!(no_git_hint(&dir, &dir, true, true), None, "-C given");
        assert_eq!(
            no_git_hint(&dir, &dir, false, false),
            None,
            "--json / why / support"
        );
        std::fs::create_dir(dir.join(".git")).unwrap();
        assert_eq!(no_git_hint(&dir, &dir, false, true), None, "a git root");
        // A root above the cwd: the walk found a repository, so no hint.
        let nested = dir.join("sub");
        std::fs::create_dir(&nested).unwrap();
        assert_eq!(no_git_hint(&nested, &dir, false, true), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hint_names_a_cwd_that_is_not_a_repository() {
        assert!(no_git_hint(
            Path::new("/nonexistent/x"),
            Path::new("/nonexistent/x"),
            false,
            true
        )
        .unwrap()
        .starts_with("rhow: no git repository above /nonexistent/x;"));
    }
}
