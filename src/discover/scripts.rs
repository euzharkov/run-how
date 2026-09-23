//! Project scripts: PowerShell (`*.ps1`), batch (`*.cmd`, `*.bat`) and shell (`*.sh`) files at
//! the project root or in `scripts/`-style directories.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Scripts;

const LIB_NAMES: &[&str] = &[
    "lib",
    "common",
    "utils",
    "util",
    "env",
    "functions",
    "helpers",
    "vars",
    "config",
    "source",
    "include",
];

fn is_script(f: &str) -> bool {
    f.ends_with(".ps1") || f.ends_with(".cmd") || f.ends_with(".bat") || f.ends_with(".sh")
}

fn synopsis(text: &str, ext: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().take(40).collect();
    match ext {
        "ps1" => {
            let i = lines
                .iter()
                .position(|l| l.trim().eq_ignore_ascii_case(".SYNOPSIS"))?;
            lines
                .get(i + 1)
                .map(|l| l.trim().to_string())
                .filter(|s| !s.is_empty())
        }
        "cmd" | "bat" => lines
            .iter()
            .map(|l| l.trim())
            .find_map(|l| {
                let l = l.strip_prefix("@").unwrap_or(l);
                l.strip_prefix("REM ")
                    .or_else(|| l.strip_prefix("rem "))
                    .or_else(|| l.strip_prefix(":: "))
                    .map(|s| s.trim().to_string())
            })
            .filter(|s| !s.is_empty() && !s.to_ascii_lowercase().starts_with("echo off")),
        _ => lines
            .iter()
            .map(|l| l.trim())
            .skip_while(|l| l.starts_with("#!") || l.is_empty())
            .take_while(|l| l.starts_with('#'))
            .map(|l| l.trim_start_matches('#').trim())
            .find(|l| {
                !l.is_empty()
                    && !l.starts_with('-')
                    && !l.starts_with('=')
                    && !l.to_ascii_lowercase().starts_with("shellcheck")
                    && !l.to_ascii_lowercase().starts_with("usage")
            })
            .map(|s| s.to_string()),
    }
}

impl Discoverer for Scripts {
    fn id(&self) -> &'static str {
        "scripts"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Scripts
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.files.iter().any(|f| is_script(f))
    }
    fn attachable(&self) -> bool {
        true
    }
    fn attach_dir_names(&self) -> Option<&'static [&'static str]> {
        // `bin/` is deliberately absent: it is in `repo::IGNORED_DIRS` and never scanned.
        Some(&[
            "scripts", "script", "eng", "tools", "ci", "hack", "sh", "cmd",
        ])
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let windows = ctx.host_os == "windows";
        for dir in dirs {
            let at_base = dir.rel == base.rel;
            for f in dir.files.iter().filter(|f| is_script(f)) {
                let stem = std::path::Path::new(f)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(f);
                let ext = std::path::Path::new(f)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let lower = stem.to_ascii_lowercase();
                if lower.starts_with('_')
                    || LIB_NAMES.contains(&lower.as_str())
                    || lower.ends_with("-lib")
                    || lower.ends_with("_lib")
                    || lower.ends_with(".lib")
                    || lower.starts_with("lib") && ext == "sh"
                {
                    continue;
                }
                let rel = dir.rel_to(base, f);
                let path = if at_base {
                    format!("./{f}")
                } else {
                    format!("./{rel}")
                };
                let command = match ext {
                    "ps1" => {
                        if windows {
                            format!(
                                "powershell -ExecutionPolicy Bypass -File {}",
                                super::q(&path.replace('/', "\\"))
                            )
                        } else {
                            format!("pwsh {}", super::q(&path))
                        }
                    }
                    "cmd" | "bat" => super::q(&path.replace('/', "\\")),
                    _ => super::q(&path),
                };
                let mut conf = Confidence::Medium;
                match ext {
                    "cmd" | "bat" if !windows => conf = Confidence::Low,
                    "sh" if windows => conf = Confidence::Low,
                    _ => {}
                }
                // Twins with the same stem (`build.sh`, `build.ps1`, `build.cmd`, common in
                // .NET repositories) are one action: the host's native one keeps Medium, the
                // others drop to Low. On Windows PowerShell beats batch; elsewhere the shell
                // script beats PowerShell.
                let twin = |e: &str| dir.has(&format!("{stem}.{e}"));
                match ext {
                    "cmd" | "bat" if windows && twin("ps1") => conf = Confidence::Low,
                    "ps1" if !windows && twin("sh") => conf = Confidence::Low,
                    _ => {}
                }
                let name = lower.clone();
                let mut a = Action::new(name, command)
                    .tool(if ext == "ps1" {
                        "pwsh"
                    } else if ext == "sh" {
                        "sh"
                    } else {
                        "cmd"
                    })
                    .inferred(conf);
                let desc = dir.read(f).and_then(|t| synopsis(&t, ext));
                match desc {
                    Some(d) => a = a.desc(d),
                    None => a = a.inferred_desc(format!("Run {}", rel)),
                }
                // Risk from the file name only; contents are not interpreted.
                if lower
                    .split(['-', '_', '.'])
                    .any(|t| t == "deploy" || t == "publish" || t == "release")
                {
                    a = a.risk(Risk::External);
                }
                out.actions.push(a);
            }
        }
        out
    }
}
