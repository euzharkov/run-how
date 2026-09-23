//! .NET solutions and projects (`*.sln`, `*.slnx`, `*.csproj`, `*.fsproj`, `*.vbproj`).
//!
//! Project files are parsed with plain string matching; MSBuild is never invoked.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;

pub struct DotNet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flavor {
    Web,
    Worker,
    Exe,
    Test,
    Library,
    Blazor,
}

fn tag(text: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let s = text.find(&open)? + open.len();
    let e = text[s..].find(&close)? + s;
    Some(text[s..e].trim().to_string())
}

fn has_package(text: &str, id: &str) -> bool {
    text.contains(&format!("Include=\"{id}\""))
        || text.contains(&format!("Include=\"{id}."))
        || text.contains(&format!("Update=\"{id}\""))
}

fn flavor(text: &str) -> Flavor {
    let sdk = text
        .split("Sdk=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap_or("");
    let is_test = tag(text, "IsTestProject")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || has_package(text, "Microsoft.NET.Test.Sdk")
        || has_package(text, "xunit")
        || has_package(text, "xunit.v3")
        || has_package(text, "NUnit")
        || has_package(text, "MSTest")
        || has_package(text, "TUnit")
        || has_package(text, "Expecto");
    if is_test {
        return Flavor::Test;
    }
    if sdk.contains("BlazorWebAssembly") {
        return Flavor::Blazor;
    }
    if sdk.contains("Sdk.Web") {
        return Flavor::Web;
    }
    if sdk.contains("Sdk.Worker") {
        return Flavor::Worker;
    }
    let output = tag(text, "OutputType").unwrap_or_default();
    if output.eq_ignore_ascii_case("Exe") || output.eq_ignore_ascii_case("WinExe") {
        return Flavor::Exe;
    }
    Flavor::Library
}

fn project_files(dir: &DirInfo) -> Vec<&str> {
    let mut v = dir.with_ext("csproj");
    v.extend(dir.with_ext("fsproj"));
    v.extend(dir.with_ext("vbproj"));
    v
}

fn solution_files(dir: &DirInfo) -> Vec<&str> {
    let mut v = dir.with_ext("sln");
    v.extend(dir.with_ext("slnx"));
    v
}

/// Project paths (with `/` separators) referenced from a solution file.
fn solution_projects(text: &str, slnx: bool) -> Vec<String> {
    let mut out = Vec::new();
    if slnx {
        for part in text.split("<Project ").skip(1) {
            if let Some(p) = part
                .split("Path=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
            {
                out.push(p.replace('\\', "/"));
            }
        }
    } else {
        for line in text.lines() {
            let l = line.trim();
            if !l.starts_with("Project(") {
                continue;
            }
            let parts: Vec<&str> = l.split('"').collect();
            // Project("{GUID}") = "Name", "path", "{GUID}"
            if parts.len() >= 6 {
                let p = parts[5];
                if p.ends_with("proj") {
                    out.push(p.replace('\\', "/"));
                }
            }
        }
    }
    out
}

fn stem(f: &str) -> &str {
    std::path::Path::new(f)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(f)
}

impl Discoverer for DotNet {
    fn id(&self) -> &'static str {
        "dotnet"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::DotNet
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        !solution_files(dir).is_empty() || !project_files(dir).is_empty()
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, _dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let slns = solution_files(base);
        let projs = project_files(base);

        // ---- solution level -------------------------------------------------------------
        for sln in &slns {
            let text = base.read(sln).unwrap_or_default();
            let members = solution_projects(&text, sln.ends_with(".slnx"));
            let mut has_test = false;
            let mut has_packable = false;
            for m in &members {
                if let Some(t) = super::text_at(ctx, base, m) {
                    if flavor(&t) == Flavor::Test {
                        has_test = true
                    }
                    if tag(&t, "IsPackable")
                        .map(|v| v.eq_ignore_ascii_case("true"))
                        .unwrap_or(false)
                    {
                        has_packable = true;
                    }
                }
            }
            let sname = stem(sln);
            let target = if slns.len() + projs.len() > 1 {
                format!(" {}", super::q(sln))
            } else {
                String::new()
            };
            let prefix = if slns.len() > 1 {
                format!("{}:", sname.to_ascii_lowercase())
            } else {
                String::new()
            };
            let d = |s: &str| format!("dotnet {s}{target}");
            out.actions.push(
                Action::new(format!("{prefix}build"), d("build"))
                    .tool("dotnet")
                    .inferred(Confidence::High)
                    .inferred_desc(format!("Build the {sname} solution"))
                    .cat(Category::Build),
            );
            if has_test {
                out.actions.push(
                    Action::new(format!("{prefix}test"), d("test"))
                        .tool("dotnet")
                        .inferred(Confidence::High)
                        .inferred_desc(format!("Run all {sname} solution tests"))
                        .cat(Category::Testing),
                );
            }
            out.actions.push(
                Action::new(format!("{prefix}restore"), d("restore"))
                    .tool("dotnet")
                    .inferred(Confidence::Medium)
                    .inferred_desc("Restore NuGet packages")
                    .cat(Category::Build),
            );
            out.actions.push(
                Action::new(format!("{prefix}clean"), d("clean"))
                    .tool("dotnet")
                    .inferred(Confidence::Low)
                    .inferred_desc("Delete .NET build outputs")
                    .cat(Category::Build),
            );
            if has_packable {
                out.actions.push(
                    Action::new(format!("{prefix}pack"), d("pack"))
                        .tool("dotnet")
                        .inferred(Confidence::Medium)
                        .inferred_desc("Create NuGet packages")
                        .cat(Category::Release),
                );
            }
            if let Some(manifest) = ctx
                .child(base, ".config")
                .and_then(|d| ctx.text(d, "dotnet-tools.json"))
            {
                if manifest.contains("\"cake.tool\"") && base.has("build.cake") {
                    out.actions.push(
                        Action::new("cake", "dotnet cake build.cake")
                            .tool("cake")
                            .inferred(Confidence::High)
                            .inferred_desc("Run the Cake build script")
                            .cat(Category::Build),
                    );
                }
                if manifest.contains("\"dotnet-format\"") || manifest.contains("\"csharpier\"") {
                    let cmd = if manifest.contains("\"csharpier\"") {
                        "dotnet csharpier ."
                    } else {
                        "dotnet format"
                    };
                    out.actions.push(
                        Action::new(format!("{prefix}format"), cmd)
                            .tool("dotnet")
                            .inferred(Confidence::Medium),
                    );
                }
            }
            if base.has("Directory.Build.props") || base.has(".editorconfig") {
                out.actions.push(
                    Action::new(format!("{prefix}format"), format!("dotnet format{target}"))
                        .tool("dotnet")
                        .inferred(Confidence::Low),
                );
            }
            if ctx.has_file(base, "build/_build.csproj")
                || ctx.has_file(base, ".nuke/build.schema.json")
            {
                out.actions.push(
                    Action::new("nuke", "dotnet nuke")
                        .tool("nuke")
                        .inferred(Confidence::High)
                        .inferred_desc("Run the NUKE build")
                        .cat(Category::Build),
                );
            }
        }

        // ---- project level --------------------------------------------------------------
        let multi = projs.len() > 1;
        for pf in &projs {
            let text = base.read(pf).unwrap_or_default();
            let fl = flavor(&text);
            let pname = stem(pf);
            let tfms = tag(&text, "TargetFramework").or_else(|| tag(&text, "TargetFrameworks"));
            for tfm in tfms
                .iter()
                .flat_map(|v| v.split(';'))
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                out.version("dotnet-tfm", tfm, *pf);
            }
            let target = if multi || !slns.is_empty() {
                format!(" {}", super::q(pf))
            } else {
                String::new()
            };
            let prefix = if multi {
                format!("{}:", pname.to_ascii_lowercase())
            } else {
                String::new()
            };
            let d = |s: &str| format!("dotnet {s}{target}");
            match fl {
                Flavor::Web => {
                    out.actions.push(
                        Action::new(format!("{prefix}run"), d("run"))
                            .tool("dotnet")
                            .inferred(Confidence::High)
                            .inferred_desc(format!("Start the .NET {pname} web app"))
                            .cat(Category::Development),
                    );
                    out.actions.push(
                        Action::new(format!("{prefix}watch"), d("watch run"))
                            .tool("dotnet")
                            .inferred(Confidence::Low)
                            .inferred_desc(format!("Start {pname} with hot reload"))
                            .cat(Category::Development),
                    );
                }
                Flavor::Blazor => {
                    out.actions.push(
                        Action::new(format!("{prefix}run"), d("run"))
                            .tool("dotnet")
                            .inferred(Confidence::High)
                            .inferred_desc(format!("Start the {pname} Blazor app"))
                            .cat(Category::Development),
                    );
                }
                Flavor::Worker => {
                    out.actions.push(
                        Action::new(format!("{prefix}run"), d("run"))
                            .tool("dotnet")
                            .inferred(Confidence::High)
                            .inferred_desc(format!("Run the .NET {pname} worker"))
                            .cat(Category::Development),
                    );
                }
                Flavor::Exe => {
                    out.actions.push(
                        Action::new(format!("{prefix}run"), d("run"))
                            .tool("dotnet")
                            .inferred(Confidence::High)
                            .inferred_desc(format!("Run the .NET {pname} app"))
                            .cat(Category::Development),
                    );
                }
                Flavor::Test => {
                    out.actions.push(
                        Action::new(format!("{prefix}test"), d("test"))
                            .tool("dotnet")
                            .inferred(Confidence::High)
                            .inferred_desc(format!("Run {pname} tests"))
                            .cat(Category::Testing),
                    );
                }
                Flavor::Library => {}
            }
            if fl != Flavor::Test {
                let conf = if slns.is_empty() {
                    Confidence::High
                } else {
                    Confidence::Medium
                };
                out.actions.push(
                    Action::new(format!("{prefix}build"), d("build"))
                        .tool("dotnet")
                        .inferred(conf)
                        .inferred_desc(format!("Build the {pname} project"))
                        .cat(Category::Build),
                );
            }
            if matches!(
                fl,
                Flavor::Web | Flavor::Worker | Flavor::Exe | Flavor::Blazor
            ) {
                out.actions.push(
                    Action::new(format!("{prefix}publish"), d("publish -c Release"))
                        .tool("dotnet")
                        .inferred(Confidence::Medium)
                        .inferred_desc(format!("Publish {pname} for deployment"))
                        .cat(Category::Release),
                );
            }
            if tag(&text, "IsPackable")
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
                || tag(&text, "PackAsTool").is_some()
            {
                out.actions.push(
                    Action::new(format!("{prefix}pack"), d("pack"))
                        .tool("dotnet")
                        .inferred(Confidence::Medium)
                        .inferred_desc(format!("Create the {pname} NuGet package"))
                        .cat(Category::Release),
                );
            }
            if has_package(&text, "Microsoft.EntityFrameworkCore.Design")
                || has_package(&text, "Microsoft.EntityFrameworkCore.Tools")
            {
                let proj_arg = if target.is_empty() {
                    String::new()
                } else {
                    format!(" --project {}", super::q(pf))
                };
                out.actions.push(
                    Action::new(
                        format!("{prefix}migrate"),
                        format!("dotnet ef database update{proj_arg}"),
                    )
                    .tool("dotnet")
                    .inferred(Confidence::Medium),
                );
                out.actions.push(
                    Action::new(
                        format!("{prefix}db:drop"),
                        format!("dotnet ef database drop{proj_arg}"),
                    )
                    .tool("dotnet")
                    .inferred(Confidence::Low),
                );
            }
        }
        let _ = ctx;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_solution() {
        let sln = r#"
Microsoft Visual Studio Solution File, Format Version 12.00
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "Api", "src\Api\Api.csproj", "{1}"
EndProject
Project("{2150E333-8FDC-42A3-9474-1A3956D46DE8}") = "src", "src", "{2}"
EndProject
"#;
        assert_eq!(solution_projects(sln, false), ["src/Api/Api.csproj"]);
        let slnx = r#"<Solution><Folder Name="/src/"><Project Path="src/Api/Api.csproj" /></Folder></Solution>"#;
        assert_eq!(solution_projects(slnx, true), ["src/Api/Api.csproj"]);
    }

    #[test]
    fn flavors() {
        assert_eq!(
            flavor(r#"<Project Sdk="Microsoft.NET.Sdk.Web"></Project>"#),
            Flavor::Web
        );
        assert_eq!(
            flavor(
                r#"<Project Sdk="Microsoft.NET.Sdk"><PackageReference Include="xunit" /></Project>"#
            ),
            Flavor::Test
        );
        assert_eq!(
            flavor(r#"<Project Sdk="Microsoft.NET.Sdk"><OutputType>Exe</OutputType></Project>"#),
            Flavor::Exe
        );
        assert_eq!(
            flavor(r#"<Project Sdk="Microsoft.NET.Sdk"></Project>"#),
            Flavor::Library
        );
    }
}
