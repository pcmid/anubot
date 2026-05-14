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

    #[tokio::test]
    async fn full_session_lifecycle_end_to_end() {
        // Exercise every public repo function in the order the
        // production flow uses them. Catches contract drift between
        // entity, repo, and migration without needing a live TG.
        use crate::db::entity::session::SessionStatus;
        use crate::db::{group, session};

        let db = Database::connect("sqlite::memory:").await.unwrap();
        migrate(&db).await.unwrap();

        let chat_id = -1001234567890i64;
        let user_id = 42i64;

        // (1) Group is empty.
        assert!(group::get(&db, chat_id).await.unwrap().is_none());
        assert!(
            session::find_active(&db, chat_id, user_id)
                .await
                .unwrap()
                .is_none()
        );

        // (2) /enable creates the group with defaults.
        group::upsert_enabled(&db, chat_id).await.unwrap();
        let g = group::get(&db, chat_id).await.unwrap().expect("group");
        assert!(g.enabled);

        // (3) New joiner: open a Pending session.
        let now = crate::util::time::now_epoch();
        session::create(
            &db,
            session::NewSession {
                chat_id,
                user_id,
                chat_title: "Gentoo zh",
                user_first_name: "Alice",
                verify_msg_id: Some(7777),
                verify_token: "test_token_0123456789abcdef",
                expires_at: now + 600,
            },
        )
        .await
        .unwrap();

        let active = session::find_active(&db, chat_id, user_id)
            .await
            .unwrap()
            .expect("pending session");
        assert_eq!(active.status, SessionStatus::Pending);
        assert_eq!(active.chat_title, "Gentoo zh");

        // (4) Web verify flow: mark verified.
        assert!(
            session::mark_verified_if_pending_unexpired(
                &db,
                chat_id,
                user_id,
                "test_token_0123456789abcdef",
                now,
            )
            .await
            .unwrap()
        );
        // Idempotent: second call is a no-op (rows_affected = 0).
        assert!(
            !session::mark_verified_if_pending_unexpired(
                &db,
                chat_id,
                user_id,
                "test_token_0123456789abcdef",
                now,
            )
            .await
            .unwrap()
        );
        let verified = session::find_verified(&db, chat_id, user_id)
            .await
            .unwrap()
            .expect("verified session");
        assert_eq!(verified.status, SessionStatus::Verified);

        // (5) Wrong token does not flip another row.
        assert!(
            !session::mark_verified_if_pending_unexpired(
                &db,
                chat_id,
                user_id,
                "wrong_token",
                now,
            )
            .await
            .unwrap()
        );

        // (6) Spam counters move atomically.
        let inc = session::increment_message_count_if_verified(&db, chat_id, user_id)
            .await
            .unwrap();
        assert!(inc);
        let sc1 = session::increment_spam_count_if_verified(&db, chat_id, user_id)
            .await
            .unwrap()
            .expect("first increment returns count");
        assert_eq!(sc1, 1);
        let sc2 = session::increment_spam_count_if_verified(&db, chat_id, user_id)
            .await
            .unwrap()
            .expect("second increment returns count");
        assert_eq!(sc2, 2);

        // (7) Counts since now-1h includes our Verified.
        let count =
            session::count_by_status_since(&db, chat_id, SessionStatus::Verified, now - 3600)
                .await
                .unwrap();
        assert_eq!(count, 1);

        // (8) ai_config round-trip.
        let mut cfg = group::get_ai_config(&db, chat_id).await.unwrap();
        cfg.provider = Some("openai".to_string());
        cfg.api_base = Some("https://api.openai.com/v1".to_string());
        cfg.api_key = Some("sk-test".to_string());
        cfg.model = Some("gpt-4o-mini".to_string());
        assert!(group::set_ai_config(&db, chat_id, &cfg).await.unwrap());
        let cfg2 = group::get_ai_config(&db, chat_id).await.unwrap();
        assert_eq!(cfg2.provider.as_deref(), Some("openai"));
        assert_eq!(cfg2.model.as_deref(), Some("gpt-4o-mini"));
        assert!(cfg2.ready().is_some());

        // (9) A second user, this time the join is expired.
        let user2 = 99i64;
        let earlier = now - 1200;
        session::create(
            &db,
            session::NewSession {
                chat_id,
                user_id: user2,
                chat_title: "Gentoo zh",
                user_first_name: "Bob",
                verify_msg_id: None,
                verify_token: "expired_token",
                expires_at: earlier,
            },
        )
        .await
        .unwrap();
        // Expiry worker would mark it.
        let marked = session::mark_expired_if_pending_due(&db, chat_id, user2, now)
            .await
            .unwrap();
        assert!(marked);
        let due = session::find_due_pending(&db, now, 10).await.unwrap();
        assert!(due.is_empty(), "no more due-pending after marking expired");
    }
}
