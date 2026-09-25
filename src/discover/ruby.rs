//! Ruby: Bundler, Rake tasks with `desc`, Rails (`bin/rails`), RSpec, RuboCop and `bin/dev`.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Ruby;

const MARKERS: &[&str] = &[
    "Gemfile",
    "Rakefile",
    "config.ru",
    "Gemfile.lock",
    ".ruby-version",
];

fn gem_listed(gemfile: &str, name: &str) -> bool {
    gemfile.lines().any(|l| {
        let l = l.trim();
        (l.starts_with("gem ") || l.starts_with("gem(")) && {
            let rest = l[3..].trim_start_matches('(').trim();
            rest.starts_with(&format!("'{name}'")) || rest.starts_with(&format!("\"{name}\""))
        }
    })
}

/// Does this line open a Ruby block that a later `end` closes: a trailing `do` (with or
/// without `|args|`) or a leading `def`/`class`/`module`/`if`/`case`/… keyword.
fn opens_block(l: &str) -> bool {
    let l = l.split('#').next().unwrap_or("").trim_end();
    let before_args = match l.rfind(" do |") {
        Some(i) if l.ends_with('|') => &l[..i + 3],
        _ => l,
    };
    if before_args == "do" || before_args.ends_with(" do") {
        return true;
    }
    let first = l.split_whitespace().next().unwrap_or("");
    matches!(
        first,
        "def" | "class" | "module" | "if" | "unless" | "case" | "while" | "until" | "begin"
    )
}

/// `desc "..."` followed by `task :name` / `task name:` / namespaced blocks. Namespaces are
/// tracked with the `do`/`end` nesting, so a task after a closed inner namespace belongs to
/// the outer one.
pub fn rake_tasks(text: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut pending: Option<String> = None;
    // One entry per open block: `Some(name)` for a namespace, `None` for any other block.
    let mut blocks: Vec<Option<String>> = Vec::new();
    for raw in text.lines() {
        let l = raw.trim();
        if let Some(rest) = l.strip_prefix("desc ") {
            pending = Some(super::unquote(rest).to_string());
            continue;
        }
        if l == "end" || l.starts_with("end ") || l.starts_with("end#") {
            blocks.pop();
            continue;
        }
        if let Some(rest) = l.strip_prefix("namespace ") {
            let ns = rest
                .trim()
                .trim_start_matches(':')
                .trim_matches(|c| c == '"' || c == '\'' || c == ' ')
                .split(' ')
                .next()
                .unwrap_or("")
                .to_string();
            blocks.push(Some(ns));
            continue;
        }
        if let Some(rest) = l.strip_prefix("task ") {
            let rest = rest
                .trim()
                .trim_start_matches(':')
                .trim_start_matches(['"', '\'']);
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':')
                .collect();
            let name = name.trim_end_matches(':').to_string();
            if !name.is_empty() {
                let namespaces: Vec<&str> = blocks.iter().flatten().map(String::as_str).collect();
                let full = if namespaces.is_empty() {
                    name
                } else {
                    format!("{}:{}", namespaces.join(":"), name)
                };
                out.push((full, pending.take()));
            }
        }
        if opens_block(l) {
            blocks.push(None);
        }
    }
    out
}

impl Discoverer for Ruby {
    fn id(&self) -> &'static str {
        "ruby"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Ruby
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has_any(MARKERS)
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let gemfile = base.read("Gemfile").unwrap_or_default();
        let bundler = base.has("Gemfile");
        if let Some(v) = base.read(".ruby-version") {
            out.version("ruby", v.trim(), ".ruby-version");
        } else if let Some((v, src)) = super::tool_version(ctx, base, "ruby") {
            out.version("ruby", v, src);
        }
        let be = |cmd: &str| {
            if bundler {
                format!("bundle exec {cmd}")
            } else {
                cmd.to_string()
            }
        };
        let bin_rails = ctx.has_file(base, "bin/rails");
        let rails = bin_rails || gem_listed(&gemfile, "rails");
        let rspec = base.has_dir("spec")
            || gem_listed(&gemfile, "rspec")
            || gem_listed(&gemfile, "rspec-rails");
        let rubocop = base.has(".rubocop.yml") || gem_listed(&gemfile, "rubocop");

        // Declared Rake tasks (with descriptions): the Rakefile, then Rails-style
        // `lib/tasks/*.rake` files that `load_tasks` pulls in.
        let mut declared: Vec<String> = Vec::new();
        let mut rake_files: Vec<(&DirInfo, String)> = Vec::new();
        if let Some(f) = base.first_of(&["Rakefile", "rakefile", "Rakefile.rb"]) {
            rake_files.push((base, f.to_string()));
        }
        if let Some(tasks_dir) = ctx
            .child(base, "lib")
            .and_then(|lib| ctx.child(lib, "tasks"))
        {
            for f in tasks_dir.with_ext("rake") {
                rake_files.push((tasks_dir, f.to_string()));
            }
        }
        for (dir, f) in rake_files {
            let Some(text) = ctx.text(dir, &f) else {
                continue;
            };
            for (name, desc) in rake_tasks(&text) {
                if declared.contains(&name) {
                    continue;
                }
                // A task built in a Ruby loop (`task "#{framework}:test"`) has no single name
                // to type; keep it out of the default view.
                let templated =
                    name.contains("#{") || desc.as_deref().is_some_and(|d| d.contains("#{"));
                declared.push(name.clone());
                let mut a = Action::new(&name, be(&format!("rake {name}"))).tool("rake");
                match desc {
                    Some(d) => a = a.desc(d),
                    None => a = a.inferred_desc(format!("Run the {name} Rake task")),
                }
                if templated {
                    a = a.confidence(Confidence::Low);
                }
                out.actions.push(a);
            }
        }
        let free = |n: &str| !declared.iter().any(|d| d == n);

