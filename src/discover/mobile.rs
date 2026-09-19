//! Native and cross-platform mobile: Xcode projects, Swift packages, Fastlane lanes,
//! Flutter / Dart. (Expo and React Native live in the JS adapter because they are package.json
//! projects; Android lives in the JVM adapter because it is Gradle.)

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Mobile;

/// `desc "..."` + `lane :name do`, inside optional `platform :ios do` blocks.
pub fn fastlane_lanes(text: &str) -> Vec<(Option<String>, String, Option<String>)> {
    let mut out = Vec::new();
    let mut pending: Option<String> = None;
    let mut platform: Option<String> = None;
    for raw in text.lines() {
        let l = raw.trim();
        if let Some(rest) = l.strip_prefix("desc ") {
            pending = Some(
                rest.trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string(),
            );
        } else if let Some(rest) = l.strip_prefix("platform ") {
            platform = Some(
                rest.trim()
                    .trim_start_matches(':')
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string(),
            );
        } else if l == "end" && !raw.starts_with(|c: char| c.is_whitespace()) {
            platform = None;
        } else if let Some(rest) = l
            .strip_prefix("lane ")
            .or_else(|| l.strip_prefix("private_lane "))
        {
            if l.starts_with("private_lane") {
                pending = None;
                continue;
            }
            let name: String = rest
                .trim()
                .trim_start_matches(':')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                out.push((platform.clone(), name, pending.take()));
            }
        }
    }
    out
}

