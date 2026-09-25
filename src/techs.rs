//! The technologies a project visibly uses, for the `[.NET, Kafka, Docker]` tag on a
//! project title.
//!
//! Everything here is read off facts discovery already has: the project's kind, the tool
//! families that produced its actions, the tools command analysis recognised, and the
//! images behind Compose services. Nothing is inferred from dependencies or source code.
//! Utilities, linters and formatters are deliberately not technologies: the tag names what
//! the project is built with and runs on, not every program its scripts call.

use crate::model::{Action, ProjectKind};
use std::collections::HashMap;

/// How many technologies a title shows.
pub const MAX: usize = 5;

/// The language or platform a project kind implies, when it implies one.
pub fn from_kind(kind: ProjectKind) -> Option<&'static str> {
    Some(match kind {
        ProjectKind::JavaScript => "Node.js",
        ProjectKind::Python => "Python",
        ProjectKind::Go => "Go",
        ProjectKind::Rust => "Rust",
        ProjectKind::Ruby => "Ruby",
        ProjectKind::Jvm => "JVM",
        ProjectKind::Php => "PHP",
        ProjectKind::Deno => "Deno",
        ProjectKind::DotNet | ProjectKind::DotNetSolution => ".NET",
        ProjectKind::Bazel => "Bazel",
        ProjectKind::Docker => "Docker",
        ProjectKind::Kubernetes => "Kubernetes",
        _ => return None,
    })
}

/// The technology behind an adapter family (`Action::tool`), when the family is one.
pub fn from_family(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "compose" | "docker" => "Docker",
        "kubernetes" | "k8s" | "kustomize" => "Kubernetes",
        "helm" => "Helm",
        "terraform" | "tf" => "Terraform",
        "tofu" => "OpenTofu",
        "terragrunt" | "tg" => "Terragrunt",
        "pulumi" => "Pulumi",
        "ansible" => "Ansible",
        "nx" => "Nx",
        "turbo" => "Turborepo",
        "rush" => "Rush",
        "moon" => "moon",
        "bazel" => "Bazel",
        "gradle" => "Gradle",
        "maven" | "mvn" => "Maven",
        "sbt" => "sbt",
        "flutter" => "Flutter",
        "xcode" | "xcodebuild" => "Xcode",
        "swift" | "swiftpm" => "Swift",
        "fastlane" => "Fastlane",
        "expo" => "Expo",
        "react-native" => "React Native",
        "detox" => "Detox",
        "maestro" => "Maestro",
        "deno" => "Deno",
        "cargo" => "Rust",
        "go" => "Go",
        "dotnet" => ".NET",
        "mix" => "Elixir",
        "zig" => "Zig",
        "cmake" => "CMake",
        "stack" | "cabal" => "Haskell",
        "lein" | "clojure" => "Clojure",
        "composer" | "artisan" => "PHP",
        "rails" => "Rails",
        "django" => "Django",
        _ => return None,
    })
}

