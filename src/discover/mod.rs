//! Ecosystem adapters and the orchestrator that turns a scanned tree into a [`Repo`].
//!
//! Each adapter implements [`Discoverer`]. Detection is purely file-based; adapters never run
//! subprocesses. Actions are normalised into [`Action`] before leaving this module.

pub mod bazel;
pub mod cargo;
pub mod deno;
pub mod docker;
pub mod dotnet;
pub mod golang;
pub mod iac;
pub mod js;
pub mod just;
pub mod jvm;
pub mod kubernetes;
pub mod make;
pub mod misc;
pub mod mobile;
pub mod mono;
pub mod php;
pub mod python;
pub mod ruby;
pub mod scripts;
pub mod taskfile;

use crate::explain;
use crate::model::*;
use crate::repo::{self, DirInfo};
use crate::runtime;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Options {
    /// Look at the local machine (PATH, installed apps) to suggest runtime commands.
    pub probe_runtime: bool,
    /// `std::env::consts::OS` normally; overridable for deterministic tests.
    pub host_os: &'static str,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            probe_runtime: true,
            host_os: std::env::consts::OS,
        }
    }
}

pub struct Context<'a> {
    pub root: &'a Path,
    pub dirs: &'a [DirInfo],
    pub host_os: &'static str,
}

impl<'a> Context<'a> {
    /// `dir` itself followed by its ancestors up to the root.
    pub fn ancestors(&self, dir: &DirInfo) -> Vec<&'a DirInfo> {
        let mut out = Vec::new();
        let mut rel = dir.rel.as_str();
        loop {
            if let Some(d) = self.dirs.iter().find(|d| d.rel == rel) {
                out.push(d);
            }
            if rel == "." {
                break;
            }
            rel = match rel.rfind('/') {
                Some(p) => &rel[..p],
                None => ".",
            };
        }
        out
    }

    /// Scanned directories strictly below `dir`.
    pub fn descendants(&self, dir: &DirInfo) -> Vec<&'a DirInfo> {
        let prefix = if dir.rel == "." {
            String::new()
        } else {
            format!("{}/", dir.rel)
        };
        self.dirs
            .iter()
            .filter(|d| d.rel != dir.rel && d.rel.starts_with(&prefix))
            .collect()
    }

    pub fn dir(&self, rel: &str) -> Option<&'a DirInfo> {
        self.dirs.iter().find(|d| d.rel == rel)
    }

    /// Relative path of `dir` from `base`, `.` when equal.
    pub fn rel_from(&self, base: &DirInfo, dir: &DirInfo) -> String {
        dir.rel_to(base, "")
    }
}

/// A sibling script an adapter knows about, so command analysis can resolve references
/// like `npm run build` or `make test` to their bodies.
#[derive(Debug, Clone)]
pub struct Script {
    pub family: &'static str,
    pub name: String,
    pub body: String,
}

#[derive(Debug, Default)]
pub struct Discovery {
    pub actions: Vec<Action>,
    pub scripts: Vec<Script>,
}

impl Discovery {
    pub fn script(&mut self, family: &'static str, name: &str, body: &str) {
        self.scripts.push(Script {
            family,
            name: name.to_string(),
            body: body.to_string(),
        });
    }
}

pub trait Discoverer: Sync {
    fn id(&self) -> &'static str;
    fn kind(&self) -> ProjectKind;
    /// Does this directory contain the adapter's markers?
    fn detect(&self, dir: &DirInfo) -> bool;
    /// A project name derived from the adapter's manifest.
    fn project_name(&self, _dir: &DirInfo) -> Option<String> {
        None
    }
    /// Attachable adapters (Docker, Kubernetes, scripts) contribute to the nearest ancestor
    /// project instead of forming their own.
    fn attachable(&self) -> bool {
        false
    }
    /// For attachable adapters: directory names in which detection is honoured when the
    /// directory is not itself a project. `None` means any directory.
    fn attach_dir_names(&self) -> Option<&'static [&'static str]> {
        None
    }
    /// Once detected, the adapter owns the whole subtree (nested detections are ignored).
    fn claims_subtree(&self) -> bool {
        false
    }
    /// Produce actions for the project rooted at `base`. `dirs` lists the detected directories
    /// attributed to that project (always contains `base` for non-attachable adapters).
    fn discover(&self, ctx: &Context, base: &DirInfo, dirs: &[&DirInfo]) -> Discovery;
}

