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
    /// Raw file text, keyed the same way: every Cargo workspace member reads the root
    /// `Cargo.toml`, every Gradle subproject the root `settings.gradle`.
    text: RefCell<HashMap<String, Option<Rc<str>>>>,
    toml: RefCell<HashMap<String, Option<Rc<toml::Value>>>>,
    yaml: RefCell<HashMap<String, Option<Rc<serde_yaml::Value>>>>,
}

impl<'a> Context<'a> {
    pub fn new(root: &'a Path, dirs: &'a [DirInfo], host_os: &'static str) -> Self {
        Context {
            root,
            dirs,
            host_os,
            index: repo::index(dirs),
            json: RefCell::new(HashMap::new()),
            text: RefCell::new(HashMap::new()),
            toml: RefCell::new(HashMap::new()),
            yaml: RefCell::new(HashMap::new()),
        }
    }

    /// The scanned directory at `rel` (`.`-relative, `/`-separated), if any. Ignored
    /// directories (`bin/`, `.config/`) are listed shallowly, so they resolve too.
    pub fn dir_at(&self, rel: &str) -> Option<&'a DirInfo> {
        self.index.get(rel).map(|&i| &self.dirs[i])
    }

    /// The child directory `name` of `dir`, when it was scanned or listed.
    pub fn child(&self, dir: &DirInfo, name: &str) -> Option<&'a DirInfo> {
        let rel = if dir.rel == "." {
            name.to_string()
        } else {
            format!("{}/{name}", dir.rel)
        };
        self.dir_at(&rel)
    }

    /// Does `path` (relative to `dir`, `/`-separated, e.g. `bin/rails` or
    /// `.config/dotnet-tools.json`) exist as a file? Answered from the scan, never by
    /// touching the filesystem.
    pub fn has_file(&self, dir: &DirInfo, path: &str) -> bool {
        match path.rsplit_once('/') {
            None => dir.has(path),
            Some((sub, file)) => self
                .child_path(dir, sub)
                .map(|d| d.has(file))
                .unwrap_or(false),
        }
    }

    /// Does `path` (relative to `dir`) exist as a directory?
    pub fn has_dir(&self, dir: &DirInfo, path: &str) -> bool {
        self.child_path(dir, path).is_some()
    }

    fn child_path(&self, dir: &DirInfo, sub: &str) -> Option<&'a DirInfo> {
        let sub = sub.trim_matches('/');
        let rel = if dir.rel == "." {
            sub.to_string()
        } else {
            format!("{}/{sub}", dir.rel)
        };
        self.dir_at(&rel)
    }

    /// `file` inside `dir` as text. Cached for the lifetime of the scan.
    pub fn text(&self, dir: &DirInfo, file: &str) -> Option<Rc<str>> {
        let key = format!("{}/{file}", dir.rel);
        if let Some(v) = self.text.borrow().get(&key) {
            return v.clone();
        }
        let t = dir.read(file).map(Rc::from);
        self.text.borrow_mut().insert(key, t.clone());
        t
    }

    /// `file` inside `dir`, parsed as TOML. Cached.
    pub fn toml(&self, dir: &DirInfo, file: &str) -> Option<Rc<toml::Value>> {
        let key = format!("{}/{file}", dir.rel);
        if let Some(v) = self.toml.borrow().get(&key) {
            return v.clone();
        }
        let parsed = self
            .text(dir, file)
            .and_then(|t| toml::from_str(&t).ok())
            .map(Rc::new);
        self.toml.borrow_mut().insert(key, parsed.clone());
        parsed
    }

    /// `file` inside `dir`, parsed as YAML. Cached.
    pub fn yaml(&self, dir: &DirInfo, file: &str) -> Option<Rc<serde_yaml::Value>> {
        let key = format!("{}/{file}", dir.rel);
        if let Some(v) = self.yaml.borrow().get(&key) {
            return v.clone();
        }
        let parsed = self
            .text(dir, file)
            .and_then(|t| serde_yaml::from_str(&t).ok())
            .map(Rc::new);
        self.yaml.borrow_mut().insert(key, parsed.clone());
        parsed
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
        let parsed = self
            .text(dir, file)
            .and_then(|t| serde_json::from_str(&t).ok())
            .map(Rc::new);
        self.json.borrow_mut().insert(key, parsed.clone());
        parsed
    }

    /// Scanned directories strictly below `dir`, in scan order, without ignored ones.
    /// The scan is depth-first, so this is one contiguous slice, not a search of every
    /// directory in the repository.
    pub fn descendants(&self, dir: &DirInfo) -> Vec<&'a DirInfo> {
        let Some(i) = self.position(dir) else {
            return vec![];
        };
        self.dirs[repo::subtree(self.dirs, i)]
            .iter()
            .filter(|d| !d.ignored)
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
        if dir.ignored {
            continue;
        }
        if repo::is_mobile_app_dir(dir) || repo::is_native_module_dir(dir) {
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
        if shadowed[di] || dir.ignored {
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
            disc.actions.retain(|a| typeable(&a.command));
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
            techs: Vec::new(),
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
        drop_duplicates(&mut p.actions);
    }
    drop_workspace_fan_out(&mut projects);
    lift_repeated_package_scripts(&mut projects);
    drop_convention_in_large_repos(&mut projects);
    drop_redundant(&mut projects);

    for p in &mut projects {
        p.techs = crate::techs::of_project(p.kind, &p.actions);
    }

    // ---- identifiers -------------------------------------------------------------------
    // An inferred action that loses its natural id to one that stands in for it is marked
    // redundant while ids are handed out; drop it and hand them out again, so the ids that
    // remain are the natural ones wherever possible.
    loop {
        assign_ids(&mut projects);
        if !projects
            .iter()
            .any(|p| p.actions.iter().any(|a| a.redundant))
        {
            break;
        }
        drop_redundant(&mut projects);
    }

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

/// The language ecosystem an adapter family (`Action::tool`) belongs to, when it belongs to
/// one. Generic runners (`make`, `just`, `task`, shell scripts) and infrastructure tools
/// have none: a `make test` may wrap anything, so it may stand in for anything.
fn ecosystem(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "npm" | "pnpm" | "yarn" | "bun" | "deno" | "nx" | "turbo" | "rush" | "moon" | "expo"
        | "eas" | "react-native" | "detox" | "playwright" | "cypress" => "js",
        "ruby" | "rails" | "rake" => "ruby",
        "php" | "composer" | "artisan" | "symfony" => "php",
        "python" | "uv" | "poetry" | "pdm" | "hatch" | "poe" | "tox" | "nox" | "invoke"
        | "django" => "python",
        "gradle" | "maven" | "sbt" => "jvm",
        "lein" | "clojure" => "clojure",
        "dotnet" | "cake" | "nuke" => "dotnet",
        "stack" | "cabal" | "haskell" => "haskell",
        "flutter" | "dart" => "dart",
        "swift" | "xcode" | "cocoapods" => "apple",
        "cargo" => "rust",
        "go" => "go",
        "mix" => "elixir",
        "zig" => "zig",
        "cmake" => "cmake",
        "dune" => "ocaml",
        "nimble" => "nim",
        // Bazel and Pants build every language in the tree they root: like `make`, they may
        // stand in for any language's command there, and no language's for theirs.
        _ => return None,
    })
}

