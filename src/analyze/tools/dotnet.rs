//! .NET: one slice of the tool knowledge table behind `analyze::tools::summarize`.

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
        "dotnet" => {
            let p = a.positionals();
            let target = p
                .get(1)
                .copied()
                .filter(|t| t.ends_with("proj") || t.ends_with(".sln") || t.ends_with(".slnx"));
            let named = |verb: &str, default: &str| -> String {
                match target {
                    Some(t) => format!(
                        "{verb} {}",
                        std::path::Path::new(t)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(t)
                    ),
                    None => default.to_string(),
                }
            };
            match sub {
                Some("build") => s(
                    "dotnet",
                    named("Build", "Build the .NET project"),
                    Build,
                    Safe,
                ),
                Some("run") => s(
                    "dotnet",
                    match a.value("--project") {
                        Some(pr) => format!(
                            "Run the {} project",
                            std::path::Path::new(pr.trim_end_matches('/'))
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or(pr)
                        ),
                        None => "Run the .NET project".into(),
                    },
                    Run,
                    Safe,
                ),
                Some("test") => s(
                    "dotnet",
                    named("Run tests for", "Run .NET tests"),
                    Test,
                    Safe,
                ),
                Some("restore") => s("dotnet", "Restore NuGet packages", Install, Safe),
                Some("clean") => s("dotnet", "Clean .NET build outputs", Clean, Safe),
                Some("pack") => s("dotnet", "Create NuGet packages", Build, Safe),
                Some("publish") => s(
                    "dotnet",
                    named("Publish", "Publish the .NET app for deployment"),
                    Build,
                    Safe,
                ),
                Some("watch") => s(
                    "dotnet",
                    match p.get(1).copied() {
                        Some("test") => "Run .NET tests on change".to_string(),
                        _ => "Run the .NET project with hot reload".to_string(),
                    },
                    Dev,
                    Safe,
                ),
                Some("format") => s(
                    "dotnet",
                    if a.has("--verify-no-changes") {
                        "Check .NET formatting"
                    } else {
                        "Format .NET source code"
                    },
                    Format,
                    Safe,
                ),
                Some("nuget") => match p.get(1).copied() {
                    Some("push") => s("dotnet", "Push NuGet packages to a feed", Publish, External),
                    _ => s("dotnet", "Run dotnet nuget", Other, Safe),
                },
                Some("tool") => match p.get(1).copied() {
                    Some("restore") => s("dotnet", "Restore .NET local tools", Install, Safe),
                    Some("install") | Some("update") => {
                        s("dotnet", "Install .NET tools", Install, Safe)
                    }
                    Some("run") => s(
                        "dotnet",
                        format!("Run the {} tool", p.get(2).copied().unwrap_or("")),
                        Other,
                        Safe,
                    ),
                    _ => s("dotnet", "Manage .NET tools", Other, Safe),
                },
                Some("ef") => match (p.get(1).copied(), p.get(2).copied()) {
                    (Some("database"), Some("update")) => s(
                        "dotnet-ef",
                        "Apply EF Core migrations to the database",
                        Migrate,
                        Safe,
                    ),
                    (Some("database"), Some("drop")) => s(
                        "dotnet-ef",
                        "Drop the database with EF Core",
                        Db,
                        Destructive,
                    ),
                    (Some("migrations"), Some("add")) => {
                        s("dotnet-ef", "Create an EF Core migration", Migrate, Safe)
                    }
                    (Some("migrations"), Some("remove")) => s(
                        "dotnet-ef",
                        "Remove the last EF Core migration",
                        Migrate,
                        Safe,
                    ),
                    (Some("migrations"), Some("script")) => {
                        s("dotnet-ef", "Generate an EF Core SQL script", Migrate, Safe)
                    }
                    (Some("migrations"), Some("list")) => {
                        s("dotnet-ef", "List EF Core migrations", Db, Safe)
                    }
                    _ => s("dotnet-ef", "Run dotnet ef", Db, Safe),
                },
                Some("cake") => s("cake", "Run the Cake build script", Build, Safe),
                Some("nuke") => s(
                    "nuke",
                    format!(
                        "Run the {} NUKE target",
                        p.get(1).copied().unwrap_or("default")
                    ),
                    Build,
                    Safe,
                ),
                Some("fantomas") => s("fantomas", "Format F# source with Fantomas", Format, Safe),
                Some("fsi") => s("dotnet", "Run an F# script", Run, Safe),
                Some("new") => s("dotnet", "Create a new .NET project", Other, Safe),
                Some("sln") => s("dotnet", "Modify the solution file", Other, Safe),
                Some("workload") => s("dotnet", "Manage .NET workloads", Install, Safe),
                Some("dev-certs") => s(
                    "dotnet",
                    "Manage HTTPS development certificates",
                    Other,
                    Safe,
                ),
                Some("user-secrets") => s("dotnet", "Manage user secrets", Other, Safe),
                Some("outdated") => s("dotnet", "List outdated NuGet packages", Other, Safe),
                Some("csharpier") => s("dotnet", "Format C# source with CSharpier", Format, Safe),
                Some(x) => s("dotnet", format!("Run dotnet {x}"), Other, Safe),
                None => s("dotnet", "Run dotnet", Other, Safe),
            }
        }
        "nuke" => s(
            "nuke",
            format!("Run the {} NUKE target", sub.unwrap_or("default")),
            Build,
            Safe,
        ),
        "msbuild" => s("msbuild", "Build with MSBuild", Build, Safe),
        "nuget" => s(
            "nuget",
            if sub == Some("push") {
                "Push NuGet packages to a feed"
            } else {
                "Run NuGet"
            },
            if sub == Some("push") { Publish } else { Other },
            if sub == Some("push") { External } else { Safe },
        ),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_sum as sum;
    use super::*;

    #[test]
    fn dotnet_arms() {
        assert_eq!(sum("dotnet build").text, "Build the .NET project");
        assert_eq!(sum("dotnet build Api.csproj").text, "Build Api");
        assert_eq!(sum("dotnet test").kind, Test);
        assert_eq!(
            sum("dotnet run --project src/Api").text,
            "Run the Api project"
        );
        assert_eq!(sum("dotnet watch").kind, Dev);
        assert_eq!(
            sum("dotnet format --verify-no-changes").text,
            "Check .NET formatting"
        );
        assert_eq!(sum("dotnet nuget push pkg.nupkg").risk, External);
        assert_eq!(sum("dotnet publish -c Release").risk, Safe);
    }
}
