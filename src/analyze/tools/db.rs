//! DB / ORM: one slice of the tool knowledge table behind `analyze::tools::summarize`.

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
        "prisma" => match (sub, a.positionals().get(1).copied()) {
            (Some("generate"), _) => s("prisma", "Generate the Prisma client", Generate, Safe),
            (Some("migrate"), Some("dev")) => s(
                "prisma",
                "Create and apply Prisma migrations",
                Migrate,
                Safe,
            ),
            (Some("migrate"), Some("deploy")) => {
                s("prisma", "Apply pending Prisma migrations", Migrate, Safe)
            }
            (Some("migrate"), Some("reset")) => s(
                "prisma",
                "Reset the database and re-apply migrations",
                Migrate,
                Destructive,
            ),
            (Some("migrate"), Some("status")) => {
                s("prisma", "Show Prisma migration status", Db, Safe)
            }
            (Some("migrate"), Some("diff")) => s(
                "prisma",
                "Diff the Prisma schema against the database",
                Db,
                Safe,
            ),
            (Some("migrate"), Some("resolve")) => s(
                "prisma",
                "Mark a Prisma migration as applied",
                Migrate,
                Safe,
            ),
            (Some("db"), Some("push")) => s(
                "prisma",
                if a.has("--force-reset") {
                    "Reset the database from the Prisma schema"
                } else {
                    "Push the Prisma schema to the database"
                },
                Db,
                if a.has("--force-reset") {
                    Destructive
                } else {
                    Safe
                },
            ),
            (Some("db"), Some("pull")) => {
                s("prisma", "Pull the database schema into Prisma", Db, Safe)
            }
            (Some("db"), Some("seed")) => s("prisma", "Seed the database", Db, Safe),
            (Some("db"), Some("execute")) => {
                s("prisma", "Execute SQL against the database", Db, Safe)
            }
            (Some("studio"), _) => s("prisma", "Open Prisma Studio", Db, Safe),
            (Some("format"), _) => s("prisma", "Format the Prisma schema", Format, Safe),
            (Some("validate"), _) => s("prisma", "Validate the Prisma schema", Lint, Safe),
            (Some(x), _) => s("prisma", format!("Run prisma {x}"), Db, Safe),
            (None, _) => s("prisma", "Run Prisma", Db, Safe),
        },
        "drizzle-kit" => match sub {
            Some("generate") => s("drizzle-kit", "Generate Drizzle migrations", Migrate, Safe),
            Some("migrate") => s("drizzle-kit", "Apply Drizzle migrations", Migrate, Safe),
            Some("push") => s(
                "drizzle-kit",
                "Push the Drizzle schema to the database",
                Db,
                Safe,
            ),
            Some("pull") | Some("introspect") => s(
                "drizzle-kit",
                "Pull the database schema into Drizzle",
                Db,
                Safe,
            ),
            Some("studio") => s("drizzle-kit", "Open Drizzle Studio", Db, Safe),
            Some("drop") => s(
                "drizzle-kit",
                "Delete a Drizzle migration",
                Migrate,
                Destructive,
            ),
            Some("check") => s(
                "drizzle-kit",
                "Check Drizzle migrations for conflicts",
                Lint,
                Safe,
            ),
            Some(x) => s("drizzle-kit", format!("Run drizzle-kit {x}"), Db, Safe),
            None => s("drizzle-kit", "Run Drizzle Kit", Db, Safe),
        },
        "knex" => match sub {
            Some("migrate:latest") => s("knex", "Apply pending Knex migrations", Migrate, Safe),
            Some("migrate:rollback") => {
                s("knex", "Roll back Knex migrations", Migrate, Destructive)
            }
            Some("migrate:make") => s("knex", "Create a Knex migration", Migrate, Safe),
            Some("seed:run") => s("knex", "Seed the database with Knex", Db, Safe),
            Some(x) => s("knex", format!("Run knex {x}"), Db, Safe),
            None => s("knex", "Run Knex", Db, Safe),
        },
        "sequelize" | "sequelize-cli" => match sub {
            Some("db:migrate") => s(
                "sequelize",
                "Apply pending Sequelize migrations",
                Migrate,
                Safe,
            ),
            Some("db:migrate:undo") | Some("db:migrate:undo:all") => s(
                "sequelize",
                "Roll back Sequelize migrations",
                Migrate,
                Destructive,
            ),
            Some("db:seed:all") | Some("db:seed") => {
                s("sequelize", "Seed the database with Sequelize", Db, Safe)
            }
            Some("db:drop") => s("sequelize", "Drop the database", Db, Destructive),
            Some("db:create") => s("sequelize", "Create the database", Db, Safe),
            Some(x) => s("sequelize", format!("Run sequelize {x}"), Db, Safe),
            None => s("sequelize", "Run Sequelize", Db, Safe),
        },
        "typeorm" | "typeorm-ts-node-commonjs" | "typeorm-ts-node-esm" => match sub {
            Some("migration:run") => {
                s("typeorm", "Apply pending TypeORM migrations", Migrate, Safe)
            }
            Some("migration:revert") => s(
                "typeorm",
                "Revert the last TypeORM migration",
                Migrate,
                Destructive,
            ),
            Some("migration:generate") => {
                s("typeorm", "Generate a TypeORM migration", Migrate, Safe)
            }
            Some("schema:drop") => s("typeorm", "Drop the database schema", Db, Destructive),
            Some("schema:sync") => s("typeorm", "Sync the database schema", Db, Safe),
            Some(x) => s("typeorm", format!("Run typeorm {x}"), Db, Safe),
            None => s("typeorm", "Run TypeORM", Db, Safe),
        },
        "mikro-orm" => match sub {
            Some("migration:up") => s(
                "mikro-orm",
                "Apply pending MikroORM migrations",
                Migrate,
                Safe,
            ),
            Some("migration:down") => s(
                "mikro-orm",
                "Revert MikroORM migrations",
                Migrate,
                Destructive,
            ),
            Some("migration:create") => {
                s("mikro-orm", "Create a MikroORM migration", Migrate, Safe)
            }
            Some("schema:fresh") | Some("schema:drop") => s(
                "mikro-orm",
                "Drop and recreate the database schema",
                Db,
                Destructive,
            ),
            Some("seeder:run") => s("mikro-orm", "Seed the database", Db, Safe),
            Some(x) => s("mikro-orm", format!("Run mikro-orm {x}"), Db, Safe),
            None => s("mikro-orm", "Run MikroORM", Db, Safe),
        },
        "dbmate" | "goose" | "migrate" | "sql-migrate" | "atlas" | "dbmigrate" | "flyway"
        | "liquibase" | "sqlx" | "diesel" | "sea-orm-cli" | "alembic" | "dotnet-ef" => {
            let name = program;
            let pos = a.positionals();
            let joined = pos.join(" ");
            let up = [
                "up",
                "migrate",
                "upgrade",
                "update",
                "run",
                "migration run",
                "database update",
                "apply",
            ];
            let down = [
                "down",
                "rollback",
                "downgrade",
                "reset",
                "drop",
                "database drop",
                "revert",
                "redo",
            ];
            if down
                .iter()
                .any(|d| joined.starts_with(d) || pos.contains(d))
            {
                s(
                    name,
                    format!("Roll back database migrations with {name}"),
                    Migrate,
                    Destructive,
                )
            } else if up
                .iter()
                .any(|u| joined.starts_with(u) || pos.first() == Some(u))
            {
                s(
                    name,
                    format!("Apply pending migrations with {name}"),
                    Migrate,
                    Safe,
                )
            } else if pos.iter().any(|p| {
                p.contains("create")
                    || p.contains("new")
                    || p.contains("generate")
                    || p.contains("revision")
            }) {
                s(
                    name,
                    format!("Create a migration with {name}"),
                    Migrate,
                    Safe,
                )
            } else if pos
                .iter()
                .any(|p| p.contains("status") || p.contains("current") || p.contains("history"))
            {
                s(name, format!("Show migration status with {name}"), Db, Safe)
            } else {
                s(
                    name,
                    format!("Run {name} {}", pos.first().copied().unwrap_or("")),
                    Db,
                    Safe,
                )
            }
        }
        "psql" | "mysql" | "sqlite3" | "sqlcmd" | "mongosh" | "redis-cli" | "clickhouse-client" => {
            let joined = args.join(" ").to_lowercase();
            if joined.contains("drop database")
                || joined.contains("drop table")
                || joined.contains("flushall")
                || joined.contains("flushdb")
                || joined.contains("dropdatabase")
            {
                s(
                    program,
                    format!("Drop data with {program}"),
                    Db,
                    Destructive,
                )
            } else {
                s(program, format!("Open a {program} session"), Db, Safe)
            }
        }
        "createdb" => s("createdb", "Create the PostgreSQL database", Db, Safe),
        "dropdb" => s("dropdb", "Drop the PostgreSQL database", Db, Destructive),
        "pg_dump" | "mysqldump" | "mongodump" => s(program, "Dump the database", Db, Safe),
        "pg_restore" | "mongorestore" => {
            s(program, "Restore the database from a dump", Db, Destructive)
        }

        _ => None,
    }
}