/// The technology a recognised command-line tool stands for (`Summary::tool`), when it is
/// a framework, runtime, platform, database or service rather than a utility.
pub fn from_tool(tool: &str) -> Option<&'static str> {
    Some(match tool {
        // Frameworks and runtimes
        "next" => "Next.js",
        "nest" => "NestJS",
        "nuxt" => "Nuxt",
        "remix" | "react-router" => "React Router",
        "astro" => "Astro",
        "svelte-kit" => "SvelteKit",
        "angular" => "Angular",
        "vue-cli" => "Vue",
        "vite" => "Vite",
        "webpack" => "webpack",
        "storybook" => "Storybook",
        "electron" | "electron-forge" | "electron-builder" => "Electron",
        "tauri" => "Tauri",
        "expo" | "eas" => "Expo",
        "react-native" => "React Native",
        "flutter" => "Flutter",
        "xcode" => "Xcode",
        "fastlane" => "Fastlane",
        "detox" => "Detox",
        "maestro" => "Maestro",
        "django" => "Django",
        "flask" => "Flask",
        "fastapi" | "uvicorn" => "FastAPI",
        "celery" => "Celery",
        "rails" => "Rails",
        "sidekiq" => "Sidekiq",
        "artisan" => "Laravel",
        "mix" | "iex" => "Elixir",
        "wrangler" => "Cloudflare Workers",
        "node" => "Node.js",
        "deno" => "Deno",
        "bun" => "Bun",
        "python" | "pytest" | "uv" | "poetry" | "pip" => "Python",
        "go" => "Go",
        "cargo" => "Rust",
        "dotnet" | "msbuild" => ".NET",
        "gradle" => "Gradle",
        "maven" => "Maven",
        "sbt" => "sbt",
        "swift" => "Swift",
        // Tests
        "vitest" => "Vitest",
        "jest" => "Jest",
        "playwright" => "Playwright",
        "cypress" => "Cypress",
        // Data
        "prisma" => "Prisma",
        "drizzle-kit" => "Drizzle",
        "typeorm" => "TypeORM",
        "sequelize" => "Sequelize",
        "knex" => "Knex",
        "mikro-orm" => "MikroORM",
        "dotnet-ef" => "EF Core",
        "dbt" => "dbt",
        "supabase" => "Supabase",
        "kafka" | "redpanda" => "Kafka",
        "rabbitmq" => "RabbitMQ",
        "nats" => "NATS",
        "graphql-codegen" => "GraphQL",
        "protoc" | "buf" => "Protobuf",
        // Containers, clusters, clouds
        "docker" | "docker compose" | "compose" => "Docker",
        "kubectl" | "kustomize" => "Kubernetes",
        "helm" | "helmfile" => "Helm",
        "skaffold" => "Skaffold",
        "tilt" => "Tilt",
        "argocd" => "Argo CD",
        "terraform" => "Terraform",
        "tofu" => "OpenTofu",
        "terragrunt" => "Terragrunt",
        "pulumi" => "Pulumi",
        "ansible" | "ansible-playbook" => "Ansible",
        "serverless" | "sam" => "AWS",
        "sst" => "SST",
        "amplify" => "AWS Amplify",
        "fly" => "Fly.io",
        "vercel" => "Vercel",
        "netlify" => "Netlify",
        "heroku" => "Heroku",
        "firebase" => "Firebase",
        "sentry-cli" => "Sentry",
        "stripe" => "Stripe",
        // Monorepo tooling
        "nx" => "Nx",
        "turbo" => "Turborepo",
        "lerna" => "Lerna",
        "rush" => "Rush",
        "bazel" => "Bazel",
        _ => return None,
    })
}

/// The technology behind a Compose service, from its image name (`postgres:16`,
/// `confluentinc/cp-kafka`), or from the service name when there is no image.
pub fn from_image(service: &str, image: Option<&str>) -> Option<&'static str> {
    let img = image.unwrap_or("").to_ascii_lowercase();
    let name = img.split([':', '@']).next().unwrap_or("");
    let last = name.rsplit('/').next().unwrap_or("");
    let svc = service.to_ascii_lowercase();
    let probe = |needle: &str| last.contains(needle) || (image.is_none() && svc.contains(needle));
    let table: &[(&str, &str)] = &[
        ("postgis", "PostgreSQL"),
        ("pgvector", "PostgreSQL"),
        ("postgres", "PostgreSQL"),
        ("timescale", "PostgreSQL"),
        ("mysql", "MySQL"),
        ("mariadb", "MariaDB"),
        ("mssql", "SQL Server"),
        ("mongo", "MongoDB"),
        ("redis", "Redis"),
        ("valkey", "Redis"),
        ("kafka", "Kafka"),
        ("redpanda", "Kafka"),
        ("zookeeper", "ZooKeeper"),
        ("rabbitmq", "RabbitMQ"),
        ("nats", "NATS"),
        ("elasticsearch", "Elasticsearch"),
        ("opensearch", "OpenSearch"),
        ("minio", "MinIO"),
        ("localstack", "LocalStack"),
        ("clickhouse", "ClickHouse"),
        ("memcached", "Memcached"),
        ("cassandra", "Cassandra"),
        ("neo4j", "Neo4j"),
        ("keycloak", "Keycloak"),
        ("mailhog", "MailHog"),
        ("mailpit", "Mailpit"),
        ("prometheus", "Prometheus"),
        ("grafana", "Grafana"),
        ("jaeger", "Jaeger"),
        ("otel", "OpenTelemetry"),
        ("temporal", "Temporal"),
        ("nginx", "nginx"),
        ("traefik", "Traefik"),
        ("caddy", "Caddy"),
        ("ollama", "Ollama"),
        ("qdrant", "Qdrant"),
        ("weaviate", "Weaviate"),
        ("milvus", "Milvus"),
        ("chroma", "Chroma"),
    ];
    table
        .iter()
        .find(|(needle, _)| probe(needle))
        .map(|(_, tech)| *tech)
}

