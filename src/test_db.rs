use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

pub(crate) const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub(crate) fn test_connection() -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(":memory:").expect("establish in-memory connection");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("run migrations");
    conn
}