        if rails {
            let r = |c: &str| {
                if bin_rails {
                    format!("bin/rails {c}")
                } else {
                    be(&format!("rails {c}"))
                }
            };
            if ctx.has_file(base, "bin/dev") && free("dev") {
                out.actions.push(
                    Action::new("dev", "bin/dev")
                        .tool("rails")
                        .inferred(Confidence::High)
                        .inferred_desc("Start the Rails app and its dev processes")
                        .cat(Category::Development),
                );
                out.actions.push(
                    Action::new("server", r("server"))
                        .tool("rails")
                        .inferred(Confidence::Medium)
                        .cat(Category::Development),
                );
            } else if free("dev") {
                out.actions.push(
                    Action::new("dev", r("server"))
                        .tool("rails")
                        .inferred(Confidence::High)
                        .cat(Category::Development),
                );
            }
            out.actions.push(
                Action::new("console", r("console"))
                    .tool("rails")
                    .inferred(Confidence::High)
                    .cat(Category::Development),
            );
            if free("db:migrate") {
                out.actions.push(
                    Action::new("db:migrate", r("db:migrate"))
                        .tool("rails")
                        .inferred(Confidence::High),
                );
            }
            if ctx.has_file(base, "db/seeds.rb") && free("db:seed") {
                out.actions.push(
                    Action::new("db:seed", r("db:seed"))
                        .tool("rails")
                        .inferred(Confidence::High),
                );
            }
            out.actions.push(
                Action::new("db:prepare", r("db:prepare"))
                    .tool("rails")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Create the database and apply migrations")
                    .cat(Category::Database),
            );
            out.actions.push(
                Action::new("db:reset", r("db:reset"))
                    .tool("rails")
                    .inferred(Confidence::Low),
            );
            out.actions.push(
                Action::new("routes", r("routes"))
                    .tool("rails")
                    .inferred(Confidence::Low)
                    .inferred_desc("List Rails routes")
                    .cat(Category::Other),
            );
            if ctx.has_file(base, "bin/setup") && free("setup") {
                out.actions.push(
                    Action::new("setup", "bin/setup")
                        .tool("rails")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Set up the app (bin/setup)")
                        .cat(Category::Other),
                );
            }
        }
        if free("test") {
            if rspec {
                out.actions.push(
                    Action::new("test", be("rspec"))
                        .tool("ruby")
                        .inferred(Confidence::High),
                );
            } else if rails {
                out.actions.push(
                    Action::new("test", "bin/rails test".to_string())
                        .tool("rails")
                        .inferred(Confidence::High),
                );
                if ctx.has_dir(base, "test/system") {
                    out.actions.push(
                        Action::new("test:system", "bin/rails test:system".to_string())
                            .tool("rails")
                            .inferred(Confidence::High)
                            .inferred_desc("Run Rails system (browser) tests")
                            .cat(Category::Testing),
                    );
                }
            } else if base.has_dir("test") && gem_listed(&gemfile, "minitest") {
                out.actions.push(
                    Action::new("test", be("rake test"))
                        .tool("ruby")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Run Minitest tests")
                        .cat(Category::Testing),
                );
            }
        }
        if rubocop && free("lint") {
            out.actions.push(
                Action::new("lint", be("rubocop"))
                    .tool("ruby")
                    .inferred(Confidence::High),
            );
        }
        if gem_listed(&gemfile, "brakeman") && free("security") {
            out.actions.push(
                Action::new("security", be("brakeman -q"))
                    .tool("ruby")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Scan the Rails app with Brakeman")
                    .cat(Category::Quality),
            );
        }
        if base.has("Gemfile") && !base.with_ext("gemspec").is_empty() {
            out.actions.push(
                Action::new("build:gem", "gem build".to_string())
                    .tool("ruby")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Build the gem")
                    .cat(Category::Build),
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rake() {
        let t = "desc 'Import data'\ntask :import => :environment do\nend\n\nnamespace :db do\n  desc \"Nuke it\"\n  task :nuke do\n  end\nend\ntask default: :spec\n";
        let tasks = rake_tasks(t);
        assert_eq!(tasks[0], ("import".into(), Some("Import data".into())));
        assert_eq!(tasks[1], ("db:nuke".into(), Some("Nuke it".into())));
        assert_eq!(tasks[2].0, "default");
    }

    #[test]
    fn closed_inner_namespace_does_not_leak() {
        let t = "namespace :db do\n  namespace :seed do\n    task :all do\n      if x\n        y\n      end\n    end\n  end\n  desc 'Migrate'\n  task :migrate => :environment do |t, args|\n  end\nend\ntask :top\n";
        let names: Vec<String> = rake_tasks(t).into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["db:seed:all", "db:migrate", "top"]);
    }
}

#[cfg(test)]
mod version_tests {
    use super::*;

    #[test]
    fn ruby_from_a_parent_tool_versions() {
        let (root, dirs) = super::super::fixture_dirs("version-sources");
        let ctx = Context::new(&root, &dirs, "linux");
        let rb = ctx.dir_at("rb").unwrap();
        let d = Ruby.discover(&ctx, rb, &[rb]);
        let v = d.versions.iter().find(|v| v.tool == "ruby").unwrap();
        assert_eq!(
            (v.value.as_str(), v.source.as_str()),
            ("3.3.0", ".tool-versions")
        );
    }
}
