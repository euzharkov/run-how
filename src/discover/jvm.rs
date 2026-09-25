//! JVM: Gradle (wrapper, Spring Boot, Android, custom tasks, multi-project) and Maven (modules).

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Jvm;

const GRADLE_BUILD: &[&str] = &["build.gradle.kts", "build.gradle"];
const GRADLE_SETTINGS: &[&str] = &["settings.gradle.kts", "settings.gradle"];

/// `tasks.register("name") { description = "..." }`, `task name(...)`, `tasks.register<Type>("name")`.
pub fn gradle_tasks(text: &str) -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    for (i, raw) in lines.iter().enumerate() {
        let l = raw.trim();
        let name = if let Some(rest) = l.strip_prefix("tasks.register") {
            rest.split('"')
                .nth(1)
                .or_else(|| rest.split('\'').nth(1))
                .map(|s| s.to_string())
        } else if let Some(rest) = l.strip_prefix("task ") {
            let n: String = rest
                .trim_start_matches('"')
                .trim_start_matches('\'')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if n.is_empty() || n == "type" {
                None
            } else {
                Some(n)
            }
        } else if let Some(rest) = l.strip_prefix("tasks.create") {
            rest.split('"').nth(1).map(|s| s.to_string())
        } else {
            None
        };
        let Some(name) = name else { continue };
        if out.iter().any(|(n, _)| *n == name) {
            continue;
        }
        // Only look inside this task's own block: a single-line registration
        // (`tasks.register<Copy>("x")`) has no body, so it never has a description.
        let mut desc = None;
        if l.trim_end().ends_with('{') {
            for line in lines.iter().skip(i + 1).take(8) {
                let t = line.trim();
                if let Some(d) = t
                    .strip_prefix("description")
                    .or_else(|| t.strip_prefix("description ="))
                    .or_else(|| t.strip_prefix("description("))
                {
                    let d =
                        super::unquote(d.trim_start_matches(['=', '(', ' ']).trim_end_matches(')'));
                    if !d.is_empty() {
                        desc = Some(d.to_string());
                    }
                    break;
                }
                if t == "}" {
                    break;
                }
            }
        }
        out.push((name, desc));
    }
    out
}

/// Extract the pinned version from a `distributionUrl` line such as
/// `distributionUrl=https\://services.gradle.org/distributions/gradle-8.10-bin.zip`.
fn gradle_wrapper_version(props: &str) -> Option<String> {
    let line = props
        .lines()
        .find(|l| l.trim_start().starts_with("distributionUrl"))?;
    let after = line.split("gradle-").nth(1)?;
    let end = after
        .find("-bin")
        .or_else(|| after.find("-all"))
        .unwrap_or(after.len());
    let v = &after[..end];
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

/// Subproject paths from `settings.gradle(.kts)`: `include(":app", ":lib")`,
/// `include("app")`, `include ':app', ':lib:core'`, `include 'app'`. Project paths use `:`
/// as the separator, with or without the leading one; `includeBuild` is not an include.
fn settings_projects(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for l in text.lines() {
        let l = l.trim();
        let Some(rest) = l.strip_prefix("include") else {
            continue;
        };
        if !(rest.starts_with('(') || rest.starts_with(' ')) {
            continue;
        }
        for part in rest.trim_start_matches('(').split(',') {
            let p = super::unquote(part.trim().trim_end_matches(')'));
            let p = p.trim_start_matches(':');
            if !p.is_empty() && !p.contains(['"', '\'', ' ']) {
                out.push(p.replace(':', "/"));
            }
        }
    }
    out
}

fn gradle_root<'a>(ctx: &Context<'a>, dir: &DirInfo) -> Option<(&'a DirInfo, String)> {
    for a in ctx.ancestors(dir).into_iter().skip(1) {
        let Some(sf) = a.first_of(GRADLE_SETTINGS) else {
            continue;
        };
        let text = ctx.text(a, sf).unwrap_or_else(|| "".into());
        let rel = dir.rel_to(a, "");
        if settings_projects(&text).contains(&rel) {
            return Some((a, format!(":{}", rel.replace('/', ":"))));
        }
    }
    None
}

fn maven_root<'a>(ctx: &Context<'a>, dir: &DirInfo) -> Option<(&'a DirInfo, String)> {
    for a in ctx.ancestors(dir).into_iter().skip(1) {
        let Some(text) = ctx.text(a, "pom.xml") else {
            continue;
        };
        let rel = dir.rel_to(a, "");
        if text.split("<module>").skip(1).any(|m| {
            m.split('<')
                .next()
                .map(|s| s.trim() == rel)
                .unwrap_or(false)
        }) {
            return Some((a, rel));
        }
    }
    None
}

impl Discoverer for Jvm {
    fn id(&self) -> &'static str {
        "jvm"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Jvm
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has_any(GRADLE_BUILD) || dir.has_any(GRADLE_SETTINGS) || dir.has("pom.xml")
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let windows = ctx.host_os == "windows";

