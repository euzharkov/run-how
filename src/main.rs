use clap::{Parser, ValueEnum};
use rhow::ui::{self, ColorMode, RenderOptions, Style};
use rhow::{discover, repo, Options};
use std::path::PathBuf;
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
  rhow --json          Emit the normalised model as JSON
  rhow why 'make test'   Where that command comes from and why it is flagged
  rhow why db:reset      The same, by action id
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

    /// Output the normalised model as JSON.
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
    let cli = Cli::parse();
    let start = cli
        .directory
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if !start.is_dir() {
        eprintln!("rhow: {} is not a directory", start.display());
        return ExitCode::from(2);
    }
    let root = match cli.directory {
        Some(_) => start.canonicalize().unwrap_or(start),
        None => repo::find_root(&start),
    };
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
            if cli.json {
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
        Some("why") => show(&repo, &cli.args.join(" "), &style),
        Some(id) => {
            if !cli.args.is_empty() {
                eprintln!(
                    "rhow: shows commands, it does not run them; `rhow {id}` takes no arguments"
                );
                return ExitCode::from(2);
            }
            show(&repo, id, &style)
        }
    }
}

/// `rhow <action>`: the command behind an action, where it comes from and why it is flagged.
fn show(repo: &rhow::Repo, id: &str, style: &Style) -> ExitCode {
    match ui::render_why(repo, id, style) {
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