/// All adapters in priority order (the first detected non-attachable one names the project).
pub fn all() -> Vec<Box<dyn Discoverer>> {
    vec![
        Box::new(dotnet::DotNet),
        Box::new(js::Js),
        Box::new(cargo::Cargo),
        Box::new(golang::Go),
        Box::new(python::Python),
        // Mobile's markers (Xcode projects/workspaces, Package.swift, pubspec.yaml, a
        // fastlane/Fastfile) are specific; it must win over Ruby, whose only marker for an
        // iOS project is often just a Gemfile pulled in for Bundler/Fastlane.
        Box::new(mobile::Mobile),
        Box::new(ruby::Ruby),
        Box::new(jvm::Jvm),
        Box::new(php::Php),
        Box::new(deno::Deno),
        Box::new(mono::Mono),
        Box::new(bazel::Bazel),
        Box::new(misc::Misc),
        Box::new(make::Make),
        Box::new(just::Just),
        Box::new(taskfile::Taskfile),
        Box::new(docker::Docker),
        Box::new(kubernetes::Kubernetes),
        Box::new(iac::Iac),
        Box::new(scripts::Scripts),
    ]
}

/// Discover everything below `root`. Read-only.
pub fn discover(root: &Path, opts: &Options) -> Repo {
    let dirs = repo::scan(root);
    let ctx = Context {
        root,
        dirs: &dirs,
        host_os: opts.host_os,
    };
    let discs = all();

    // ---- detection ---------------------------------------------------------------------
    let mut det: Vec<Vec<bool>> = vec![vec![false; discs.len()]; dirs.len()];
    for (di, dir) in dirs.iter().enumerate() {
        for (ki, d) in discs.iter().enumerate() {
            if d.claims_subtree() {
                let claimed = ctx.ancestors(dir).iter().skip(1).any(|a| {
                    dirs.iter()
                        .position(|x| x.rel == a.rel)
                        .map(|ai| det[ai][ki])
                        .unwrap_or(false)
                });
                if claimed {
                    continue;
                }
            }
            det[di][ki] = d.detect(dir);
        }
    }
    let is_project: Vec<bool> = (0..dirs.len())
        .map(|di| {
            di == 0
                || discs
                    .iter()
                    .enumerate()
                    .any(|(ki, d)| !d.attachable() && det[di][ki])
        })
        .collect();

    // ---- attribution -------------------------------------------------------------------
    let mut groups: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (di, dir) in dirs.iter().enumerate() {
        for (ki, d) in discs.iter().enumerate() {
            if !det[di][ki] {
                continue;
            }
            if is_project[di] {
                groups.entry((di, ki)).or_default().push(di);
                continue;
            }
            if !d.attachable() {
                continue;
            }
            let Some(pi) = repo::nearest(&dirs, di, |i| is_project[i]) else {
                continue;
            };
            if let Some(names) = d.attach_dir_names() {
                // Named helper directories (`docker/`, `scripts/`) only attach to their direct parent.
                if !names.contains(&dir.name()) || dirs[pi].depth + 1 != dir.depth {
                    continue;
                }
            }
            groups.entry((pi, ki)).or_default().push(di);
        }
    }

    // ---- projects ----------------------------------------------------------------------
    let repo_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "repository".to_string());
    let mut projects: Vec<Project> = Vec::new();
    let mut project_scripts: Vec<Vec<Script>> = Vec::new();
    for (di, dir) in dirs.iter().enumerate() {
        if !is_project[di] {
            continue;
        }
        let primary = discs
            .iter()
            .enumerate()
            .find(|(ki, d)| !d.attachable() && det[di][*ki]);
        let kind = if di == 0 && primary.is_none() {
            ProjectKind::Root
        } else {
            primary.map(|(_, d)| d.kind()).unwrap_or(ProjectKind::Root)
        };
        let name = if di == 0 {
            repo_name.clone()
        } else {
            primary
                .and_then(|(_, d)| d.project_name(dir))
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| dir.name().to_string())
        };
        let mut actions = Vec::new();
        let mut scripts = Vec::new();
        let mut tools: Vec<&'static str> = Vec::new();
        for (ki, d) in discs.iter().enumerate() {
            let Some(idxs) = groups.get(&(di, ki)) else {
                continue;
            };
            let group_dirs: Vec<&DirInfo> = idxs.iter().map(|&i| &dirs[i]).collect();
            let mut disc = d.discover(&ctx, dir, &group_dirs);
            for a in &mut disc.actions {
                if a.working_directory.is_empty() {
                    a.working_directory = dir.rel.clone();
                }
                if a.tool.is_empty() {
                    a.tool = d.id();
                }
                if !tools.contains(&a.tool) {
                    tools.push(a.tool);
                }
            }
            actions.extend(disc.actions);
            scripts.extend(disc.scripts);
        }
        // A non-root project that ends up with no actions at all is noise, not a finding
        // (e.g. a bare `composer.json` with no scripts, deps, or artisan). The root project is
        // always kept so `rhow` still explains why nothing was found.
        if di != 0 && actions.is_empty() {
            continue;
        }
        projects.push(Project {
            name,
            path: dir.rel.clone(),
            kind,
            tools,
            actions,
        });
        project_scripts.push(scripts);
    }

    // ---- explanation, categorisation, risk ---------------------------------------------
    for (p, scripts) in projects.iter_mut().zip(project_scripts.iter()) {
        let table: HashMap<(&str, &str), &str> = scripts
            .iter()
            .map(|s| ((s.family, s.name.as_str()), s.body.as_str()))
            .collect();
        let resolver = |family: &str, name: &str| -> Option<String> {
            table.get(&(family, name)).map(|b| b.to_string())
        };
        for a in &mut p.actions {
            explain::finalize(a, &resolver);
        }
    }

    // ---- identifiers -------------------------------------------------------------------
    assign_ids(&mut projects);

    // ---- runtime suggestions -----------------------------------------------------------
    let suggestions = if opts.probe_runtime {
        runtime::suggest(&projects, opts.host_os)
    } else {
        vec![]
    };

    Repo {
        root: root.to_path_buf(),
        name: repo_name,
        projects,
        suggestions,
    }
}

