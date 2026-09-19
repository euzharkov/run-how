//! Kubernetes: Helm charts, Kustomize overlays and plain manifest directories.
//!
//! Discovery is entirely static; no `kubectl`/`helm` process is ever started and no cluster
//! is contacted. Actions that would contact a cluster are marked External or Destructive.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Kubernetes;

const KUSTOMIZATION: &[&str] = &["kustomization.yaml", "kustomization.yml", "Kustomization"];
const SKIP_FILES: &[&str] = &[
    "compose.yaml",
    "compose.yml",
    "docker-compose.yaml",
    "docker-compose.yml",
    "Taskfile.yml",
    "Taskfile.yaml",
    "pnpm-workspace.yaml",
    "values.yaml",
    "values.yml",
    "Chart.yaml",
    "Chart.lock",
    ".pre-commit-config.yaml",
    "mkdocs.yml",
    "codecov.yml",
    ".golangci.yml",
    "skaffold.yaml",
    "tilt.yaml",
    "openapi.yaml",
    "openapi.yml",
    "swagger.yaml",
    "swagger.yml",
    "config.yaml",
    "config.yml",
    "environment.yml",
    "conda.yaml",
    "buf.yaml",
    "buf.gen.yaml",
    "netlify.yaml",
    "render.yaml",
    "fly.yaml",
    "app.yaml",
    "serverless.yml",
    "serverless.yaml",
    "cloudbuild.yaml",
    "bitbucket-pipelines.yml",
    ".gitlab-ci.yml",
    "azure-pipelines.yml",
    "lerna.yaml",
    "pubspec.yaml",
    "dependabot.yml",
    "renovate.yaml",
    "mise.toml",
    "action.yml",
    "action.yaml",
    "docker-bake.yaml",
];

fn is_manifest(dir: &DirInfo, file: &str) -> bool {
    if SKIP_FILES.contains(&file) || file.starts_with("values") || file.starts_with('.') {
        return false;
    }
    let Some(text) = dir.read(file) else {
        return false;
    };
    let head: String = text.chars().take(4096).collect();
    let mut api = false;
    let mut kind = false;
    for l in head.lines() {
        if l.starts_with("apiVersion:") {
            api = true;
        } else if l.starts_with("kind:") {
            kind = true;
        }
        if api && kind {
            return true;
        }
    }
    false
}

fn is_helm(dir: &DirInfo) -> bool {
    dir.has("Chart.yaml") || dir.has("Chart.yml")
}
fn is_kustomize(dir: &DirInfo) -> bool {
    dir.has_any(KUSTOMIZATION)
}
fn has_manifests(dir: &DirInfo) -> bool {
    dir.files
        .iter()
        .filter(|f| f.ends_with(".yaml") || f.ends_with(".yml"))
        .take(30)
        .any(|f| is_manifest(dir, f))
}

