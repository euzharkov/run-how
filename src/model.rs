//! The normalised model every ecosystem adapter produces.
//!
//! Nothing ecosystem-specific (npm scripts, Make targets, Cargo aliases…) leaks out of the
//! adapters: everything becomes an [`Action`] inside a [`Project`] inside a [`Repo`].

use serde::Serialize;
use std::path::PathBuf;

/// Whether the project explicitly declared the action or `rhow` inferred it from conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionSource {
    Declared,
    Inferred,
}

/// How sure we are the action exists and works as described.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Exact,
    High,
    Medium,
    Low,
}

/// Side-effect classification. Ordered so that `max()` picks the most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    /// Local, reversible, or read-only.
    Safe,
    /// Talks to something outside the working copy (registry, cluster, cloud, remote git).
    External,
    /// Deletes data or state that cannot be trivially recovered.
    Destructive,
}

impl Risk {
    pub fn label(self) -> &'static str {
        match self {
            Risk::Safe => "safe",
            Risk::External => "external",
            Risk::Destructive => "destructive",
        }
    }
}

/// A practical note about what running the command involves. Derived from the command text
/// alone, so only facts the text states: it never guesses at what code does at run time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Note {
    /// Keeps running until stopped: a dev server, a watcher, following logs.
    LongRunning,
    /// Needs a phone, tablet, simulator or emulator attached.
    Device,
    /// Downloads something (packages, images, providers) without changing anything remote.
    Download,
    /// Changes the git working tree, history or tags.
    Git,
}

impl Note {
    /// A single-width glyph that every default monospace font on macOS, Linux and Windows
    /// draws: only Mathematical Operators (U+22xx), basic Arrows (U+219x) and the common
    /// Geometric Shapes (●■▲◆) are safe. Blocks like Miscellaneous Technical, OCR or the
    /// less common Geometric Shapes (`▯`, `◷`) show as boxes in some fonts, and emoji are
    /// double width and break the columns.
    pub fn icon(self) -> &'static str {
        match self {
            Note::LongRunning => "∞",
            Note::Device => "⊙",
            Note::Download => "↓",
            Note::Git => "∆",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Note::LongRunning => "long-running",
            Note::Device => "device",
            Note::Download => "download",
            Note::Git => "git",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Development,
    Testing,
    Quality,
    Build,
    Database,
    Infrastructure,
    Release,
    Other,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Category::Development,
        Category::Testing,
        Category::Quality,
        Category::Build,
        Category::Database,
        Category::Infrastructure,
        Category::Release,
        Category::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Development => "Development",
            Category::Testing => "Testing",
            Category::Quality => "Quality",
            Category::Build => "Build",
            Category::Database => "Database",
            Category::Infrastructure => "Infrastructure",
            Category::Release => "Release",
            Category::Other => "Other",
        }
    }
}

/// Coarse project kind — the "primary" ecosystem of a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectKind {
    Root,
    JavaScript,
    Python,
    Go,
    Rust,
    Ruby,
    Jvm,
    Php,
    Deno,
    Mobile,
    Workspace,
    Bazel,
    Infrastructure,
    Other,
    DotNetSolution,
    DotNet,
    Make,
    Just,
    Taskfile,
    Docker,
    Kubernetes,
    Scripts,
}

#[derive(Debug, Clone, Serialize)]
pub struct Action {
    /// Stable identifier used on the command line (`test`, `api:test`, `k8s:apply`).
    pub id: String,
    /// The action's own name inside its project (`test`).
    pub name: String,
    /// Short plain-English description (target: under ~60 characters).
    pub description: String,
    /// The native command the project would run for this action (shown, never executed).
    pub command: String,
    /// Directory the command runs in, relative to the repository root (`.` for the root).
    pub working_directory: String,
    pub source: ActionSource,
    pub confidence: Confidence,
    pub risk: Risk,
    pub category: Category,
    /// Which tool family produced the action (`npm`, `make`, `cargo`, `compose`, …).
    pub tool: &'static str,
    /// Internal / private / lifecycle actions that only show with `--all`.
    pub hidden: bool,
    /// What running it involves (long-running, device, download, git). See [`Note`].
    pub notes: Vec<Note>,
    /// The command text to analyse when it differs from `command`
    /// (e.g. the npm script body behind `npm run test`).
    #[serde(skip)]
    pub raw: Option<String>,
    /// Explicit description supplied by the project (Just comments, Taskfile `desc`, …).
    #[serde(skip)]
    pub explicit_description: bool,
    /// Analysis recognised nothing in the command (or only `echo`): the description is a
    /// "Run <program>" fallback that explains nothing, and the listing leaves it out. Kept
    /// in JSON so a consumer can leave it out too.
    pub opaque: bool,
}

