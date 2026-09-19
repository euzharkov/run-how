//! PHP: Composer scripts, Laravel `artisan`, Symfony `bin/console`, PHPUnit/Pest, PHPStan/Psalm, Pint.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;
use serde_json::Value;

pub struct Php;

impl Discoverer for Php {
    fn id(&self) -> &'static str {
        "php"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Php
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has("composer.json") || dir.has("artisan")
    }
    fn project_name(&self, dir: &DirInfo) -> Option<String> {
        let v: Value = serde_json::from_str(&dir.read("composer.json")?).ok()?;
        v.get("name")?
            .as_str()
            .map(|s| s.rsplit('/').next().unwrap_or(s).to_string())
    }
    fn discover(&self, _ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let composer: Option<Value> = base
            .read("composer.json")
            .and_then(|t| serde_json::from_str(&t).ok());
        let deps: Vec<String> = composer
            .as_ref()
            .map(|c| {
                ["require", "require-dev"]
                    .iter()
                    .filter_map(|k| c.get(k).and_then(|o| o.as_object()))
                    .flat_map(|o| o.keys().cloned())
                    .collect()
            })
            .unwrap_or_default();
        let dep = |n: &str| deps.iter().any(|d| d == n);
        if let Some(v) = composer
            .as_ref()
            .and_then(|c| c.get("require"))
            .and_then(|r| r.get("php"))
            .and_then(|v| v.as_str())
        {
            out.version("php", v, "composer.json");
        }
        let mut declared: Vec<String> = Vec::new();

        if let Some(scripts) = composer
            .as_ref()
            .and_then(|c| c.get("scripts"))
            .and_then(|s| s.as_object())
        {
            let script_names: Vec<String> = scripts.keys().cloned().collect();
            // Composer's `@` prefix means one of two things: `@other-script` (no spaces) calls
            // another script by name; anything else (almost always `@php ...`) is a literal
            // shell command with the `@` only meaning "run quietly" — `@php` is replaced with
            // the PHP binary, so it becomes plain `php`.
            let expand = |x: &str| -> String {
                match x.strip_prefix('@') {
                    Some(rest)
                        if !rest.contains(char::is_whitespace)
                            && script_names.iter().any(|n| n == rest) =>
                    {
                        format!("composer {rest}")
                    }
                    Some(rest) => rest.to_string(),
                    None => x.to_string(),
                }
            };
            let descs = composer
                .as_ref()
                .and_then(|c| c.get("scripts-descriptions"))
                .and_then(|d| d.as_object());
            for (name, v) in scripts {
                if name.starts_with("pre-") || name.starts_with("post-") {
                    continue; // Composer event hooks
                }
                let body = match v {
                    Value::String(s) => expand(s),
                    Value::Array(a) => a
                        .iter()
                        .filter_map(|x| x.as_str())
                        .map(expand)
                        .collect::<Vec<_>>()
                        .join(" && "),
                    _ => continue,
                };
                out.script("composer", name, &body);
                declared.push(name.clone());
                let mut a = Action::new(name, format!("composer {name}"))
                    .tool("composer")
                    .raw(body);
                if let Some(d) = descs.and_then(|d| d.get(name)).and_then(|d| d.as_str()) {
                    a = a.desc(d);
                }
                out.actions.push(a);
            }
        }
        let free = |n: &str| !declared.iter().any(|d| d == n);

        if base.has("artisan") {
            let art = |c: &str| format!("php artisan {c}");
            if free("dev") {
                out.actions.push(
                    Action::new("dev", art("serve"))
                        .tool("artisan")
                        .inferred(Confidence::High),
                );
            }
            out.actions.push(
                Action::new("tinker", art("tinker"))
                    .tool("artisan")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Open the Laravel REPL")
                    .cat(Category::Development),
            );
            if free("migrate") {
                out.actions.push(
                    Action::new("migrate", art("migrate"))
                        .tool("artisan")
                        .inferred(Confidence::High),
                );
            }
            out.actions.push(
                Action::new("db:seed", art("db:seed"))
                    .tool("artisan")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Seed the database")
                    .cat(Category::Database),
            );
            out.actions.push(
                Action::new("migrate:fresh", art("migrate:fresh --seed"))
                    .tool("artisan")
                    .inferred(Confidence::Low)
                    .inferred_desc("Drop all tables, re-run migrations and seed")
                    .cat(Category::Database)
                    .risk(Risk::Destructive),
            );
            out.actions.push(
                Action::new("queue", art("queue:work"))
                    .tool("artisan")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Start a Laravel queue worker")
                    .cat(Category::Development),
            );
            out.actions.push(
                Action::new("routes", art("route:list"))
                    .tool("artisan")
                    .inferred(Confidence::Low)
                    .inferred_desc("List Laravel routes")
                    .cat(Category::Other),
            );
            if free("test") {
                out.actions.push(
                    Action::new("test", art("test"))
                        .tool("artisan")
                        .inferred(Confidence::High),
                );
            }
            if dep("laravel/pint") && free("format") {
                out.actions.push(
                    Action::new("format", "./vendor/bin/pint")
                        .tool("php")
                        .inferred(Confidence::High)
                        .inferred_desc("Format PHP code with Pint")
                        .cat(Category::Quality),
                );
            }
        } else if base.path.join("bin/console").is_file() {
            let sf = |c: &str| format!("php bin/console {c}");
            out.actions.push(
                Action::new("dev", "symfony server:start".to_string())
                    .tool("symfony")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Start the Symfony local web server")
                    .cat(Category::Development),
            );
            out.actions.push(
                Action::new("migrate", sf("doctrine:migrations:migrate"))
                    .tool("symfony")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Apply Doctrine migrations")
                    .cat(Category::Database),
            );
            out.actions.push(
                Action::new("cache:clear", sf("cache:clear"))
                    .tool("symfony")
                    .inferred(Confidence::Low)
                    .inferred_desc("Clear the Symfony cache")
                    .cat(Category::Build),
            );
        }
        if free("test") && !base.has("artisan") {
            if dep("pestphp/pest") {
                out.actions.push(
                    Action::new("test", "./vendor/bin/pest")
                        .tool("php")
                        .inferred(Confidence::High),
                );
            } else if dep("phpunit/phpunit") || base.has_any(&["phpunit.xml", "phpunit.xml.dist"]) {
                out.actions.push(
                    Action::new("test", "./vendor/bin/phpunit")
                        .tool("php")
                        .inferred(Confidence::High),
                );
            }
        }
        if free("analyse") && free("analyze") {
            if dep("phpstan/phpstan") || base.has_any(&["phpstan.neon", "phpstan.neon.dist"]) {
                out.actions.push(
                    Action::new("analyse", "./vendor/bin/phpstan analyse")
                        .tool("php")
                        .inferred(Confidence::High)
                        .inferred_desc("Analyse PHP code with PHPStan")
                        .cat(Category::Quality),
                );
            } else if dep("vimeo/psalm") || base.has("psalm.xml") {
                out.actions.push(
                    Action::new("analyse", "./vendor/bin/psalm")
                        .tool("php")
                        .inferred(Confidence::High)
                        .inferred_desc("Analyse PHP code with Psalm")
                        .cat(Category::Quality),
                );
            }
        }
        if free("format") && !base.has("artisan") {
            if dep("friendsofphp/php-cs-fixer")
                || base.has_any(&[".php-cs-fixer.php", ".php-cs-fixer.dist.php"])
            {
                out.actions.push(
                    Action::new("format", "./vendor/bin/php-cs-fixer fix")
                        .tool("php")
                        .inferred(Confidence::High)
                        .inferred_desc("Format PHP code with PHP CS Fixer")
                        .cat(Category::Quality),
                );
            } else if dep("squizlabs/php_codesniffer") {
                out.actions.push(
                    Action::new("lint", "./vendor/bin/phpcs")
                        .tool("php")
                        .inferred(Confidence::High)
                        .inferred_desc("Check PHP code style with PHP_CodeSniffer")
                        .cat(Category::Quality),
                );
            }
        }
        out
    }
}
