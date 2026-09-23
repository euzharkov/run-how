//! Repository root detection and read-only directory scanning.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Directories that never contain project definitions worth showing.
pub const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    "target",
    "dist",
    "build",
    "out",
    "coverage",
    "bin",
    "obj",
    "venv",
    "env",
    "__pycache__",
    "site-packages",
    "storybook-static",
    "Pods",
    "DerivedData",
    "bower_components",
    "jspm_packages",
    "testdata",
    // Test inputs, never projects in their own right.
    "fixtures",
    "__fixtures__",
    "test-fixtures",
    "__snapshots__",
    "__mocks__",
];

/// Directory names whose contents are demonstrations or test subjects rather than the
/// project itself (`examples/`, `test/apps/`, `playground/`). Projects found below them are
/// still discovered, but everything they offer is low confidence: shown with `--all`, not by
/// default, and they never claim helper directories (Docker, Kubernetes, Terraform).
pub fn is_demoted_segment(name: &str) -> bool {
    matches!(
        name,
        "test"
            | "tests"
            | "__tests__"
            | "spec"
            | "specs"
            | "e2e"
            | "example"
            | "examples"
            | "sample"
            | "samples"
            | "demo"
            | "demos"
            | "playground"
            | "playgrounds"
            | "bench"
            | "benches"
            | "benchmarks"
            | "template"
            | "templates"
    ) || name.ends_with("-tests")
        || name.ends_with("_tests")
        || name.ends_with("-fixtures")
        || name.ends_with("_fixtures")
}

/// Is any segment of a relative path (not the root itself) a demoted directory?
pub fn is_demoted_path(rel: &str) -> bool {
    rel != "." && rel.split('/').any(is_demoted_segment)
}

/// Generated platform runners inside a mobile app (`android/`, `ios/`, `macos/`, `linux/`,
/// `windows/`, `web/` next to a `pubspec.yaml` or an Expo/React Native app config). They
/// carry Gradle, Xcode and CMake projects of their own that are not what a developer runs.
pub const PLATFORM_DIRS: &[&str] = &["android", "ios", "macos", "linux", "windows", "web"];

pub fn is_mobile_app_dir(dir: &DirInfo) -> bool {
    dir.has("pubspec.yaml")
        || (dir.has("package.json")
            && (dir.has("app.json")
                || dir.files.iter().any(|f| {
                    f.starts_with("app.config.")
                        || f.starts_with("metro.config.")
                        || f.starts_with("react-native.config.")
                })))
}

const MAX_DEPTH: usize = 10;
const MAX_DIRS: usize = 25_000;
/// Files larger than this are never read during discovery.
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// A scanned directory: its listing, cached so adapters never touch the filesystem twice.
#[derive(Debug, Clone)]
pub struct DirInfo {
    pub path: PathBuf,
    /// Path relative to the repository root with `/` separators; `.` for the root.
    pub rel: String,
    pub depth: usize,
    /// Sorted file names (not directories).
    pub files: Vec<String>,
    /// Sorted subdirectory names (including ignored ones, which are not descended into).
    pub dirs: Vec<String>,
}

impl DirInfo {
    pub fn name(&self) -> &str {
        self.path.file_name().and_then(|s| s.to_str()).unwrap_or("")
    }

    pub fn has(&self, file: &str) -> bool {
        self.files.iter().any(|f| f == file)
    }

    pub fn has_any(&self, names: &[&str]) -> bool {
        names.iter().any(|n| self.has(n))
    }

    pub fn has_dir(&self, dir: &str) -> bool {
        self.dirs.iter().any(|d| d == dir)
    }

    /// First existing file among `names`, in the given priority order.
    pub fn first_of<'a>(&self, names: &[&'a str]) -> Option<&'a str> {
        names.iter().copied().find(|n| self.has(n))
    }

    /// File names with the given extension (without the dot), sorted.
    pub fn with_ext(&self, ext: &str) -> Vec<&str> {
        self.files
            .iter()
            .filter(|f| {
                Path::new(f)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case(ext))
                    .unwrap_or(false)
            })
            .map(|s| s.as_str())
            .collect()
    }

    pub fn join(&self, file: &str) -> PathBuf {
        self.path.join(file)
    }

    /// Read a file inside this directory as UTF-8 (lossy). Returns `None` when missing or huge.
    pub fn read(&self, file: &str) -> Option<String> {
        read_text(&self.path.join(file))
    }

    /// Path of `file` inside this directory, relative to another scanned directory `base`,
    /// with `/` separators and no leading `./`.
    pub fn rel_to(&self, base: &DirInfo, file: &str) -> String {
        let dir = if base.rel == "." {
            self.rel.clone()
        } else if self.rel == base.rel {
            ".".to_string()
        } else {
            self.rel
                .strip_prefix(&format!("{}/", base.rel))
                .map(|s| s.to_string())
                .unwrap_or_else(|| self.rel.clone())
        };
        if dir == "." {
            file.to_string()
        } else if file.is_empty() {
            dir
        } else {
            format!("{dir}/{file}")
        }
    }
}