fn prefix_for(p: &Project, taken: &HashSet<String>) -> String {
    let base = p
        .name
        .rsplit('/')
        .next()
        .unwrap_or(&p.name)
        .to_ascii_lowercase()
        .replace([' ', '_'], "-");
    let base = base.trim_matches('-').to_string();
    if !taken.contains(&base) && !base.is_empty() {
        return base;
    }
    p.path.replace('\\', "/")
}

/// Assign globally unique ids: root actions keep their names, nested projects are prefixed
/// (`api:test`), a nested project's `run`/`dev` collapses to the project name when free.
fn assign_ids(projects: &mut [Project]) {
    let mut taken: HashSet<String> = HashSet::new();
    let mut prefixes: Vec<String> = Vec::new();
    let mut used_prefixes: HashSet<String> = HashSet::new();
    for p in projects.iter() {
        let pre = if p.is_root() {
            String::new()
        } else {
            prefix_for(p, &used_prefixes)
        };
        used_prefixes.insert(pre.clone());
        prefixes.push(pre);
    }
    // Root first so that root ids win collisions.
    for (p, pre) in projects.iter_mut().zip(prefixes.iter()) {
        for a in &mut p.actions {
            let candidates: Vec<String> = if pre.is_empty() {
                vec![a.name.clone(), format!("{}:{}", a.tool, a.name)]
            } else {
                let mut v = Vec::new();
                if matches!(a.name.as_str(), "run" | "dev" | "start" | "serve")
                    && a.category == Category::Development
                {
                    v.push(pre.clone());
                }
                v.push(format!("{pre}:{}", a.name));
                v.push(format!("{pre}:{}:{}", a.tool, a.name));
                v
            };
            let mut chosen = None;
            let collapsible = !pre.is_empty()
                && matches!(a.name.as_str(), "run" | "dev" | "start" | "serve")
                && a.category == Category::Development;
            for (i, c) in candidates.into_iter().enumerate() {
                if !taken.contains(&c) {
                    // An inferred action that loses its natural id to a declared one is redundant:
                    // the project already exposes that name. Keep it, but out of the default view.
                    let natural = i == 0 || (collapsible && i == 1);
                    if !natural && a.source == ActionSource::Inferred {
                        a.hidden = true;
                    }
                    chosen = Some(c);
                    break;
                }
            }
            let id = chosen.unwrap_or_else(|| {
                let base = if pre.is_empty() {
                    a.name.clone()
                } else {
                    format!("{pre}:{}", a.name)
                };
                (2..)
                    .map(|n| format!("{base}-{n}"))
                    .find(|c| !taken.contains(c))
                    .unwrap()
            });
            taken.insert(id.clone());
            a.id = id;
        }
    }
}

// ---------------------------------------------------------------------------------------
// Shared helpers for adapters
// ---------------------------------------------------------------------------------------

/// Quote a path for a shell command line when it contains spaces.
pub fn q(p: &str) -> String {
    if p.contains(' ') {
        format!("\"{p}\"")
    } else {
        p.to_string()
    }
}

/// A human title from an identifier: `my-api` → `my-api` (kept as-is; we do not invent casing).
pub fn ident(s: &str) -> String {
    s.trim().to_string()
}
