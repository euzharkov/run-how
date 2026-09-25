//! Go: one slice of the tool knowledge table behind `analyze::tools::summarize`.

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
        "go" => {
            let pkg = a.positionals().get(1).copied();
            let scope = |default: &str| -> String {
                match pkg {
                    Some("./...") | None => default.to_string(),
                    Some(".") => default.to_string(),
                    Some(p) => clean_path(p),
                }
            };
            match sub {
                Some("build") => s(
                    "go",
                    if pkg.is_none() || pkg == Some("./...") {
                        "Build Go packages".into()
                    } else {
                        format!("Build {}", scope("Go packages"))
                    },
                    Build,
                    Safe,
                ),
                Some("test") => s(
                    "go",
                    if a.has("-race") {
                        "Run Go tests with the race detector".into()
                    } else if a.has("-bench") {
                        "Run Go benchmarks".into()
                    } else if a.has("-cover") || a.has("-coverprofile") {
                        "Run Go tests with coverage".into()
                    } else if pkg.is_none() || pkg == Some("./...") {
                        "Run Go tests".into()
                    } else {
                        format!("Run Go tests in {}", scope(""))
                    },
                    if a.has("-bench") { Bench } else { Test },
                    Safe,
                ),
                Some("vet") => s("go", "Check Go source with go vet", Lint, Safe),
                Some("fmt") => s("go", "Format Go source", Format, Safe),
                Some("run") => s(
                    "go",
                    format!(
                        "Run {}",
                        match pkg {
                            Some(".") | None => "the Go program".to_string(),
                            Some(p) => clean_path(p),
                        }
                    ),
                    Run,
                    Safe,
                ),
                Some("generate") => s("go", "Run go:generate directives", Generate, Safe),
                Some("install") => s("go", "Install Go binaries", Install, Safe),
                Some("mod") => match pkg {
                    Some("tidy") => s("go", "Tidy Go module dependencies", Install, Safe),
                    Some("download") => s("go", "Download Go module dependencies", Install, Safe),
                    Some("vendor") => s("go", "Vendor Go module dependencies", Install, Safe),
                    Some("verify") => s("go", "Verify Go module dependencies", Lint, Safe),
                    _ => s("go", "Manage Go modules", Install, Safe),
                },
                Some("work") => s("go", "Manage the Go workspace", Install, Safe),
                Some("tool") => s(
                    "go",
                    format!("Run the Go tool {}", pkg.unwrap_or("")),
                    Other,
                    Safe,
                ),
                Some("clean") => s("go", "Clean Go build cache and outputs", Clean, Safe),
                Some("get") => s("go", "Add or update Go dependencies", Install, Safe),
                Some("doc") => s("go", "Show Go documentation", Docs, Safe),
                Some(x) => s("go", format!("Run go {x}"), Other, Safe),
                None => s("go", "Run go", Other, Safe),
            }
        }
        "gofmt" => s(
            "gofmt",
            if a.has("-l") {
                "Check Go formatting"
            } else {
                "Format Go source"
            },
            Format,
            Safe,
        ),
        "goimports" => s("goimports", "Format Go imports", Format, Safe),
        "gofumpt" => s("gofumpt", "Format Go source with gofumpt", Format, Safe),
        "golangci-lint" => s(
            "golangci-lint",
            if sub == Some("fmt") {
                "Format Go source with golangci-lint"
            } else {
                "Lint Go source with golangci-lint"
            },
            Lint,
            Safe,
        ),
        "staticcheck" => s(
            "staticcheck",
            "Check Go source with staticcheck",
            Lint,
            Safe,
        ),
        "govulncheck" => s(
            "govulncheck",
            "Scan Go dependencies for vulnerabilities",
            Lint,
            Safe,
        ),
        "gotestsum" => s("gotestsum", "Run Go tests with gotestsum", Test, Safe),
        "air" => s("air", "Run the Go app with live reload", Dev, Safe),
        "goreleaser" => match sub {
            Some("release") => s(
                "goreleaser",
                if a.has("--snapshot") {
                    "Build a GoReleaser snapshot"
                } else {
                    "Publish a release with GoReleaser"
                },
                Publish,
                if a.has("--snapshot") { Safe } else { External },
            ),
            Some("build") => s(
                "goreleaser",
                "Build release binaries with GoReleaser",
                Build,
                Safe,
            ),
            Some("check") => s("goreleaser", "Validate the GoReleaser config", Lint, Safe),
            Some(x) => s("goreleaser", format!("Run goreleaser {x}"), Other, Safe),
            None => s("goreleaser", "Run GoReleaser", Other, Safe),
        },
        "mockgen" | "mockery" => s(program, "Generate Go mocks", Generate, Safe),
        "swag" => s(
            "swag",
            "Generate Swagger docs from Go annotations",
            Generate,
            Safe,
        ),
        "sqlc" => s(
            "sqlc",
            "Generate Go code from SQL with sqlc",
            Generate,
            Safe,
        ),
        "buf" => match sub {
            Some("generate") => s(
                "buf",
                "Generate code from Protobuf with buf",
                Generate,
                Safe,
            ),
            Some("lint") => s("buf", "Lint Protobuf definitions", Lint, Safe),
            Some("breaking") => s("buf", "Check Protobuf for breaking changes", Lint, Safe),
            Some("push") => s(
                "buf",
                "Push Protobuf schemas to the registry",
                Publish,
                External,
            ),
            Some(x) => s("buf", format!("Run buf {x}"), Other, Safe),
            None => s("buf", "Run buf", Other, Safe),
        },
        "protoc" | "protoc-gen-go" => s("protoc", "Compile Protobuf definitions", Generate, Safe),
        "graphql-codegen" | "gql-gen" | "graphql-code-generator" => {
            s("graphql-codegen", "Generate GraphQL types", Generate, Safe)
        }
        "openapi-generator"
        | "openapi-generator-cli"
        | "openapi-typescript"
        | "orval"
        | "kubb"
        | "swagger-codegen" => s(program, "Generate API client code", Generate, Safe),
        "wasm-pack" => s(
            "wasm-pack",
            if sub == Some("build") {
                "Build the WebAssembly package"
            } else {
                "Run wasm-pack"
            },
            Build,
            Safe,
        ),
        "trunk" => match sub {
            Some("serve") => s("trunk", "Serve the WebAssembly app with Trunk", Dev, Safe),
            Some("build") => s("trunk", "Build the WebAssembly app with Trunk", Build, Safe),
            Some(x) => s("trunk", format!("Run trunk {x}"), Other, Safe),
            None => s("trunk", "Run Trunk", Other, Safe),
        },

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_sum as sum;
    use super::*;

    #[test]
    fn go_arms() {
        assert_eq!(sum("go build ./...").text, "Build Go packages");
        assert_eq!(sum("go build ./cmd/api").text, "Build cmd/api");
        assert_eq!(
            sum("go test -race ./...").text,
            "Run Go tests with the race detector"
        );
        assert_eq!(sum("go test -bench=. ./...").kind, Bench);
        assert_eq!(sum("go vet ./...").kind, Lint);
        assert_eq!(sum("golangci-lint run").kind, Lint);
        assert_eq!(sum("goreleaser release").risk, External);
        assert_eq!(sum("air").kind, Dev);
    }
}
