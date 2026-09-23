//! Snapshot tests over the terminal renderers: the listing (default and `--all`), `--ci`,
//! `why`, and `rhow support`, each on a handful of fixtures with colour off, plus one
//! colour-on listing that pins the glyph and bracket fallback rules (AGENTS.md §7).
//!
//! `tests/fixtures.rs` pins the model; these pin what a developer actually sees.

use rhow::ui::{self, RenderOptions, Style};
use rhow::{discover, Options, Repo};
use std::path::PathBuf;

fn load(name: &str) -> Repo {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    discover(
        &root,
        &Options {
            probe_runtime: false,
            host_os: "linux",
        },
    )
}

fn plain() -> Style {
    Style {
        color: false,
        tty: false,
    }
}

fn snap(name: &str, out: String) {
    insta::with_settings!({ snapshot_path => "snapshots/render", prepend_module_to_snapshot => false }, {
        insta::assert_snapshot!(name, out);
    });
}

fn render_all(fixture: &str, why: [&str; 2]) {
    let repo = load(fixture);
    let style = plain();
    for all in [false, true] {
        let out = ui::render(&repo, &style, &RenderOptions { all, group: false });
        snap(
            &format!("{fixture}-list{}", if all { "-all" } else { "" }),
            out,
        );
    }
    snap(&format!("{fixture}-ci"), ui::render_ci(&repo, &style));
    for id in why {
        let out = ui::render_why(&repo, id, &style)
            .unwrap_or_else(|| panic!("{fixture}: no action `{id}`"));
        snap(&format!("{fixture}-why-{}", id.replace(':', "-")), out);
    }
    snap(
        &format!("{fixture}-support"),
        ui::support::render(Some(&repo), &style),
    );
}

#[test]
fn polyglot() {
    render_all("polyglot", ["dev", "reset"]);
}

#[test]
fn noise() {
    render_all("noise", ["lint", "test"]);
}

#[test]
fn nx() {
    render_all("nx", ["test", "api:build"]);
}

#[test]
fn ci() {
    render_all("ci", ["test", "deploy"]);
}

#[test]
fn ruby_rails() {
    render_all("ruby-rails", ["import", "db:reset"]);
}

/// With colour on, risk is a coloured dot and a word, notes are dimmed glyphs, and the
/// footer appears (a TTY); with colour off the same lines carry `[external]` and the label
/// text only. The escape codes are part of the snapshot on purpose.
#[test]
fn polyglot_with_colour() {
    let repo = load("polyglot");
    let style = Style {
        color: true,
        tty: true,
    };
    snap(
        "polyglot-list-colour",
        ui::render(
            &repo,
            &style,
            &RenderOptions {
                all: false,
                group: false,
            },
        ),
    );
    snap(
        "polyglot-why-reset-colour",
        ui::render_why(&repo, "reset", &style).unwrap(),
    );
}

/// `--group` orders each project's commands by type; pinned once, on the fixture with the
/// most projects.
#[test]
fn polyglot_grouped() {
    let repo = load("polyglot");
    snap(
        "polyglot-list-group",
        ui::render(
            &repo,
            &plain(),
            &RenderOptions {
                all: false,
                group: true,
            },
        ),
    );
}
