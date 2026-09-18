//! Execution of a discovered action through its native tool.
//!
//! `rhow` is a dispatcher: it runs the exact command the project would run itself, in the
//! action's working directory, inheriting stdio, and returns the child's exit code.

use crate::model::{Action, Repo, Risk};
use std::io::{self, IsTerminal, Write};
use std::process::Command;

pub struct ExecOptions {
    pub dry_run: bool,
    pub yes: bool,
}

fn shell_quote(a: &str) -> String {
    if a.is_empty() {
        return "''".into();
    }
    if a.chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./=:@+,".contains(c))
    {
        return a.to_string();
    }
    if cfg!(windows) {
        format!("\"{}\"", a.replace('"', "\\\""))
    } else {
        format!("'{}'", a.replace('\'', "'\\''"))
    }
}

/// The full command line including extra arguments.
pub fn command_line(action: &Action, extra: &[String]) -> String {
    let mut line = action.command.clone();
    for e in extra {
        line.push(' ');
        line.push_str(&shell_quote(e));
    }
    line
}

fn confirm(prompt: &str) -> bool {
    if !io::stdin().is_terminal() {
        return false;
    }
    eprint!("{prompt} [y/N] ");
    let _ = io::stderr().flush();
    let mut s = String::new();
    if io::stdin().read_line(&mut s).is_err() {
        return false;
    }
    matches!(s.trim(), "y" | "Y" | "yes" | "YES")
}

/// Run `id` from `repo`. Returns the exit code to use.
pub fn run(repo: &Repo, id: &str, extra: &[String], opts: &ExecOptions) -> i32 {
    let Some(action) = repo.find(id) else {
        eprintln!("rhow: no action named `{id}`.");
        let mut close: Vec<&str> = repo
            .projects
            .iter()
            .flat_map(|p| p.actions.iter())
            .map(|a| a.id.as_str())
            .filter(|c| c.contains(id) || id.contains(*c) || c.split(':').next_back() == Some(id))
            .take(6)
            .collect();
        close.sort();
        if !close.is_empty() {
            eprintln!("Did you mean: {}", close.join(", "));
        }
        eprintln!("Run `rhow` to list available actions.");
        return 2;
    };
    let line = command_line(action, extra);
    let cwd = repo.root.join(&action.working_directory);
    if opts.dry_run {
        println!("{line}");
        if action.working_directory != "." {
            println!("  (in {})", action.working_directory);
        }
        return 0;
    }
    match action.risk {
        Risk::Safe => {}
        r => {
            let what = match r {
                Risk::External => "talks to an external system",
                Risk::Destructive => "is destructive",
                Risk::Safe => "",
            };
            if !opts.yes {
                eprintln!("⚠ `{}` {}: {}", action.id, what, action.description);
                eprintln!("  $ {line}");
                if !confirm("Continue?") {
                    eprintln!("Aborted. Pass --yes to skip this prompt.");
                    return 130;
                }
            }
        }
    }
    eprintln!("$ {line}");
    let status = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(&line)
            .current_dir(&cwd)
            .status()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(&line)
            .current_dir(&cwd)
            .status()
    };
    match status {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("rhow: failed to start `{line}`: {e}");
            127
        }
    }
}
