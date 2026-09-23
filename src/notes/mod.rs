//! Practical notes about a command: does it keep running, need a device, read from the
//! network, or touch git. Like [`crate::risk`], this looks only at what the command text says
//! (its tokens and the tools recognised during analysis). It never infers what a program does
//! internally, so a script that reads an API key from code gets no note: that is unknowable
//! from the outside, and a wrong note is worse than none.

use crate::analyze::tools::Kind;
use crate::analyze::Analysis;
use crate::model::{Note, Risk};

/// Notes for a command, from tool-level evidence plus token patterns on the raw text.
pub fn classify(command: &str, analysis: &Analysis) -> Vec<Note> {
    let mut out = Vec::new();
    let lower = command.to_ascii_lowercase();
    let segs: Vec<Vec<&str>> = lower
        .split([';', '|', '&'])
        .map(|s| {
            s.split_whitespace()
                .map(|t| t.trim_matches(|c| c == '\'' || c == '"'))
                .collect::<Vec<&str>>()
        })
        .filter(|v| !v.is_empty())
        .map(peel_runner)
        .filter(|v| !v.is_empty())
        .collect();

    if analysis.steps.iter().any(|s| s.kind == Kind::Dev) || segs.iter().any(|s| long_running(s)) {
        out.push(Note::LongRunning);
    }
    if analysis
        .steps
        .iter()
        .any(|s| s.tool.as_deref().is_some_and(|t| DEVICE_TOOLS.contains(&t)))
        || segs.iter().any(|s| device(s))
    {
        out.push(Note::Device);
    }
    // External already implies the network; the note is for reads that change nothing remote.
    if analysis.max_risk() < Risk::External && segs.iter().any(|s| network(s)) {
        out.push(Note::Download);
    }
    if segs.iter().any(|s| git(s)) {
        out.push(Note::Git);
    }
    out
}

/// `npx expo run:ios`, `pnpm exec eslint .`, `bun x vitest`: the rules want the tool, not the
/// runner in front of it.
fn peel_runner(mut t: Vec<&str>) -> Vec<&str> {
    loop {
        match (t.first().copied(), t.get(1).copied()) {
            (Some("npx" | "bunx" | "pnpx"), _) => {
                t.remove(0);
            }
            (Some("pnpm" | "yarn" | "npm"), Some("exec" | "dlx")) | (Some("bun"), Some("x")) => {
                t.drain(0..2);
            }
            _ => return t,
        }
        // Skip the runner's own flags (`npx -y tool`).
        while t.first().is_some_and(|w| w.starts_with('-')) {
            t.remove(0);
        }
    }
}

const DEVICE_TOOLS: &[&str] = &[
    "maestro",
    "detox",
    "xcodebuild",
    "adb",
    "simctl",
    "emulator",
];

fn has(toks: &[&str], w: &str) -> bool {
    toks.contains(&w)
}

fn long_running(t: &[&str]) -> bool {
    let prog = t[0];
    has(t, "--watch")
        || (has(t, "-w")
            && matches!(
                prog,
                "tsc" | "jest" | "vitest" | "mocha" | "cargo" | "esbuild"
            ))
        || (has(t, "watch")
            && matches!(
                prog,
                "cargo" | "bun" | "deno" | "swc" | "tsc" | "webpack" | "nx"
            ))
        || (prog == "tail" && has(t, "-f"))
        || (has(t, "logs") && (has(t, "-f") || has(t, "--follow")))
        || matches!(
            prog,
            "nodemon" | "ts-node-dev" | "tsnd" | "watchexec" | "entr" | "air" | "reflex"
        )
        || (has(t, "up")
            && matches!(
                prog,
                "docker" | "docker-compose" | "podman" | "podman-compose"
            )
            && !has(t, "-d")
            && !has(t, "--detach"))
        || (prog == "storybook" && has(t, "dev"))
        || (prog == "flutter" && has(t, "run"))
        || (prog == "expo" && (has(t, "start") || t.iter().any(|x| x.starts_with("run:"))))
}

fn device(t: &[&str]) -> bool {
    let prog = t[0];
    (prog == "expo" && t.iter().any(|x| x.starts_with("run:")))
        || (prog == "flutter" && (has(t, "run") || has(t, "install") || has(t, "drive")))
        || (prog == "react-native" && (has(t, "run-ios") || has(t, "run-android")))
        || t.iter().any(|x| {
            x.ends_with("installdebug")
                || x.ends_with("installrelease")
                || x.ends_with("connectedandroidtest")
                || x.ends_with("connectedcheck")
        })
        || (prog == "xcodebuild"
            && t.iter()
                .any(|x| x.contains("simulator") || x.starts_with("id=")))
        || prog == "maestro"
        || (prog == "detox" && has(t, "test"))
        || prog == "adb"
        || (prog == "xcrun" && has(t, "simctl"))
}