        // ---- Gradle ---------------------------------------------------------------------
        if let Some(bf) = base
            .first_of(GRADLE_BUILD)
            .or_else(|| base.first_of(GRADLE_SETTINGS))
        {
            // `bf` is the build file when there is one, else the settings file.
            let build = base.read(bf).unwrap_or_default();
            let member = gradle_root(ctx, base);
            let (root, path) = match &member {
                Some((r, p)) => (*r, p.clone()),
                None => (base, String::new()),
            };
            let wrapper = if root.has("gradlew") {
                if windows {
                    "gradlew.bat".to_string()
                } else {
                    "./gradlew".to_string()
                }
            } else {
                "gradle".to_string()
            };
            // Only the wrapper-owning root carries `gradle/wrapper/gradle-wrapper.properties`;
            // sub-projects share it, so record it once from there.
            if member.is_none() {
                if let Some(props) = ctx
                    .child(root, "gradle/wrapper")
                    .and_then(|d| ctx.text(d, "gradle-wrapper.properties"))
                {
                    if let Some(v) = gradle_wrapper_version(&props) {
                        out.version("gradle", v, "gradle/wrapper/gradle-wrapper.properties");
                    }
                }
            }
            // A subproject's task is addressed by its absolute project path (`:app:build`),
            // run from the root where the wrapper lives.
            let g = |task: &str| {
                if path.is_empty() {
                    format!("{wrapper} {task}")
                } else {
                    format!("{wrapper} {path}:{task}")
                }
            };
            let mk = |a: Action| {
                if member.is_some() {
                    a.cwd(root.rel.clone())
                } else {
                    a
                }
            };
            let spring = build.contains("org.springframework.boot");
            let android_app = build.contains("com.android.application");
            let android_lib = build.contains("com.android.library");
            let is_app = build.contains("application") && !build.contains("com.android");

            for (name, desc) in gradle_tasks(&build) {
                let mut a = Action::new(&name, g(&name)).tool("gradle");
                match desc {
                    Some(d) => a = a.desc(d),
                    None => a = a.inferred_desc(format!("Run the {name} Gradle task")),
                }
                out.actions.push(mk(a));
            }
            if spring {
                out.actions.push(mk(Action::new("dev", g("bootRun"))
                    .tool("gradle")
                    .inferred(Confidence::High)
                    .inferred_desc("Start the Spring Boot app")
                    .cat(Category::Development)));
            } else if android_app {
                out.actions.push(mk(Action::new("dev", g("installDebug"))
                    .tool("gradle")
                    .inferred(Confidence::High)
                    .inferred_desc("Build and install the debug app on a device")
                    .cat(Category::Development)));
                out.actions
                    .push(mk(Action::new("build:debug", g("assembleDebug"))
                        .tool("gradle")
                        .inferred(Confidence::High)
                        .inferred_desc("Build the debug APK")
                        .cat(Category::Build)));
                out.actions
                    .push(mk(Action::new("build:release", g("assembleRelease"))
                        .tool("gradle")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Build the release APK")
                        .cat(Category::Build)));
                out.actions
                    .push(mk(Action::new("bundle", g("bundleRelease"))
                        .tool("gradle")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Build the release App Bundle")
                        .cat(Category::Release)));
                out.actions
                    .push(mk(Action::new("test:android", g("connectedAndroidTest"))
                        .tool("gradle")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Run instrumented tests on a device")
                        .cat(Category::Testing)));
                out.actions.push(mk(Action::new("lint", g("lint"))
                    .tool("gradle")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Run Android Lint")
                    .cat(Category::Quality)));
            } else if is_app {
                out.actions.push(mk(Action::new("run", g("run"))
                    .tool("gradle")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Run the app with Gradle")
                    .cat(Category::Development)));
            }
            if !android_lib || member.is_none() {
                out.actions.push(mk(Action::new("build", g("build"))
                    .tool("gradle")
                    .inferred(Confidence::High)
                    .inferred_desc("Build with Gradle")
                    .cat(Category::Build)));
            }
            out.actions.push(mk(Action::new("test", g("test"))
                .tool("gradle")
                .inferred(Confidence::High)
                .inferred_desc("Run Gradle tests")
                .cat(Category::Testing)));
            out.actions.push(mk(Action::new("check", g("check"))
                .tool("gradle")
                .inferred(Confidence::Medium)
                .inferred_desc("Run Gradle checks (tests and linters)")
                .cat(Category::Quality)));
            if build.contains("ktlint") {
                out.actions.push(mk(Action::new("ktlint", g("ktlintCheck"))
                    .tool("gradle")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Check Kotlin style with ktlint")
                    .cat(Category::Quality)));
            }
            if build.contains("detekt") {
                out.actions.push(mk(Action::new("detekt", g("detekt"))
                    .tool("gradle")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Analyse Kotlin code with detekt")
                    .cat(Category::Quality)));
            }
            if build.contains("spotless") {
                out.actions
                    .push(mk(Action::new("format", g("spotlessApply"))
                        .tool("gradle")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Format code with Spotless")
                        .cat(Category::Quality)));
            }
            if build.contains("maven-publish") || build.contains("publishing") {
                out.actions.push(mk(Action::new("publish", g("publish"))
                    .tool("gradle")
                    .inferred(Confidence::Low)
                    .inferred_desc("Publish artifacts with Gradle")
                    .cat(Category::Release)
                    .risk(Risk::External)));
            }
            out.actions.push(mk(Action::new("clean", g("clean"))
                .tool("gradle")
                .inferred(Confidence::Low)
                .inferred_desc("Delete Gradle build outputs")
                .cat(Category::Build)));
            if member.is_none() && base.has_any(GRADLE_SETTINGS) {
                out.actions.push(mk(Action::new("tasks", g("tasks"))
                    .tool("gradle")
                    .inferred(Confidence::Low)
                    .inferred_desc("List all Gradle tasks")
                    .cat(Category::Other)));
            }
            return out;
        }

