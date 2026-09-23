//! Docker: `Dockerfile` image builds and Docker Compose services (first-class).
//!
//! Kafka and other infrastructure services are only surfaced when they exist as Compose
//! services; nothing is invented from dependencies alone.

use super::{Context, Discoverer, Discovery};
use crate::model::*;
use crate::repo::DirInfo;
use serde_yaml::Value;

pub struct Docker;

const COMPOSE: &[&str] = &[
    "compose.yaml",
    "compose.yml",
    "docker-compose.yaml",
    "docker-compose.yml",
];

/// Description for a service, derived from its image name when it is a well-known one.
pub fn service_description(name: &str, image: Option<&str>, has_build: bool) -> String {
    let img = image.unwrap_or("").to_ascii_lowercase();
    let img_name = img.split(':').next().unwrap_or("");
    let last = img_name.rsplit('/').next().unwrap_or("");
    let known: &[(&str, &str)] = &[
        ("cp-kafka", "Start local Kafka broker"),
        ("kafka", "Start local Kafka broker"),
        ("redpanda", "Start local Redpanda (Kafka-compatible) broker"),
        ("cp-zookeeper", "Start ZooKeeper"),
        ("zookeeper", "Start ZooKeeper"),
        ("cp-schema-registry", "Start the Kafka Schema Registry"),
        ("schema-registry", "Start the Kafka Schema Registry"),
        ("kafka-ui", "Start the Kafka UI"),
        ("kafdrop", "Start the Kafdrop Kafka UI"),
        ("postgres", "Start PostgreSQL"),
        ("postgis", "Start PostgreSQL with PostGIS"),
        ("timescaledb", "Start TimescaleDB"),
        ("pgvector", "Start PostgreSQL with pgvector"),
        ("pgbouncer", "Start PgBouncer"),
        ("pgadmin4", "Start pgAdmin"),
        ("mysql", "Start MySQL"),
        ("mariadb", "Start MariaDB"),
        ("server", "Start SQL Server"),
        ("azure-sql-edge", "Start Azure SQL Edge"),
        ("redis", "Start Redis"),
        ("redis-stack", "Start Redis Stack"),
        ("valkey", "Start Valkey"),
        ("keydb", "Start KeyDB"),
        ("memcached", "Start Memcached"),
        ("mongo", "Start MongoDB"),
        ("mongodb", "Start MongoDB"),
        ("rabbitmq", "Start RabbitMQ"),
        ("nats", "Start NATS"),
        ("activemq", "Start ActiveMQ"),
        ("elasticsearch", "Start Elasticsearch"),
        ("opensearch", "Start OpenSearch"),
        ("kibana", "Start Kibana"),
        ("meilisearch", "Start Meilisearch"),
        ("typesense", "Start Typesense"),
        ("clickhouse", "Start ClickHouse"),
        ("clickhouse-server", "Start ClickHouse"),
        ("cassandra", "Start Cassandra"),
        ("scylla", "Start ScyllaDB"),
        ("neo4j", "Start Neo4j"),
        ("influxdb", "Start InfluxDB"),
        ("couchbase", "Start Couchbase"),
        ("couchdb", "Start CouchDB"),
        ("dynamodb-local", "Start local DynamoDB"),
        ("localstack", "Start LocalStack (local AWS)"),
        ("azurite", "Start Azurite (local Azure Storage)"),
        ("cosmosdb-emulator", "Start the Cosmos DB emulator"),
        ("minio", "Start MinIO object storage"),
        ("nginx", "Start nginx"),
        ("traefik", "Start Traefik"),
        ("caddy", "Start Caddy"),
        ("haproxy", "Start HAProxy"),
        ("mailhog", "Start MailHog (test mail server)"),
        ("mailpit", "Start Mailpit (test mail server)"),
        ("maildev", "Start MailDev (test mail server)"),
        ("grafana", "Start Grafana"),
        ("prometheus", "Start Prometheus"),
        ("jaeger", "Start Jaeger tracing"),
        ("all-in-one", "Start Jaeger tracing"),
        ("otel-collector", "Start the OpenTelemetry collector"),
        (
            "opentelemetry-collector",
            "Start the OpenTelemetry collector",
        ),
        (
            "opentelemetry-collector-contrib",
            "Start the OpenTelemetry collector",
        ),
        ("loki", "Start Loki"),
        ("tempo", "Start Tempo"),
        ("keycloak", "Start Keycloak"),
        ("vault", "Start HashiCorp Vault"),
        ("consul", "Start Consul"),
        ("temporal", "Start Temporal"),
        ("auto-setup", "Start Temporal"),
        ("adminer", "Start Adminer"),
        ("phpmyadmin", "Start phpMyAdmin"),
        ("redisinsight", "Start RedisInsight"),
        ("redis-commander", "Start Redis Commander"),
        ("mongo-express", "Start Mongo Express"),
        ("wiremock", "Start WireMock"),
        ("mockserver", "Start MockServer"),
        ("selenium", "Start Selenium"),
        ("stripe-mock", "Start stripe-mock"),
        ("ollama", "Start Ollama"),
        ("qdrant", "Start Qdrant"),
        ("weaviate", "Start Weaviate"),
        ("milvus", "Start Milvus"),
        ("chroma", "Start Chroma"),
        ("sonarqube", "Start SonarQube"),
        ("gitea", "Start Gitea"),
        ("registry", "Start a local container registry"),
        ("unleash-server", "Start Unleash"),
        ("flagd", "Start flagd"),
    ];
    if img_name.contains("mssql") {
        return "Start SQL Server".into();
    }
    if img_name.contains("kafka") {
        return "Start local Kafka broker".into();
    }
    if img_name.contains("zookeeper") {
        return "Start ZooKeeper".into();
    }
    for (k, d) in known {
        if last == *k {
            return d.to_string();
        }
    }
    let lname = name.to_ascii_lowercase();
    for (k, d) in known {
        if lname == *k || (k.len() > 4 && lname.starts_with(k)) {
            return d.to_string();
        }
    }
    if has_build {
        format!("Start the {name} service (built from source)")
    } else if !img_name.is_empty() {
        format!("Start the {name} service ({last})")
    } else {
        format!("Start the {name} service")
    }
}