pub fn read_text(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Walk up from `start` to find the nearest `.git` boundary. Falls back to `start`.
pub fn find_root(start: &Path) -> PathBuf {
    let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    for dir in start.ancestors() {
        if dir.join(".git").exists() {
            return dir.to_path_buf();
        }
    }
    start
}

/// Scan the tree below `root`. The root is always element 0; children follow their parents.
pub fn scan(root: &Path) -> Vec<DirInfo> {
    let mut out = Vec::new();
    walk(root, root, 0, &mut out);
    out
}

fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<DirInfo>) {
    if out.len() >= MAX_DIRS {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for entry in rd.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if ft.is_dir() {
            dirs.push(name.to_string());
        } else if ft.is_file() {
            files.push(name.to_string());
        }
        // symlinks are intentionally skipped
    }
    files.sort();
    dirs.sort();
    let rel = if dir == root {
        ".".to_string()
    } else {
        dir.strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default()
    };
    out.push(DirInfo {
        path: dir.to_path_buf(),
        rel,
        depth,
        files,
        dirs: dirs.clone(),
    });
    if depth >= MAX_DEPTH {
        return;
    }
    for d in dirs {
        if is_ignored_dir(&d) {
            continue;
        }
        walk(root, &dir.join(&d), depth + 1, out);
    }
}

pub fn is_ignored_dir(name: &str) -> bool {
    name.starts_with('.') || IGNORED_DIRS.contains(&name)
}

/// Relative path → position in the scanned list, so ancestor walks are O(depth) instead of
/// O(depth × directories).
pub fn index(dirs: &[DirInfo]) -> HashMap<String, usize> {
    dirs.iter()
        .enumerate()
        .map(|(i, d)| (d.rel.clone(), i))
        .collect()
}

/// The parent of a relative path (`a/b` → `a`, `a` → `.`); `None` for the root.
pub fn parent_rel(rel: &str) -> Option<&str> {
    if rel == "." {
        return None;
    }
    Some(match rel.rfind('/') {
        Some(p) => &rel[..p],
        None => ".",
    })
}

/// Index of the nearest ancestor (or self) of `idx` in `dirs` that satisfies `pred`.
pub fn nearest(
    dirs: &[DirInfo],
    index: &HashMap<String, usize>,
    idx: usize,
    pred: impl Fn(usize) -> bool,
) -> Option<usize> {
    let mut rel = dirs[idx].rel.as_str();
    loop {
        if let Some(&i) = index.get(rel) {
            if pred(i) {
                return Some(i);
            }
        }
        rel = parent_rel(rel)?;
    }
}

/// Minimal glob matcher for workspace member patterns (`apps/*`, `packages/**`, `libs/ui`).
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern = pattern
        .trim()
        .trim_start_matches("./")
        .trim_end_matches('/');
    let path = path.trim_start_matches("./");
    if pattern.is_empty() {
        return false;
    }
    let pat: Vec<&str> = pattern.split('/').collect();
    let segs: Vec<&str> = path.split('/').collect();
    fn go(pat: &[&str], segs: &[&str]) -> bool {
        match (pat.first(), segs.first()) {
            (None, None) => true,
            (None, Some(_)) => false,
            (Some(&"**"), _) => (0..=segs.len()).any(|i| go(&pat[1..], &segs[i..])),
            (Some(p), Some(s)) => seg_match(p, s) && go(&pat[1..], &segs[1..]),
            (Some(_), None) => false,
        }
    }
    fn seg_match(p: &str, s: &str) -> bool {
        if p == "*" {
            return true;
        }
        if let Some(prefix) = p.strip_suffix('*') {
            return s.starts_with(prefix);
        }
        if let Some(suffix) = p.strip_prefix('*') {
            return s.ends_with(suffix);
        }
        p == s
    }
    go(&pat, &segs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob_match("apps/*", "apps/web"));
        assert!(!glob_match("apps/*", "apps/web/nested"));
        assert!(glob_match("packages/**", "packages/a/b"));
        assert!(glob_match("packages/**", "packages"));
        assert!(glob_match("libs/ui", "libs/ui"));
        assert!(!glob_match("libs/ui", "libs/uix"));
        assert!(glob_match("services/svc-*", "services/svc-api"));
    }
}