fn network(t: &[&str]) -> bool {
    let prog = t[0];
    let sub = t.get(1).copied().unwrap_or("");
    let third = t.get(2).copied().unwrap_or("");
    match prog {
        "npm" | "pnpm" | "yarn" | "bun" => {
            matches!(
                sub,
                "install"
                    | "i"
                    | "ci"
                    | "add"
                    | "update"
                    | "upgrade"
                    | "up"
                    | "dlx"
                    | "create"
                    | "outdated"
                    | "audit"
            ) || (prog == "yarn" && t.len() == 1)
                || (prog == "bun" && sub == "x")
        }
        // npx/bunx run the local binary when it is installed; only the Python ones always fetch.
        "uvx" | "pipx" => true,
        "pip" | "pip3" => matches!(sub, "install" | "download"),
        "uv" | "poetry" | "pdm" | "pipenv" | "rye" => {
            matches!(sub, "sync" | "install" | "add" | "lock" | "update")
        }
        "cargo" => matches!(
            sub,
            "fetch" | "update" | "install" | "add" | "search" | "binstall"
        ),
        "go" => {
            (sub == "mod" && matches!(third, "download" | "tidy"))
                || sub == "get"
                || sub == "install"
        }
        "bundle" => matches!(sub, "install" | "update" | ""),
        "gem" => matches!(sub, "install" | "update"),
        "composer" => matches!(sub, "install" | "update" | "require" | "create-project"),
        "docker" | "podman" => {
            sub == "pull"
                || (sub == "compose" && has(t, "pull"))
                || (sub == "build" && has(t, "--pull"))
        }
        "docker-compose" | "podman-compose" => has(t, "pull"),
        "git" => matches!(sub, "fetch" | "pull" | "clone" | "submodule" | "ls-remote"),
        "curl" | "wget" | "http" | "xh" | "aria2c" => true,
        "brew" | "apt" | "apt-get" | "dnf" | "yum" | "apk" | "pacman" | "choco" | "scoop"
        | "winget" | "nix" | "mise" | "asdf" | "rustup" | "nvm" | "fnm" | "volta" => true,
        "terraform" | "tofu" => sub == "init",
        "helm" => matches!(sub, "repo" | "pull" | "dependency"),
        "flutter" => sub == "pub" || sub == "precache",
        "dart" => sub == "pub",
        "dotnet" => sub == "restore" || (sub == "tool" && third == "install"),
        "gradle" | "gradlew" | "./gradlew" | "mvn" | "mvnw" | "./mvnw" => {
            has(t, "dependencies")
                || has(t, "--refresh-dependencies")
                || has(t, "dependency:resolve")
        }
        "mix" => sub == "deps.get" || (sub == "deps" && third == "get"),
        "pre-commit" => matches!(sub, "install" | "autoupdate" | "install-hooks"),
        "playwright" | "cypress" => sub == "install",
        _ => false,
    }
}

fn git(t: &[&str]) -> bool {
    let prog = t[0];
    let sub = t.get(1).copied().unwrap_or("");
    match prog {
        "git" => matches!(
            sub,
            "commit"
                | "tag"
                | "add"
                | "rm"
                | "mv"
                | "checkout"
                | "switch"
                | "restore"
                | "reset"
                | "rebase"
                | "merge"
                | "cherry-pick"
                | "revert"
                | "stash"
                | "clean"
                | "am"
                | "apply"
                | "push"
                | "branch"
                | "worktree"
                | "submodule"
                | "notes"
                | "gc"
        ),
        "gh" => matches!(sub, "pr" | "release" | "repo"),
        "glab" => matches!(sub, "mr" | "release"),
        "npm" | "pnpm" | "yarn" | "bun" => sub == "version",
        "cargo" => sub == "release",
        "changeset" => matches!(sub, "version" | "publish" | "tag"),
        "standard-version" | "release-it" | "semantic-release" | "lerna" | "commitizen" | "cz"
        | "git-cz" => true,
        "husky" | "lefthook" | "pre-commit" | "lint-staged" | "commitlint" | "simple-git-hooks" => {
            true
        }
        "nx" => sub == "release",
        "mvn" | "./mvnw" | "mvnw" => t.iter().any(|x| x.starts_with("release:")),
        "gradle" | "gradlew" | "./gradlew" => {
            t.iter().any(|x| *x == "release" || x.ends_with(":release"))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::{analyze, no_resolver};

    fn n(cmd: &str) -> Vec<Note> {
        classify(cmd, &analyze(cmd, &no_resolver))
    }

    #[test]
    fn long_running_commands() {
        for c in [
            "vite",
            "next dev",
            "tsc --watch",
            "docker compose logs -f",
            "docker compose up",
            "nodemon server.js",
            "cargo watch -x test",
        ] {
            assert!(n(c).contains(&Note::LongRunning), "{c}");
        }
        for c in [
            "vitest run",
            "docker compose up -d",
            "tsc --noEmit",
            "cargo build",
        ] {
            assert!(!n(c).contains(&Note::LongRunning), "{c}");
        }
    }

    #[test]
    fn device_commands() {
        for c in [
            "expo run:ios",
            "npx expo run:android",
            "flutter run",
            "./gradlew app:installDebug",
            "maestro test .maestro",
            "react-native run-android",
        ] {
            assert!(n(c).contains(&Note::Device), "{c}");
        }
        assert!(!n("flutter build apk").contains(&Note::Device));
        assert!(!n("expo start").contains(&Note::Device));
    }

    #[test]
    fn network_reads_but_not_external_writes() {
        for c in [
            "npm install",
            "pnpm i",
            "pip install -r requirements.txt",
            "go mod download",
            "docker pull postgres",
            "terraform init",
        ] {
            assert!(n(c).contains(&Note::Download), "{c}");
        }
        assert!(!n("npm publish").contains(&Note::Download));
        assert!(!n("git push").contains(&Note::Download));
        assert!(!n("vitest run").contains(&Note::Download));
    }

    #[test]
    fn git_mutations() {
        for c in [
            "git commit -am wip",
            "git tag v1",
            "git clean -fdx",
            "npm version patch",
            "changeset version",
            "npx changeset version",
            "husky install",
            "git push origin main",
        ] {
            assert!(n(c).contains(&Note::Git), "{c}");
        }
        for c in ["git status", "git log --oneline", "git diff"] {
            assert!(!n(c).contains(&Note::Git), "{c}");
        }
    }
}
