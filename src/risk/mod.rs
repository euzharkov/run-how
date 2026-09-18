//! Risk classification, independent from the description engine.
//!
//! Two sources of evidence are combined: the per-tool knowledge collected during analysis, and a
//! small set of textual patterns applied to the raw command. The classifier is deliberately
//! conservative: when confidence is low it prefers a missing warning over a wrong one.

use crate::analyze::Analysis;
use crate::model::Risk;

/// Classify a command's risk. `analysis` provides tool-level evidence.
pub fn classify(command: &str, analysis: &Analysis) -> Risk {
    let mut r = analysis.max_risk();
    let t = textual(command);
    if t > r {
        r = t;
    }
    r
}

/// Pattern-based classification on the raw command text only.
pub fn textual(command: &str) -> Risk {
    let lower = command.to_ascii_lowercase();
    // Split into segments so patterns don't accidentally span `&&`.
    let mut risk = Risk::Safe;
    for seg in lower
        .split([';', '|', '&'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let toks: Vec<&str> = seg
            .split_whitespace()
            .map(|t| t.trim_matches(|c| c == '\'' || c == '"'))
            .collect();
        let r = segment_risk(&toks);
        if r > risk {
            risk = r;
        }
    }
    risk
}

fn seq(toks: &[&str], pat: &[&str]) -> bool {
    toks.windows(pat.len()).any(|w| w == pat)
}

fn has(toks: &[&str], word: &str) -> bool {
    toks.contains(&word)
}

fn has_after(toks: &[&str], anchor: &str, flags: &[&str]) -> bool {
    match toks.iter().position(|t| *t == anchor) {
        Some(i) => toks[i + 1..].iter().any(|t| flags.contains(t)),
        None => false,
    }
}

fn segment_risk(toks: &[&str]) -> Risk {
    if toks.is_empty() {
        return Risk::Safe;
    }
    let joined = toks.join(" ");

    // ---- destructive -------------------------------------------------------------------
    let destructive = [
        &["down", "-v"][..],
        &["down", "--volumes"],
        &["migrate", "reset"],
        &["migrate:reset"],
        &["db:reset"],
        &["schema:drop"],
        &["db:drop"],
        &["db:wipe"],
        &["migrate:fresh"],
        &["terraform", "destroy"],
        &["tofu", "destroy"],
        &["terragrunt", "destroy"],
        &["pulumi", "destroy"],
        &["kubectl", "delete"],
        &["helm", "uninstall"],
        &["helm", "delete"],
        &["system", "prune"],
        &["volume", "rm"],
        &["volume", "prune"],
        &["reset", "--hard"],
        &["push", "--force"],
        &["push", "-f"],
        &["drop", "database"],
        &["drop", "table"],
        &["drop", "schema"],
        &["database", "drop"],
        &["truncate", "table"],
        &["flushall"],
        &["flushdb"],
        &["--delete-topic"],
        &["--reset-offsets"],
        &["dropdb"],
        &["rm", "-rf", "/"],
        &["rm", "-rf", "~"],
        &["rm", "-rf", "*"],
        &["rm", "-rf", "."],
        &["format", "c:"],
        &["deluser"],
        &["ecto.reset"],
        &["ecto.drop"],
        &["--force-reset"],
        &["migrate:rollback"],
        &["db:migrate:undo"],
        &["migration:revert"],
        &["migration:down"],
        &["schema:fresh"],
    ];
    for pat in destructive {
        if seq(toks, pat) {
            return Risk::Destructive;
        }
    }
    if has(toks, "compose") && has_after(toks, "down", &["-v", "--volumes"]) {
        return Risk::Destructive;
    }
    if (toks[0] == "docker-compose" || toks[0] == "podman-compose")
        && has_after(toks, "down", &["-v", "--volumes"])
    {
        return Risk::Destructive;
    }
    if toks[0] == "git"
        && has(toks, "clean")
        && toks
            .iter()
            .any(|t| t.starts_with('-') && t.contains('x') && t.contains('f'))
    {
        return Risk::Destructive;
    }
    if toks[0] == "git" && has(toks, "push") && has(toks, "--force-with-lease") {
        return Risk::External;
    }
    if (toks[0] == "kafka-topics" || toks[0] == "kafka-topics.sh") && has(toks, "--delete") {
        return Risk::Destructive;
    }

    // ---- external ----------------------------------------------------------------------
    let external = [
        &["git", "push"][..],
        &["npm", "publish"],
        &["pnpm", "publish"],
        &["yarn", "publish"],
        &["yarn", "npm", "publish"],
        &["bun", "publish"],
        &["docker", "push"],
        &["podman", "push"],
        &["compose", "push"],
        &["terraform", "apply"],
        &["tofu", "apply"],
        &["terragrunt", "apply"],
        &["terragrunt", "run-all"],
        &["kubectl", "apply"],
        &["kubectl", "rollout"],
        &["kubectl", "scale"],
        &["kubectl", "patch"],
        &["kubectl", "create"],
        &["kubectl", "exec"],
        &["helm", "install"],
        &["helm", "upgrade"],
        &["pulumi", "up"],
        &["cdk", "deploy"],
        &["sls", "deploy"],
        &["serverless", "deploy"],
        &["fly", "deploy"],
        &["flyctl", "deploy"],
        &["vercel", "--prod"],
        &["vercel", "deploy"],
        &["netlify", "deploy"],
        &["wrangler", "deploy"],
        &["wrangler", "publish"],
        &["firebase", "deploy"],
        &["gh", "release", "create"],
        &["cargo", "publish"],
        &["nuget", "push"],
        &["twine", "upload"],
        &["uv", "publish"],
        &["poetry", "publish"],
        &["gem", "push"],
        &["goreleaser", "release"],
        &["eas", "submit"],
        &["eas", "build"],
        &["mvn", "deploy"],
        &["gradle", "publish"],
        &["./gradlew", "publish"],
        &["ansible-playbook"],
        &["aws"],
        &["gcloud"],
        &["az"],
        &["ssh"],
        &["scp"],
        &["rsync"],
        &["mkdocs", "gh-deploy"],
        &["gh-pages"],
        &["skaffold", "run"],
        &["skaffold", "dev"],
        &["argocd", "app", "sync"],
    ];
    for pat in external {
        if seq(toks, pat) {
            // `git push` inside a `-f` check already handled; tokens like "aws" must be the program.
            if pat.len() == 1 && toks[0] != pat[0] {
                continue;
            }
            return Risk::External;
        }
    }
    if toks[0] == "curl" || toks[0] == "wget" || toks[0] == "http" || toks[0] == "xh" {
        let mutating = toks.windows(2).any(|w| {
            (w[0] == "-x" || w[0] == "--request")
                && matches!(w[1], "post" | "put" | "delete" | "patch")
        }) || toks.iter().any(|t| {
            t.starts_with("-x")
                && t.len() > 2
                && matches!(&t[2..], "post" | "put" | "delete" | "patch")
        });
        if mutating {
            return Risk::External;
        }
    }
    let _ = joined;
    Risk::Safe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_patterns() {
        for c in [
            "rm -rf /",
            "docker compose down -v",
            "docker compose -f dev.yml down --volumes",
            "docker-compose down -v",
            "prisma migrate reset --force",
            "terraform destroy -auto-approve",
            "kubectl delete -f k8s/",
            "git clean -fdx",
            "git reset --hard HEAD",
            "git push --force origin main",
            "psql -c 'DROP DATABASE app'",
            "kafka-topics.sh --bootstrap-server localhost:9092 --delete --topic events",
        ] {
            assert_eq!(textual(c), Risk::Destructive, "{c}");
        }
    }

    #[test]
    fn external_patterns() {
        for c in [
            "npm publish",
            "docker push ghcr.io/x/y",
            "terraform apply",
            "kubectl apply -f k8s/",
            "git push",
            "helm upgrade --install app ./chart",
            "curl -X POST https://api.example.com/deploy",
            "aws s3 sync dist s3://bucket",
        ] {
            assert_eq!(textual(c), Risk::External, "{c}");
        }
    }

    #[test]
    fn safe_when_unsure() {
        for c in [
            "rm -rf dist",
            "vitest run",
            "docker compose up -d",
            "docker compose down",
            "kubectl kustomize k8s/overlays/dev",
            "curl https://example.com",
            "terraform plan",
            "git status",
        ] {
            assert_eq!(textual(c), Risk::Safe, "{c}");
        }
    }
}