impl Discoverer for Docker {
    fn id(&self) -> &'static str {
        "docker"
    }
    fn kind(&self) -> ProjectKind {
        ProjectKind::Docker
    }
    fn detect(&self, dir: &DirInfo) -> bool {
        dir.has_any(COMPOSE)
            || dir
                .files
                .iter()
                .any(|f| f == "Dockerfile" || f.starts_with("Dockerfile."))
    }
    fn attachable(&self) -> bool {
        true
    }
    fn attach_dir_names(&self) -> Option<&'static [&'static str]> {
        Some(&[
            "docker",
            "compose",
            "deploy",
            "dev",
            "infra",
            "local",
            "docker-compose",
            "ops",
            "devops",
            "environment",
            "env",
        ])
    }
    fn discover(&self, ctx: &Context, base: &DirInfo, dirs: &[&DirInfo]) -> Discovery {
        let mut out = Discovery::default();
        let multi_dirs = dirs.len() > 1;
        for dir in dirs {
            let at_base = dir.rel == base.rel;
            let sub = ctx.rel_from(base, dir);
            let pre = if at_base || !multi_dirs {
                String::new()
            } else {
                format!("{}:", super::attach_prefix(ctx, base, dir, dirs))
            };

            // ---- Dockerfile ---------------------------------------------------------------
            let dockerfiles: Vec<&String> = dir
                .files
                .iter()
                .filter(|f| *f == "Dockerfile" || f.starts_with("Dockerfile."))
                .collect();
            for df in &dockerfiles {
                let file_arg = if *df == "Dockerfile" && at_base {
                    String::new()
                } else {
                    format!(" -f {}", super::q(&dir.rel_to(base, df)))
                };
                let variant = df
                    .strip_prefix("Dockerfile.")
                    .map(|v| format!(":{v}"))
                    .unwrap_or_default();
                let image = base.name().to_ascii_lowercase();
                let id = format!("{pre}image{variant}");
                out.actions.push(
                    Action::new(id, format!("docker build{file_arg} -t {image} ."))
                        .tool("docker")
                        .inferred(if dockerfiles.len() > 2 {
                            Confidence::Medium
                        } else {
                            Confidence::High
                        })
                        .inferred_desc(if variant.is_empty() {
                            "Build the Docker image".to_string()
                        } else {
                            format!("Build the Docker image from {df}")
                        })
                        .cat(Category::Build),
                );
            }

            // ---- Compose --------------------------------------------------------------------
            let Some(file) = dir.first_of(COMPOSE) else {
                continue;
            };
            let Some(text) = dir.read(file) else { continue };
            let Ok(doc) = serde_yaml::from_str::<Value>(&text) else {
                continue;
            };
            let file_arg = if at_base {
                String::new()
            } else {
                format!(" -f {}", super::q(&format!("{sub}/{file}")))
            };
            let dc = |rest: &str| format!("docker compose{file_arg} {rest}");
            let services = doc.get("services").and_then(|s| s.as_mapping());
            let has_build = services
                .map(|m| m.values().any(|v| v.get("build").is_some()))
                .unwrap_or(false);
            let has_volumes = doc
                .get("volumes")
                .and_then(|v| v.as_mapping())
                .map(|m| !m.is_empty())
                .unwrap_or(false);
            let conf = Confidence::High;

            out.actions.push(
                Action::new(format!("{pre}services"), dc("up -d"))
                    .tool("compose")
                    .inferred(conf)
                    .inferred_desc("Start all Docker services")
                    .cat(Category::Infrastructure),
            );
            if let Some(m) = services {
                for (k, v) in m {
                    let Some(name) = k.as_str() else { continue };
                    let image = v.get("image").and_then(|i| i.as_str());
                    let build = v.get("build").is_some();
                    let desc = service_description(name, image, build);
                    out.actions.push(
                        Action::new(format!("{pre}{name}"), dc(&format!("up -d {name}")))
                            .tool("compose")
                            .inferred(conf)
                            .inferred_desc(desc)
                            .cat(Category::Infrastructure),
                    );
                }
            }
            let mut profiles: Vec<String> = Vec::new();
            if let Some(m) = services {
                for v in m.values() {
                    if let Some(ps) = v.get("profiles").and_then(|p| p.as_sequence()) {
                        for p in ps.iter().filter_map(|p| p.as_str()) {
                            if !profiles.iter().any(|x| x == p) {
                                profiles.push(p.to_string());
                            }
                        }
                    }
                }
            }
            for p in &profiles {
                out.actions.push(
                    Action::new(
                        format!("{pre}services:{p}"),
                        dc(&format!("--profile {p} up -d")),
                    )
                    .tool("compose")
                    .inferred(conf)
                    .inferred_desc(format!("Start Docker services in the {p} profile"))
                    .cat(Category::Infrastructure),
                );
            }
            out.actions.push(
                Action::new(format!("{pre}logs"), dc("logs -f"))
                    .tool("compose")
                    .inferred(conf)
                    .inferred_desc("Follow Docker service logs")
                    .cat(Category::Infrastructure),
            );
            if has_build {
                out.actions.push(
                    Action::new(format!("{pre}services:build"), dc("build"))
                        .tool("compose")
                        .inferred(conf)
                        .inferred_desc("Build Docker service images")
                        .cat(Category::Infrastructure),
                );
            }
            out.actions.push(
                Action::new(format!("{pre}services:down"), dc("down"))
                    .tool("compose")
                    .inferred(conf)
                    .inferred_desc("Stop and remove Docker services")
                    .cat(Category::Infrastructure),
            );
            if has_volumes {
                out.actions.push(
                    Action::new(format!("{pre}services:reset"), dc("down -v"))
                        .tool("compose")
                        .inferred(Confidence::Low)
                        .inferred_desc("Stop Docker services and delete their volumes")
                        .cat(Category::Infrastructure)
                        .risk(Risk::Destructive),
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_names() {
        assert_eq!(
            service_description("db", Some("postgres:16"), false),
            "Start PostgreSQL"
        );
        assert_eq!(
            service_description("kafka", Some("confluentinc/cp-kafka:7.6.0"), false),
            "Start local Kafka broker"
        );
        assert_eq!(
            service_description(
                "mssql",
                Some("mcr.microsoft.com/mssql/server:2022-latest"),
                false
            ),
            "Start SQL Server"
        );
        assert_eq!(
            service_description("api", None, true),
            "Start the api service (built from source)"
        );
        assert_eq!(
            service_description("thing", Some("ghcr.io/x/thing:1"), false),
            "Start the thing service (thing)"
        );
    }
}
