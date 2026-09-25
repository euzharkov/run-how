//! Local runtime suggestions. These are environment dependencies, not project tasks, and are
//! only shown when the repository needs them and the local machine lacks them.

use crate::model::*;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Locate an executable on `PATH` without running it.
pub fn which(name: &str) -> Option<PathBuf> {
    which_in(name, &std::env::var_os("PATH")?)
}

/// Locate an executable on an explicit search path (`PATH` syntax). On Unix the file must be
/// executable; on Windows the `PATHEXT` extensions are tried.
pub fn which_in(name: &str, path: &OsStr) -> Option<PathBuf> {
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".into())
            .split(';')
            .map(|s| s.to_ascii_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(path) {
        for ext in &exts {
            let p = dir.join(format!("{name}{ext}"));
            if is_executable(&p) {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// A macOS application bundle in `/Applications` or the user's `~/Applications`.
fn app_installed(name: &str) -> bool {
    app_installed_in(name, std::env::var_os("HOME").as_deref())
}

fn app_installed_in(name: &str, home: Option<&OsStr>) -> bool {
    Path::new("/Applications").join(name).is_dir()
        || home
            .map(|h| Path::new(h).join("Applications").join(name).is_dir())
            .unwrap_or(false)
}

/// Suggest runtime commands for the repository's needs on this machine.
pub fn suggest(projects: &[Project], host_os: &str) -> Vec<Action> {
    suggest_with(projects, host_os, &|name| which(name), &app_installed)
}

/// [`suggest`] with the machine probes injected, so the decision can be tested without a
/// particular machine.
fn suggest_with(
    projects: &[Project],
    host_os: &str,
    which: &dyn Fn(&str) -> Option<PathBuf>,
    app_installed: &dyn Fn(&str) -> bool,
) -> Vec<Action> {
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("rhow-runtime-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_exe(dir: &Path, name: &str, executable: bool) -> PathBuf {
        let name = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_string()
        };
        let p = dir.join(name);
        std::fs::write(&p, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if executable { 0o755 } else { 0o644 };
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
        }
        let _ = executable;
        p
    }

    fn search_path(dirs: &[&Path]) -> std::ffi::OsString {
        std::env::join_paths(dirs.iter().map(|d| d.to_path_buf())).unwrap()
    }

    #[test]
    fn which_searches_the_given_path_in_order() {
        let a = TempDir::new("which-a");
        let b = TempDir::new("which-b");
        let in_b = write_exe(&b.0, "colima", true);
        let path = search_path(&[&a.0, &b.0]);
        assert_eq!(which_in("colima", &path), Some(in_b));
        assert_eq!(which_in("docker", &path), None);
        assert_eq!(which_in("colima", &search_path(&[&a.0])), None);
        // A directory on PATH that does not exist is skipped, not an error.
        let path = search_path(&[&a.0.join("missing"), &b.0]);
        assert!(which_in("colima", &path).is_some());
        // The first directory wins.
        let in_a = write_exe(&a.0, "colima", true);
        assert_eq!(which_in("colima", &search_path(&[&a.0, &b.0])), Some(in_a));
    }

    #[cfg(unix)]
    #[test]
    fn which_requires_the_executable_bit_on_unix() {
        let d = TempDir::new("which-mode");
        write_exe(&d.0, "docker", false);
        std::fs::create_dir(d.0.join("podman")).unwrap();
        let path = search_path(&[&d.0]);
        assert_eq!(which_in("docker", &path), None, "not executable");
        assert_eq!(which_in("podman", &path), None, "a directory");
        write_exe(&d.0, "docker", true);
        assert!(which_in("docker", &path).is_some());
    }

    #[test]
    fn user_applications_folder_counts_as_installed() {
        let home = TempDir::new("home");
        std::fs::create_dir_all(home.0.join("Applications").join("Docker.app")).unwrap();
        assert!(app_installed_in("Docker.app", Some(home.0.as_os_str())));
        assert!(!app_installed_in("Nope.app", Some(home.0.as_os_str())));
        assert!(!app_installed_in("Nope.app", None));
    }

    fn project(commands: &[&str]) -> Vec<Project> {
        vec![Project {
            name: "r".into(),
            path: ".".into(),
            kind: ProjectKind::Docker,
            tools: vec!["compose"],
            techs: vec![],
            actions: commands
                .iter()
                .map(|c| Action::new("x", *c).cwd("."))
                .collect(),
            versions: vec![],
        }]
    }

    #[test]
    fn suggests_a_runtime_only_when_docker_is_needed_and_missing() {
        let none = |_: &str| None;
        let no_app = |_: &str| false;
        let ids = |v: Vec<Action>| v.into_iter().map(|a| a.id).collect::<Vec<_>>();

        // Nothing needs Docker: nothing to suggest, whatever the machine has.
        let plain = project(&["npm test", "cargo build"]);
        assert!(suggest_with(&plain, "linux", &none, &no_app).is_empty());

        // Docker is needed and present: nothing to suggest.
        let compose = project(&["docker compose up -d", "npm test"]);
        let has_docker = |n: &str| (n == "docker").then(|| PathBuf::from("/usr/bin/docker"));
        assert!(suggest_with(&compose, "macos", &has_docker, &no_app).is_empty());

        // Docker is needed and missing: the first available runtime, in preference order.
        assert!(suggest_with(&compose, "linux", &none, &no_app).is_empty());
        let colima = |n: &str| (n == "colima").then(|| PathBuf::from("/opt/colima"));
        assert_eq!(
            ids(suggest_with(&compose, "macos", &colima, &no_app)),
            ["colima:start"]
        );
        let podman = |n: &str| (n == "podman").then(|| PathBuf::from("/opt/podman"));
        assert_eq!(
            ids(suggest_with(&compose, "linux", &podman, &no_app)),
            ["podman:start"]
        );
        let docker_app = |n: &str| n == "Docker.app";
        assert_eq!(
            ids(suggest_with(&compose, "macos", &none, &docker_app)),
            ["docker-desktop:start"]
        );
        // An installed app is a macOS answer only.
        assert!(suggest_with(&compose, "linux", &none, &docker_app).is_empty());
        let s = &suggest_with(&compose, "macos", &none, &docker_app)[0];
        assert_eq!(s.command, "open -a Docker");
        assert_eq!(s.source, ActionSource::Inferred);
        assert_eq!(s.category, Category::Infrastructure);
        assert_eq!(s.tool, "runtime");
    }
}