impl Action {
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        let name = name.into();
        Action {
            id: name.clone(),
            name,
            description: String::new(),
            command: command.into(),
            working_directory: String::new(),
            source: ActionSource::Declared,
            confidence: Confidence::Exact,
            risk: Risk::Safe,
            category: Category::Other,
            tool: "",
            hidden: false,
            notes: Vec::new(),
            raw: None,
            explicit_description: false,
            opaque: false,
        }
    }

    /// A description declared by the project itself; takes precedence over analysis.
    pub fn desc(mut self, d: impl Into<String>) -> Self {
        let d = d.into();
        if !d.trim().is_empty() {
            self.description = crate::explain::tidy(&d);
            self.explicit_description = true;
        }
        self
    }

    /// A description produced by the adapter from observable facts (not project-declared).
    pub fn inferred_desc(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn inferred(mut self, c: Confidence) -> Self {
        self.source = ActionSource::Inferred;
        self.confidence = c;
        self
    }

    pub fn confidence(mut self, c: Confidence) -> Self {
        self.confidence = c;
        self
    }

    pub fn cat(mut self, c: Category) -> Self {
        self.category = c;
        self
    }

    pub fn risk(mut self, r: Risk) -> Self {
        self.risk = r;
        self
    }

    pub fn tool(mut self, t: &'static str) -> Self {
        self.tool = t;
        self
    }

    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    pub fn raw(mut self, r: impl Into<String>) -> Self {
        let r = r.into();
        if !r.trim().is_empty() {
            self.raw = Some(r);
        }
        self
    }

    pub fn cwd(mut self, p: impl Into<String>) -> Self {
        self.working_directory = p.into();
        self
    }

    /// The text that command analysis should look at.
    pub fn analysis_text(&self) -> &str {
        self.raw.as_deref().unwrap_or(&self.command)
    }

    /// Shown by default (without `--all`)?
    pub fn is_primary(&self) -> bool {
        !self.hidden && self.confidence != Confidence::Low
    }
}

/// A version or schema value a project declares for itself (a Cargo edition, a `go.mod`
/// directive, a Taskfile schema, a `required_version` constraint, …), read from a file the
/// project's own adapter already parses. This is what `rhow support` compares against the
/// [`crate::support`] registry to tell a still-current config from one that has moved past
/// what this build of `rhow` has been verified against.
#[derive(Debug, Clone, Serialize)]
pub struct ToolVersion {
    /// Matches a [`crate::support::Entry::id`].
    pub tool: &'static str,
    /// The declared value, exactly as written (`"2021"`, `"1.22"`, `">= 1.9"`, `"net9.0"`, …).
    pub value: String,
    /// The file it was read from, relative to the project.
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    /// The directory name (`owner` for `apps/owner`); the repository name for the root.
    pub name: String,
    /// Path relative to the repository root, `.` for the root project.
    pub path: String,
    pub kind: ProjectKind,
    /// Every tool family that contributed actions (`npm`, `make`, `compose`, …).
    pub tools: Vec<&'static str>,
    pub actions: Vec<Action>,
    /// Versions/schemas this project declares for itself, for `rhow support` to check.
    pub versions: Vec<ToolVersion>,
}

impl Project {
    pub fn is_root(&self) -> bool {
        self.path == "."
    }
}

/// One `run` step of a CI job, explained like an action.
#[derive(Debug, Clone, Serialize)]
pub struct CiStep {
    /// The step's `name`, when it has one.
    pub name: Option<String>,
    /// The shell text exactly as written (may span several lines).
    pub command: String,
    pub description: String,
    pub risk: Risk,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CiJob {
    /// The job id (`test`) or its display name when given.
    pub name: String,
    /// `runs-on` / `image` / `stage`, whatever the system uses to say where it runs.
    pub runs_on: Option<String>,
    pub steps: Vec<CiStep>,
}

/// A CI pipeline definition: a GitHub Actions workflow file or the GitLab CI file.
#[derive(Debug, Clone, Serialize)]
pub struct CiPipeline {
    /// `github-actions` or `gitlab-ci`.
    pub system: &'static str,
    /// Path relative to the repository root.
    pub file: String,
    pub name: String,
    /// What starts it (`push main`, `pull_request`, `schedule`).
    pub triggers: Vec<String>,
    pub jobs: Vec<CiJob>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Repo {
    pub root: PathBuf,
    pub name: String,
    pub projects: Vec<Project>,
    /// Environment suggestions that are not project tasks (e.g. `colima start`).
    pub suggestions: Vec<Action>,
    /// CI pipelines found in the repository, as a structure of jobs and run steps.
    pub ci: Vec<CiPipeline>,
}

impl Repo {
    pub fn all_actions(&self) -> impl Iterator<Item = (&Project, &Action)> {
        self.projects
            .iter()
            .flat_map(|p| p.actions.iter().map(move |a| (p, a)))
    }

    pub fn find(&self, id: &str) -> Option<&Action> {
        self.projects
            .iter()
            .flat_map(|p| p.actions.iter())
            .chain(self.suggestions.iter())
            .find(|a| a.id == id)
    }
}
