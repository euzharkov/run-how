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

/// `desc "..."` followed by `task :name` / `task name:` / namespaced blocks.
pub fn rake_tasks(text: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut pending: Option<String> = None;
    let mut namespaces: Vec<String> = Vec::new();
    for raw in text.lines() {
        let l = raw.trim();
        if let Some(rest) = l.strip_prefix("desc ") {
            let d = rest.trim().trim_matches(|c| c == '"' || c == '\'');
            pending = Some(d.to_string());
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
            namespaces.push(ns);
            continue;
        }
        if l == "end" && !namespaces.is_empty() && raw.starts_with(|c: char| !c.is_whitespace()) {
            namespaces.pop();
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
            if name.is_empty() {
                continue;
            }
            let full = if namespaces.is_empty() {
                name
            } else {
                format!("{}:{}", namespaces.join(":"), name)
            };
            out.push((full, pending.take()));
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
    fn discover(&self, _ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let gemfile = base.read("Gemfile").unwrap_or_default();
        let bundler = base.has("Gemfile");
        if let Some(v) = base.read(".ruby-version") {
            out.version("ruby", v.trim(), ".ruby-version");
        }
        let be = |cmd: &str| {
            if bundler {
                format!("bundle exec {cmd}")
            } else {
                cmd.to_string()
            }
        };
        let rails = base.path.join("bin/rails").is_file() || gem_listed(&gemfile, "rails");
        let rspec = base.has_dir("spec")
            || gem_listed(&gemfile, "rspec")
            || gem_listed(&gemfile, "rspec-rails");
        let rubocop = base.has(".rubocop.yml") || gem_listed(&gemfile, "rubocop");

        // Declared Rake tasks (with descriptions).
        let mut declared: Vec<String> = Vec::new();
        for f in ["Rakefile", "rakefile", "Rakefile.rb"] {
            let Some(text) = base.read(f) else { continue };
            for (name, desc) in rake_tasks(&text) {
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
            break;
        }
        let free = |n: &str| !declared.iter().any(|d| d == n);

        if rails {
            let r = |c: &str| {
                if base.path.join("bin/rails").is_file() {
                    format!("bin/rails {c}")
                } else {
                    be(&format!("rails {c}"))
                }
            };
            if base.path.join("bin/dev").is_file() && free("dev") {
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
            if base.path.join("db/seeds.rb").is_file() && free("db:seed") {
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
            if base.path.join("bin/setup").is_file() && free("setup") {
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
                if base.has_dir("test") && base.path.join("test/system").is_dir() {
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
        if base.has("Gemfile") && base.has("*.gemspec") {
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
}
