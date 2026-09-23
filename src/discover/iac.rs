//! Infrastructure as code: Terraform / OpenTofu, Pulumi, Ansible. Attachable, so `infra/envs/prod`
//! becomes actions of the containing project. Discovery never touches remote state.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct Iac;

fn has_tf(dir: &DirInfo) -> bool {
    !dir.with_ext("tf").is_empty() || !dir.with_ext("tofu").is_empty()
}

/// The `required_version` constraint from a `terraform { ... }' block, and the file it came
/// from. A light text scan, not an HCL parser, consistent with how the rest of `rhow` reads
/// config: good enough for the common single-line `required_version = "..."` form.
fn terraform_required_version(dir: &DirInfo) -> Option<(String, String)> {
    for f in dir.with_ext("tf").into_iter().chain(dir.with_ext("tofu")) {
        let Some(text) = dir.read(f) else { continue };
        let Some(idx) = text.find("required_version") else {
            continue;
        };
        let rest = &text[idx + "required_version".len()..];
        let Some(q1) = rest.find('"') else { continue };
        let after = &rest[q1 + 1..];
        let Some(q2) = after.find('"') else { continue };
        let v = &after[..q2];
        if !v.is_empty() {
            return Some((v.to_string(), f.to_string()));
        }
    }
    None
}
fn has_ansible(dir: &DirInfo) -> bool {
    dir.has("ansible.cfg")
        || dir.has_any(&[
            "site.yml",
            "site.yaml",
            "playbook.yml",
            "playbook.yaml",
            "main.yml",
        ]) && dir.has_dir("roles")
        || dir.has_dir("playbooks")
}

