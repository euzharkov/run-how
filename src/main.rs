use clap::{Parser, Subcommand, ValueEnum};
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
#[command(
    name = "rhow",
    version,
    about,
    long_about = None,
    override_usage = "rhow [OPTIONS] [DIR]\n       rhow support [OPTIONS] [DIR]",
    disable_help_subcommand = true,
    after_help = "\
Examples:
  rhow                 List every project's commands, each explained
  rhow --group         Order each project's commands by type (run, build, deploy, test, …)
  rhow --ci            Show CI pipelines as workflows, jobs and steps
  rhow --json          Emit the normalised model as JSON
  rhow --ci --json     Only the CI pipelines, as JSON
  rhow path/to/dir     Inspect exactly that directory
  rhow support         Show which tool versions this build has been verified against")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Directory to inspect, exactly as given. Without it, rhow starts from the current
    /// directory and inspects the enclosing git repository. A directory named `support` is
    /// `./support`.
    #[arg(value_name = "DIR")]
    dir: Option<PathBuf>,

    /// Order each project's commands by type (run, build, deploy, test, …) instead of the
    /// order the project declares them in.
    #[arg(long, short = 'g')]
    group: bool,

    /// Show the CI pipelines (GitHub Actions, GitLab CI) as workflows, jobs and steps.
    #[arg(long)]
    ci: bool,

    /// Output as JSON: the normalised model (every action rhow lists), the CI
    /// pipelines with --ci, or the support matrix.
    #[arg(long, global = true)]
    json: bool,

    /// Colour output.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto, global = true)]
    color: ColorArg,

    /// `-C DIR`, the same as the DIR argument; kept for git and make muscle memory.
    #[arg(
        short = 'C',
        long = "directory",
        value_name = "DIR",
        hide = true,
        global = true
    )]
    directory: Option<PathBuf>,

    /// Do not look at the local machine for runtime suggestions (Colima, Docker Desktop, …).
    #[arg(long, global = true)]
    no_runtime: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show which tool versions this build has been verified against, and what the
    /// repository declares.
    Support {
        /// Directory to inspect, exactly as given.
        #[arg(value_name = "DIR")]
        dir: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let support = cli.command.is_some();
    let named = match &cli.command {
        Some(Command::Support { .. }) if cli.dir.is_some() => {
            eprintln!("rhow: the directory goes after `support`: rhow support [DIR]");
            return ExitCode::from(2);
        }
        Some(Command::Support { dir }) => dir.clone(),
        None => cli.dir.clone(),
    };
    let directory = match (named, cli.directory.clone()) {
        (Some(_), Some(_)) => {
            eprintln!("rhow: give the directory once, either as DIR or with -C");
            return ExitCode::from(2);
        }
        (a, b) => a.or(b),
    };
    if cli.group && cli.json {
        eprintln!("rhow: --group orders the terminal listing; it has no meaning with --json");
        return ExitCode::from(2);
    }
    let start = directory
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if !start.is_dir() {
        eprintln!("rhow: {} is not a directory", start.display());
        return ExitCode::from(2);
    }
    let root = match directory {
        Some(_) => start.canonicalize().unwrap_or(start.clone()),
        None => repo::find_root(&start),
    };
    let listing = !support && !cli.json;
    if let Some(hint) = no_git_hint(&start, &root, directory.is_some(), listing) {
        eprintln!("{hint}");
    }
    let opts = Options {
        probe_runtime: !cli.no_runtime,
        ..Options::default()
    };
    let repo = discover(&root, &opts);
    let view = RenderOptions { group: cli.group };
    let style = Style::detect(match cli.color {
        ColorArg::Auto => ColorMode::Auto,
        ColorArg::Always => ColorMode::Always,
        ColorArg::Never => ColorMode::Never,
    });

    if support {
        if cli.json {
            println!("{}", ui::support::render_json(Some(&repo)));
        } else {
            print!("{}", ui::support::render(Some(&repo), &style));
        }
        return ExitCode::SUCCESS;
    }
    if cli.json && cli.ci {
        println!("{}", ui::json::render_ci(&repo));
    } else if cli.json {
        println!("{}", ui::json::render(&repo));
    } else if cli.ci {
        print!("{}", ui::render_ci(&repo, &style));
    } else {
        print!("{}", ui::render(&repo, &style, &view));
    }
    ExitCode::SUCCESS
}

/// The one-line hint for a run that walked up from `cwd` and found no `.git`: rhow is then
/// inspecting `cwd` alone, which is usually not what was meant. Only for the terminal
/// listing (never for `--json`, which scripts parse) and never with a directory given, which asks for
/// exactly that directory.
fn no_git_hint(cwd: &Path, root: &Path, explicit_dir: bool, listing: bool) -> Option<String> {
    if explicit_dir || !listing || root.join(".git").exists() {
        return None;
    }
    let cwd = cwd.display();
    Some(format!(
        "rhow: no git repository above {cwd}; inspecting {cwd} only (use `rhow <dir>` to pick a directory)"
    ))
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
                "rhow: no git repository above {d}; inspecting {d} only (use `rhow <dir>` to pick a directory)",
                d = dir.display()
            )
        );
        assert_eq!(no_git_hint(&dir, &dir, true, true), None, "directory given");
        assert_eq!(
            no_git_hint(&dir, &dir, false, false),
            None,
            "--json / support"
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