fn chart_name(dir: &DirInfo) -> String {
    dir.read("Chart.yaml")
        .and_then(|t| {
            t.lines().find(|l| l.starts_with("name:")).map(|l| {
                l.trim_start_matches("name:")
                    .trim()
                    .trim_matches('"')
                    .to_string()
            })
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| dir.name().to_string())
}

impl Discoverer for Kubernetes {
    fn id(&self) -> &'static str {
        "kubernetes"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Kubernetes
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        is_helm(dir)
            || is_kustomize(dir)
            || has_manifests(dir)
            || dir.has("skaffold.yaml")
            || dir.has_any(&["helmfile.yaml", "helmfile.yml", "helmfile.yaml.gotmpl"])
            || dir.has("Tiltfile")
    }
    fn attachable(&self) -> bool {
        true
    }
    fn claims_subtree(&self) -> bool {
        true
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let multi = dirs.len() > 1;
        for dir in dirs {
            let at_base = dir.rel == base.rel;
            let rel = if at_base {
                ".".to_string()
            } else {
                ctx.rel_from(base, dir)
            };
            let pre = if multi && !at_base {
                format!("k8s:{}", dir.name())
            } else {
                "k8s".to_string()
            };
            let infra = Category::Infrastructure;
            let a = |id: String, cmd: String| {
                Action::new(id, cmd)
                    .tool("kubernetes")
                    .inferred(Confidence::High)
                    .cat(infra)
            };

            if dir.has("skaffold.yaml") {
                let sk = dir.read("skaffold.yaml").unwrap_or_default();
                out.actions.push(
                    a(format!("{pre}:dev"), "skaffold dev".into())
                        .risk(Risk::External)
                        .confidence(Confidence::Medium),
                );
                out.actions.push(
                    a(format!("{pre}:skaffold:render"), "skaffold render".into())
                        .inferred_desc("Render manifests with Skaffold"),
                );
                if let Some(pi) = sk.find("\nprofiles:") {
                    for l in sk[pi..].lines() {
                        if let Some(n) = l.trim().strip_prefix("- name:") {
                            let n = n.trim();
                            if !n.is_empty()
                                && n.chars()
                                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                            {
                                out.actions.push(
                                    a(format!("{pre}:dev:{n}"), format!("skaffold dev -p {n}"))
                                        .risk(Risk::External)
                                        .confidence(Confidence::Medium)
                                        .inferred_desc(format!(
                                            "Build and deploy the {n} Skaffold profile on change"
                                        )),
                                );
                            }
                        }
                    }
                }
            }
            if dir.has_any(&["helmfile.yaml", "helmfile.yml", "helmfile.yaml.gotmpl"]) {
                out.actions.push(
                    a(format!("{pre}:helmfile:diff"), "helmfile diff".into())
                        .risk(Risk::External)
                        .confidence(Confidence::Medium)
                        .inferred_desc("Diff Helm releases with Helmfile"),
                );
                out.actions.push(
                    a(format!("{pre}:helmfile:apply"), "helmfile apply".into())
                        .risk(Risk::External)
                        .confidence(Confidence::Medium)
                        .inferred_desc("Apply Helm releases with Helmfile")
                        .cat(Category::Release),
                );
                out.actions.push(
                    a(
                        format!("{pre}:helmfile:template"),
                        "helmfile template".into(),
                    )
                    .inferred_desc("Render Helm releases with Helmfile"),
                );
            }
            if dir.has("Tiltfile") {
                out.actions.push(
                    a(format!("{pre}:tilt"), "tilt up".into())
                        .risk(Risk::External)
                        .confidence(Confidence::Medium),
                );
            }

            if is_helm(dir) {
                let chart = chart_name(dir);
                let path = if at_base {
                    ".".to_string()
                } else {
                    rel.clone()
                };
                let p = super::q(&path);
                out.actions.push(
                    a(
                        format!("{pre}:render"),
                        format!("helm template {chart} {p}"),
                    )
                    .inferred_desc(format!("Render the {chart} Helm chart")),
                );
                out.actions.push(
                    a(format!("{pre}:lint"), format!("helm lint {p}"))
                        .inferred_desc(format!("Validate the {chart} Helm chart"))
                        .cat(Category::Quality),
                );
                out.actions.push(
                    a(
                        format!("{pre}:install"),
                        format!("helm upgrade --install {chart} {p}"),
                    )
                    .risk(Risk::External)
                    .confidence(Confidence::Medium)
                    .cat(Category::Release),
                );
                out.actions.push(
                    a(
                        format!("{pre}:uninstall"),
                        format!("helm uninstall {chart}"),
                    )
                    .risk(Risk::Destructive)
                    .confidence(Confidence::Low),
                );
                continue;
            }

            // Kustomize: collect overlays below this directory.
            let subtree = ctx.descendants(dir);
            let mut overlays: Vec<&DirInfo> = subtree
                .iter()
                .copied()
                .filter(|d| is_kustomize(d) && d.rel_to(dir, "").starts_with("overlays/"))
                .collect();
            overlays.sort_by(|x, y| x.rel.cmp(&y.rel));
            if !overlays.is_empty() {
                for o in overlays {
                    let env = o.name();
                    let p = super::q(&o.rel_to(base, ""));
                    out.actions.push(
                        a(
                            format!("{pre}:render:{env}"),
                            format!("kubectl kustomize {p}"),
                        )
                        .inferred_desc(format!("Render Kubernetes manifests for {env}")),
                    );
                    out.actions.push(
                        a(
                            format!("{pre}:apply:{env}"),
                            format!("kubectl apply -k {p}"),
                        )
                        .inferred_desc(format!("Apply {env} resources to the cluster"))
                        .risk(Risk::External)
                        .confidence(Confidence::Medium)
                        .cat(Category::Release),
                    );
                    out.actions.push(
                        a(
                            format!("{pre}:delete:{env}"),
                            format!("kubectl delete -k {p}"),
                        )
                        .inferred_desc(format!("Delete {env} resources from the cluster"))
                        .risk(Risk::Destructive)
                        .confidence(Confidence::Low),
                    );
                }
                continue;
            }
            if is_kustomize(dir) {
                let p = super::q(&rel);
                out.actions.push(
                    a(format!("{pre}:render"), format!("kubectl kustomize {p}"))
                        .inferred_desc("Render Kubernetes manifests"),
                );
                out.actions.push(
                    a(format!("{pre}:apply"), format!("kubectl apply -k {p}"))
                        .inferred_desc("Apply resources to the cluster")
                        .risk(Risk::External)
                        .confidence(Confidence::Medium)
                        .cat(Category::Release),
                );
                out.actions.push(
                    a(format!("{pre}:delete"), format!("kubectl delete -k {p}"))
                        .inferred_desc("Delete resources from the cluster")
                        .risk(Risk::Destructive)
                        .confidence(Confidence::Low),
                );
                continue;
            }
            if has_manifests(dir) {
                let p = super::q(&format!("{}/", rel.trim_end_matches('/')));
                out.actions.push(
                    a(
                        format!("{pre}:validate"),
                        format!("kubectl apply --dry-run=client -f {p}"),
                    )
                    .inferred_desc("Validate Kubernetes manifests")
                    .cat(Category::Quality),
                );
                out.actions.push(
                    a(format!("{pre}:apply"), format!("kubectl apply -f {p}"))
                        .inferred_desc("Apply resources to the cluster")
                        .risk(Risk::External)
                        .confidence(Confidence::Medium)
                        .cat(Category::Release),
                );
                out.actions.push(
                    a(format!("{pre}:delete"), format!("kubectl delete -f {p}"))
                        .inferred_desc("Delete resources from the cluster")
                        .risk(Risk::Destructive)
                        .confidence(Confidence::Low),
                );
            }
        }
        out
    }
}
