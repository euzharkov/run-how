use clap::{Parser, ValueEnum};
use rhow::exec::{self, ExecOptions};
use rhow::ui::{self, ColorMode, RenderOptions, Style};
use rhow::{discover, repo, Options};
use std::path::PathBuf;
use std::process::ExitCode;

/// Discover how to run and operate a repository.
///
/// Run `rhow` inside any project to list its useful actions. Discovery is read-only:
/// nothing is installed, started, or contacted until you explicitly run an action.
#[derive(Parser, Debug)]
#[command(name = "rhow", version, about, long_about = None, after_help = "\
Examples:
  rhow                 List actions grouped by category
  rhow --all           Include low-confidence and internal actions
  rhow --json          Emit the normalised model as JSON
  rhow test            Run the `test` action with its native tool
  rhow api:test -- -v  Pass extra arguments to the underlying command
  rhow why db:reset    Show where an action comes from and why it is flagged
  rhow support         Show which tool versions this build has been verified against")]
struct Cli {
    /// Action to run (see `rhow` for the list), or `why <action>`.
    action: Option<String>,

    /// Extra arguments appended to the action's command.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,

    /// Show every discovered action, including internal and low-confidence ones.
    #[arg(long, short = 'a')]
    all: bool,

    /// Output the normalised model as JSON.
    #[arg(long)]
    json: bool,

    /// Print the command that would run instead of running it.
    #[arg(long, short = 'n')]
    dry_run: bool,

    /// Skip the confirmation prompt for external or destructive actions.
    #[arg(long, short = 'y')]
    yes: bool,

    /// Colour output.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,

    /// Directory to inspect (defaults to the current directory).
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
    let root = repo::find_root(&start);
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
            } else {
                print!(
                    "{}",
                    ui::render(&repo, &style, &RenderOptions { all: cli.all })
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
        Some("why") => {
            let Some(id) = cli.args.first() else {
                eprintln!("usage: rhow why <action>");
                return ExitCode::from(2);
            };
            match ui::render_why(&repo, id, &style) {
                Some(s) => {
                    print!("{s}");
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!("rhow: no action named `{id}`");
                    ExitCode::from(2)
                }
            }
        }
        Some(id) => {
            let code = exec::run(
                &repo,
                id,
                &cli.args,
                &ExecOptions {
                    dry_run: cli.dry_run,
                    yes: cli.yes,
                },
            );
            ExitCode::from(code.clamp(0, 255) as u8)
        }
    }
}
