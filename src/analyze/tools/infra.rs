//! Kubernetes / IaC: one slice of the tool knowledge table behind `analyze::tools::summarize`.

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
        "kubectl" | "oc" => {
            let p = a.positionals();
            let dry = a.has("--dry-run");
            let target = p.iter().skip(1).find(|x| !x.starts_with('-')).copied();
            let ns = a
                .value("-n")
                .or_else(|| a.value("--namespace"))
                .map(|n| format!(" in namespace {n}"))
                .unwrap_or_default();
            match sub {
                Some("apply") => {
                    if dry {
                        s(
                            "kubectl",
                            "Validate Kubernetes manifests (dry run)",
                            Infra,
                            Safe,
                        )
                    } else {
                        s(
                            "kubectl",
                            format!("Apply Kubernetes resources{ns}"),
                            Deploy,
                            External,
                        )
                    }
                }
                Some("delete") => s(
                    "kubectl",
                    format!("Delete Kubernetes resources{ns}"),
                    Infra,
                    Destructive,
                ),
                Some("create") => s(
                    "kubectl",
                    if dry {
                        "Generate a Kubernetes manifest (dry run)".into()
                    } else {
                        format!("Create Kubernetes resources{ns}")
                    },
                    Infra,
                    if dry { Safe } else { External },
                ),
                Some("get") | Some("describe") | Some("top") | Some("events") => s(
                    "kubectl",
                    format!("Query {} from the cluster", target.unwrap_or("resources")),
                    Infra,
                    External,
                ),
                Some("logs") => s(
                    "kubectl",
                    format!("Stream logs from {}", target.unwrap_or("a pod")),
                    Infra,
                    External,
                ),
                Some("exec") => s(
                    "kubectl",
                    format!("Run a command in {}", target.unwrap_or("a pod")),
                    Infra,
                    External,
                ),
                Some("port-forward") => s(
                    "kubectl",
                    format!("Forward a local port to {}", target.unwrap_or("a pod")),
                    Infra,
                    External,
                ),
                Some("rollout") => s(
                    "kubectl",
                    format!(
                        "{} a rollout{ns}",
                        match p.get(1).copied() {
                            Some("restart") => "Restart",
                            Some("status") => "Check",
                            Some("undo") => "Roll back",
                            Some("history") => "Show history of",
                            _ => "Manage",
                        }
                    ),
                    Deploy,
                    External,
                ),
                Some("scale") => s(
                    "kubectl",
                    format!("Scale {}{ns}", target.unwrap_or("a deployment")),
                    Infra,
                    External,
                ),
                Some("kustomize") => s(
                    "kubectl",
                    format!(
                        "Render Kustomize manifests from {}",
                        target.map(clean_path).unwrap_or_else(|| ".".into())
                    ),
                    Infra,
                    Safe,
                ),
                Some("diff") => s(
                    "kubectl",
                    "Diff manifests against the cluster",
                    Infra,
                    External,
                ),
                Some("config") => s("kubectl", "Change the kubectl context", Infra, Safe),
                Some("wait") => s(
                    "kubectl",
                    "Wait for a Kubernetes condition",
                    Infra,
                    External,
                ),
                Some("patch") | Some("edit") | Some("set") | Some("label") | Some("annotate")
                | Some("replace") => s(
                    "kubectl",
                    format!("Modify {} in the cluster", target.unwrap_or("resources")),
                    Infra,
                    External,
                ),
                Some("drain") | Some("cordon") | Some("taint") => {
                    s("kubectl", "Change cluster node scheduling", Infra, External)
                }
                Some("cp") => s("kubectl", "Copy files to or from a pod", Infra, External),
                Some("run") => s("kubectl", "Run a pod in the cluster", Infra, External),
                Some("version") | Some("api-resources") | Some("explain") => {
                    s("kubectl", "Show kubectl information", Infra, Safe)
                }
                Some(x) => s("kubectl", format!("Run kubectl {x}"), Infra, External),
                None => s("kubectl", "Run kubectl", Infra, Safe),
            }
        }
        "kustomize" => match sub {
            Some("build") => s(
                "kustomize",
                format!(
                    "Render Kustomize manifests from {}",
                    a.positionals()
                        .get(1)
                        .map(|p| clean_path(p))
                        .unwrap_or_else(|| ".".into())
                ),
                Infra,
                Safe,
            ),
            Some("edit") => s("kustomize", "Edit the kustomization", Infra, Safe),
            Some(x) => s("kustomize", format!("Run kustomize {x}"), Infra, Safe),
            None => s("kustomize", "Run Kustomize", Infra, Safe),
        },
        "helm" => {
            let p = a.positionals();
            match sub {
                Some("template") => s("helm", "Render the Helm chart templates", Infra, Safe),
                Some("lint") => s("helm", "Validate the Helm chart", Lint, Safe),
                Some("install") => s(
                    "helm",
                    format!(
                        "Install the {} Helm release",
                        p.get(1).copied().unwrap_or("")
                    )
                    .trim()
                    .to_string(),
                    Deploy,
                    External,
                ),
                Some("upgrade") => s(
                    "helm",
                    if a.has("--install") || a.has("-i") {
                        format!(
                            "Install or upgrade the {} Helm release",
                            p.get(1).copied().unwrap_or("")
                        )
                    } else {
                        format!(
                            "Upgrade the {} Helm release",
                            p.get(1).copied().unwrap_or("")
                        )
                    }
                    .trim()
                    .to_string(),
                    Deploy,
                    External,
                ),
                Some("uninstall") | Some("delete") | Some("del") => s(
                    "helm",
                    format!(
                        "Uninstall the {} Helm release",
                        p.get(1).copied().unwrap_or("")
                    )
                    .trim()
                    .to_string(),
                    Infra,
                    Destructive,
                ),
                Some("rollback") => s("helm", "Roll back a Helm release", Deploy, External),
                Some("dependency") | Some("dep") => {
                    s("helm", "Update Helm chart dependencies", Install, Safe)
                }
                Some("package") => s("helm", "Package the Helm chart", Build, Safe),
                Some("push") => s(
                    "helm",
                    "Push the Helm chart to a registry",
                    Publish,
                    External,
                ),
                Some("repo") => s("helm", "Manage Helm repositories", Install, Safe),
                Some("list") | Some("ls") | Some("status") | Some("get") | Some("history") => s(
                    "helm",
                    "Query Helm releases in the cluster",
                    Infra,
                    External,
                ),
                Some("test") => s(
                    "helm",
                    "Run Helm release tests in the cluster",
                    Test,
                    External,
                ),
                Some("diff") => s(
                    "helm",
                    "Diff the Helm release against the cluster",
                    Infra,
                    External,
                ),
                Some(x) => s("helm", format!("Run helm {x}"), Infra, Safe),
                None => s("helm", "Run Helm", Infra, Safe),
            }
        }
        "helmfile" => match sub {
            Some("apply") | Some("sync") => s(
                "helmfile",
                "Apply Helm releases with Helmfile",
                Deploy,
                External,
            ),
            Some("destroy") | Some("delete") => s(
                "helmfile",
                "Destroy Helm releases with Helmfile",
                Infra,
                Destructive,
            ),
            Some("diff") => s(
                "helmfile",
                "Diff Helm releases with Helmfile",
                Infra,
                External,
            ),
            Some("template") => s(
                "helmfile",
                "Render Helm releases with Helmfile",
                Infra,
                Safe,
            ),
            Some("lint") => s("helmfile", "Lint Helm releases with Helmfile", Lint, Safe),
            Some(x) => s("helmfile", format!("Run helmfile {x}"), Infra, Safe),
            None => s("helmfile", "Run Helmfile", Infra, Safe),
        },
        "skaffold" => match sub {
            Some("dev") => s(
                "skaffold",
                "Build and deploy to the cluster on change",
                Deploy,
                External,
            ),
            Some("run") | Some("deploy") => s(
                "skaffold",
                "Build and deploy to the cluster with Skaffold",
                Deploy,
                External,
            ),
            Some("build") => s("skaffold", "Build images with Skaffold", Build, Safe),
            Some("render") => s("skaffold", "Render manifests with Skaffold", Infra, Safe),
            Some("delete") => s(
                "skaffold",
                "Delete Skaffold-deployed resources",
                Infra,
                Destructive,
            ),
            Some(x) => s("skaffold", format!("Run skaffold {x}"), Infra, Safe),
            None => s("skaffold", "Run Skaffold", Infra, Safe),
        },
        "tilt" => match sub {
            Some("up") => s(
                "tilt",
                "Start the Tilt development environment",
                Dev,
                External,
            ),
            Some("down") => s("tilt", "Tear down Tilt resources", Infra, Destructive),
            Some("ci") => s("tilt", "Run Tilt in CI mode", Deploy, External),
            Some(x) => s("tilt", format!("Run tilt {x}"), Infra, Safe),
            None => s("tilt", "Run Tilt", Infra, Safe),
        },
        "minikube" | "kind" | "k3d" => match sub {
            Some("start") | Some("create") => s(
                program,
                format!("Create a local Kubernetes cluster with {program}"),
                Infra,
                Safe,
            ),
            Some("stop") => s(program, format!("Stop the {program} cluster"), Infra, Safe),
            Some("delete") => s(
                program,
                format!("Delete the {program} cluster"),
                Infra,
                Destructive,
            ),
            Some("load") | Some("image") => s(
                program,
                format!("Load an image into the {program} cluster"),
                Infra,
                Safe,
            ),
            Some(x) => s(program, format!("Run {program} {x}"), Infra, Safe),
            None => s(program, format!("Run {program}"), Infra, Safe),
        },
        "kubeconform" | "kubeval" => s(
            program,
            "Validate Kubernetes manifests against schemas",
            Lint,
            Safe,
        ),
        "kube-linter" | "kube-score" | "polaris" => {
            s(program, "Lint Kubernetes manifests", Lint, Safe)
        }
        "argocd" => match sub {
            Some("app") => match a.positionals().get(1).copied() {
                Some("sync") => s("argocd", "Sync the Argo CD application", Deploy, External),
                Some("delete") => s(
                    "argocd",
                    "Delete the Argo CD application",
                    Infra,
                    Destructive,
                ),
                _ => s("argocd", "Manage Argo CD applications", Infra, External),
            },
            _ => s("argocd", "Run Argo CD", Infra, External),
        },
        "terraform" | "tofu" | "terragrunt" => {
            let name = if program == "tofu" {
                "OpenTofu"
            } else if program == "terragrunt" {
                "Terragrunt"
            } else {
                "Terraform"
            };
            match sub {
                Some("init") => s(
                    program,
                    format!("Initialise the {name} working directory"),
                    Infra,
                    Safe,
                ),
                Some("plan") => s(
                    program,
                    if a.has("-destroy") {
                        format!("Plan a {name} destroy")
                    } else {
                        format!("Plan {name} changes")
                    },
                    Infra,
                    External,
                ),
                Some("apply") => s(
                    program,
                    format!("Apply {name} changes to infrastructure"),
                    Deploy,
                    External,
                ),
                Some("destroy") => s(
                    program,
                    format!("Destroy {name}-managed infrastructure"),
                    Infra,
                    Destructive,
                ),
                Some("fmt") => s(
                    program,
                    if a.has("-check") {
                        format!("Check {name} formatting")
                    } else {
                        format!("Format {name} files")
                    },
                    Format,
                    Safe,
                ),
                Some("validate") => s(
                    program,
                    format!("Validate the {name} configuration"),
                    Lint,
                    Safe,
                ),
                Some("output") | Some("show") | Some("state") => {
                    s(program, format!("Inspect {name} state"), Infra, External)
                }
                Some("import") => s(
                    program,
                    format!("Import a resource into {name} state"),
                    Infra,
                    External,
                ),
                Some("taint") | Some("untaint") => s(
                    program,
                    format!("Mark a {name} resource for recreation"),
                    Infra,
                    External,
                ),
                Some("workspace") => {
                    s(program, format!("Switch the {name} workspace"), Infra, Safe)
                }
                Some("run-all") => s(
                    program,
                    format!("Run {name} across all modules"),
                    Infra,
                    External,
                ),
                Some("test") => s(program, format!("Run {name} tests"), Test, External),
                Some(x) => s(program, format!("Run {} {x}", program), Infra, Safe),
                None => s(program, format!("Run {name}"), Infra, Safe),
            }
        }
        "tflint" => s("tflint", "Lint Terraform files", Lint, Safe),
        "tfsec" | "checkov" => s(
            program,
            "Scan infrastructure code for security issues",
            Lint,
            Safe,
        ),
        "pulumi" => match sub {
            Some("up") | Some("update") => s("pulumi", "Deploy the Pulumi stack", Deploy, External),
            Some("preview") | Some("pre") => s("pulumi", "Preview Pulumi changes", Infra, External),
            Some("destroy") | Some("down") | Some("dn") => {
                s("pulumi", "Destroy the Pulumi stack", Infra, Destructive)
            }
            Some("refresh") => s("pulumi", "Refresh Pulumi state", Infra, External),
            Some("stack") => s("pulumi", "Manage Pulumi stacks", Infra, External),
            Some(x) => s("pulumi", format!("Run pulumi {x}"), Infra, External),
            None => s("pulumi", "Run Pulumi", Infra, Safe),
        },
        "cdk" | "cdktf" => match sub {
            Some("deploy") => s(program, "Deploy the CDK stack", Deploy, External),
            Some("destroy") => s(program, "Destroy the CDK stack", Infra, Destructive),
            Some("synth") | Some("synthesize") => {
                s(program, "Synthesize the CDK stack", Build, Safe)
            }
            Some("diff") => s(
                program,
                "Diff the CDK stack against deployed state",
                Infra,
                External,
            ),
            Some("bootstrap") => s(program, "Bootstrap the CDK environment", Infra, External),
            Some(x) => s(program, format!("Run {program} {x}"), Infra, Safe),
            None => s(program, "Run CDK", Infra, Safe),
        },
        "sam" => match sub {
            Some("build") => s("sam", "Build the SAM application", Build, Safe),
            Some("deploy") => s("sam", "Deploy the SAM application", Deploy, External),
            Some("local") => s("sam", "Run the SAM application locally", Dev, Safe),
            Some("delete") => s("sam", "Delete the SAM stack", Infra, Destructive),
            Some(x) => s("sam", format!("Run sam {x}"), Infra, Safe),
            None => s("sam", "Run SAM", Infra, Safe),
        },
        "serverless" | "sls" => match sub {
            Some("deploy") => s(
                "serverless",
                "Deploy with the Serverless Framework",
                Deploy,
                External,
            ),
            Some("remove") => s(
                "serverless",
                "Remove the Serverless deployment",
                Infra,
                Destructive,
            ),
            Some("offline") => s("serverless", "Run the Serverless app locally", Dev, Safe),
            Some("invoke") => s(
                "serverless",
                "Invoke a Serverless function",
                Infra,
                External,
            ),
            Some("logs") => s(
                "serverless",
                "Stream Serverless function logs",
                Infra,
                External,
            ),
            Some(x) => s("serverless", format!("Run serverless {x}"), Infra, Safe),
            None => s("serverless", "Run the Serverless Framework", Infra, Safe),
        },
        "ansible-playbook" => s(
            "ansible",
            format!(
                "Run the {} Ansible playbook",
                a.positionals()
                    .first()
                    .map(|p| clean_path(p))
                    .unwrap_or_default()
            )
            .trim()
            .to_string(),
            Deploy,
            if a.has("--check") { Safe } else { External },
        ),
        "ansible" | "ansible-galaxy" | "ansible-lint" => s(
            "ansible",
            if program == "ansible-lint" {
                "Lint Ansible playbooks"
            } else if program == "ansible-galaxy" {
                "Install Ansible roles and collections"
            } else {
                "Run Ansible ad-hoc commands"
            },
            if program == "ansible-lint" {
                Lint
            } else {
                Infra
            },
            if program == "ansible" { External } else { Safe },
        ),
        "packer" => s(
            "packer",
            if sub == Some("build") {
                "Build machine images with Packer"
            } else {
                "Run Packer"
            },
            Build,
            if sub == Some("build") { External } else { Safe },
        ),
        "vagrant" => s(
            "vagrant",
            format!(
                "{} the Vagrant VM",
                match sub {
                    Some("up") => "Start",
                    Some("halt") => "Stop",
                    Some("destroy") => "Destroy",
                    Some("ssh") => "Connect to",
                    Some("provision") => "Provision",
                    _ => "Manage",
                }
            ),
            Infra,
            if sub == Some("destroy") {
                Destructive
            } else {
                Safe
            },
        ),
        "aws" => {
            let p = a.positionals();
            let joined = p.join(" ");
            let destructive = ["delete", "terminate", "remove", "rm ", "purge", "rb "];
            let read = [
                "describe",
                "list",
                "get",
                "ls",
                "sts get-caller-identity",
                "logs tail",
            ];
            if destructive.iter().any(|d| joined.contains(d)) {
                s(
                    "aws",
                    format!(
                        "Delete AWS resources ({})",
                        p.first().copied().unwrap_or("")
                    ),
                    Infra,
                    Destructive,
                )
            } else if read.iter().any(|r| joined.contains(r)) {
                s(
                    "aws",
                    format!("Query AWS {}", p.first().copied().unwrap_or("resources")),
                    Infra,
                    External,
                )
            } else {
                s(
                    "aws",
                    format!(
                        "Call AWS {}",
                        p.iter().take(2).copied().collect::<Vec<_>>().join(" ")
                    ),
                    Infra,
                    External,
                )
            }
        }
        "gcloud" | "az" | "doctl" | "linode-cli" | "hcloud" | "oci" => {
            let p = a.positionals();
            let joined = p.join(" ");
            if joined.contains("delete") || joined.contains("destroy") || joined.contains("remove")
            {
                s(
                    program,
                    format!("Delete cloud resources with {program}"),
                    Infra,
                    Destructive,
                )
            } else if joined.starts_with("auth")
                || joined.starts_with("login")
                || joined.starts_with("config")
            {
                s(
                    program,
                    format!("Configure {program} authentication"),
                    Infra,
                    External,
                )
            } else {
                s(
                    program,
                    format!(
                        "Call {program} {}",
                        p.iter().take(2).copied().collect::<Vec<_>>().join(" ")
                    ),
                    Infra,
                    External,
                )
            }
        }
        "fly" | "flyctl" => match sub {
            Some("deploy") => s("fly", "Deploy the app to Fly.io", Deploy, External),
            Some("launch") => s("fly", "Create and deploy a Fly.io app", Deploy, External),
            Some("logs") => s("fly", "Stream Fly.io logs", Infra, External),
            Some("ssh") => s("fly", "Connect to the Fly.io machine", Infra, External),
            Some("scale") => s("fly", "Scale the Fly.io app", Infra, External),
            Some("destroy") | Some("apps") if a.has("destroy") || sub == Some("destroy") => {
                s("fly", "Destroy the Fly.io app", Infra, Destructive)
            }
            Some(x) => s("fly", format!("Run fly {x}"), Infra, External),
            None => s("fly", "Run flyctl", Infra, Safe),
        },
        "vercel" | "vc" => match sub {
            Some("dev") => s("vercel", "Run the app locally with Vercel", Dev, Safe),
            Some("build") => s("vercel", "Build the app for Vercel", Build, Safe),
            Some("deploy") | None => s(
                "vercel",
                if a.has("--prod") {
                    "Deploy to Vercel production"
                } else {
                    "Deploy a preview to Vercel"
                },
                Deploy,
                External,
            ),
            Some("pull") | Some("env") => {
                s("vercel", "Sync Vercel project settings", Infra, External)
            }
            Some("link") => s("vercel", "Link the Vercel project", Infra, External),
            Some(x) => s("vercel", format!("Run vercel {x}"), Infra, External),
        },
        "netlify" | "ntl" => match sub {
            Some("dev") => s("netlify", "Run the app locally with Netlify", Dev, Safe),
            Some("build") => s("netlify", "Build the app with Netlify", Build, Safe),
            Some("deploy") => s(
                "netlify",
                if a.has("--prod") {
                    "Deploy to Netlify production"
                } else {
                    "Deploy a preview to Netlify"
                },
                Deploy,
                External,
            ),
            Some(x) => s("netlify", format!("Run netlify {x}"), Infra, External),
            None => s("netlify", "Run Netlify", Infra, Safe),
        },
        "wrangler" => match sub {
            Some("dev") => s("wrangler", "Run the Cloudflare Worker locally", Dev, Safe),
            Some("deploy") | Some("publish") => {
                s("wrangler", "Deploy the Cloudflare Worker", Deploy, External)
            }
            Some("pages") => match a.positionals().get(1).copied() {
                Some("dev") => s("wrangler", "Run Cloudflare Pages locally", Dev, Safe),
                Some("deploy") | Some("publish") => {
                    s("wrangler", "Deploy to Cloudflare Pages", Deploy, External)
                }
                _ => s("wrangler", "Manage Cloudflare Pages", Infra, External),
            },
            Some("d1") => {
                let p = a.positionals();
                if p.contains(&"migrations") && p.contains(&"apply") {
                    s(
                        "wrangler",
                        if a.has("--local") {
                            "Apply D1 migrations locally"
                        } else {
                            "Apply D1 migrations"
                        },
                        Migrate,
                        if a.has("--local") { Safe } else { External },
                    )
                } else {
                    s(
                        "wrangler",
                        "Manage the D1 database",
                        Db,
                        if a.has("--local") { Safe } else { External },
                    )
                }
            }
            Some("tail") => s("wrangler", "Stream Cloudflare Worker logs", Infra, External),
            Some("delete") => s(
                "wrangler",
                "Delete the Cloudflare Worker",
                Infra,
                Destructive,
            ),
            Some("types") => s(
                "wrangler",
                "Generate Cloudflare Worker types",
                Generate,
                Safe,
            ),
            Some(x) => s("wrangler", format!("Run wrangler {x}"), Infra, External),
            None => s("wrangler", "Run Wrangler", Infra, Safe),
        },
        "firebase" => match sub {
            Some("deploy") => s("firebase", "Deploy to Firebase", Deploy, External),
            Some("emulators:start") => s("firebase", "Start the Firebase emulators", Dev, Safe),
            Some("serve") => s("firebase", "Serve the Firebase project locally", Dev, Safe),
            Some(x) => s("firebase", format!("Run firebase {x}"), Infra, External),
            None => s("firebase", "Run Firebase", Infra, Safe),
        },
        "supabase" => match sub {
            Some("start") => s("supabase", "Start the local Supabase stack", Dev, Safe),
            Some("stop") => s("supabase", "Stop the local Supabase stack", Infra, Safe),
            Some("db") => match a.positionals().get(1).copied() {
                Some("reset") => s(
                    "supabase",
                    "Reset the local Supabase database",
                    Db,
                    Destructive,
                ),
                Some("push") => s(
                    "supabase",
                    "Push migrations to the remote Supabase database",
                    Migrate,
                    External,
                ),
                Some("pull") => s("supabase", "Pull the remote Supabase schema", Db, External),
                Some("diff") => s("supabase", "Diff the Supabase schema", Db, Safe),
                _ => s("supabase", "Manage the Supabase database", Db, Safe),
            },
            Some("gen") => s("supabase", "Generate Supabase types", Generate, Safe),
            Some("functions") => match a.positionals().get(1).copied() {
                Some("serve") => s(
                    "supabase",
                    "Serve Supabase edge functions locally",
                    Dev,
                    Safe,
                ),
                Some("deploy") => s(
                    "supabase",
                    "Deploy Supabase edge functions",
                    Deploy,
                    External,
                ),
                _ => s("supabase", "Manage Supabase edge functions", Infra, Safe),
            },
            Some("migration") => s("supabase", "Manage Supabase migrations", Migrate, Safe),
            Some(x) => s("supabase", format!("Run supabase {x}"), Infra, Safe),
            None => s("supabase", "Run Supabase", Infra, Safe),
        },
        "heroku" => match sub {
            Some("local") => s("heroku", "Run the app locally with Heroku", Dev, Safe),
            Some("logs") => s("heroku", "Stream Heroku logs", Infra, External),
            Some("run") => s("heroku", "Run a command on Heroku", Infra, External),
            Some(x) => s("heroku", format!("Run heroku {x}"), Infra, External),
            None => s("heroku", "Run Heroku", Infra, Safe),
        },
        "railway" | "render" | "dokku" | "caprover" => s(
            program,
            format!("Run {program} {}", sub.unwrap_or(""))
                .trim()
                .to_string(),
            Infra,
            External,
        ),
        "gh" => match (sub, a.positionals().get(1).copied()) {
            (Some("release"), Some("create")) => {
                s("gh", "Create a GitHub release", Publish, External)
            }
            (Some("release"), Some("upload")) => {
                s("gh", "Upload assets to a GitHub release", Publish, External)
            }
            (Some("release"), Some("delete")) => {
                s("gh", "Delete a GitHub release", Publish, Destructive)
            }
            (Some("pr"), Some("create")) => s("gh", "Create a pull request", Other, External),
            (Some("pr"), Some("merge")) => s("gh", "Merge a pull request", Other, External),
            (Some("workflow"), Some("run")) => {
                s("gh", "Trigger a GitHub Actions workflow", Other, External)
            }
            (Some("run"), _) => s("gh", "Inspect GitHub Actions runs", Other, External),
            (Some("auth"), _) => s("gh", "Authenticate with GitHub", Other, External),
            (Some("api"), _) => s("gh", "Call the GitHub API", Other, External),
            (Some("repo"), Some("delete")) => {
                s("gh", "Delete a GitHub repository", Other, Destructive)
            }
            (Some(x), _) => s("gh", format!("Run gh {x}"), Other, External),
            (None, _) => s("gh", "Run the GitHub CLI", Other, Safe),
        },
        "git" => {
            let p = a.positionals();
            match sub {
                Some("push") => s(
                    "git",
                    if a.has("--force") || a.has("-f") || a.has("--force-with-lease") {
                        "Force-push commits to the remote"
                    } else if a.has("--tags") {
                        "Push tags to the remote"
                    } else {
                        "Push commits to the remote"
                    },
                    Publish,
                    if a.has("--force") || a.has("-f") {
                        Destructive
                    } else {
                        External
                    },
                ),
                Some("pull") | Some("fetch") => {
                    s("git", "Fetch changes from the remote", Other, External)
                }
                Some("clone") => s("git", "Clone a repository", Other, External),
                Some("clean") => s(
                    "git",
                    "Delete untracked files",
                    Clean,
                    if a.has("-x")
                        || a.has("-fdx")
                        || a.has("-xdf")
                        || a.has("-fxd")
                        || a.has("-dfx")
                        || a.has("-xfd")
                        || a.has("-dxf")
                    {
                        Destructive
                    } else {
                        Safe
                    },
                ),
                Some("reset") => s(
                    "git",
                    if a.has("--hard") {
                        "Discard all local changes"
                    } else {
                        "Reset the Git index"
                    },
                    Other,
                    if a.has("--hard") { Destructive } else { Safe },
                ),
                Some("checkout") | Some("restore") | Some("switch") => {
                    s("git", "Check out files or a branch", Other, Safe)
                }
                Some("tag") => s("git", "Create a Git tag", Other, Safe),
                Some("commit") => s("git", "Create a commit", Other, Safe),
                Some("add") => s("git", "Stage files", Other, Safe),
                Some("status") | Some("diff") | Some("log") | Some("rev-parse")
                | Some("describe") | Some("branch") | Some("show") | Some("ls-files") => {
                    s("git", "Inspect the Git repository", Other, Safe)
                }
                Some("submodule") => s(
                    "git",
                    if p.get(1) == Some(&"update") {
                        "Update Git submodules"
                    } else {
                        "Manage Git submodules"
                    },
                    Install,
                    External,
                ),
                Some("lfs") => s("git", "Manage Git LFS files", Install, Safe),
                Some("stash") => s("git", "Stash local changes", Other, Safe),
                Some("config") => s("git", "Change Git configuration", Other, Safe),
                Some("rebase") | Some("merge") | Some("cherry-pick") => {
                    s("git", format!("Run git {}", sub.unwrap()), Other, Safe)
                }
                Some("cliff") => s(
                    "git-cliff",
                    "Generate the changelog with git-cliff",
                    Docs,
                    Safe,
                ),
                Some(x) => s("git", format!("Run git {x}"), Other, Safe),
                None => s("git", "Run git", Other, Safe),
            }
        }
        "git-cliff" => s(
            "git-cliff",
            "Generate the changelog with git-cliff",
            Docs,
            Safe,
        ),
        "kafka-topics" | "kafka-topics.sh" => {
            if a.has("--delete") {
                s("kafka", "Delete Kafka topics", Infra, Destructive)
            } else if a.has("--create") {
                s(
                    "kafka",
                    format!(
                        "Create the {} Kafka topic",
                        a.value("--topic").unwrap_or("")
                    )
                    .trim()
                    .to_string(),
                    Infra,
                    External,
                )
            } else {
                s("kafka", "List Kafka topics", Infra, External)
            }
        }
        "kafka-console-producer" | "kafka-console-producer.sh" => s(
            "kafka",
            "Produce messages to a Kafka topic",
            Infra,
            External,
        ),
        "kafka-console-consumer" | "kafka-console-consumer.sh" => s(
            "kafka",
            "Consume messages from a Kafka topic",
            Infra,
            External,
        ),
        "kafka-consumer-groups" | "kafka-consumer-groups.sh" => s(
            "kafka",
            if a.has("--reset-offsets") {
                "Reset Kafka consumer group offsets"
            } else if a.has("--delete") {
                "Delete Kafka consumer groups"
            } else {
                "Inspect Kafka consumer groups"
            },
            Infra,
            if a.has("--reset-offsets") || a.has("--delete") {
                Destructive
            } else {
                External
            },
        ),
        "kcat" | "kafkacat" => s("kafka", "Interact with Kafka via kcat", Infra, External),
        "rpk" => {
            let p = a.positionals();
            if p.contains(&"delete") {
                s("redpanda", "Delete Redpanda resources", Infra, Destructive)
            } else if p.first() == Some(&"container") {
                s(
                    "redpanda",
                    "Manage the local Redpanda container",
                    Infra,
                    Safe,
                )
            } else {
                s(
                    "redpanda",
                    "Interact with Redpanda via rpk",
                    Infra,
                    External,
                )
            }
        }
        "fastlane" => {
            let p = a.positionals();
            let lane = if p.len() >= 2 && matches!(p[0], "ios" | "android" | "mac") {
                format!("{} {}", p[0], p[1])
            } else {
                p.first().copied().unwrap_or("").to_string()
            };
            let l = lane.to_ascii_lowercase();
            if l.contains("release")
                || l.contains("deploy")
                || l.contains("beta")
                || l.contains("testflight")
                || l.contains("upload")
                || l.contains("submit")
                || l.contains("distribute")
                || l.contains("publish")
                || l == "pilot"
                || l == "deliver"
                || l == "supply"
            {
                s(
                    "fastlane",
                    format!("Run the {lane} Fastlane lane (uploads a build)"),
                    Deploy,
                    External,
                )
            } else if l.contains("test") || l == "scan" {
                s(
                    "fastlane",
                    format!("Run the {lane} Fastlane lane"),
                    Test,
                    Safe,
                )
            } else if l == "match" || l == "cert" || l == "sigh" {
                s(
                    "fastlane",
                    "Sync code-signing certificates and profiles",
                    Other,
                    External,
                )
            } else if lane.is_empty() {
                s("fastlane", "Run Fastlane", Other, Safe)
            } else {
                s(
                    "fastlane",
                    format!("Run the {lane} Fastlane lane"),
                    Other,
                    Safe,
                )
            }
        }
        "pod" => match sub {
            Some("install") => s("cocoapods", "Install CocoaPods dependencies", Install, Safe),
            Some("update") => s("cocoapods", "Update CocoaPods dependencies", Install, Safe),
            Some(x) => s("cocoapods", format!("Run pod {x}"), Other, Safe),
            None => s("cocoapods", "Run CocoaPods", Other, Safe),
        },
        "xcrun" => match a.positionals().first().copied() {
            Some("simctl") => s("xcode", "Control the iOS Simulator", Other, Safe),
            Some("xcodebuild") => s("xcode", "Build with Xcode", Build, Safe),
            Some("altool") | Some("notarytool") => {
                s("xcode", "Upload or notarize with Apple", Publish, External)
            }
            Some(x) => s("xcode", format!("Run xcrun {x}"), Other, Safe),
            None => s("xcode", "Run xcrun", Other, Safe),
        },
        "swiftlint" => s(
            "swiftlint",
            if a.has("--fix") || a.has("autocorrect") {
                "Fix Swift lint issues with SwiftLint"
            } else {
                "Check Swift source with SwiftLint"
            },
            Lint,
            Safe,
        ),
        "swiftformat" => s(
            "swiftformat",
            "Format Swift source with SwiftFormat",
            Format,
            Safe,
        ),
        "xcodegen" => s(
            "xcodegen",
            "Generate the Xcode project with XcodeGen",
            Generate,
            Safe,
        ),
        "tuist" => s(
            "tuist",
            format!("Run tuist {}", sub.unwrap_or("generate")),
            Generate,
            Safe,
        ),
        "adb" => match a.positionals().first().copied() {
            Some("install") => s("adb", "Install the APK on a device", Dev, Safe),
            Some("logcat") => s("adb", "Stream Android device logs", Dev, Safe),
            Some("reverse") => s("adb", "Forward a device port to the host", Dev, Safe),
            Some("shell") => s("adb", "Run a command on the Android device", Other, Safe),
            Some("uninstall") => s("adb", "Uninstall the app from the device", Other, Safe),
            Some(x) => s("adb", format!("Run adb {x}"), Other, Safe),
            None => s("adb", "Run adb", Other, Safe),
        },
        "emulator" => s("android", "Start an Android emulator", Dev, Safe),
        "expo-doctor" => s(
            "expo",
            "Check the Expo project for common issues",
            Lint,
            Safe,
        ),
        "sbt" => match sub {
            Some("compile") => s("sbt", "Compile with sbt", Build, Safe),
            Some("test") => s("sbt", "Run tests with sbt", Test, Safe),
            Some("run") => s("sbt", "Run the main class with sbt", Run, Safe),
            Some("package") | Some("assembly") => s("sbt", "Package with sbt", Build, Safe),
            Some("publish") | Some("publishSigned") => {
                s("sbt", "Publish artifacts with sbt", Publish, External)
            }
            Some("scalafmtAll") | Some("scalafmt") => {
                s("sbt", "Format Scala source with scalafmt", Format, Safe)
            }
            Some(x) => s("sbt", format!("Run sbt {x}"), Other, Safe),
            None => s("sbt", "Start the sbt shell", Other, Safe),
        },
        "lein" => match sub {
            Some("test") => s("lein", "Run tests with Leiningen", Test, Safe),
            Some("run") => s("lein", "Run the app with Leiningen", Run, Safe),
            Some("repl") => s("lein", "Start a REPL with Leiningen", Dev, Safe),
            Some("uberjar") | Some("jar") => s("lein", "Build the JAR with Leiningen", Build, Safe),
            Some("deploy") => s("lein", "Deploy artifacts with Leiningen", Publish, External),
            Some(x) => s("lein", format!("Run lein {x}"), Other, Safe),
            None => s("lein", "Run Leiningen", Other, Safe),
        },
        "clojure" | "clj" => {
            let alias = args.iter().find_map(|x| {
                x.strip_prefix("-M:")
                    .or_else(|| x.strip_prefix("-X:"))
                    .or_else(|| x.strip_prefix("-T:"))
                    .or_else(|| x.strip_prefix("-A:"))
            });
            match alias {
                Some(al) => s(
                    "clojure",
                    format!("Run the {al} deps.edn alias"),
                    crate::explain::name_hint(al)
                        .map(|h| h.kind)
                        .unwrap_or(Other),
                    Safe,
                ),
                None => s("clojure", "Start a Clojure REPL", Dev, Safe),
            }
        }
        "stack" => match sub {
            Some("build") => s("stack", "Build with Stack", Build, Safe),
            Some("test") => s("stack", "Run tests with Stack", Test, Safe),
            Some("run") | Some("exec") => s("stack", "Run the executable with Stack", Run, Safe),
            Some("ghci") | Some("repl") => {
                s("stack", "Open GHCi with the project loaded", Dev, Safe)
            }
            Some(x) => s("stack", format!("Run stack {x}"), Other, Safe),
            None => s("stack", "Run Stack", Other, Safe),
        },
        "cabal" => match sub {
            Some("build") => s("cabal", "Build with Cabal", Build, Safe),
            Some("test") => s("cabal", "Run tests with Cabal", Test, Safe),
            Some("run") => s("cabal", "Run the executable with Cabal", Run, Safe),
            Some("repl") => s("cabal", "Open GHCi with Cabal", Dev, Safe),
            Some("upload") => s("cabal", "Upload the package to Hackage", Publish, External),
            Some(x) => s("cabal", format!("Run cabal {x}"), Other, Safe),
            None => s("cabal", "Run Cabal", Other, Safe),
        },
        "hlint" => s("hlint", "Check Haskell source with HLint", Lint, Safe),
        "ormolu" | "fourmolu" => s(program, "Format Haskell source", Format, Safe),
        "dune" => match sub {
            Some("build") => s("dune", "Build with dune", Build, Safe),
            Some("test") | Some("runtest") => s("dune", "Run tests with dune", Test, Safe),
            Some("exec") => s("dune", "Run the executable with dune", Run, Safe),
            Some("fmt") => s("dune", "Format OCaml source with dune", Format, Safe),
            Some(x) => s("dune", format!("Run dune {x}"), Other, Safe),
            None => s("dune", "Run dune", Other, Safe),
        },
        "nimble" => match sub {
            Some("build") => s("nimble", "Build with Nimble", Build, Safe),
            Some("test") => s("nimble", "Run tests with Nimble", Test, Safe),
            Some("install") => s("nimble", "Install Nim dependencies", Install, Safe),
            Some(x) => s("nimble", format!("Run nimble {x}"), Other, Safe),
            None => s("nimble", "Run Nimble", Other, Safe),
        },
        "ctest" => s("ctest", "Run CTest tests", Test, Safe),
        "meson" => s(
            "meson",
            format!("Run meson {}", sub.unwrap_or("setup")),
            Build,
            Safe,
        ),
        "clang-format" => s(
            "clang-format",
            "Format C/C++ sources with clang-format",
            Format,
            Safe,
        ),
        "clang-tidy" => s(
            "clang-tidy",
            "Check C/C++ sources with clang-tidy",
            Lint,
            Safe,
        ),
        "cppcheck" => s(
            "cppcheck",
            "Analyse C/C++ sources with cppcheck",
            Lint,
            Safe,
        ),
        "iex" => s("iex", "Open an IEx shell", Dev, Safe),
        "dbt" => match sub {
            Some("run") => s("dbt", "Run dbt models against the warehouse", Db, External),
            Some("build") => s("dbt", "Build dbt models, tests and seeds", Db, External),
            Some("test") => s("dbt", "Run dbt tests", Test, External),
            Some("seed") => s("dbt", "Load dbt seed data into the warehouse", Db, External),
            Some("snapshot") => s("dbt", "Run dbt snapshots", Db, External),
            Some("compile") => s("dbt", "Compile dbt models", Build, Safe),
            Some("docs") => s("dbt", "Generate or serve dbt docs", Docs, Safe),
            Some("deps") => s("dbt", "Install dbt packages", Install, Safe),
            Some("debug") | Some("parse") | Some("ls") | Some("list") => {
                s("dbt", "Inspect the dbt project", Other, Safe)
            }
            Some("clean") => s("dbt", "Delete dbt artifacts", Clean, Safe),
            Some(x) => s("dbt", format!("Run dbt {x}"), Db, Safe),
            None => s("dbt", "Run dbt", Db, Safe),
        },
        "sqlfluff" => s(
            "sqlfluff",
            if sub == Some("fix") {
                "Fix SQL style issues with SQLFluff"
            } else {
                "Lint SQL with SQLFluff"
            },
            Lint,
            Safe,
        ),
        "dagger" => match sub {
            Some("call") => s(
                "dagger",
                format!(
                    "Run the {} Dagger function",
                    a.positionals().get(1).copied().unwrap_or("")
                )
                .trim()
                .to_string(),
                Other,
                Safe,
            ),
            Some("run") => s("dagger", "Run a Dagger pipeline", Other, Safe),
            Some(x) => s("dagger", format!("Run dagger {x}"), Other, Safe),
            None => s("dagger", "Run Dagger", Other, Safe),
        },
        "earthly" => s(
            "earthly",
            format!(
                "Run the {} Earthly target",
                a.positionals().first().copied().unwrap_or("default")
            ),
            Build,
            if a.has("--push") { External } else { Safe },
        ),
        "garden" => match sub {
            Some("deploy") => s("garden", "Deploy with Garden", Deploy, External),
            Some("test") => s("garden", "Run tests with Garden", Test, External),
            Some("build") => s("garden", "Build with Garden", Build, Safe),
            Some(x) => s("garden", format!("Run garden {x}"), Infra, External),
            None => s("garden", "Run Garden", Infra, Safe),
        },
        "rabbitmqadmin" | "rabbitmqctl" => {
            let p = a.positionals();
            if p.iter().any(|x| {
                x.starts_with("delete") || x.starts_with("purge") || x.starts_with("reset")
            }) {
                s("rabbitmq", "Delete RabbitMQ resources", Infra, Destructive)
            } else if p.iter().any(|x| x.starts_with("declare")) {
                s("rabbitmq", "Declare RabbitMQ resources", Infra, External)
            } else {
                s("rabbitmq", "Inspect RabbitMQ", Infra, External)
            }
        }
        "nats" => {
            let p = a.positionals();
            if p.iter()
                .any(|x| *x == "rm" || *x == "purge" || *x == "delete")
            {
                s("nats", "Delete NATS resources", Infra, Destructive)
            } else if p.first() == Some(&"server") {
                s("nats", "Manage the NATS server", Infra, Safe)
            } else {
                s("nats", "Interact with NATS", Infra, External)
            }
        }
        "nats-server" => s("nats", "Start a NATS server", Infra, Safe),
        "invoke" | "inv" => s(
            "invoke",
            format!("Run the {} invoke task", sub.unwrap_or("default")),
            Other,
            Safe,
        ),
        "mkcert" => s("mkcert", "Create local TLS certificates", Other, Safe),
        "ngrok" | "cloudflared" => s(
            program,
            "Expose a local port through a tunnel",
            Infra,
            External,
        ),
        "stripe" => s("stripe", "Run the Stripe CLI", Infra, External),
        "sentry-cli" => s("sentry-cli", "Upload to Sentry", Publish, External),
        "python-module:http.server" => {
            s("python", "Serve the current directory over HTTP", Dev, Safe)
        }
        _ => None,
    }
}
