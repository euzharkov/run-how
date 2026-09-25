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
        _ => {
            // The header is a run of comment paragraphs. A paragraph with one licensing line
            // is a licence block (the Apache one spans six lines, most of them without the
            // word); a paragraph opening with "This script …" is prose. The synopsis is the
            // first line of the first paragraph that is neither.
            let header: Vec<&str> = lines
                .iter()
                .map(|l| l.trim())
                .skip_while(|l| l.starts_with("#!") || l.is_empty())
                .take_while(|l| l.starts_with('#') || l.is_empty())
                .map(|l| l.trim_start_matches('#').trim())
                .collect();
            for paragraph in header.split(|l| l.is_empty()) {
                let lower: Vec<String> = paragraph.iter().map(|l| l.to_ascii_lowercase()).collect();
                if lower
                    .iter()
                    .any(|l| LICENCE_WORDS.iter().any(|w| l.contains(w)))
                {
                    continue;
                }
                let Some(first) = lower.first() else { continue };
                if first.starts_with("this script") || first.starts_with("this file") {
                    return None;
                }
                // Within a paragraph, provenance lines (`Author:`, `Usage:`) are skipped
                // one by one: the synopsis may share the paragraph with them.
                if let Some(l) = paragraph.iter().zip(&lower).find(|(l, low)| {
                    !l.starts_with('-')
                        && !l.starts_with('=')
                        && !HEADER_NOISE.iter().any(|p| low.starts_with(p))
                }) {
                    return Some(l.0.to_string());
                }
            }
            None
        }
    }
}

/// Any header line about licensing is provenance, wherever the word falls in the sentence
/// (the Apache header spans six lines, only the first of which starts with "Licensed").
const LICENCE_WORDS: &[&str] = &["license", "licence", "copyright", "warrant"];

/// Comment lines that describe the file's provenance, not what it does.
const HEADER_NOISE: &[&str] = &[
    "shellcheck",
    "usage",
    "author",
    "maintainer",
    "spdx-",
    "(c)",
    "©",
];

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

#[cfg(test)]
mod tests {
    use super::synopsis;

    #[test]
    fn synopsis_skips_provenance_and_prose_headers() {
        let licensed = "#!/bin/bash\n# Copyright 2018- Pixie Authors\n# SPDX-License-Identifier: Apache-2.0\n\n# Build the release artifacts.\nset -e\n";
        assert_eq!(
            synopsis(licensed, "sh").as_deref(),
            Some("Build the release artifacts.")
        );
        let prose = "#!/bin/bash\n# This script must be sourced so that credentials are exported in\n# the calling shell.\nexport X=1\n";
        assert_eq!(synopsis(prose, "sh"), None);
        let plain = "#!/bin/sh\n# Rebuild the icon set.\nmake icons\n";
        assert_eq!(
            synopsis(plain, "sh").as_deref(),
            Some("Rebuild the icon set.")
        );
        let see = "#!/bin/sh\n# Copyright (c) 2015-present Mattermost, Inc. All Rights Reserved\n# See LICENSE.txt for license information\nmake icons\n";
        assert_eq!(synopsis(see, "sh"), None);
        let apache = "#!/bin/sh\n# Licensed to the Apache Software Foundation (ASF) under one or more\n# contributor license agreements.  See the NOTICE file distributed with\n# this work for additional information regarding copyright ownership.\n# The ASF licenses this file to You under the Apache License, Version 2.0\n#\n# Unless required by applicable law or agreed to in writing, software\n# distributed under the License is distributed on an AS IS BASIS.\n\n# Start the daemon.\nstart\n";
        assert_eq!(synopsis(apache, "sh").as_deref(), Some("Start the daemon."));
        let with_usage = "#!/bin/sh\n# Script to recursively copy terragrunt.hcl files\n# Usage: ./propagate.sh <from> <to>\n# Author: someone\nset -e\n";
        assert_eq!(
            synopsis(with_usage, "sh").as_deref(),
            Some("Script to recursively copy terragrunt.hcl files")
        );
        let usage_first = "#!/bin/sh\n# Usage: ./x.sh <from>\n# Copy files between trees\nset -e\n";
        assert_eq!(
            synopsis(usage_first, "sh").as_deref(),
            Some("Copy files between trees")
        );
        let only_licence = "#!/bin/sh\n# Licensed under MIT\nmake icons\n";
        assert_eq!(synopsis(only_licence, "sh"), None);
    }
}
