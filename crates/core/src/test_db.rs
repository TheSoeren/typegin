use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::MigrationHarness;

use crate::migrations::MIGRATIONS;

pub(crate) fn test_connection() -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(":memory:").expect("establish in-memory connection");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("run migrations");
    conn
}