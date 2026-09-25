//! `tests/integration/repos.txt` names the real products the Docker integration run checks
//! rhow against, each pinned to a commit whose expected default view is stored under
//! `tests/integration/expected/`. README.md lists them between two marker comments; the list
//! must be the exact rendering of that file, so the README never claims a product that is
//! not tested, and every row must carry its commit and expected output. `must.txt` holds the
//! hand-picked commands each product's default view must keep; it may only name listed
//! products, and every product needs at least one, or a `-` row saying why it has none yet.

const REPOS: &str = include_str!("integration/repos.txt");
const MUST: &str = include_str!("integration/must.txt");
const README: &str = include_str!("../README.md");

fn stack_name(code: &str) -> &str {
    match code {
        "node" => "Node.js (npm)",
        "pnpm" => "pnpm",
        "yarn" => "Yarn",
        "bun" => "Bun",
        "nx" => "Nx",
        "turbo" => "Turborepo",
        "deno" => "Deno",
        "expo" => "Expo / React Native",
        "rust" => "Rust / Cargo",
        "go" => "Go",
        "python" => "Python",
        "django" => "Django",
        "ruby" => "Ruby / Rails",
        "php" => "PHP / Laravel",
        "dotnet" => ".NET",
        "gradle" => "Gradle",
        "maven" => "Maven",
        "bazel" => "Bazel",
        "terraform" => "Terraform",
        "kubernetes" => "Kubernetes",
        "compose" => "Docker Compose",
        "pulumi" => "Pulumi",
        "ansible" => "Ansible",
        "taskfile" => "Taskfile",
        "just" => "Just",
        "make" => "Make",
        "swift" => "Swift / Xcode",
        "flutter" => "Flutter",
        "elixir" => "Elixir / Mix",
        "zig" => "Zig",
        "cmake" => "CMake",
        "clojure" => "Clojure",
        "haskell" => "Haskell",
        other => {
            panic!("repos.txt uses stack `{other}` with no display name in tests/real_repos.rs")
        }
    }
}

fn markdown_table() -> String {
    let mut stacks: Vec<(&str, Vec<&str>)> = Vec::new();
    for line in REPOS
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let mut fields = line.split_whitespace();
        let (stack, slug) = (fields.next().unwrap(), fields.next().unwrap());
        match stacks.iter_mut().find(|(s, _)| *s == stack) {
            Some((_, slugs)) => slugs.push(slug),
            None => stacks.push((stack, vec![slug])),
        }
    }
    let mut out = String::from("| Stack | Products |\n|---|---|\n");
    for (stack, slugs) in stacks {
        let links: Vec<String> = slugs
            .iter()
            .map(|s| format!("[{s}](https://github.com/{s})"))
            .collect();
        out.push_str(&format!(
            "| {} | {} |\n",
            stack_name(stack),
            links.join(", ")
        ));
    }
    out
}

#[test]
fn every_product_is_pinned_and_has_expected_output() {
    for line in REPOS
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let fields: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(
            fields.len(),
            3,
            "repos.txt row needs <stack> <owner/repo> <commit>: {line}"
        );
        let commit = fields[2];
        assert!(
            commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit()),
            "{}: `{commit}` is not a full commit sha",
            fields[1]
        );
        let expected = format!(
            "{}/tests/integration/expected/{}.txt",
            env!("CARGO_MANIFEST_DIR"),
            fields[1].replace('/', "__")
        );
        assert!(
            std::path::Path::new(&expected).is_file(),
            "{}: no expected output; run the integration image with UPDATE_EXPECTED=1",
            fields[1]
        );
    }
}

fn rows(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
}

fn slugs() -> Vec<&'static str> {
    rows(REPOS)
        .filter_map(|l| l.split_whitespace().nth(1))
        .collect()
}

#[test]
fn every_expected_output_belongs_to_a_product() {
    let dir = format!("{}/tests/integration/expected", env!("CARGO_MANIFEST_DIR"));
    let listed: Vec<String> = slugs().iter().map(|s| s.replace('/', "__")).collect();
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let stem = name.strip_suffix(".txt").unwrap_or(&name);
        assert!(
            listed.iter().any(|s| s == stem),
            "tests/integration/expected/{name} has no product in repos.txt; delete it"
        );
    }
}

#[test]
fn every_product_has_must_have_commands() {
    let listed = slugs();
    let mut covered = std::collections::HashSet::new();
    for line in rows(MUST) {
        // <owner/repo> <project path> <command…>, columns separated by any whitespace.
        let field = |s: &'static str| {
            let s = s.trim_start();
            let end = s.find(char::is_whitespace).unwrap_or(s.len());
            (&s[..end], &s[end..])
        };
        let (slug, rest) = field(line);
        let (path, command) = field(rest);
        assert!(
            !path.is_empty() && !command.trim().is_empty(),
            "must.txt row needs <owner/repo> <project path> <command>: {line}"
        );
        assert!(
            listed.contains(&slug),
            "must.txt names {slug}, which repos.txt does not list"
        );
        covered.insert(slug);
    }
    let bare: Vec<&str> = listed
        .into_iter()
        .filter(|s| !covered.contains(s))
        .collect();
    assert!(
        bare.is_empty(),
        "no must-have command in must.txt for: {bare:?}"
    );
}

#[test]
fn readme_lists_the_tested_products() {
    let (_, rest) = README
        .split_once("<!-- real-repos:start -->\n")
        .expect("README.md has <!-- real-repos:start -->");
    let (table, _) = rest
        .split_once("<!-- real-repos:end -->")
        .expect("README.md has <!-- real-repos:end -->");
    let expected = markdown_table();
    assert!(
        table == expected,
        "README.md's product list is out of date. Replace the block between the markers with:\n\n{expected}"
    );
}
