//! Snapshot tests over the terminal renderers, with colour off unless stated. Each view is
//! pinned only where its output differs from another fixture's: `tests/fixtures.rs` pins the
//! model, these pin what a developer actually sees, and a snapshot that repeats another
//! fixture's rendering proves nothing new.
//!
//! - The default listing, on every fixture.
//! - `--ci`, once with pipelines (ci) and once without (polyglot).
//! - `rhow support`, once with versions detected across nested projects (polyglot) and once
//!   with nothing detected (nx). The static registry header is the same everywhere.
//! - One colour-on listing that pins the glyph and bracket fallback rules (AGENTS.md §7).

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

fn list(fixture: &str) {
    let out = ui::render(&load(fixture), &plain(), &RenderOptions { group: false });
    snap(&format!("{fixture}-list"), out);
}

fn ci(fixture: &str) {
    snap(
        &format!("{fixture}-ci"),
        ui::render_ci(&load(fixture), &plain()),
    );
}

fn support(fixture: &str) {
    snap(
        &format!("{fixture}-support"),
        ui::support::render(Some(&load(fixture)), &plain()),
    );
}

#[test]
fn polyglot() {
    list("polyglot");
    ci("polyglot");
    support("polyglot");
}

#[test]
fn noise() {
    list("noise");
}

#[test]
fn nx() {
    list("nx");
    support("nx");
}

#[test]
fn ci_fixture() {
    list("ci");
    ci("ci");
}

#[test]
fn ruby_rails() {
    list("ruby-rails");
}

/// With colour on, risk is a coloured dot and a word and notes are dimmed glyphs; with
/// colour off the same lines carry `[external]` and the label text only. The escape codes are part of the snapshot on purpose.
#[test]
fn polyglot_with_colour() {
    let repo = load("polyglot");
    let style = Style {
        color: true,
        tty: true,
    };
    snap(
        "polyglot-list-colour",
        ui::render(&repo, &style, &RenderOptions { group: false }),
    );
}

/// `--group` orders each project's commands by type; pinned once, on the fixture with the
/// most projects.
#[test]
fn polyglot_grouped() {
    let repo = load("polyglot");
    snap(
        "polyglot-list-group",
        ui::render(&repo, &plain(), &RenderOptions { group: true }),
    );
}
