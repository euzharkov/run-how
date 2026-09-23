//! Static analysis of shell command strings.
//!
//! Nothing here executes anything. A command string is tokenised, split into a sequence of
//! invocations (`a && b | c`), runner wrappers are peeled off (`npx`, `uv run`, `cross-env`…),
//! references to sibling scripts are resolved through a caller-supplied [`Resolver`], and each
//! invocation is summarised with the knowledge table in [`tools`].

pub mod tools;

use crate::model::{Category, Risk};
use tools::{Kind, Summary};

/// Resolves a script reference (`npm run build`, `make test`) to the underlying command text.
/// The first argument is the runner family (`js`, `make`, `just`, `task`, `cargo`, `poe`, `pdm`).
pub type Resolver<'a> = &'a dyn Fn(&str, &str) -> Option<String>;

pub fn no_resolver(_: &str, _: &str) -> Option<String> {
    None
}

const MAX_DEPTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// Leading `KEY=value` assignments.
    pub env: Vec<String>,
    pub program: String,
    pub args: Vec<String>,
}

impl Invocation {
    pub fn new(program: &str, args: &[&str]) -> Self {
        Invocation {
            env: vec![],
            program: program.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// One explained step of a command.
#[derive(Debug, Clone)]
pub struct Step {
    pub text: String,
    pub kind: Kind,
    pub risk: Risk,
    /// The recognised tool, if any (`vitest`, `docker compose`, …).
    pub tool: Option<String>,
    /// True when the program was not recognised at all.
    pub unknown: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Analysis {
    pub steps: Vec<Step>,
}

impl Analysis {
    pub fn max_risk(&self) -> Risk {
        self.steps
            .iter()
            .map(|s| s.risk)
            .max()
            .unwrap_or(Risk::Safe)
    }

    pub fn is_opaque(&self) -> bool {
        self.steps.iter().all(|s| s.unknown)
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.steps.iter().any(|s| s.tool.as_deref() == Some(name))
    }
}

// ---------------------------------------------------------------------------
// Tokenising
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Word(String),
    Op(&'static str),
}

fn tokenize(s: &str) -> Vec<Tok> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut i = 0;
    let flush = |cur: &mut String, in_word: &mut bool, out: &mut Vec<Tok>| {
        if *in_word {
            out.push(Tok::Word(std::mem::take(cur)));
            *in_word = false;
        }
    };
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '\'' => {
                in_word = true;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    cur.push(chars[i]);
                    i += 1;
                }
            }
            '"' => {
                in_word = true;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() && "\"\\$`".contains(chars[i + 1]) {
                        i += 1;
                    }
                    cur.push(chars[i]);
                    i += 1;
                }
            }
            '\\' => {
                // Only escape shell-significant characters; keep Windows paths intact.
                match next {
                    Some(n) if "\"'\\$` \t&|;<>()#".contains(n) => {
                        cur.push(n);
                        in_word = true;
                        i += 1;
                    }
                    Some('\n') => {
                        i += 1;
                    }
                    _ => {
                        cur.push(c);
                        in_word = true;
                    }
                }
            }
            ' ' | '\t' | '\r' => flush(&mut cur, &mut in_word, &mut out),
            '\n' => {
                // A line break ends the command, like `;` (a `\`-continued line was joined above).
                flush(&mut cur, &mut in_word, &mut out);
                out.push(Tok::Op(";"));
            }
            '&' if next == Some('&') => {
                flush(&mut cur, &mut in_word, &mut out);
                out.push(Tok::Op("&&"));
                i += 1;
            }
            '|' if next == Some('|') => {
                flush(&mut cur, &mut in_word, &mut out);
                out.push(Tok::Op("||"));
                i += 1;
            }
            '|' => {
                flush(&mut cur, &mut in_word, &mut out);
                out.push(Tok::Op("|"));
            }
            '&' if cur.ends_with('>') => {
                cur.push(c);
            }
            ';' | '&' => {
                flush(&mut cur, &mut in_word, &mut out);
                out.push(Tok::Op(";"));
            }
            '#' if !in_word => break,
            '(' | ')' if !in_word => {}
            '$' if next == Some('(') => {
                // `$(...)` command substitution: keep it as one opaque word so the `|`, `;`
                // or `&&` inside it do not split the surrounding command.
                let mut depth = 0;
                while i < chars.len() {
                    match chars[i] {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                cur.push(')');
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    cur.push(chars[i]);
                    i += 1;
                }
                in_word = true;
                continue;
            }
            '`' => {
                // command substitution: keep as opaque text
                cur.push(c);
                in_word = true;
            }
            _ => {
                cur.push(c);
                in_word = true;
            }
        }
        i += 1;
    }
    flush(&mut cur, &mut in_word, &mut out);
    out
}

