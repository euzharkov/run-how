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
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

/// Built-in `rhow` subcommands. An action can never take one of these names as its id.
pub const RESERVED_IDS: &[&str] = &["why", "support"];

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
    /// `DirInfo::rel` → index into `dirs`.
    index: HashMap<String, usize>,
    /// Parsed JSON manifests, keyed by relative path, so workspace lookups that revisit the
    /// same `package.json` for every member parse it once.
    json: RefCell<HashMap<String, Option<Rc<serde_json::Value>>>>,
}

impl<'a> Context<'a> {
    pub fn new(root: &'a Path, dirs: &'a [DirInfo], host_os: &'static str) -> Self {
        Context {
            root,
            dirs,
            host_os,
            index: repo::index(dirs),
            json: RefCell::new(HashMap::new()),
        }
    }

    /// `dir` itself followed by its ancestors up to the root.
    pub fn ancestors(&self, dir: &DirInfo) -> Vec<&'a DirInfo> {
        let mut out = Vec::new();
        let mut rel = Some(dir.rel.as_str());
        while let Some(r) = rel {
            if let Some(&i) = self.index.get(r) {
                out.push(&self.dirs[i]);
            }
            rel = repo::parent_rel(r);
        }
        out
    }

    /// Position of `dir` in `dirs`.
    pub fn position(&self, dir: &DirInfo) -> Option<usize> {
        self.index.get(&dir.rel).copied()
    }

    /// `file` inside `dir`, parsed as JSON. Cached for the lifetime of the scan.
    pub fn json(&self, dir: &DirInfo, file: &str) -> Option<Rc<serde_json::Value>> {
        let key = format!("{}/{file}", dir.rel);
        if let Some(v) = self.json.borrow().get(&key) {
            return v.clone();
        }
        let parsed = dir
            .read(file)
            .and_then(|t| serde_json::from_str(&t).ok())
            .map(Rc::new);
        self.json.borrow_mut().insert(key, parsed.clone());
        parsed
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
    pub versions: Vec<ToolVersion>,
}

impl Discovery {
    pub fn script(&mut self, family: &'static str, name: &str, body: &str) {
        self.scripts.push(Script {
            family,
            name: name.to_string(),
            body: body.to_string(),
        });
    }

    /// Record a version/schema the project declares for itself, for `rhow support` to check
    /// against [`crate::support`]. `tool` must match a `support::Entry::id`.
    pub fn version(
        &mut self,
        tool: &'static str,
        value: impl Into<String>,
        source: impl Into<String>,
    ) {
        self.versions.push(ToolVersion {
            tool,
            value: value.into(),
            source: source.into(),
        });
    }
}

pub trait Discoverer: Sync {
    fn id(&self) -> &'static str;
    fn kind(&self) -> ProjectKind;
    /// Does this directory contain the adapter's markers?
    fn detect(&self, dir: &DirInfo) -> bool;
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
    let ctx = Context::new(root, &dirs, opts.host_os);
    let discs = all();

    // ---- shadowed platform runners -----------------------------------------------------
    // `android/`, `ios/`, … inside a Flutter / Expo / React Native app are generated build
    // scaffolding; nothing below them is a project or a helper directory of its own.
    let mut shadow_roots: Vec<String> = Vec::new();
    for dir in dirs.iter() {
        if repo::is_mobile_app_dir(dir) {
            for p in repo::PLATFORM_DIRS {
                if dir.has_dir(p) {
                    shadow_roots.push(if dir.rel == "." {
                        p.to_string()
                    } else {
                        format!("{}/{p}", dir.rel)
                    });
                }
            }
        }
    }
    let shadowed: Vec<bool> = dirs
        .iter()
        .map(|d| {
            shadow_roots
                .iter()
                .any(|r| d.rel == *r || d.rel.starts_with(&format!("{r}/")))
        })
        .collect();
    let demoted: Vec<bool> = dirs.iter().map(|d| repo::is_demoted_path(&d.rel)).collect();

