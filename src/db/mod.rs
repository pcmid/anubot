pub(crate) mod entity;
pub(crate) mod group;
pub(crate) mod repo;
pub(crate) mod session;

mod migration;
use crate::db::migration::Migrator;

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, DbErr,
};
use sea_orm_migration::MigratorTrait;
use std::time::Duration;

pub(crate) async fn connect(url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opts = ConnectOptions::new(url.to_owned());
    opts.sqlx_logging(false)
        .connect_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300));
    let db = Database::connect(opts).await?;
    configure_sqlite(&db).await?;
    migrate(&db).await?;
    Ok(db)
}

pub(crate) async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await?;
    Ok(())
}

async fn configure_sqlite(db: &DatabaseConnection) -> Result<(), DbErr> {
    if db.get_database_backend() == DatabaseBackend::Sqlite {
        db.execute_unprepared("PRAGMA foreign_keys = ON").await?;
        db.execute_unprepared("PRAGMA busy_timeout = 5000").await?;
        db.execute_unprepared("PRAGMA journal_mode = WAL").await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Statement};

    #[tokio::test]
    async fn migrates_fresh_sqlite_schema() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        migrate(&db).await.unwrap();

        let backend = db.get_database_backend();
        let rows = db
            .query_all(Statement::from_string(
                backend,
                "SELECT name FROM sqlite_master WHERE type IN ('table', 'index')".to_string(),
            ))
            .await
            .unwrap();
        let names: Vec<String> = rows
            .into_iter()
            .map(|row| row.try_get("", "name").unwrap())
            .collect();

        assert!(names.iter().any(|name| name == "groups"));
        assert!(names.iter().any(|name| name == "sessions"));
        assert!(
            names
                .iter()
                .any(|name| name == "idx_sessions_status_expires")
        );
    }

    #[tokio::test]
    async fn migrations_reverse_cleanly() {
        // Every up should have a working down so a maintainer can roll
        // back without manually dropping tables. The migrator does not
        // enforce this on its own.
        let db = Database::connect("sqlite::memory:").await.unwrap();
        migrate(&db).await.unwrap();

        Migrator::down(&db, None).await.unwrap();

        let backend = db.get_database_backend();
        let rows = db
            .query_all(Statement::from_string(
                backend,
                "SELECT name FROM sqlite_master \
                 WHERE type = 'table' \
                   AND name NOT LIKE 'sqlite_%' \
                   AND name <> 'seaql_migrations'"
                    .to_string(),
            ))
            .await
            .unwrap();
        let remaining: Vec<String> = rows
            .into_iter()
            .map(|row| row.try_get("", "name").unwrap())
            .collect();
        assert!(
            remaining.is_empty(),
            "expected no application tables left after rollback, found: {remaining:?}",
        );

        // Re-up after full down must succeed (idempotent migration set).
        migrate(&db).await.unwrap();
    }
}