fn is_env_assignment(w: &str) -> bool {
    match w.find('=') {
        Some(0) | None => false,
        Some(p) => {
            let key = &w[..p];
            !key.starts_with('-') && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
    }
}

/// Split a command line into its invocations, dropping operators and redirections.
pub fn parse(cmd: &str) -> Vec<Invocation> {
    let toks = tokenize(cmd);
    let mut groups: Vec<Vec<String>> = vec![vec![]];
    for t in toks {
        match t {
            Tok::Op(_) => groups.push(vec![]),
            Tok::Word(w) => groups.last_mut().unwrap().push(w),
        }
    }
    let mut out = Vec::new();
    for g in groups {
        let mut words = Vec::new();
        let mut skip_next = false;
        for w in g {
            if skip_next {
                skip_next = false;
                continue;
            }
            let bare = w.trim_start_matches(|c: char| c.is_ascii_digit());
            if bare == ">" || bare == ">>" || bare == "<" || bare == "<<" {
                skip_next = true;
                continue;
            }
            if bare.starts_with('>') || bare.starts_with("&>") || bare.starts_with("<<") {
                continue;
            }
            words.push(w);
        }
        let mut env = Vec::new();
        let mut iter = words.into_iter().peekable();
        while let Some(w) = iter.peek() {
            if w == "export" {
                // `export FOO=bar` / `export $(cat .env)`: environment set-up, not a step.
                env.push(iter.next().unwrap());
                if let Some(v) = iter.next() {
                    env.push(v);
                }
            } else if is_env_assignment(w) {
                env.push(iter.next().unwrap());
            } else {
                break;
            }
        }
        let mut program = match iter.next() {
            Some(p) => p,
            None => continue,
        };
        // Shell control flow: `then cmd`, `do cmd`, `{ cmd` lead into a command; `if cond`,
        // `for x in …`, `fi`, `done`, `set -e`, `cd dir` are not commands worth a step.
        while matches!(program.as_str(), "then" | "else" | "do" | "{" | "(") {
            match iter.next() {
                Some(p) => program = p,
                None => break,
            }
        }
        if matches!(
            program.as_str(),
            "exit"
                | "true"
                | "false"
                | "cd"
                | "set"
                | "if"
                | "elif"
                | "while"
                | "until"
                | "for"
                | "case"
                | "fi"
                | "done"
                | "esac"
                | "then"
                | "else"
                | "do"
                | "{"
                | "}"
                | "("
                | ")"
                | ":"
                | "source"
                | "."
                | "shift"
                | "return"
                | "break"
                | "continue"
                | "local"
                | "readonly"
                | "declare"
                | "typeset"
                | "unset"
                | "trap"
                | "wait"
        ) {
            continue;
        }
        out.push(Invocation {
            env,
            program,
            args: iter.collect(),
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Unwrapping runners and wrappers
// ---------------------------------------------------------------------------

fn basename(p: &str) -> &str {
    let p = p.trim_end_matches('/');
    match p.rfind(['/', '\\']) {
        Some(i) => &p[i + 1..],
        None => p,
    }
}

/// Normalise the program name: strip `./node_modules/.bin/`, `.exe`, `.cmd`.
fn norm_program(p: &str) -> String {
    let b = basename(p);
    let b = b
        .strip_suffix(".exe")
        .or_else(|| b.strip_suffix(".cmd"))
        .unwrap_or(b);
    let bin_stub = p
        .trim_start_matches("./")
        .strip_prefix("bin/")
        .map(|r| !r.contains('/'))
        .unwrap_or(false);
    if bin_stub
        && matches!(
            b,
            "rails"
                | "rake"
                | "rspec"
                | "rubocop"
                | "bundle"
                | "dev"
                | "setup"
                | "jobs"
                | "importmap"
        )
    {
        return format!("bin/{b}");
    }
    if p.contains("node_modules/.bin")
        || p.contains("vendor/bin")
        || !p.contains('/')
        || p.starts_with("./node_modules")
    {
        b.to_string()
    } else {
        // keep relative paths for scripts like ./scripts/foo.sh
        p.trim_start_matches("./").to_string()
    }
}

/// The intermediate result of peeling wrappers.
enum Peeled {
    /// A concrete tool invocation to summarise.
    Tool(Invocation),
    /// A reference to a sibling script through a runner family.
    ScriptRef {
        family: &'static str,
        name: String,
        scope: Option<String>,
        args: Vec<String>,
    },
    /// Several commands to analyse independently (e.g. `concurrently "a" "b"`).
    Many(Vec<String>),
}

const YARN_BUILTINS: &[&str] = &[
    "add",
    "install",
    "remove",
    "upgrade",
    "up",
    "why",
    "info",
    "init",
    "link",
    "unlink",
    "pack",
    "publish",
    "version",
    "versions",
    "cache",
    "config",
    "set",
    "dlx",
    "exec",
    "node",
    "npm",
    "workspace",
    "workspaces",
    "constraints",
    "dedupe",
    "explain",
    "patch",
    "rebuild",
    "run",
    "bin",
    "audit",
    "outdated",
    "list",
    "global",
    "create",
    "test",
    "start",
    "build",
    "dev",
];
const PNPM_BUILTINS: &[&str] = &[
    "add",
    "install",
    "i",
    "remove",
    "rm",
    "update",
    "up",
    "why",
    "init",
    "link",
    "unlink",
    "pack",
    "publish",
    "version",
    "store",
    "config",
    "dlx",
    "exec",
    "run",
    "test",
    "start",
    "audit",
    "outdated",
    "list",
    "ls",
    "import",
    "prune",
    "fetch",
    "deploy",
    "patch",
    "create",
    "env",
    "setup",
    "licenses",
    "rebuild",
    "dedupe",
    "approve-builds",
];
const BUN_BUILTINS: &[&str] = &[
    "add", "install", "i", "remove", "rm", "update", "run", "x", "test", "build", "init", "create",
    "link", "unlink", "pm", "publish", "outdated", "upgrade", "patch", "exec", "repl",
];

fn take_flag_values(args: &[String], value_flags: &[&str]) -> Vec<String> {
    // returns args with the given value flags (and their values) removed, plus other flags removed
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if value_flags.contains(&a.as_str()) {
            i += 2;
            continue;
        }
        if a.starts_with('-') {
            i += 1;
            continue;
        }
        out.push(a.clone());
        i += 1;
    }
    out
}

fn split_at_first_positional(
    args: &[String],
    value_flags: &[&str],
) -> Option<(String, Vec<String>)> {
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if value_flags.contains(&a.as_str()) {
            i += 2;
            continue;
        }
        if a == "--" {
            i += 1;
            continue;
        }
        if a.starts_with('-') {
            i += 1;
            continue;
        }
        return Some((a.clone(), args[i + 1..].to_vec()));
    }
    None
}

fn peel(inv: &Invocation) -> Peeled {
    let mut prog = norm_program(&inv.program);
    let mut args: Vec<String> = inv.args.clone();
    for _ in 0..6 {
        match prog.as_str() {
            "npx" => {
                let vf = ["-p", "--package", "-c", "--call"];
                match split_at_first_positional(&args, &vf) {
                    Some((p, rest)) => {
                        prog = norm_program(&p);
                        args = rest;
                    }
                    None => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "bunx" => match split_at_first_positional(&args, &[]) {
                Some((p, rest)) => {
                    prog = norm_program(&p);
                    args = rest;
                }
                None => {
                    return Peeled::Tool(Invocation {
                        env: vec![],
                        program: prog,
                        args,
                    })
                }
            },
            "cross-env" | "env" | "dotenvx" => {
                let rest: Vec<String> = args
                    .iter()
                    .skip_while(|a| is_env_assignment(a) || a.starts_with('-'))
                    .cloned()
                    .collect();
                if rest.is_empty() {
                    return Peeled::Tool(Invocation {
                        env: vec![],
                        program: prog,
                        args,
                    });
                }
                prog = norm_program(&rest[0]);
                args = rest[1..].to_vec();
            }
            "dotenv" | "env-cmd" => {
                let vf = ["-e", "-f", "--file", "-r", "--rc-file", "-c", "-p"];
                let mut i = 0;
                let mut found = None;
                while i < args.len() {
                    if vf.contains(&args[i].as_str()) {
                        i += 2;
                        continue;
                    }
                    if args[i] == "--" {
                        found = Some(i + 1);
                        break;
                    }
                    if args[i].starts_with('-') {
                        i += 1;
                        continue;
                    }
                    found = Some(i);
                    break;
                }
                match found {
                    Some(i) if i < args.len() => {
                        prog = norm_program(&args[i]);
                        args = args[i + 1..].to_vec();
                    }
                    _ => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "php" => {
                // Composer scripts commonly call a vendor tool through the PHP interpreter
                // (`@php vendor/bin/pint --test`). Peel to the tool itself so it hits the
                // knowledge table; leave `php artisan ...` and plain `php script.php` alone,
                // since those already have their own well-known handling.
                match split_at_first_positional(&args, &[]) {
                    Some((p, rest)) if p.contains("vendor/bin/") || p.contains("vendor\\bin\\") => {
                        prog = norm_program(&p);
                        args = rest;
                    }
                    _ => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "time" | "nice" | "exec" | "command" | "sudo" | "doas" | "xvfb-run" | "wait-on" => {
                if prog == "wait-on" {
                    // wait-on url && cmd  → usually chained; treat as its own step
                    return Peeled::Tool(Invocation {
                        env: vec![],
                        program: prog,
                        args,
                    });
                }
                match split_at_first_positional(&args, &[]) {
                    Some((p, rest)) => {
                        prog = norm_program(&p);
                        args = rest;
                    }
                    None => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "concurrently" | "npm-run-all" | "run-p" | "run-s" | "npm-run-all2" => {
                let vf = [
                    "-n",
                    "--names",
                    "-c",
                    "--prefix-colors",
                    "-p",
                    "--prefix",
                    "--max-processes",
                    "-m",
                    "--timings",
                    "--handle-input",
                    "--restart-tries",
                ];
                let items = take_flag_values(&args, &vf);
                let mut cmds = Vec::new();
                for it in items {
                    if prog == "concurrently" {
                        if let Some(name) = it.strip_prefix("npm:") {
                            cmds.push(format!("npm run {name}"));
                        } else if let Some(name) = it.strip_prefix("pnpm:") {
                            cmds.push(format!("pnpm run {name}"));
                        } else if let Some(name) = it.strip_prefix("yarn:") {
                            cmds.push(format!("yarn run {name}"));
                        } else {
                            cmds.push(it);
                        }
                    } else {
                        cmds.push(format!("npm run {it}"));
                    }
                }
                return Peeled::Many(cmds);
            }
            "npm" => {
                let vf = ["--workspace", "-w", "--prefix", "-C"];
                let ws = args.iter().any(|a| {
                    a == "--workspaces"
                        || a == "-ws"
                        || a == "--workspace"
                        || a == "-w"
                        || a.starts_with("--workspace=")
                });
                let scope = if ws {
                    Some("all workspaces".to_string())
                } else {
                    None
                };
                match split_at_first_positional(&args, &vf) {
                    Some((sub, rest)) => match sub.as_str() {
                        "run" | "run-script" | "rum" | "urn" => {
                            if let Some((name, _)) = split_at_first_positional(&rest, &vf) {
                                return Peeled::ScriptRef {
                                    family: "js",
                                    name,
                                    scope,
                                    args: vec![],
                                };
                            }
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            });
                        }
                        "test" | "t" | "tst" | "start" | "stop" | "restart" => {
                            let n = if sub == "t" || sub == "tst" {
                                "test".to_string()
                            } else {
                                sub
                            };
                            return Peeled::ScriptRef {
                                family: "js",
                                name: n,
                                scope,
                                args: vec![],
                            };
                        }
                        "exec" | "x" => match split_at_first_positional(&rest, &["-c", "--call"]) {
                            Some((p, r)) => {
                                prog = norm_program(&p);
                                args = r;
                            }
                            None => {
                                return Peeled::Tool(Invocation {
                                    env: vec![],
                                    program: prog,
                                    args,
                                })
                            }
                        },
                        _ => {
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            })
                        }
                    },
                    None => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "pnpm" => {
                let vf = ["--filter", "-F", "-C", "--dir", "--workspace-concurrency"];
                let recursive = args.iter().any(|a| a == "-r" || a == "--recursive");
                let filter = args
                    .iter()
                    .position(|a| a == "--filter" || a == "-F")
                    .and_then(|i| args.get(i + 1))
                    .cloned()
                    .or_else(|| {
                        args.iter()
                            .find_map(|a| a.strip_prefix("--filter=").map(|s| s.to_string()))
                    });
                let scope = match (&filter, recursive) {
                    (Some(f), _) if f.contains('*') || f.contains("...") || f == "." => {
                        Some("workspace packages".to_string())
                    }
                    (Some(f), _) => Some(f.clone()),
                    (None, true) => Some("workspace packages".to_string()),
                    _ => None,
                };
                match split_at_first_positional(&args, &vf) {
                    Some((sub, rest)) => match sub.as_str() {
                        "run" | "run-script" => {
                            if let Some((name, _)) = split_at_first_positional(&rest, &vf) {
                                return Peeled::ScriptRef {
                                    family: "js",
                                    name,
                                    scope,
                                    args: vec![],
                                };
                            }
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            });
                        }
                        "exec" | "dlx" => match split_at_first_positional(&rest, &[]) {
                            Some((p, r)) => {
                                prog = norm_program(&p);
                                args = r;
                            }
                            None => {
                                return Peeled::Tool(Invocation {
                                    env: vec![],
                                    program: prog,
                                    args,
                                })
                            }
                        },
                        "test" | "t" | "start" => {
                            return Peeled::ScriptRef {
                                family: "js",
                                name: if sub == "t" { "test".into() } else { sub },
                                scope,
                                args: vec![],
                            };
                        }
                        s if PNPM_BUILTINS.contains(&s) => {
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            })
                        }
                        _ => {
                            return Peeled::ScriptRef {
                                family: "js",
                                name: sub,
                                scope,
                                args: rest,
                            }
                        }
                    },
                    None => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "yarn" => {
                match split_at_first_positional(&args, &["--cwd"]) {
                    Some((sub, rest)) => match sub.as_str() {
                        "run" => {
                            if let Some((name, _)) = split_at_first_positional(&rest, &[]) {
                                return Peeled::ScriptRef {
                                    family: "js",
                                    name,
                                    scope: None,
                                    args: vec![],
                                };
                            }
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            });
                        }
                        "workspace" => {
                            // yarn workspace <name> <script>
                            let pos = take_flag_values(&rest, &[]);
                            if pos.len() >= 2 {
                                let name = if pos[1] == "run" && pos.len() >= 3 {
                                    pos[2].clone()
                                } else {
                                    pos[1].clone()
                                };
                                return Peeled::ScriptRef {
                                    family: "js",
                                    name,
                                    scope: Some(pos[0].clone()),
                                    args: vec![],
                                };
                            }
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            });
                        }
                        "workspaces" => {
                            let pos = take_flag_values(&rest, &[]);
                            if pos.iter().any(|p| p == "npm") && pos.iter().any(|p| p == "publish")
                            {
                                return Peeled::Tool(Invocation {
                                    env: vec![],
                                    program: "yarn".into(),
                                    args: vec!["npm".into(), "publish".into()],
                                });
                            }
                            let name = pos.iter().find(|p| *p != "foreach" && *p != "run").cloned();
                            if let Some(name) = name {
                                return Peeled::ScriptRef {
                                    family: "js",
                                    name,
                                    scope: Some("all workspaces".into()),
                                    args: vec![],
                                };
                            }
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            });
                        }
                        "exec" | "dlx" => match split_at_first_positional(&rest, &[]) {
                            Some((p, r)) => {
                                prog = norm_program(&p);
                                args = r;
                            }
                            None => {
                                return Peeled::Tool(Invocation {
                                    env: vec![],
                                    program: prog,
                                    args,
                                })
                            }
                        },
                        "test" | "start" | "build" | "dev" => {
                            return Peeled::ScriptRef {
                                family: "js",
                                name: sub,
                                scope: None,
                                args: vec![],
                            }
                        }
                        s if YARN_BUILTINS.contains(&s) => {
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            })
                        }
                        _ => {
                            return Peeled::ScriptRef {
                                family: "js",
                                name: sub,
                                scope: None,
                                args: rest,
                            }
                        }
                    },
                    None => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "bun" => {
                let vf = ["--filter", "-F", "--cwd"];
                match split_at_first_positional(&args, &vf) {
                    Some((sub, rest)) => match sub.as_str() {
                        "run" => match split_at_first_positional(&rest, &vf) {
                            Some((name, r)) => {
                                if name.ends_with(".ts")
                                    || name.ends_with(".js")
                                    || name.ends_with(".tsx")
                                    || name.ends_with(".mjs")
                                {
                                    prog = "bun-file".into();
                                    let flags: Vec<String> = args
                                        .iter()
                                        .filter(|a| a.starts_with('-'))
                                        .cloned()
                                        .collect();
                                    args = std::iter::once(name).chain(r).chain(flags).collect();
                                } else {
                                    return Peeled::ScriptRef {
                                        family: "js",
                                        name,
                                        scope: None,
                                        args: vec![],
                                    };
                                }
                            }
                            None => {
                                return Peeled::Tool(Invocation {
                                    env: vec![],
                                    program: prog,
                                    args,
                                })
                            }
                        },
                        "x" => match split_at_first_positional(&rest, &[]) {
                            Some((p, r)) => {
                                prog = norm_program(&p);
                                args = r;
                            }
                            None => {
                                return Peeled::Tool(Invocation {
                                    env: vec![],
                                    program: prog,
                                    args,
                                })
                            }
                        },
                        s if BUN_BUILTINS.contains(&s) => {
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            })
                        }
                        s if s.ends_with(".ts")
                            || s.ends_with(".js")
                            || s.ends_with(".tsx")
                            || s.ends_with(".mjs") =>
                        {
                            prog = "bun-file".into();
                            args = std::iter::once(sub).chain(rest).collect();
                        }
                        _ => {
                            return Peeled::ScriptRef {
                                family: "js",
                                name: sub,
                                scope: None,
                                args: rest,
                            }
                        }
                    },
                    None => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "uv" | "poetry" | "pdm" | "pipenv" | "hatch" | "rye" => {
                let pos = take_flag_values(
                    &args,
                    &["--python", "-p", "--directory", "--project", "--env", "-e"],
                );
                if pos.first().map(String::as_str) == Some("run") {
                    if prog == "pdm" {
                        // could be a declared [tool.pdm.scripts] entry
                        if let Some(name) = pos.get(1) {
                            let known = tools::summarize(name, &[]).is_some()
                                || name.contains('/')
                                || name.contains('.')
                                || matches!(
                                    name.as_str(),
                                    "python" | "python3" | "py" | "node" | "sh" | "bash"
                                );
                            if !known {
                                return Peeled::ScriptRef {
                                    family: "pdm",
                                    name: name.clone(),
                                    scope: None,
                                    args: vec![],
                                };
                            }
                        }
                    }
                    let idx = args.iter().position(|a| a == "run").unwrap();
                    match split_at_first_positional(
                        &args[idx + 1..],
                        &[
                            "--python", "-p", "--with", "--env", "-e", "--group", "--extra",
                        ],
                    ) {
                        Some((p, r)) => {
                            prog = norm_program(&p);
                            args = r;
                        }
                        None => {
                            return Peeled::Tool(Invocation {
                                env: vec![],
                                program: prog,
                                args,
                            })
                        }
                    }
                } else {
                    return Peeled::Tool(Invocation {
                        env: vec![],
                        program: prog,
                        args,
                    });
                }
            }
            "uvx" | "pipx" => {
                let pos = take_flag_values(&args, &["--python", "-p", "--with", "--from"]);
                if prog == "pipx" && pos.first().map(String::as_str) != Some("run") {
                    return Peeled::Tool(Invocation {
                        env: vec![],
                        program: prog,
                        args,
                    });
                }
                let skip = if prog == "pipx" { 1 } else { 0 };
                match pos.get(skip) {
                    Some(p) => {
                        let p = p.clone();
                        let idx = args.iter().position(|a| *a == p).unwrap();
                        prog = norm_program(&p);
                        args = args[idx + 1..].to_vec();
                    }
                    None => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "python" | "python3" | "py" | "python3.12" | "python3.11" | "python3.13" => {
                let first = args.iter().find(|a| !a.starts_with('-')).cloned();
                if let Some(f) = first {
                    if f.ends_with("manage.py") {
                        let idx = args.iter().position(|a| *a == f).unwrap();
                        prog = "manage.py".into();
                        args = args[idx + 1..].to_vec();
                        continue;
                    }
                    if f.ends_with(".py") && args.first().map(String::as_str) != Some("-m") {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: "run-script".into(),
                            args: vec![f],
                        });
                    }
                }
                if args.first().map(String::as_str) == Some("-m") {
                    if let Some(m) = args.get(1) {
                        prog = match m.as_str() {
                            "pytest" => "pytest".into(),
                            "ruff" => "ruff".into(),
                            "mypy" => "mypy".into(),
                            "black" => "black".into(),
                            "flake8" => "flake8".into(),
                            "pylint" => "pylint".into(),
                            "pip" => "pip".into(),
                            "uvicorn" => "uvicorn".into(),
                            "http.server" => "http.server".into(),
                            "build" => "pyproject-build".into(),
                            "twine" => "twine".into(),
                            "tox" => "tox".into(),
                            "nox" => "nox".into(),
                            "pre_commit" => "pre-commit".into(),
                            "alembic" => "alembic".into(),
                            "celery" => "celery".into(),
                            "flask" => "flask".into(),
                            "isort" => "isort".into(),
                            "pyright" => "pyright".into(),
                            "coverage" => "coverage".into(),
                            "sphinx" => "sphinx-build".into(),
                            "mkdocs" => "mkdocs".into(),
                            other => format!("python-module:{other}"),
                        };
                        args = args[2..].to_vec();
                        continue;
                    }
                }
                return Peeled::Tool(Invocation {
                    env: vec![],
                    program: prog,
                    args,
                });
            }
            "make" | "gmake" | "mingw32-make" => {
                let vf = ["-C", "-f", "--file", "-j", "--jobs", "-e"];
                let pos: Vec<String> = take_flag_values(&args, &vf)
                    .into_iter()
                    .filter(|a| !is_env_assignment(a))
                    .collect();
                match pos.first() {
                    Some(t) => {
                        return Peeled::ScriptRef {
                            family: "make",
                            name: t.clone(),
                            scope: None,
                            args: vec![],
                        }
                    }
                    None => {
                        return Peeled::ScriptRef {
                            family: "make",
                            name: "".into(),
                            scope: None,
                            args: vec![],
                        }
                    }
                }
            }
            "just" => {
                let pos = take_flag_values(
                    &args,
                    &["-f", "--justfile", "-d", "--working-directory", "--set"],
                );
                match pos.first() {
                    Some(t) => {
                        return Peeled::ScriptRef {
                            family: "just",
                            name: t.clone(),
                            scope: None,
                            args: vec![],
                        }
                    }
                    None => {
                        return Peeled::ScriptRef {
                            family: "just",
                            name: "default".into(),
                            scope: None,
                            args: vec![],
                        }
                    }
                }
            }
            "task" | "go-task" => {
                let pos = take_flag_values(&args, &["-t", "--taskfile", "-d", "--dir"]);
                match pos.first() {
                    Some(t) => {
                        return Peeled::ScriptRef {
                            family: "task",
                            name: t.clone(),
                            scope: None,
                            args: vec![],
                        }
                    }
                    None => {
                        return Peeled::ScriptRef {
                            family: "task",
                            name: "default".into(),
                            scope: None,
                            args: vec![],
                        }
                    }
                }
            }
            "poe" => {
                let pos = take_flag_values(&args, &["-C", "--root"]);
                if let Some(t) = pos.first() {
                    return Peeled::ScriptRef {
                        family: "poe",
                        name: t.clone(),
                        scope: None,
                        args: vec![],
                    };
                }
                return Peeled::Tool(Invocation {
                    env: vec![],
                    program: prog,
                    args,
                });
            }
            "bin/rails" | "bin/rake" | "bin/rspec" | "bin/rubocop" | "bin/bundle" => {
                prog = prog.trim_start_matches("bin/").to_string();
            }
            "bundle" if args.first().map(String::as_str) == Some("exec") => {
                match split_at_first_positional(&args[1..], &[]) {
                    Some((p, r)) => {
                        prog = norm_program(&p);
                        args = r;
                    }
                    None => {
                        return Peeled::Tool(Invocation {
                            env: vec![],
                            program: prog,
                            args,
                        })
                    }
                }
            }
            "cargo" => {
                // cargo <alias> → resolvable by the cargo family; the resolver decides.
                if let Some((sub, _)) = split_at_first_positional(&args, &[]) {
                    if !tools::CARGO_BUILTINS.contains(&sub.as_str()) {
                        return Peeled::ScriptRef {
                            family: "cargo",
                            name: sub,
                            scope: None,
                            args: vec![],
                        };
                    }
                }
                return Peeled::Tool(Invocation {
                    env: vec![],
                    program: prog,
                    args,
                });
            }
            "nx" if args.first().map(String::as_str) == Some("exec") => {
                // `nx exec -- <command>` runs the command with Nx caching around it.
                let inner: Vec<String> = args
                    .iter()
                    .skip(1)
                    .skip_while(|a| a.starts_with('-') && *a != "--")
                    .skip_while(|a| *a == "--")
                    .cloned()
                    .collect();
                if inner.is_empty() {
                    return Peeled::Tool(Invocation {
                        env: vec![],
                        program: prog,
                        args,
                    });
                }
                prog = norm_program(&inner[0]);
                args = inner[1..].to_vec();
            }
            "sh" | "bash" | "zsh" | "pwsh" | "powershell" | "cmd" => {
                // sh -c "cmd" → analyse the inner command
                if let Some(i) = args
                    .iter()
                    .position(|a| a == "-c" || a == "-Command" || a == "/c" || a == "/C")
                {
                    if let Some(inner) = args.get(i + 1) {
                        return Peeled::Many(vec![inner.clone()]);
                    }
                }
                let pos: Vec<String> = args
                    .iter()
                    .filter(|a| !a.starts_with('-') || a.as_str() == "-File")
                    .cloned()
                    .collect();
                if let Some(script) = pos.iter().find(|p| !p.starts_with('-')) {
                    return Peeled::Tool(Invocation {
                        env: vec![],
                        program: "run-script".into(),
                        args: vec![script.clone()],
                    });
                }
                return Peeled::Tool(Invocation {
                    env: vec![],
                    program: prog,
                    args,
                });
            }
            _ => break,
        }
    }
    Peeled::Tool(Invocation {
        env: inv.env.clone(),
        program: prog,
        args,
    })
}

// ---------------------------------------------------------------------------
// Analysis
// ---------------------------------------------------------------------------

/// Analyse a command string. `resolver` maps script references to their bodies.
pub fn analyze(cmd: &str, resolver: Resolver) -> Analysis {
    let mut a = Analysis::default();
    analyze_into(cmd, resolver, 0, &mut Vec::new(), &mut a);
    a
}

fn analyze_into(
    cmd: &str,
    resolver: Resolver,
    depth: usize,
    seen: &mut Vec<String>,
    out: &mut Analysis,
) {
    if depth > MAX_DEPTH {
        return;
    }
    for inv in parse(cmd) {
        match peel(&inv) {
            Peeled::Tool(t) => out.steps.push(summarize_step(&t)),
            Peeled::Many(cmds) => {
                for c in cmds {
                    analyze_into(&c, resolver, depth + 1, seen, out);
                }
            }
            Peeled::ScriptRef {
                family,
                name,
                scope,
                args,
            } => {
                let key = format!("{family}:{name}");
                if scope.is_some() || seen.contains(&key) {
                    out.steps
                        .push(script_ref_step(family, &name, scope.as_deref()));
                    continue;
                }
                let resolved = resolver(family, &name);
                // `pnpm eslint .` runs a binary from node_modules; `npm test` with no script
                // table in reach must stay "the test script", not the shell `test` builtin.
                let generic_script = matches!(
                    name.as_str(),
                    "test"
                        | "["
                        | "which"
                        | "where"
                        | "start"
                        | "build"
                        | "dev"
                        | "serve"
                        | "clean"
                        | "install"
                        | "check"
                        | "format"
                        | "preview"
                        | "run"
                );
                if resolved.is_none() && family == "js" && !generic_script {
                    if let Some(sum) = tools::summarize(&name, &args) {
                        out.steps.push(Step {
                            text: sum.text,
                            kind: sum.kind,
                            risk: sum.risk,
                            tool: Some(sum.tool),
                            unknown: false,
                        });
                        continue;
                    }
                }
                match resolved {
                    Some(body) if !body.trim().is_empty() => {
                        seen.push(key.clone());
                        let before = out.steps.len();
                        analyze_into(&body, resolver, depth + 1, seen, out);
                        seen.pop();
                        if out.steps.len() == before {
                            out.steps.push(script_ref_step(family, &name, None));
                        }
                    }
                    _ => out
                        .steps
                        .push(script_ref_step(family, &name, scope.as_deref())),
                }
            }
        }
    }
}

fn script_ref_step(family: &str, name: &str, scope: Option<&str>) -> Step {
    let what = match family {
        "js" => "script",
        "make" => "make target",
        "just" => "just recipe",
        "task" => "task",
        "cargo" => "cargo alias",
        _ => "task",
    };
    let hint = crate::explain::name_hint(name);
    let text = match scope {
        Some(s) if s == "all workspaces" || s == "workspace packages" => {
            format!("Run {name} in all workspace packages")
        }
        Some(s) => format!("Run {name} in {s}"),
        None => {
            if name.is_empty() {
                "Run the default make target".to_string()
            } else if name.contains('*') {
                format!("Run all {name} scripts")
            } else {
                format!("Run the {name} {what}")
            }
        }
    };
    Step {
        text,
        kind: hint.as_ref().map(|h| h.kind).unwrap_or(Kind::Other),
        risk: Risk::Safe,
        tool: None,
        unknown: hint.is_none(),
    }
}

fn summarize_step(inv: &Invocation) -> Step {
    match tools::summarize(&inv.program, &inv.args) {
        Some(Summary {
            text,
            kind,
            risk,
            tool,
        }) => Step {
            text,
            kind,
            risk,
            tool: Some(tool),
            unknown: false,
        },
        None => {
            let shown = display_program(&inv.program);
            Step {
                text: format!("Run {shown}"),
                kind: Kind::Other,
                risk: Risk::Safe,
                tool: None,
                unknown: true,
            }
        }
    }
}

/// How an unknown program is shown to the user: `./scripts/foo.sh` → `scripts/foo.sh`.
pub fn display_program(p: &str) -> String {
    let p = p.trim_start_matches("./").trim_start_matches(".\\");
    if let Some(m) = p.strip_prefix("python-module:") {
        return format!("the {m} Python module");
    }
    p.to_string()
}

/// Map a step kind to a category.
pub fn kind_category(kind: Kind) -> Option<Category> {
    Some(match kind {
        Kind::Dev => Category::Development,
        Kind::Test | Kind::E2e | Kind::Bench => Category::Testing,
        Kind::Lint | Kind::TypeCheck | Kind::Format => Category::Quality,
        Kind::Build | Kind::Clean | Kind::Generate | Kind::Docs => Category::Build,
        Kind::Migrate | Kind::Db => Category::Database,
        Kind::Infra | Kind::Container => Category::Infrastructure,
        Kind::Deploy | Kind::Publish => Category::Release,
        Kind::Install | Kind::Other | Kind::Run => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progs(cmd: &str) -> Vec<String> {
        parse(cmd).into_iter().map(|i| i.program).collect()
    }

    #[test]
    fn splits_chains() {
        assert_eq!(
            progs("eslint . && tsc --noEmit && vitest run"),
            ["eslint", "tsc", "vitest"]
        );
        assert_eq!(progs("a; b | c || d"), ["a", "b", "c", "d"]);
    }

    #[test]
    fn quotes_and_env() {
        let inv = parse(
            r#"NODE_ENV=production FOO="bar baz" node -e 'console.log("x && y")' > out.log 2>&1"#,
        );
        assert_eq!(inv.len(), 1);
        assert_eq!(inv[0].env, ["NODE_ENV=production", "FOO=bar baz"]);
        assert_eq!(inv[0].program, "node");
        assert_eq!(inv[0].args, ["-e", r#"console.log("x && y")"#]);
    }

    #[test]
    fn newlines_split_and_control_words_are_skipped() {
        assert_eq!(progs("npm ci\nnpm test\n"), ["npm", "npm"]);
        assert_eq!(
            progs("set -euo pipefail\nif [ -f x ]; then cargo build; fi\nfor f in a b; do echo $f; done"),
            ["cargo", "echo"]
        );
        let a = analyze("npm test", &no_resolver);
        assert_eq!(a.steps[0].text, "Run the test script");
    }

    #[test]
    fn substitutions_and_exports_do_not_split_commands() {
        assert_eq!(
            progs("export $(grep -v '^#' .env | xargs) && echo \"type is $T\""),
            ["echo"]
        );
        let inv = parse("FOO=$(cat a | head -1) tool --x");
        assert_eq!(inv.len(), 1);
        assert_eq!(inv[0].program, "tool");
        assert_eq!(inv[0].env, ["FOO=$(cat a | head -1)"]);
    }

    #[test]
    fn windows_paths_survive() {
        let inv = parse(r#"node .\scripts\build.js --out "C:\Program Files\out""#);
        assert_eq!(
            inv[0].args,
            [r".\scripts\build.js", "--out", r"C:\Program Files\out"]
        );
    }

    #[test]
    fn peels_npx_and_cross_env() {
        let a = analyze("cross-env NODE_ENV=test npx vitest run", &no_resolver);
        assert_eq!(a.steps.len(), 1);
        assert!(a.has_tool("vitest"));
    }

    #[test]
    fn resolves_script_refs() {
        let scripts = |fam: &str, name: &str| -> Option<String> {
            if fam != "js" {
                return None;
            }
            match name {
                "lint" => Some("eslint .".into()),
                "test" => Some("vitest run".into()),
                "loop" => Some("npm run loop".into()),
                _ => None,
            }
        };
        let a = analyze("npm run lint && npm test", &scripts);
        assert!(a.has_tool("eslint") && a.has_tool("vitest"));
        let a = analyze("npm run loop", &scripts);
        assert_eq!(a.steps.len(), 1);
        assert_eq!(a.steps[0].text, "Run the loop script");
    }

    #[test]
    fn concurrently_expands() {
        let scripts = |_: &str, name: &str| -> Option<String> {
            match name {
                "dev:web" => Some("vite".into()),
                "dev:api" => Some("nodemon server.js".into()),
                _ => None,
            }
        };
        let a = analyze(
            r#"concurrently -n web,api "npm:dev:web" "npm:dev:api""#,
            &scripts,
        );
        assert_eq!(a.steps.len(), 2);
        assert!(a.has_tool("vite"));
    }
}