impl Discoverer for Mobile {
    fn id(&self) -> &'static str {
        "mobile"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Mobile
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        // A bare `Podfile` alone is not enough: every Expo/React Native app ships an `ios/`
        // directory holding only a Podfile before the native project is generated, and that
        // must not spawn its own (otherwise empty) project. It only counts alongside an
        // actual Xcode project/workspace, handled below.
        dir.dirs
            .iter()
            .any(|d| d.ends_with(".xcodeproj") || d.ends_with(".xcworkspace"))
            || dir.has("Package.swift")
            || dir.has("pubspec.yaml")
            || (dir.has_dir("fastlane") && dir.path.join("fastlane/Fastfile").is_file())
    }
    fn project_name(&self, dir: &DirInfo) -> Option<String> {
        if let Some(x) = dir
            .dirs
            .iter()
            .find(|d| d.ends_with(".xcworkspace"))
            .or_else(|| dir.dirs.iter().find(|d| d.ends_with(".xcodeproj")))
        {
            return Some(
                x.rsplit_once('.')
                    .map(|(s, _)| s.to_string())
                    .unwrap_or(x.clone()),
            );
        }
        if let Some(text) = dir.read("pubspec.yaml") {
            return text
                .lines()
                .find_map(|l| l.strip_prefix("name:"))
                .map(|s| s.trim().to_string());
        }
        None
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let macos = ctx.host_os == "macos";

        // ---- Flutter / Dart ---------------------------------------------------------------
        if let Some(pub_) = base.read("pubspec.yaml") {
            let flutter = pub_.contains("flutter:") || pub_.contains("sdk: flutter");
            if flutter {
                let f = |s: &str| format!("flutter {s}");
                out.actions.push(
                    Action::new("run", f("run"))
                        .tool("flutter")
                        .inferred(Confidence::High)
                        .cat(Category::Development),
                );
                out.actions.push(
                    Action::new("test", f("test"))
                        .tool("flutter")
                        .inferred(Confidence::High),
                );
                out.actions.push(
                    Action::new("analyze", f("analyze"))
                        .tool("flutter")
                        .inferred(Confidence::High),
                );
                out.actions.push(
                    Action::new("format", "dart format .")
                        .tool("flutter")
                        .inferred(Confidence::Medium),
                );
                out.actions.push(
                    Action::new("build:apk", f("build apk"))
                        .tool("flutter")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Build the Android APK")
                        .cat(Category::Build),
                );
                out.actions.push(
                    Action::new("build:appbundle", f("build appbundle"))
                        .tool("flutter")
                        .inferred(Confidence::Low)
                        .inferred_desc("Build the Android App Bundle")
                        .cat(Category::Build),
                );
                if base.has_dir("ios") {
                    out.actions.push(
                        Action::new("build:ios", f("build ios"))
                            .tool("flutter")
                            .inferred(if macos {
                                Confidence::Medium
                            } else {
                                Confidence::Low
                            })
                            .inferred_desc("Build the iOS app")
                            .cat(Category::Build),
                    );
                }
                if base.has_dir("web") {
                    out.actions.push(
                        Action::new("build:web", f("build web"))
                            .tool("flutter")
                            .inferred(Confidence::Medium)
                            .inferred_desc("Build the web app")
                            .cat(Category::Build),
                    );
                }
                if pub_.contains("build_runner") {
                    out.actions.push(
                        Action::new(
                            "generate",
                            "dart run build_runner build --delete-conflicting-outputs",
                        )
                        .tool("flutter")
                        .inferred(Confidence::High)
                        .inferred_desc("Generate code with build_runner")
                        .cat(Category::Build),
                    );
                }
                if base.has_dir("integration_test") {
                    out.actions.push(
                        Action::new("test:integration", f("test integration_test"))
                            .tool("flutter")
                            .inferred(Confidence::High)
                            .inferred_desc("Run Flutter integration tests on a device")
                            .cat(Category::Testing),
                    );
                }
                out.actions.push(
                    Action::new("pub:get", f("pub get"))
                        .tool("flutter")
                        .inferred(Confidence::Low)
                        .inferred_desc("Install Dart dependencies")
                        .cat(Category::Other),
                );
            } else {
                out.actions.push(
                    Action::new("test", "dart test")
                        .tool("dart")
                        .inferred(Confidence::High)
                        .inferred_desc("Run Dart tests")
                        .cat(Category::Testing),
                );
                out.actions.push(
                    Action::new("analyze", "dart analyze")
                        .tool("dart")
                        .inferred(Confidence::High)
                        .inferred_desc("Analyse Dart source")
                        .cat(Category::Quality),
                );
                out.actions.push(
                    Action::new("format", "dart format .")
                        .tool("dart")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Format Dart source")
                        .cat(Category::Quality),
                );
                if base.path.join("bin/main.dart").is_file() {
                    out.actions.push(
                        Action::new("run", "dart run")
                            .tool("dart")
                            .inferred(Confidence::High)
                            .inferred_desc("Run the Dart program")
                            .cat(Category::Development),
                    );
                }
            }
        }

        // ---- Swift Package Manager ----------------------------------------------------------
        if let Some(pkg) = base.read("Package.swift") {
            out.actions.push(
                Action::new("build", "swift build")
                    .tool("swift")
                    .inferred(Confidence::High),
            );
            out.actions.push(
                Action::new("test", "swift test")
                    .tool("swift")
                    .inferred(Confidence::High),
            );
            if pkg.contains(".executableTarget") {
                out.actions.push(
                    Action::new("run", "swift run")
                        .tool("swift")
                        .inferred(Confidence::High)
                        .cat(Category::Development),
                );
            }
            if base.has(".swiftlint.yml") {
                out.actions.push(
                    Action::new("lint", "swiftlint")
                        .tool("swift")
                        .inferred(Confidence::High)
                        .inferred_desc("Check Swift source with SwiftLint")
                        .cat(Category::Quality),
                );
            }
            if base.has(".swiftformat") || base.has(".swift-format") {
                out.actions.push(
                    Action::new(
                        "format",
                        if base.has(".swiftformat") {
                            "swiftformat ."
                        } else {
                            "swift format -i -r ."
                        },
                    )
                    .tool("swift")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Format Swift source")
                    .cat(Category::Quality),
                );
            }
        }

        // ---- Xcode ---------------------------------------------------------------------------
        let workspace = base
            .dirs
            .iter()
            .find(|d| d.ends_with(".xcworkspace"))
            .cloned();
        let project = base
            .dirs
            .iter()
            .find(|d| d.ends_with(".xcodeproj"))
            .cloned();
        if let Some(container) = workspace.clone().or(project.clone()) {
            let scheme = container
                .rsplit_once('.')
                .map(|(s, _)| s.to_string())
                .unwrap_or(container.clone());
            let flag = if container.ends_with(".xcworkspace") {
                "-workspace"
            } else {
                "-project"
            };
            let conf = if macos {
                Confidence::Medium
            } else {
                Confidence::Low
            };
            let x = |action: &str, dest: &str| {
                format!(
                    "xcodebuild {flag} {} -scheme {} -destination '{dest}' {action}",
                    super::q(&container),
                    super::q(&scheme)
                )
            };
            let sim = "platform=iOS Simulator,name=iPhone 16";
            out.actions.push(
                Action::new("build", x("build", sim))
                    .tool("xcode")
                    .inferred(conf)
                    .inferred_desc(format!("Build the {scheme} scheme for the iOS Simulator"))
                    .cat(Category::Build),
            );
            out.actions.push(
                Action::new("test", x("test", sim))
                    .tool("xcode")
                    .inferred(conf)
                    .inferred_desc(format!("Run {scheme} tests in the iOS Simulator"))
                    .cat(Category::Testing),
            );
            if base.has("Podfile") {
                out.actions.push(
                    Action::new("pods", "pod install")
                        .tool("cocoapods")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Install CocoaPods dependencies")
                        .cat(Category::Other),
                );
            }
            if base.has("project.yml") {
                out.actions.push(
                    Action::new("xcodegen", "xcodegen generate")
                        .tool("xcode")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Regenerate the Xcode project with XcodeGen")
                        .cat(Category::Build),
                );
            }
            if base.has("Tuist") || base.has("Project.swift") {
                out.actions.push(
                    Action::new("tuist", "tuist generate")
                        .tool("xcode")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Generate the Xcode project with Tuist")
                        .cat(Category::Build),
                );
            }
        }

        // ---- Fastlane -------------------------------------------------------------------------
        if let Some(ff) = crate::repo::read_text(&base.path.join("fastlane/Fastfile")) {
            let bundler = base.has("Gemfile");
            for (platform, lane, desc) in fastlane_lanes(&ff) {
                let (id, cmd) = match &platform {
                    Some(p) => (format!("{p}:{lane}"), format!("fastlane {p} {lane}")),
                    None => (lane.clone(), format!("fastlane {lane}")),
                };
                let cmd = if bundler {
                    format!("bundle exec {cmd}")
                } else {
                    cmd
                };
                let mut a = Action::new(id, cmd).tool("fastlane");
                match desc {
                    Some(d) => a = a.desc(d),
                    None => a = a.inferred_desc(format!("Run the {lane} Fastlane lane")),
                }
                let l = lane.to_ascii_lowercase();
                if l.contains("release")
                    || l.contains("deploy")
                    || l.contains("upload")
                    || l.contains("submit")
                    || l.contains("beta")
                    || l.contains("testflight")
                    || l.contains("distribute")
                    || l.contains("publish")
                {
                    a = a.risk(Risk::External).cat(Category::Release);
                } else if l.contains("test") {
                    a = a.cat(Category::Testing);
                } else if l.contains("build") || l.contains("archive") {
                    a = a.cat(Category::Build);
                }
                out.actions.push(a);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fastfile() {
        let ff = "default_platform(:ios)\n\nplatform :ios do\n  desc \"Run unit tests\"\n  lane :test do\n    scan\n  end\n\n  desc \"Ship to TestFlight\"\n  lane :beta do\n  end\nend\n\nlane :bump do\nend\n";
        let l = fastlane_lanes(ff);
        assert_eq!(
            l[0],
            (
                Some("ios".into()),
                "test".into(),
                Some("Run unit tests".into())
            )
        );
        assert_eq!(l[1].1, "beta");
        assert_eq!(l[2], (None, "bump".into(), None));
    }
}