/// Whether an action of tool `holder` makes one of tool `other` with the same name redundant:
/// same ecosystem, or the holder is a generic runner and the other a language's command. `yarn test` (Jest) says nothing about
/// `bundle exec rspec`, and `dotnet build` says nothing about the project's own `build.ps1`;
/// `make test` may well run either.
fn stands_in_for(holder: &str, other: &str) -> bool {
    match (ecosystem(holder), ecosystem(other)) {
        (Some(x), Some(y)) => x == y,
        (None, Some(_)) => true,
        // Two runners (`build.cmd` and `build.ps1`, `make test` and `./test.sh`) are two
        // commands, not one said twice.
        (None, None) => holder == other,
        (Some(_), None) => false,
    }
}

/// Bazel and Pants build and test the whole tree they root: at that root their inferred
/// `build`/`test` are the ones to show, not a language convention that happens to share the
/// directory (`cargo test` next to `bazel test //...`).
fn builds_whole_tree(tool: &str) -> bool {
    matches!(tool, "bazel" | "pants")
}

/// Two adapters describing the same thing must not both show: a `package.json` script and an
/// Nx target both called `test`, an inferred Expo `start` next to a declared one, or `ios`
/// (script) and `run-ios` (Nx) that explain identically. Declared beats inferred; among
/// equals the earlier adapter (priority order) wins, except that a whole-tree build system
/// wins over a language convention. A declared script only shadows an inferred action it can
/// stand in for (`stands_in_for`): a Rails app's `yarn test` does not hide `bundle exec
/// rspec`. Same-tool variants (`test:watch`, `test:update`) are never touched: a project that
/// lists them meant them.
fn drop_duplicates(actions: &mut [Action]) {
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
        if actions[i].redundant {
            continue;
        }
        for j in 0..i {
            if actions[j].redundant {
                continue;
            }
            let (a, b) = (&actions[j], &actions[i]);
            // Which one would stay: declared beats inferred; among equals the earlier adapter,
            // unless the later one builds the whole tree.
            let later_wins = match (a.source, b.source) {
                (ActionSource::Inferred, ActionSource::Declared) => true,
                (ActionSource::Inferred, ActionSource::Inferred) => {
                    builds_whole_tree(b.tool) && !builds_whole_tree(a.tool)
                }
                _ => false,
            };
            let (keeper, other) = if later_wins { (b, a) } else { (a, b) };
            let same_name = a.name == b.name && stands_in_for(keeper.tool, other.tool);
            let same_meaning = a.tool != b.tool
                && !variant[i]
                && !variant[j]
                && a.category == b.category
                && a.description.eq_ignore_ascii_case(&b.description);
            // A declared script whose body is exactly the command inferred next to it
            // (`"services": "docker compose -f docker/compose.yml up -d"` and the Compose
            // adapter's own `docker compose -f docker/compose.yml up -d`): one command, twice.
            let same_command = a.source != b.source && {
                let (d, i) = if a.source == ActionSource::Declared {
                    (a, b)
                } else {
                    (b, a)
                };
                let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
                d.raw
                    .as_deref()
                    .is_some_and(|raw| words(raw) == words(&i.command))
            };
            if !(same_name || same_meaning || same_command) {
                continue;
            }
            match (a.source, b.source) {
                (ActionSource::Inferred, ActionSource::Declared) => {
                    actions[j].redundant = true;
                }
                (ActionSource::Declared, ActionSource::Declared)
                    if same_name && !same_meaning && a.tool != b.tool =>
                {
                    // Same name, different explanation, different tools (`make test` vs
                    // `npm test`): both are real and may differ. Keep both.
                }
                (ActionSource::Inferred, ActionSource::Inferred)
                    if builds_whole_tree(b.tool) && !builds_whole_tree(a.tool) =>
                {
                    actions[j].redundant = true;
                }
                _ => {
                    actions[i].redundant = true;
                    break;
                }
            }
        }
    }
}

