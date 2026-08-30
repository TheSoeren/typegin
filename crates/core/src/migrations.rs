use diesel_migrations::EmbeddedMigrations;

pub(crate) const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!();