impl Discoverer for Iac {
    fn id(&self) -> &'static str {
        "iac"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Infrastructure
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        has_tf(dir) || dir.has_any(&["Pulumi.yaml", "Pulumi.yml"]) || has_ansible(dir)
    }
    fn attachable(&self) -> bool {
        true
    }
    fn claims_subtree(&self) -> bool {
        false
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
            let chdir = if at_base {
                String::new()
            } else {
                format!(" -chdir={}", super::q(&rel))
            };
            // Terraform modules under `modules/` are libraries, not deployable roots.
            if !at_base && rel.split('/').any(|s| s == "modules" || s == "module") {
                continue;
            }
            let pre = |tool: &str| {
                if multi && !at_base {
                    format!("{tool}:{}", dir.name())
                } else {
                    tool.to_string()
                }
            };

            if has_tf(dir) {
                let tofu = !dir.with_ext("tofu").is_empty()
                    || dir.has(".opentofu-version")
                    || ctx.ancestors(dir).iter().any(|a| {
                        a.read(".tool-versions")
                            .map(|t| t.contains("opentofu"))
                            .unwrap_or(false)
                    });
                let bin = if tofu { "tofu" } else { "terraform" };
                let name = if tofu { "OpenTofu" } else { "Terraform" };
                if let Some((v, f)) = terraform_required_version(dir) {
                    let source = if at_base { f } else { format!("{rel}/{f}") };
                    out.version(bin, v, source);
                }
                let p = pre("tf");
                let t = |sub: &str| format!("{bin}{chdir} {sub}");
                let infra = Category::Infrastructure;
                let a = |id: String, cmd: String| {
                    Action::new(id, cmd)
                        .tool(bin)
                        .inferred(Confidence::High)
                        .cat(infra)
                };
                out.actions.push(
                    a(format!("{p}:init"), t("init"))
                        .inferred_desc(format!("Initialise the {name} working directory"))
                        .confidence(Confidence::Medium),
                );
                out.actions.push(
                    a(format!("{p}:plan"), t("plan"))
                        .inferred_desc(format!("Plan {name} changes"))
                        .risk(Risk::External),
                );
                out.actions.push(
                    a(format!("{p}:apply"), t("apply"))
                        .inferred_desc(format!("Apply {name} changes to infrastructure"))
                        .risk(Risk::External)
                        .cat(Category::Release),
                );
                out.actions.push(
                    a(format!("{p}:destroy"), t("destroy"))
                        .inferred_desc(format!("Destroy {name}-managed infrastructure"))
                        .risk(Risk::Destructive)
                        .confidence(Confidence::Low),
                );
                out.actions.push(
                    a(format!("{p}:validate"), t("validate"))
                        .inferred_desc(format!("Validate the {name} configuration"))
                        .cat(Category::Quality)
                        .confidence(Confidence::Low),
                );
                out.actions.push(
                    a(format!("{p}:fmt"), t("fmt -recursive"))
                        .inferred_desc(format!("Format {name} files"))
                        .cat(Category::Quality)
                        .confidence(Confidence::Low),
                );
                if dir.has(".tflint.hcl") {
                    out.actions.push(
                        a(
                            format!("{p}:lint"),
                            format!("tflint --chdir={}", super::q(&rel)),
                        )
                        .inferred_desc("Lint Terraform files")
                        .cat(Category::Quality)
                        .confidence(Confidence::Medium),
                    );
                }
                if dir.has("terragrunt.hcl") {
                    out.actions.push(
                        a(
                            format!("{p}:terragrunt"),
                            format!(
                                "terragrunt run-all plan --terragrunt-working-dir {}",
                                super::q(&rel)
                            ),
                        )
                        .inferred_desc("Plan all Terragrunt modules")
                        .risk(Risk::External)
                        .confidence(Confidence::Medium),
                    );
                }
            }
            if dir.has_any(&["Pulumi.yaml", "Pulumi.yml"]) {
                let p = pre("pulumi");
                let cwd = if at_base {
                    String::new()
                } else {
                    format!(" -C {}", super::q(&rel))
                };
                let infra = Category::Infrastructure;
                let a = |id: String, cmd: String| {
                    Action::new(id, cmd)
                        .tool("pulumi")
                        .inferred(Confidence::High)
                        .cat(infra)
                };
                out.actions.push(
                    a(format!("{p}:preview"), format!("pulumi{cwd} preview"))
                        .inferred_desc("Preview Pulumi changes")
                        .risk(Risk::External),
                );
                out.actions.push(
                    a(format!("{p}:up"), format!("pulumi{cwd} up"))
                        .inferred_desc("Deploy the Pulumi stack")
                        .risk(Risk::External)
                        .cat(Category::Release),
                );
                out.actions.push(
                    a(format!("{p}:destroy"), format!("pulumi{cwd} destroy"))
                        .inferred_desc("Destroy the Pulumi stack")
                        .risk(Risk::Destructive)
                        .confidence(Confidence::Low),
                );
            }
            if has_ansible(dir) {
                let p = pre("ansible");
                let mut playbooks: Vec<String> = [
                    "site.yml",
                    "site.yaml",
                    "playbook.yml",
                    "playbook.yaml",
                    "main.yml",
                ]
                .iter()
                .filter(|f| dir.has(f))
                .map(|f| f.to_string())
                .collect();
                if let Some(pb) = ctx
                    .descendants(dir)
                    .into_iter()
                    .find(|d| d.rel_to(dir, "") == "playbooks")
                {
                    playbooks.extend(
                        pb.files
                            .iter()
                            .filter(|f| f.ends_with(".yml") || f.ends_with(".yaml"))
                            .map(|f| format!("playbooks/{f}")),
                    );
                }
                let single = playbooks.len() == 1;
                for pb in playbooks.iter().take(12) {
                    let stem = std::path::Path::new(pb)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(pb);
                    let id = if single {
                        p.clone()
                    } else {
                        format!("{p}:{stem}")
                    };
                    let path = if at_base {
                        pb.clone()
                    } else {
                        format!("{rel}/{pb}")
                    };
                    out.actions.push(
                        Action::new(
                            format!("{id}:check"),
                            format!("ansible-playbook --check {}", super::q(&path)),
                        )
                        .tool("ansible")
                        .inferred(Confidence::Medium)
                        .inferred_desc(format!("Dry-run the {stem} playbook"))
                        .cat(Category::Infrastructure)
                        .risk(Risk::External),
                    );
                    out.actions.push(
                        Action::new(id, format!("ansible-playbook {}", super::q(&path)))
                            .tool("ansible")
                            .inferred(Confidence::Medium)
                            .inferred_desc(format!("Run the {stem} Ansible playbook"))
                            .cat(Category::Release)
                            .risk(Risk::External),
                    );
                }
                if dir.has(".ansible-lint") || dir.has(".ansible-lint.yml") {
                    out.actions.push(
                        Action::new(format!("{p}:lint"), "ansible-lint".to_string())
                            .tool("ansible")
                            .inferred(Confidence::Medium)
                            .inferred_desc("Lint Ansible playbooks")
                            .cat(Category::Quality),
                    );
                }
            }
        }
        // Five or more Terraform/Pulumi roots under one project is a module library or a test
        // corpus, not five deployments: keep them, but out of the default view.
        if dirs.len() >= 5 {
            for a in &mut out.actions {
                a.confidence = Confidence::Low;
            }
        }
        out
    }
}