/// A workspace root already offers `test`, `build`, `check`, … for every member; the same
/// inferred action repeated per member (`tokio-util:test`, `api:clippy`) adds nothing.
/// Drop an inferred, non-Development action of a nested project when the nearest ancestor
/// project shows one with the same name and tool. Declared scripts are never touched, and
/// `run`/`dev`-style actions stay because each app's is its own.
fn drop_workspace_fan_out(projects: &mut [Project]) {
    let offered: HashMap<&str, HashSet<(&str, &'static str)>> = projects
        .iter()
        .map(|p| {
            (
                p.path.as_str(),
                // Redundant ones count too: a root `test` dropped for a declared twin still
                // means the workspace offers it.
                p.actions
                    .iter()
                    .map(|a| (a.name.as_str(), a.tool))
                    .collect(),
            )
        })
        .collect();
    let mut hide: Vec<(usize, usize)> = Vec::new();
    for (pi, p) in projects.iter().enumerate().filter(|(_, p)| !p.is_root()) {
        let mut rel = repo::parent_rel(&p.path);
        let mut ancestor = None;
        while let Some(r) = rel {
            if let Some(set) = offered.get(r) {
                ancestor = Some(set);
                break;
            }
            rel = repo::parent_rel(r);
        }
        let Some(set) = ancestor else { continue };
        for (ai, a) in p.actions.iter().enumerate() {
            if a.source == ActionSource::Inferred
                && a.category != Category::Development
                && set.contains(&(a.name.as_str(), a.tool))
            {
                hide.push((pi, ai));
            }
        }
    }
    for (pi, ai) in hide {
        projects[pi].actions[ai].redundant = true;
    }
}

/// A script that five or more nested packages declare with the same explanation (`compile`,
/// `test`, `lint` in every package of a pnpm monorepo) is a workspace convention. Show it once
/// at the root as "run in all packages" and drop the per-package copies. Only JavaScript
/// package managers have a single command for that.
fn lift_repeated_package_scripts(projects: &mut [Project]) {
    const MIN_REPEATS: usize = 5;
    // Keys in first-appearance order: the lifted actions must come out in the order the
    // packages declare them, never in hash order, so the output is stable run to run.
    let mut seen: HashMap<(String, &'static str, String), usize> = HashMap::new();
    let mut order: Vec<(String, &'static str, String)> = Vec::new();
    for p in projects.iter().filter(|p| !p.is_root()) {
        for a in p
            .actions
            .iter()
            .filter(|a| a.source == ActionSource::Declared)
        {
            let key = (a.name.clone(), a.tool, a.description.to_ascii_lowercase());
            let n = seen.entry(key.clone()).or_default();
            if *n == 0 {
                order.push(key);
            }
            *n += 1;
        }
    }
    let repeated: Vec<(String, &'static str, String)> = order
        .into_iter()
        .filter(|k| seen[k] >= MIN_REPEATS && matches!(k.1, "npm" | "pnpm" | "yarn" | "bun"))
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
                    a.redundant = true;
                }
            }
        }
        let root = &projects[0];
        if root.actions.iter().any(|a| a.name == *name && !a.redundant) {
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
/// useful list; the inferred `test`/`lint`/`build`/… that many of them have by convention is
/// boilerplate (`cargo test` in 200 crates), so it is dropped. Only the repetition goes: a
/// command whose tool and name appear in fewer than `REPEATED` nested projects is that
/// project's own (the one Pulumi stack's `pulumi up`, the Phoenix backend's `mix test`).
fn drop_convention_in_large_repos(projects: &mut [Project]) {
    const MANY: usize = 12;
    const REPEATED: usize = 3;
    let nested = projects
        .iter()
        .filter(|p| !p.is_root() && !p.actions.is_empty())
        .count();
    if nested <= MANY {
        return;
    }
    let convention =
        |a: &Action| a.source == ActionSource::Inferred && a.category != Category::Development;
    let mut seen: HashMap<(&'static str, String), usize> = HashMap::new();
    for p in projects.iter().filter(|p| !p.is_root()) {
        let mut own: Vec<(&'static str, String)> = p
            .actions
            .iter()
            .filter(|a| convention(a))
            .map(|a| (a.tool, a.name.clone()))
            .collect();
        own.sort_unstable();
        own.dedup();
        for k in own {
            *seen.entry(k).or_default() += 1;
        }
    }
    for p in projects.iter_mut().filter(|p| !p.is_root()) {
        for a in &mut p.actions {
            if convention(a) && seen[&(a.tool, a.name.clone())] >= REPEATED {
                a.redundant = true;
            }
        }
    }
}

/// A command a developer can type: no control characters beyond line breaks and tabs. A
/// Makefile saved with terminal colours (`make \x1b[38;2;166;226;46mall`) parses into targets
/// that no one could run; they are not commands, so they are not reported.
fn typeable(command: &str) -> bool {
    !command
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\t' | '\r'))
}

/// Remove every action marked redundant, then every nested project left with nothing to
/// show (no action, no declared version). rhow shows all it reports, so what one tool or one
/// ancestor already covers is not reported at all.
fn drop_redundant(projects: &mut Vec<Project>) {
    for p in projects.iter_mut() {
        p.actions.retain(|a| !a.redundant);
    }
    projects.retain(|p| p.is_root() || !p.actions.is_empty() || !p.versions.is_empty());
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
    // Which tool holds each id, to judge whether losing it makes an action redundant.
    let mut holders: HashMap<String, &'static str> = HashMap::new();
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
            let naturals = if collapsible { 2 } else { 1 };
            // Losing the natural id makes an inferred action redundant only when the holder can
            // stand in for it: `yarn test` holding `test` does not make
            // `bundle exec rspec` redundant.
            let shadowed = candidates[..naturals.min(candidates.len())]
                .iter()
                .any(|c| {
                    holders
                        .get(c)
                        .is_some_and(|tool| stands_in_for(tool, a.tool))
                });

            for (i, c) in candidates.into_iter().enumerate() {
                if !taken.contains(&c) {
                    let natural = i < naturals;
                    if !natural && a.source == ActionSource::Inferred && shadowed {
                        a.redundant = true;
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
            holders.insert(id.clone(), a.tool);
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

/// Strip surrounding whitespace and one pair of matching quotes (`"…"` or `'…'`), the way
/// a `desc "…"` / `description = '…'` line in a Rakefile, Fastfile, Gradle script or
/// noxfile is written.
pub fn unquote(s: &str) -> &str {
    let s = s.trim();
    for quote in ['"', '\''] {
        if s.len() >= 2 && s.starts_with(quote) && s.ends_with(quote) {
            return &s[1..s.len() - 1];
        }
    }
    s
}

/// The id prefix for one of several helper directories attached to the same project
/// (`docker/`, `deploy/dev/`, `infra/envs/prod/`). Derived from the directory's path below
/// the project, never from a file's content: the shortest trailing run of path segments
/// that no other attached directory shares, joined with `-`. `infra/envs/dev` alone among
/// the group is `dev`; next to `deploy/dev` the two become `infra-dev` and `deploy-dev`.
pub fn attach_prefix(ctx: &Context, base: &DirInfo, dir: &DirInfo, group: &[&DirInfo]) -> String {
    let segs = |d: &DirInfo| -> Vec<String> {
        ctx.rel_from(base, d)
            .split('/')
            .map(|s| s.to_string())
            .collect()
    };
    let mine = segs(dir);
    let others: Vec<Vec<String>> = group
        .iter()
        .filter(|d| d.rel != dir.rel && d.rel != base.rel)
        .map(|d| segs(d))
        .collect();
    for n in 1..=mine.len() {
        let tail = &mine[mine.len() - n..];
        let clash = others
            .iter()
            .any(|o| o.len() >= n && &o[o.len() - n..] == tail);
        if !clash {
            return tail.join("-");
        }
    }
    mine.join("-")
}

/// `path` (relative to `dir`, `/` or `\\` separated, e.g. `src/Api/Api.csproj`) as text,
/// through the scan and the cache: `None` when the directory or the file is not there.
pub fn text_at(ctx: &Context, dir: &DirInfo, path: &str) -> Option<Rc<str>> {
    let path = path.replace('\\', "/");
    let path = path.trim_start_matches("./");
    match path.rsplit_once('/') {
        None => ctx.text(dir, path),
        Some((sub, file)) => ctx.text(ctx.child(dir, sub)?, file),
    }
}

/// The version pinned for `tool` (asdf/mise naming: `nodejs`, `python`, `ruby`, `golang`,
/// `terraform`) in the nearest `.tool-versions` at `dir` or above it, with the path of the
/// file it came from relative to `dir`.
pub fn tool_version(ctx: &Context, dir: &DirInfo, tool: &str) -> Option<(String, String)> {
    for a in ctx.ancestors(dir) {
        let Some(text) = ctx.text(a, ".tool-versions") else {
            continue;
        };
        for l in text.lines() {
            let l = l.split('#').next().unwrap_or("").trim();
            let mut parts = l.split_whitespace();
            if parts.next() == Some(tool) {
                if let Some(v) = parts.next() {
                    // The source is the file's path from the repository root, so a pin
                    // inherited from a parent directory is not mistaken for a local one.
                    let source = if a.rel == "." {
                        ".tool-versions".to_string()
                    } else {
                        format!("{}/.tool-versions", a.rel)
                    };
                    return Some((v.to_string(), source));
                }
            }
        }
    }
    None
}

/// Scan a fixture repository under `fixtures/` for an adapter's own unit tests; the caller
/// builds a [`Context`] over the returned listing.
#[cfg(test)]
pub(crate) fn fixture_dirs(name: &str) -> (std::path::PathBuf, Vec<DirInfo>) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    let dirs = repo::scan(&root);
    (root, dirs)
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
            techs: vec![],
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
    fn unquote_strips_one_matching_pair() {
        assert_eq!(unquote("  \"Run it\" "), "Run it");
        assert_eq!(unquote("'Run it'"), "Run it");
        assert_eq!(unquote("\"mixed'"), "\"mixed'");
        assert_eq!(unquote("plain"), "plain");
    }

    fn dir(rel: &str) -> DirInfo {
        DirInfo {
            path: std::path::PathBuf::from(rel),
            rel: rel.to_string(),
            depth: if rel == "." {
                0
            } else {
                rel.matches('/').count() + 1
            },
            files: vec![],
            dirs: vec![],
            ignored: false,
        }
    }

    #[test]
    fn attach_prefix_is_the_shortest_unique_path_tail() {
        let dirs = vec![
            dir("."),
            dir("env"),
            dir("env/staging"),
            dir("infra"),
            dir("infra/envs"),
            dir("infra/envs/dev"),
            dir("stacks"),
            dir("stacks/dev"),
        ];
        let root = Path::new(".");
        let ctx = Context::new(root, &dirs, "linux");
        let base = &dirs[0];
        let group: Vec<&DirInfo> = vec![&dirs[2], &dirs[5], &dirs[7]];
        assert_eq!(attach_prefix(&ctx, base, group[0], &group), "staging");
        assert_eq!(attach_prefix(&ctx, base, group[1], &group), "envs-dev");
        assert_eq!(attach_prefix(&ctx, base, group[2], &group), "stacks-dev");
        // Alone in its group a directory keeps its own name.
        assert_eq!(attach_prefix(&ctx, base, group[1], &group[1..2]), "dev");
    }

    #[test]
    fn commands_with_control_characters_are_not_typeable() {
        assert!(typeable("make all"));
        assert!(typeable("set -e\n\tmake all"));
        assert!(!typeable("make \u{1b}[38;2;166;226;46mall\u{1b}[0m"));
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
                    Action::new("test", "yarn test")
                        .tool("yarn")
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
                "api:yarn:test",
                "services/api:test"
            ]
        );
        // An inferred action that lost its natural id to one that can stand in for it (same
        // ecosystem) is redundant; `npm run test` says nothing about `cargo test`.
        assert!(!ps[1].actions[0].redundant);
        assert!(!ps[1].actions[2].redundant);
        assert!(ps[1].actions[3].redundant);
    }
}
