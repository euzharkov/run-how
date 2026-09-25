//! package managers: one slice of the tool knowledge table behind `analyze::tools::summarize`.

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
        "apt-get" | "apt" | "apk" | "dnf" | "yum" | "pacman" | "zypper" => match sub {
            Some("install") | Some("add") | Some("-S") => s(
                program,
                format!("Install system packages with {program}"),
                Install,
                Safe,
            ),
            Some("update") | Some("upgrade") => s(
                program,
                format!("Update system packages with {program}"),
                Install,
                Safe,
            ),
            _ => s(program, format!("Run {program}"), Other, Safe),
        },
        "brew" => match sub {
            Some("install") | Some("tap") => {
                s("brew", "Install packages with Homebrew", Install, Safe)
            }
            Some("bundle") => s("brew", "Install the Brewfile packages", Install, Safe),
            Some("update") | Some("upgrade") => {
                s("brew", "Update Homebrew packages", Install, Safe)
            }
            _ => s("brew", "Run Homebrew", Other, Safe),
        },
        "npm" => match sub {
            Some("install") | Some("i") | Some("ci") | Some("add") | Some("clean-install") => {
                s("npm", "Install dependencies with npm", Install, Safe)
            }
            Some("publish") => s("npm", "Publish the package to npm", Publish, External),
            Some("version") => s("npm", "Bump the package version", Other, Safe),
            Some("pack") => s("npm", "Create an npm tarball", Build, Safe),
            Some("audit") => s(
                "npm",
                if a.has("fix") {
                    "Fix vulnerable dependencies with npm audit"
                } else {
                    "Audit dependencies for vulnerabilities"
                },
                Lint,
                Safe,
            ),
            Some("outdated") => s("npm", "List outdated dependencies", Other, Safe),
            Some("update") | Some("up") | Some("upgrade") => {
                s("npm", "Update dependencies", Install, Safe)
            }
            Some("link") => s("npm", "Link the package locally", Install, Safe),
            Some("cache") => s("npm", "Manage the npm cache", Clean, Safe),
            Some("prune") => s("npm", "Remove extraneous packages", Clean, Safe),
            Some("dedupe") | Some("ddp") => s("npm", "Deduplicate dependencies", Install, Safe),
            Some("rebuild") => s("npm", "Rebuild native dependencies", Install, Safe),
            Some("login") => s("npm", "Log in to the npm registry", Other, External),
            Some("deprecate") | Some("unpublish") => s(
                "npm",
                "Change the package on the npm registry",
                Publish,
                External,
            ),
            Some(x) => s("npm", format!("Run npm {x}"), Other, Safe),
            None => s("npm", "Run npm", Other, Safe),
        },
        "pnpm" => match sub {
            Some("install") | Some("i") | Some("add") | None => {
                s("pnpm", "Install dependencies with pnpm", Install, Safe)
            }
            Some("publish") => s("pnpm", "Publish the package to npm", Publish, External),
            Some("pack") => s("pnpm", "Create an npm tarball", Build, Safe),
            Some("audit") => s("pnpm", "Audit dependencies for vulnerabilities", Lint, Safe),
            Some("outdated") => s("pnpm", "List outdated dependencies", Other, Safe),
            Some("update") | Some("up") => s("pnpm", "Update dependencies", Install, Safe),
            Some("prune") => s("pnpm", "Remove unreferenced packages", Clean, Safe),
            Some("store") => s("pnpm", "Manage the pnpm store", Clean, Safe),
            Some("deploy") => s(
                "pnpm",
                "Copy a workspace package with its dependencies",
                Build,
                Safe,
            ),
            Some("dedupe") => s("pnpm", "Deduplicate dependencies", Install, Safe),
            Some("patch") | Some("patch-commit") => s("pnpm", "Patch a dependency", Install, Safe),
            Some("approve-builds") => s("pnpm", "Approve dependency build scripts", Install, Safe),
            Some(x) => s("pnpm", format!("Run pnpm {x}"), Other, Safe),
        },
        "yarn" => match sub {
            None | Some("install") | Some("add") => {
                s("yarn", "Install dependencies with Yarn", Install, Safe)
            }
            Some("publish") => s("yarn", "Publish the package to npm", Publish, External),
            Some("npm") if a.positionals().get(1) == Some(&"publish") => {
                s("yarn", "Publish the package to npm", Publish, External)
            }
            Some("pack") => s("yarn", "Create an npm tarball", Build, Safe),
            Some("audit") => s("yarn", "Audit dependencies for vulnerabilities", Lint, Safe),
            Some("upgrade") | Some("up") | Some("upgrade-interactive") => {
                s("yarn", "Update dependencies", Install, Safe)
            }
            Some("dedupe") => s("yarn", "Deduplicate dependencies", Install, Safe),
            Some("version") => s("yarn", "Bump the package version", Other, Safe),
            Some("constraints") => s("yarn", "Check workspace constraints", Lint, Safe),
            Some("dlx") => s("yarn", "Run a package with yarn dlx", Other, Safe),
            Some(x) => s("yarn", format!("Run yarn {x}"), Other, Safe),
        },
        "bun" => match sub {
            None | Some("install") | Some("i") | Some("add") => {
                s("bun", "Install dependencies with Bun", Install, Safe)
            }
            Some("test") => s("bun", "Run tests with Bun", Test, Safe),
            Some("build") => s("bun", "Bundle with Bun", Build, Safe),
            Some("publish") => s("bun", "Publish the package to npm", Publish, External),
            Some("update") => s("bun", "Update dependencies", Install, Safe),
            Some("outdated") => s("bun", "List outdated dependencies", Other, Safe),
            Some("pm") => s("bun", "Manage packages with Bun", Other, Safe),
            Some(x) => s("bun", format!("Run bun {x}"), Other, Safe),
        },
        "corepack" => s(
            "corepack",
            "Set up the package manager with Corepack",
            Install,
            Safe,
        ),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_sum as sum;
    use super::*;

    #[test]
    fn js_package_managers() {
        assert_eq!(sum("npm ci").text, "Install dependencies with npm");
        assert_eq!(sum("npm ci").kind, Install);
        assert_eq!(sum("npm publish").risk, External);
        assert_eq!(
            sum("npm audit fix").text,
            "Fix vulnerable dependencies with npm audit"
        );
        assert_eq!(sum("pnpm install --frozen-lockfile").kind, Install);
        assert_eq!(sum("pnpm publish -r").risk, External);
        assert_eq!(sum("yarn npm publish").risk, External);
        assert_eq!(sum("yarn").text, "Install dependencies with Yarn");
        assert_eq!(sum("bun test").kind, Test);
    }

    #[test]
    fn system_package_managers() {
        assert_eq!(sum("apt-get install -y curl").kind, Install);
        assert_eq!(sum("brew bundle").text, "Install the Brewfile packages");
        assert_eq!(sum("brew update").risk, Safe);
    }
}
