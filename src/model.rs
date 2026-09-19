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
    /// The native command that `rhow <id>` would execute.
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
    /// The command text to analyse when it differs from `command`
    /// (e.g. the npm script body behind `npm run test`).
    #[serde(skip)]
    pub raw: Option<String>,
    /// Explicit description supplied by the project (Just comments, Taskfile `desc`, …).
    #[serde(skip)]
    pub explicit_description: bool,
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
            raw: None,
            explicit_description: false,
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

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub name: String,
    /// Path relative to the repository root, `.` for the root project.
    pub path: String,
    pub kind: ProjectKind,
    /// Every tool family that contributed actions (`npm`, `make`, `compose`, …).
    pub tools: Vec<&'static str>,
    pub actions: Vec<Action>,
}

impl Project {
    pub fn is_root(&self) -> bool {
        self.path == "."
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Repo {
    pub root: PathBuf,
    pub name: String,
    pub projects: Vec<Project>,
    /// Environment suggestions that are not project tasks (e.g. `colima start`).
    pub suggestions: Vec<Action>,
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
