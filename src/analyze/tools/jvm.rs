//! JVM / others: one slice of the tool knowledge table behind `analyze::tools::summarize`.

#[allow(unused_imports)]
use super::Kind::*;
#[allow(unused_imports)]
use super::{clean_path, compose, is_build_output, list};
use super::{s, Args, Summary};
#[allow(unused_imports)]
use crate::model::Risk::*;

#[allow(unused_variables, clippy::needless_return)]
pub(super) fn summarize(
    program: &str,
    args: &[String],
    a: &Args,
    sub: Option<&str>,
) -> Option<Summary> {
    match program {
        "gradle" | "gradlew" => match sub {
            Some("build") => s("gradle", "Build with Gradle", Build, Safe),
            Some("test") => s("gradle", "Run Gradle tests", Test, Safe),
            Some("run") | Some("bootRun") => s("gradle", "Run the app with Gradle", Run, Safe),
            Some("clean") => s("gradle", "Clean Gradle build outputs", Clean, Safe),
            Some("publish") | Some("publishToMavenCentral") => {
                s("gradle", "Publish artifacts with Gradle", Publish, External)
            }
            Some("check") => s("gradle", "Run Gradle checks", Lint, Safe),
            Some(x) => s("gradle", format!("Run the {x} Gradle task"), Other, Safe),
            None => s("gradle", "Run Gradle", Other, Safe),
        },
        "mvn" | "mvnw" => match sub {
            Some("package") => s("maven", "Package with Maven", Build, Safe),
            Some("test") => s("maven", "Run Maven tests", Test, Safe),
            Some("verify") => s("maven", "Run Maven verification", Test, Safe),
            Some("install") => s(
                "maven",
                "Install artifacts to the local Maven repo",
                Build,
                Safe,
            ),
            Some("deploy") => s("maven", "Deploy artifacts with Maven", Publish, External),
            Some("clean") => s("maven", "Clean Maven build outputs", Clean, Safe),
            Some("compile") => s("maven", "Compile with Maven", Build, Safe),
            Some(x) => s("maven", format!("Run Maven {x}"), Other, Safe),
            None => s("maven", "Run Maven", Other, Safe),
        },
        "bundle" => match sub {
            Some("exec") => s(
                "bundler",
                format!(
                    "Run {} with Bundler",
                    a.positionals().get(1).copied().unwrap_or("a command")
                ),
                Other,
                Safe,
            ),
            Some("install") | None => s("bundler", "Install Ruby gems", Install, Safe),
            Some(x) => s("bundler", format!("Run bundle {x}"), Other, Safe),
        },
        "rake" => match sub {
            Some("db:migrate")
            | Some("db:reset")
            | Some("db:drop")
            | Some("db:seed")
            | Some("db:rollback")
            | Some("db:schema:load")
            | Some("db:prepare")
            | Some("db:create")
            | Some("test")
            | Some("test:system")
            | Some("routes")
            | Some("assets:precompile") => super::summarize("rails", args),
            Some(t) => s("rake", format!("Run the {t} Rake task"), Other, Safe),
            None => s("rake", "Run the default Rake task", Other, Safe),
        },
        "rails" => match sub {
            Some("server") | Some("s") => s("rails", "Start the Rails server", Dev, Safe),
            Some("console") | Some("c") => s("rails", "Open the Rails console", Dev, Safe),
            Some("db:migrate") => s("rails", "Apply Rails migrations", Migrate, Safe),
            Some("db:reset") | Some("db:drop") => {
                s("rails", "Drop and recreate the database", Db, Destructive)
            }
            Some("db:seed") => s("rails", "Seed the database", Db, Safe),
            Some("db:rollback") => s(
                "rails",
                "Roll back the last Rails migration",
                Migrate,
                Destructive,
            ),
            Some("db:schema:load") => s(
                "rails",
                "Load the schema into the database",
                Db,
                Destructive,
            ),
            Some("db:prepare") | Some("db:setup") => s(
                "rails",
                "Create the database and apply migrations",
                Db,
                Safe,
            ),
            Some("db:create") => s("rails", "Create the database", Db, Safe),
            Some("test") => s("rails", "Run Rails tests", Test, Safe),
            Some("test:system") => s("rails", "Run Rails system (browser) tests", E2e, Safe),
            Some("routes") => s("rails", "List Rails routes", Other, Safe),
            Some("assets:precompile") => s("rails", "Precompile Rails assets", Build, Safe),
            Some("tailwindcss:build") | Some("css:build") | Some("javascript:build") => {
                s("rails", "Build Rails front-end assets", Build, Safe)
            }
            Some("generate") | Some("g") => s("rails", "Run a Rails generator", Generate, Safe),
            Some(x) => s("rails", format!("Run rails {x}"), Other, Safe),
            None => s("rails", "Run Rails", Other, Safe),
        },
        "rspec" => s(
            "rspec",
            match a.positionals().first() {
                Some(p) => format!("Run RSpec tests in {}", clean_path(p)),
                None => "Run RSpec tests".into(),
            },
            Test,
            Safe,
        ),
        "bin/dev" => s(
            "rails",
            "Start the Rails app and its dev processes",
            Dev,
            Safe,
        ),
        "bin/setup" => s("rails", "Set up the app (bin/setup)", Install, Safe),
        "foreman" | "overmind" | "hivemind" => {
            s(program, "Start the processes from the Procfile", Dev, Safe)
        }
        "puma" => s("puma", "Start the Puma web server", Dev, Safe),
        "sidekiq" => s("sidekiq", "Start a Sidekiq worker", Dev, Safe),
        "brakeman" => s("brakeman", "Scan the Rails app with Brakeman", Lint, Safe),
        "erb_lint" | "erblint" => s("erb_lint", "Check ERB templates", Lint, Safe),
        "standardrb" => s("standardrb", "Check Ruby source with Standard", Lint, Safe),
        "srb" => s(
            "sorbet",
            "Run static type checking with Sorbet",
            TypeCheck,
            Safe,
        ),
        "steep" => s(
            "steep",
            "Run static type checking with Steep",
            TypeCheck,
            Safe,
        ),
        "gem" => match sub {
            Some("build") => s("gem", "Build the gem", Build, Safe),
            Some("push") => s("gem", "Push the gem to RubyGems", Publish, External),
            Some("install") => s("gem", "Install Ruby gems", Install, Safe),
            Some(x) => s("gem", format!("Run gem {x}"), Other, Safe),
            None => s("gem", "Run gem", Other, Safe),
        },
        "maestro" => match sub {
            Some("test") => s(
                "maestro",
                format!(
                    "Run Maestro mobile end-to-end flows{}",
                    a.positionals()
                        .get(1)
                        .map(|p| format!(" in {}", clean_path(p)))
                        .unwrap_or_default()
                ),
                E2e,
                Safe,
            ),
            Some("studio") => s("maestro", "Open Maestro Studio", E2e, Safe),
            Some("record") => s("maestro", "Record a Maestro flow video", E2e, Safe),
            Some("cloud") => s(
                "maestro",
                "Run Maestro flows in Maestro Cloud",
                E2e,
                External,
            ),
            Some(x) => s("maestro", format!("Run maestro {x}"), E2e, Safe),
            None => s("maestro", "Run Maestro", E2e, Safe),
        },
        "appium" => s("appium", "Start the Appium server", E2e, Safe),
        "nightwatch" => s("nightwatch", "Run Nightwatch end-to-end tests", E2e, Safe),
        "testcafe" => s("testcafe", "Run TestCafe end-to-end tests", E2e, Safe),
        "codeceptjs" => s("codeceptjs", "Run CodeceptJS end-to-end tests", E2e, Safe),
        "web-test-runner" | "wtr" => s(
            "web-test-runner",
            "Run browser tests with Web Test Runner",
            Test,
            Safe,
        ),
        "qunit" => s("qunit", "Run QUnit tests", Test, Safe),
        "chromatic" => s(
            "chromatic",
            "Publish Storybook to Chromatic",
            Publish,
            External,
        ),
        "test-storybook" => s("storybook", "Run Storybook interaction tests", Test, Safe),
        "backstop" => s(
            "backstop",
            "Run BackstopJS visual regression tests",
            E2e,
            Safe,
        ),
        "pa11y" | "pa11y-ci" | "axe" => s(program, "Run accessibility checks", Test, Safe),
        "k6" => s("k6", "Run k6 load tests", Test, Safe),
        "artillery" => s("artillery", "Run Artillery load tests", Test, Safe),
        "stryker" => s("stryker", "Run Stryker mutation tests", Test, Safe),
        "rubocop" => s("rubocop", "Check Ruby source with RuboCop", Lint, Safe),
        "composer" => match sub {
            Some("install") | None => s("composer", "Install PHP dependencies", Install, Safe),
            Some("test") => s("composer", "Run PHP tests", Test, Safe),
            Some(x) => s("composer", format!("Run composer {x}"), Other, Safe),
        },
        "phpunit" | "pest" => s(program, "Run PHP tests", Test, Safe),
        "pint" => s(
            "pint",
            if a.has("--test") {
                "Check PHP code style with Pint"
            } else {
                "Format PHP code with Pint"
            },
            if a.has("--test") { Lint } else { Format },
            Safe,
        ),
        "phpstan" => {
            // PHPStan's rule strictness is its numeric level (0 loosest, 9 strictest, "max" is
            // an alias for the highest); surface it when the command passes it explicitly.
            match a.value("-l").or_else(|| a.value("--level")) {
                Some(level) => s(
                    "phpstan",
                    format!("Analyse PHP code with PHPStan (level {level})"),
                    Lint,
                    Safe,
                ),
                None => s("phpstan", "Analyse PHP code with PHPStan", Lint, Safe),
            }
        }
        "psalm" => s("psalm", "Analyse PHP code with Psalm", Lint, Safe),
        "php-cs-fixer" => s(
            "php-cs-fixer",
            if a.has("--dry-run") {
                "Check PHP code style with PHP CS Fixer"
            } else {
                "Format PHP code with PHP CS Fixer"
            },
            Format,
            Safe,
        ),
        "phpcs" => s(
            "phpcs",
            "Check PHP code style with PHP_CodeSniffer",
            Lint,
            Safe,
        ),
        "phpcbf" => s(
            "phpcbf",
            "Fix PHP code style with PHP_CodeSniffer",
            Format,
            Safe,
        ),
        "rector" => s(
            "rector",
            if a.has("--dry-run") {
                "Preview Rector refactorings"
            } else {
                "Apply Rector refactorings"
            },
            Other,
            Safe,
        ),
        "infection" => s(
            "infection",
            "Run PHP mutation tests with Infection",
            Test,
            Safe,
        ),
        "php" => match sub {
            Some("artisan") => match a.positionals().get(1).copied() {
                Some("serve") => s("artisan", "Start the Laravel development server", Dev, Safe),
                Some("migrate") => s("artisan", "Apply Laravel migrations", Migrate, Safe),
                Some("migrate:fresh") | Some("migrate:reset") | Some("db:wipe") => s(
                    "artisan",
                    "Drop all tables and re-run migrations",
                    Db,
                    Destructive,
                ),
                Some("test") => s("artisan", "Run Laravel tests", Test, Safe),
                Some(x) => s("artisan", format!("Run artisan {x}"), Other, Safe),
                None => s("artisan", "Run artisan", Other, Safe),
            },
            Some(f) => s("php", format!("Run {}", clean_path(f)), Run, Safe),
            None => s("php", "Run PHP", Run, Safe),
        },
        "zig" => s(
            "zig",
            format!("Run zig {}", sub.unwrap_or("build")),
            Build,
            Safe,
        ),
        "cmake" => s(
            "cmake",
            if a.has("--build") {
                "Build with CMake"
            } else {
                "Configure the CMake project"
            },
            Build,
            Safe,
        ),
        "ninja" => s("ninja", "Build with Ninja", Build, Safe),
        "bazel" | "bazelisk" => match sub {
            Some("build") => s("bazel", "Build with Bazel", Build, Safe),
            Some("test") => s("bazel", "Run Bazel tests", Test, Safe),
            Some("run") => s(
                "bazel",
                format!(
                    "Run {} with Bazel",
                    a.positionals().get(1).copied().unwrap_or("a target")
                ),
                Run,
                Safe,
            ),
            Some("clean") => s("bazel", "Clean Bazel outputs", Clean, Safe),
            Some(x) => s("bazel", format!("Run bazel {x}"), Other, Safe),
            None => s("bazel", "Run Bazel", Other, Safe),
        },
        "pants" => s(
            "pants",
            format!("Run pants {}", sub.unwrap_or("")),
            Other,
            Safe,
        ),
        "swift" => match sub {
            Some("build") => s("swift", "Build the Swift package", Build, Safe),
            Some("test") => s("swift", "Run Swift tests", Test, Safe),
            Some("run") => s("swift", "Run the Swift executable", Run, Safe),
            Some(x) => s("swift", format!("Run swift {x}"), Other, Safe),
            None => s("swift", "Run Swift", Other, Safe),
        },
        "xcodebuild" => s(
            "xcodebuild",
            if a.has("test") {
                "Run Xcode tests"
            } else {
                "Build with Xcode"
            },
            if a.has("test") { Test } else { Build },
            Safe,
        ),
        "flutter" => match sub {
            Some("run") => s("flutter", "Run the Flutter app", Dev, Safe),
            Some("build") => s("flutter", "Build the Flutter app", Build, Safe),
            Some("test") => s("flutter", "Run Flutter tests", Test, Safe),
            Some("analyze") => s("flutter", "Analyze Dart source", Lint, Safe),
            Some("pub") => s("flutter", "Manage Dart packages", Install, Safe),
            Some(x) => s("flutter", format!("Run flutter {x}"), Other, Safe),
            None => s("flutter", "Run Flutter", Other, Safe),
        },
        "dart" => s(
            "dart",
            format!("Run dart {}", sub.unwrap_or("")),
            Other,
            Safe,
        ),
        "mix" => match sub {
            Some("test") => s("mix", "Run Elixir tests", Test, Safe),
            Some("phx.server") => s("mix", "Start the Phoenix server", Dev, Safe),
            Some("ecto.migrate") => s("mix", "Apply Ecto migrations", Migrate, Safe),
            Some("ecto.reset") | Some("ecto.drop") => {
                s("mix", "Drop and recreate the database", Db, Destructive)
            }
            Some("format") => s("mix", "Format Elixir source", Format, Safe),
            Some("credo") => s("mix", "Check Elixir source with Credo", Lint, Safe),
            Some("compile") => s("mix", "Compile the Elixir project", Build, Safe),
            Some(x) => s("mix", format!("Run mix {x}"), Other, Safe),
            None => s("mix", "Run mix", Other, Safe),
        },

        _ => None,
    }
}