        // ---- Maven ----------------------------------------------------------------------
        if let Some(pom) = base.read("pom.xml") {
            let member = maven_root(ctx, base);
            let (root, module) = match &member {
                Some((r, m)) => (*r, Some(m.clone())),
                None => (base, None),
            };
            let mvn = if root.has("mvnw") {
                if windows {
                    "mvnw.cmd".to_string()
                } else {
                    "./mvnw".to_string()
                }
            } else {
                "mvn".to_string()
            };
            let m = |goal: &str| match &module {
                Some(mod_) => format!("{mvn} -pl {mod_} -am {goal}"),
                None => format!("{mvn} {goal}"),
            };
            let mk = |a: Action| {
                if member.is_some() {
                    a.cwd(root.rel.clone())
                } else {
                    a
                }
            };
            let spring = pom.contains("spring-boot-maven-plugin")
                || pom.contains("spring-boot-starter-parent");
            let quarkus = pom.contains("quarkus-maven-plugin");
            let is_pom_only = pom.contains("<packaging>pom</packaging>");
            if spring {
                out.actions.push(mk(Action::new("dev", m("spring-boot:run"))
                    .tool("maven")
                    .inferred(Confidence::High)
                    .inferred_desc("Start the Spring Boot app")
                    .cat(Category::Development)));
            } else if quarkus {
                out.actions.push(mk(Action::new("dev", m("quarkus:dev"))
                    .tool("maven")
                    .inferred(Confidence::High)
                    .inferred_desc("Start Quarkus in dev mode")
                    .cat(Category::Development)));
            }
            out.actions.push(mk(Action::new("build", m("package"))
                .tool("maven")
                .inferred(Confidence::High)
                .inferred_desc(if is_pom_only {
                    "Package all Maven modules"
                } else {
                    "Package with Maven"
                })
                .cat(Category::Build)));
            out.actions.push(mk(Action::new("test", m("test"))
                .tool("maven")
                .inferred(Confidence::High)
                .inferred_desc("Run Maven tests")
                .cat(Category::Testing)));
            out.actions.push(mk(Action::new("verify", m("verify"))
                .tool("maven")
                .inferred(Confidence::Medium)
                .inferred_desc("Run Maven verification (integration tests)")
                .cat(Category::Testing)));
            out.actions.push(mk(Action::new("install", m("install"))
                .tool("maven")
                .inferred(Confidence::Low)
                .inferred_desc("Install artifacts to the local Maven repo")
                .cat(Category::Build)));
            out.actions.push(mk(Action::new("clean", m("clean"))
                .tool("maven")
                .inferred(Confidence::Low)
                .inferred_desc("Delete Maven build outputs")
                .cat(Category::Build)));
            if pom.contains("spotless-maven-plugin") {
                out.actions
                    .push(mk(Action::new("format", m("spotless:apply"))
                        .tool("maven")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Format code with Spotless")
                        .cat(Category::Quality)));
            }
            if pom.contains("<distributionManagement>")
                || pom.contains("nexus-staging")
                || pom.contains("central-publishing")
            {
                out.actions.push(mk(Action::new("deploy", m("deploy"))
                    .tool("maven")
                    .inferred(Confidence::Low)
                    .inferred_desc("Deploy artifacts to the Maven repository")
                    .cat(Category::Release)
                    .risk(Risk::External)));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gradle_tasks() {
        let kts = r#"
tasks.register("generateDocs") {
    group = "documentation"
    description = "Generate the API docs"
}
tasks.register<Copy>("copyAssets")
task integrationTest(type: Test) {
    description 'Run integration tests'
}
"#;
        let t = gradle_tasks(kts);
        assert_eq!(
            t[0],
            ("generateDocs".into(), Some("Generate the API docs".into()))
        );
        assert_eq!(t[1], ("copyAssets".into(), None));
        assert_eq!(
            t[2],
            (
                "integrationTest".into(),
                Some("Run integration tests".into())
            )
        );
        assert_eq!(
            settings_projects(
                "rootProject.name = 'x'\ninclude ':app', ':lib:core'\ninclude(\":tools\")\ninclude(\"web\", \"cli\")\ninclude 'api'\nincludeBuild(\"build-logic\")"
            ),
            ["app", "lib/core", "tools", "web", "cli", "api"]
        );
    }
}
