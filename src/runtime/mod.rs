//! Local runtime suggestions. These are environment dependencies, not project tasks, and are
//! only shown when the repository needs them and the local machine lacks them.

use crate::model::*;
use std::path::{Path, PathBuf};

/// Locate an executable on PATH without running it.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".into())
            .split(';')
            .map(|s| s.to_ascii_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let p = dir.join(format!("{name}{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn app_installed(name: &str) -> bool {
    Path::new("/Applications").join(name).is_dir()
}

/// Suggest runtime commands for the repository's needs on this machine.
pub fn suggest(projects: &[Project], host_os: &str) -> Vec<Action> {
    let mut out = Vec::new();
    let needs_docker = projects
        .iter()
        .flat_map(|p| p.actions.iter())
        .any(|a| a.command.starts_with("docker ") || a.command.starts_with("docker-compose "));
    if needs_docker && which("docker").is_none() {
        let mk = |name: &str, cmd: &str, desc: &str| {
            Action::new(name, cmd)
                .tool("runtime")
                .inferred(Confidence::High)
                .inferred_desc(desc)
                .cat(Category::Infrastructure)
        };
        if which("colima").is_some() {
            out.push(mk(
                "colima:start",
                "colima start",
                "Start the local Docker runtime (Colima)",
            ));
        } else if which("orb").is_some() || which("orbctl").is_some() {
            out.push(mk(
                "orbstack:start",
                "orb start",
                "Start the local Docker runtime (OrbStack)",
            ));
        } else if which("podman").is_some() {
            out.push(mk(
                "podman:start",
                "podman machine start",
                "Start the local Podman machine",
            ));
        } else if host_os == "macos" && app_installed("Docker.app") {
            out.push(mk(
                "docker-desktop:start",
                "open -a Docker",
                "Start Docker Desktop",
            ));
        } else if host_os == "macos" && app_installed("Rancher Desktop.app") {
            out.push(mk(
                "rancher:start",
                "open -a \"Rancher Desktop\"",
                "Start Rancher Desktop",
            ));
        }
    }
    out
}