    // ---- detection ---------------------------------------------------------------------
    let mut det: Vec<Vec<bool>> = vec![vec![false; discs.len()]; dirs.len()];
    for (di, dir) in dirs.iter().enumerate() {
        if shadowed[di] {
            continue;
        }
        for (ki, d) in discs.iter().enumerate() {
            if d.claims_subtree() {
                let claimed = ctx
                    .ancestors(dir)
                    .iter()
                    .skip(1)
                    .any(|a| ctx.position(a).map(|ai| det[ai][ki]).unwrap_or(false));
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
            if !d.attachable() || demoted[di] {
                // Compose files or manifests inside `examples/` or `test/` belong to a demo or
                // a test, not to the project above them.
                continue;
            }
            let Some(pi) = repo::nearest(&dirs, &ctx.index, di, |i| is_project[i]) else {
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
        // A project is named after its directory. Manifests disagree with each other
        // (`package.json` says `boatsetterowners`, `project.json` says `owner`), and the file
        // structure is the one identity every tool and every developer shares.
        let name = if di == 0 {
            repo_name.clone()
        } else {
            dir.name().to_string()
        };
        let mut actions = Vec::new();
        let mut scripts = Vec::new();
        let mut versions = Vec::new();
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
            versions.extend(disc.versions);
        }
        if demoted[di] {
            for a in &mut actions {
                a.confidence = Confidence::Low;
            }
        }
        // A non-root project that ends up with no actions AND no declared version is noise,
        // not a finding (e.g. a bare `composer.json` with no scripts, deps, or artisan). One
        // that only carries a declared version (a `pyproject.toml` with `requires-python` and
        // nothing else to run) is still worth keeping for `rhow support`. The root project is
        // always kept so `rhow` still explains why nothing was found.
        if di != 0 && actions.is_empty() && versions.is_empty() {
            continue;
        }
        projects.push(Project {
            name,
            path: dir.rel.clone(),
            kind,
            tools,
            actions,
            versions,
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
        hide_duplicates(&mut p.actions);
    }
    hide_workspace_fan_out(&mut projects);
    lift_repeated_package_scripts(&mut projects);
    hide_convention_in_large_repos(&mut projects);

    // ---- identifiers -------------------------------------------------------------------
    assign_ids(&mut projects);

    // ---- runtime suggestions -----------------------------------------------------------
    let suggestions = if opts.probe_runtime {
        runtime::suggest(&projects, opts.host_os)
    } else {
        vec![]
    };

    // CI steps run at the root, so `npm test` in a workflow means the root's test script.
    let root_table: HashMap<(&str, &str), &str> = project_scripts
        .first()
        .map(|scripts| {
            scripts
                .iter()
                .map(|s| ((s.family, s.name.as_str()), s.body.as_str()))
                .collect()
        })
        .unwrap_or_default();
    let root_resolver = |family: &str, name: &str| -> Option<String> {
        root_table.get(&(family, name)).map(|b| b.to_string())
    };
    let ci = crate::ci::discover(root, &root_resolver);

    Repo {
        ci,
        root: root.to_path_buf(),
        name: repo_name,
        projects,
        suggestions,
    }
}

/// Two adapters describing the same thing must not both show: a `package.json` script and an
/// Nx target both called `test`, an inferred Expo `start` next to a declared one, or `ios`
/// (script) and `run-ios` (Nx) that explain identically. Declared beats inferred; among
/// equals the earlier adapter (priority order) wins. Same-tool variants (`test:watch`,
/// `test:update`) are never touched: a project that lists them meant them.
fn hide_duplicates(actions: &mut [Action]) {
    // An explanation shared by several actions of the same tool (`tf:dev:plan`,
    // `tf:prod:plan`) marks parametrised variants: a declared script that reads the same
    // does not make all of them redundant, so those never match on meaning.
    let mut per_tool: HashMap<(&str, String), usize> = HashMap::new();
    for a in actions.iter() {
        *per_tool
            .entry((a.tool, a.description.to_ascii_lowercase()))
            .or_default() += 1;
    }
    let variant: Vec<bool> = actions
        .iter()
        .map(|a| per_tool[&(a.tool, a.description.to_ascii_lowercase())] > 1)
        .collect();
    for i in 0..actions.len() {
        if actions[i].hidden {
            continue;
        }
        for j in 0..i {
            if actions[j].hidden {
                continue;
            }
            let (a, b) = (&actions[j], &actions[i]);
            let same_name = a.name == b.name;
            let same_meaning = a.tool != b.tool
                && !variant[i]
                && !variant[j]
                && a.category == b.category
                && a.description.eq_ignore_ascii_case(&b.description);
            if !(same_name || same_meaning) {
                continue;
            }
            match (a.source, b.source) {
                (ActionSource::Inferred, ActionSource::Declared) => {
                    actions[j].hidden = true;
                }
                (ActionSource::Declared, ActionSource::Declared)
                    if same_name && !same_meaning && a.tool != b.tool =>
                {
                    // Same name, different explanation, different tools (`make test` vs
                    // `npm test`): both are real and may differ. Keep both.
                }
                _ => {
                    actions[i].hidden = true;
                    break;
                }
            }
        }
    }
}

/// A workspace root already offers `test`, `build`, `check`, … for every member; the same
/// inferred action repeated per member (`tokio-util:test`, `api:clippy`) adds nothing.
/// Hide an inferred, non-Development action of a nested project when the nearest ancestor
/// project shows one with the same name and tool. Declared scripts are never touched, and
/// `run`/`dev`-style actions stay because each app's is its own.
fn hide_workspace_fan_out(projects: &mut [Project]) {
    let offered: Vec<(String, HashSet<(String, &'static str)>)> = projects
        .iter()
        .map(|p| {
            (
                p.path.clone(),
                // Hidden ones count too: a root `test` hidden behind a declared twin still
                // means the workspace offers it.
                p.actions.iter().map(|a| (a.name.clone(), a.tool)).collect(),
            )
        })
        .collect();
    for p in projects.iter_mut().filter(|p| !p.is_root()) {
        let mut rel = repo::parent_rel(&p.path);
        let mut ancestor: Option<&HashSet<(String, &'static str)>> = None;
        while let Some(r) = rel {
            if let Some((_, set)) = offered.iter().find(|(path, _)| path == r) {
                ancestor = Some(set);
                break;
            }
            rel = repo::parent_rel(r);
        }
        let Some(set) = ancestor else { continue };
        for a in &mut p.actions {
            if a.source == ActionSource::Inferred
                && a.category != Category::Development
                && set.contains(&(a.name.clone(), a.tool))
            {
                a.hidden = true;
            }
        }
    }
}

/// A script that five or more nested packages declare with the same explanation (`compile`,
/// `test`, `lint` in every package of a pnpm monorepo) is a workspace convention. Show it once
/// at the root as "run in all packages" and take the per-package copies out of the default
/// view. Only JavaScript package managers have a single command for that.
fn lift_repeated_package_scripts(projects: &mut [Project]) {
    const MIN_REPEATS: usize = 5;
    let mut seen: HashMap<(String, &'static str, String), usize> = HashMap::new();
    for p in projects.iter().filter(|p| !p.is_root()) {
        for a in p
            .actions
            .iter()
            .filter(|a| a.source == ActionSource::Declared)
        {
            *seen
                .entry((a.name.clone(), a.tool, a.description.to_ascii_lowercase()))
                .or_default() += 1;
        }
    }
    let repeated: Vec<(String, &'static str, String)> = seen
        .into_iter()
        .filter(|(k, n)| *n >= MIN_REPEATS && matches!(k.1, "npm" | "pnpm" | "yarn" | "bun"))
        .map(|(k, _)| k)
        .collect();
    if repeated.is_empty() {
        return;
    }
    let mut lifted: Vec<Action> = Vec::new();
    for key in &repeated {
        let (name, tool, desc) = key;
        let mut sample: Option<Action> = None;
        for p in projects.iter_mut().filter(|p| !p.is_root()) {
            for a in &mut p.actions {
                if a.source == ActionSource::Declared
                    && a.tool == *tool
                    && a.name == *name
                    && a.description.eq_ignore_ascii_case(desc)
                {
                    if sample.is_none() {
                        sample = Some(a.clone());
                    }
                    a.confidence = Confidence::Low;
                }
            }
        }
        let root = &projects[0];
        if root.actions.iter().any(|a| a.name == *name && !a.hidden) {
            continue;
        }
        let yarn_berry = root
            .versions
            .iter()
            .any(|v| v.tool == "yarn" && !v.value.starts_with('1'));
        let command = match *tool {
            "pnpm" => format!("pnpm -r run {name}"),
            "yarn" if yarn_berry => format!("yarn workspaces foreach -A run {name}"),
            "yarn" => format!("yarn workspaces run {name}"),
            "bun" => format!("bun run --filter '*' {name}"),
            _ => format!("npm run {name} --workspaces --if-present"),
        };
        let Some(sample) = sample else { continue };
        lifted.push(
            Action::new(name.clone(), command)
                .tool(tool)
                .cwd(root.path.clone())
                .inferred(Confidence::Medium)
                .inferred_desc(format!(
                    "{} in every workspace package",
                    explain::tidy(&sample.description)
                ))
                .cat(sample.category)
                .risk(sample.risk),
        );
    }
    projects[0].actions.extend(lifted);
}

/// With more than a dozen projects, what each one *declares* and how each app *runs* is the
/// useful list; the inferred `test`/`lint`/`build`/… every one of them would have by
/// convention is not. `--all` still shows it.
fn hide_convention_in_large_repos(projects: &mut [Project]) {
    const MANY: usize = 12;
    let nested = projects
        .iter()
        .filter(|p| !p.is_root() && !p.actions.is_empty())
        .count();
    if nested <= MANY {
        return;
    }
    for p in projects.iter_mut().filter(|p| !p.is_root()) {
        for a in &mut p.actions {
            if a.source == ActionSource::Inferred && a.category != Category::Development {
                a.hidden = true;
            }
        }
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
    let mut taken: HashSet<String> = RESERVED_IDS.iter().map(|s| s.to_string()).collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn project(path: &str, name: &str, actions: Vec<Action>) -> Project {
        Project {
            name: name.into(),
            path: path.into(),
            kind: ProjectKind::JavaScript,
            tools: vec![],
            actions,
            versions: vec![],
        }
    }

    fn ids(projects: &[Project]) -> Vec<String> {
        projects
            .iter()
            .flat_map(|p| p.actions.iter().map(|a| a.id.clone()))
            .collect()
    }

    #[test]
    fn reserved_subcommand_names_are_never_action_ids() {
        let mut ps = vec![project(
            ".",
            "root",
            vec![
                Action::new("support", "node support.js").tool("npm"),
                Action::new("why", "node why.js").tool("npm"),
                Action::new("test", "vitest").tool("npm"),
            ],
        )];
        assign_ids(&mut ps);
        assert_eq!(ids(&ps), ["npm:support", "npm:why", "test"]);
        assert!(ids(&ps)
            .iter()
            .all(|id| !RESERVED_IDS.contains(&id.as_str())));
    }

    #[test]
    fn nested_projects_are_prefixed_and_collisions_resolved() {
        let mut ps = vec![
            project(
                ".",
                "root",
                vec![Action::new("test", "make test").tool("make")],
            ),
            project(
                "packages/api",
                "api",
                vec![
                    Action::new("test", "npm run test").tool("npm"),
                    Action::new("dev", "npm run dev")
                        .tool("npm")
                        .cat(Category::Development),
                    Action::new("test", "cargo test")
                        .tool("cargo")
                        .inferred(Confidence::High),
                ],
            ),
            project(
                "services/api",
                "api",
                vec![Action::new("test", "go test ./...").tool("go")],
            ),
        ];
        assign_ids(&mut ps);
        assert_eq!(
            ids(&ps),
            [
                "test",
                "api:test",
                "api",
                "api:cargo:test",
                "services/api:test"
            ]
        );
        // The inferred duplicate lost its natural id, so it is hidden by default.
        assert!(ps[1].actions[2].hidden);
        assert!(!ps[1].actions[0].hidden);
    }
}