/// The technologies of a project: its kind first, then everything its actions name, most
/// mentioned first, ties in first-seen order, at most [`MAX`]. Hidden actions count too:
/// a `kafka:reset` behind `--all` still says the project talks to Kafka.
pub fn of_project(kind: ProjectKind, actions: &[Action]) -> Vec<String> {
    let mut count: HashMap<String, usize> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut note = |t: &str| {
        let n = count.entry(t.to_string()).or_insert(0);
        if *n == 0 {
            order.push(t.to_string());
        }
        *n += 1;
    };
    for a in actions {
        if let Some(t) = from_family(a.tool) {
            note(t);
        }
        for t in &a.techs {
            note(t);
        }
    }
    // The platform an app is built for (Xcode, Flutter, Expo) says more than how many
    // lanes or scripts mention something else, so it sorts ahead of the mention count.
    let platform = |t: &str| {
        matches!(
            t,
            "Xcode" | "Flutter" | "Expo" | "React Native" | "Swift" | "Electron" | "Tauri"
        )
    };
    let mut ranked: Vec<(&str, usize)> = order.iter().map(|t| (t.as_str(), count[t])).collect();
    ranked.sort_by(|a, b| platform(b.0).cmp(&platform(a.0)).then(b.1.cmp(&a.1)));
    let mut out: Vec<String> = Vec::new();
    if let Some(k) = from_kind(kind) {
        out.push(k.to_string());
    }
    for (t, _) in ranked {
        if out.len() >= MAX {
            break;
        }
        if !out.iter().any(|o| o == t) {
            out.push(t.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_map_to_technologies() {
        assert_eq!(from_image("db", Some("postgres:16")), Some("PostgreSQL"));
        assert_eq!(
            from_image("broker", Some("confluentinc/cp-kafka:7.6.0")),
            Some("Kafka")
        );
        assert_eq!(from_image("cache", Some("redis:7-alpine")), Some("Redis"));
        assert_eq!(from_image("redis", None), Some("Redis"));
        assert_eq!(from_image("api", Some("ghcr.io/acme/api")), None);
        // A service named like a database but built from an image that is not one.
        assert_eq!(from_image("postgres-exporter", Some("acme/exporter")), None);
    }

    #[test]
    fn utilities_are_not_technologies() {
        assert_eq!(from_tool("rm"), None);
        assert_eq!(from_tool("eslint"), None);
        assert_eq!(from_tool("prettier"), None);
        assert_eq!(from_tool("echo"), None);
        assert_eq!(from_tool("next"), Some("Next.js"));
    }

    #[test]
    fn project_techs_lead_with_the_kind_and_rank_by_mentions() {
        let a = |tool: &'static str, techs: &[&str]| {
            let mut a = Action::new("x", "x").tool(tool);
            a.techs = techs.iter().map(|s| s.to_string()).collect();
            a
        };
        let actions = vec![
            a("pnpm", &["Vitest"]),
            a("pnpm", &["Kafka"]),
            a("pnpm", &["Kafka"]),
            a("compose", &["PostgreSQL"]),
            a("compose", &["Kafka"]),
            a("kubernetes", &[]),
            a("sh", &[]),
        ];
        assert_eq!(
            of_project(ProjectKind::JavaScript, &actions),
            ["Node.js", "Kafka", "Docker", "Vitest", "PostgreSQL"]
        );
        assert_eq!(of_project(ProjectKind::Root, &[]), Vec::<String>::new());
        // The kind never repeats when a tool says the same thing.
        assert_eq!(
            of_project(ProjectKind::DotNet, &[a("dotnet", &[".NET"])]),
            [".NET"]
        );
    }
}
