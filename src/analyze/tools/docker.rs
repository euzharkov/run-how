//! Docker: one slice of the tool knowledge table behind `analyze::tools::summarize`.

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
        "docker" | "podman" | "nerdctl" => {
            let name = program;
            if sub == Some("compose") {
                // Everything after the `compose` word is a Compose command line of its own.
                let idx = args.iter().position(|x| x == "compose").unwrap_or(0);
                return compose(&args[idx + 1..]);
            }
            match sub {
                Some("build") => s(name, "Build the Docker image", Container, Safe),
                Some("buildx") | Some("builder") => match a.positionals().get(1).copied() {
                    Some("prune") => s(name, "Delete the Docker build cache", Clean, Destructive),
                    Some("build") | None => s(name, "Build the Docker image", Container, Safe),
                    Some("bake") => s(
                        name,
                        "Build Docker images from a bake file",
                        Container,
                        Safe,
                    ),
                    Some(x) => s(
                        name,
                        format!("Run {name} {} {x}", sub.unwrap()),
                        Container,
                        Safe,
                    ),
                },
                Some("push") => s(
                    name,
                    "Push the Docker image to a registry",
                    Publish,
                    External,
                ),
                Some("pull") => s(name, "Pull a Docker image", Container, Safe),
                Some("run") => {
                    let pos = a.positionals();
                    s(
                        name,
                        format!(
                            "Run a {} container",
                            pos.get(1).copied().unwrap_or("Docker")
                        ),
                        Container,
                        Safe,
                    )
                }
                Some("exec") => s(
                    name,
                    format!(
                        "Run a command in the {} container",
                        a.positionals().get(1).copied().unwrap_or("running")
                    ),
                    Container,
                    Safe,
                ),
                Some("logs") => s(name, "Show container logs", Container, Safe),
                Some("ps") => s(name, "List containers", Container, Safe),
                Some("stop") => s(name, "Stop containers", Container, Safe),
                Some("start") => s(name, "Start containers", Container, Safe),
                Some("rm") => s(
                    name,
                    "Remove containers",
                    Container,
                    if a.has("-f") || a.has("--force") {
                        Destructive
                    } else {
                        Safe
                    },
                ),
                Some("rmi") => s(name, "Remove Docker images", Clean, Safe),
                Some("system") => match a.positionals().get(1).copied() {
                    Some("prune") => s(
                        name,
                        if a.has("-a") || a.has("--all") || a.has("--volumes") {
                            "Delete all unused Docker data"
                        } else {
                            "Delete unused Docker data"
                        },
                        Clean,
                        Destructive,
                    ),
                    Some("df") | Some("info") => {
                        s(name, "Show Docker system information", Container, Safe)
                    }
                    _ => s(name, "Manage the Docker system", Container, Safe),
                },
                Some("volume") => match a.positionals().get(1).copied() {
                    Some("rm") | Some("prune") => s(name, "Delete Docker volumes", Db, Destructive),
                    _ => s(name, "Manage Docker volumes", Container, Safe),
                },
                Some("image") => match a.positionals().get(1).copied() {
                    Some("prune") => s(name, "Delete unused Docker images", Clean, Safe),
                    Some("build") => s(name, "Build the Docker image", Container, Safe),
                    _ => s(name, "Manage Docker images", Container, Safe),
                },
                Some("container") => match a.positionals().get(1).copied() {
                    Some("prune") => s(name, "Delete stopped containers", Clean, Safe),
                    _ => s(name, "Manage Docker containers", Container, Safe),
                },
                Some("network") => s(name, "Manage Docker networks", Container, Safe),
                Some("login") => s(name, "Log in to a container registry", Other, External),
                Some("tag") => s(name, "Tag the Docker image", Container, Safe),
                Some("save") | Some("load") => {
                    s(name, "Transfer a Docker image archive", Container, Safe)
                }
                Some("context") => s(name, "Switch the Docker context", Container, Safe),
                Some("machine") if name == "podman" => s(
                    name,
                    format!(
                        "{} the Podman machine",
                        match a.positionals().get(1).copied() {
                            Some("start") => "Start",
                            Some("stop") => "Stop",
                            Some("init") => "Create",
                            _ => "Manage",
                        }
                    ),
                    Infra,
                    Safe,
                ),
                Some("info") | Some("version") => {
                    s(name, "Show Docker information", Container, Safe)
                }
                Some(x) => s(name, format!("Run {name} {x}"), Container, Safe),
                None => s(name, format!("Run {name}"), Container, Safe),
            }
        }
        "docker-compose" | "podman-compose" => compose(args),
        "colima" => s(
            "colima",
            format!(
                "{} the Colima Docker runtime",
                match sub {
                    Some("start") => "Start",
                    Some("stop") => "Stop",
                    Some("delete") => "Delete",
                    Some("status") => "Show status of",
                    _ => "Manage",
                }
            ),
            Infra,
            if sub == Some("delete") {
                Destructive
            } else {
                Safe
            },
        ),
        "orb" | "orbctl" => s(
            "orbstack",
            format!(
                "{} OrbStack",
                match sub {
                    Some("start") => "Start",
                    Some("stop") => "Stop",
                    _ => "Manage",
                }
            ),
            Infra,
            Safe,
        ),
        "lima" | "limactl" => s("lima", "Manage the Lima VM", Infra, Safe),
        "dive" => s("dive", "Inspect Docker image layers", Container, Safe),
        "hadolint" => s("hadolint", "Lint the Dockerfile", Lint, Safe),
        "trivy" | "grype" => s(program, "Scan for vulnerabilities", Lint, Safe),
        "act" => s("act", "Run GitHub Actions workflows locally", Test, Safe),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_sum as sum;
    use super::*;

    #[test]
    fn docker_arms() {
        assert_eq!(sum("docker build -t app .").text, "Build the Docker image");
        assert_eq!(sum("docker push ghcr.io/x/app").risk, External);
        assert_eq!(sum("docker push ghcr.io/x/app").kind, Publish);
        assert_eq!(sum("docker run --rm -it app").text, "Run a app container");
        assert_eq!(
            sum("docker system prune -af").text,
            "Delete all unused Docker data"
        );
        assert_eq!(sum("docker volume prune").risk, Destructive);
        assert_eq!(sum("docker image prune").risk, Safe);
    }

    #[test]
    fn build_cache_prune_matches_system_prune() {
        assert_eq!(
            sum("docker builder prune").text,
            "Delete the Docker build cache"
        );
        assert_eq!(sum("docker builder prune -af").risk, Destructive);
        assert_eq!(sum("docker buildx prune").risk, Destructive);
        assert_eq!(sum("docker buildx build .").risk, Safe);
        assert_eq!(sum("docker buildx build .").text, "Build the Docker image");
    }

    #[test]
    fn global_value_flags_do_not_shift_the_subcommand() {
        assert_eq!(sum("docker -H ssh://box push app").risk, External);
        assert_eq!(sum("docker --context prod ps").text, "List containers");
        let c = sum("docker compose -f x.yml down -v");
        assert_eq!(c.risk, Destructive);
        assert_eq!(c.text, "Stop Docker services and delete their volumes");
        assert_eq!(
            sum("docker compose -f a.yml -f b.yml --profile dev up -d api").text,
            "Start the api service in the background"
        );
        assert_eq!(sum("docker compose rm -fsv").risk, Destructive);
        assert_eq!(sum("docker compose rm -f").risk, Safe);
        assert_eq!(sum("colima delete").risk, Destructive);
    }
}
