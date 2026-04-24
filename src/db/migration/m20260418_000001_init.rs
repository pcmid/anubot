use sea_orm::Schema;
use sea_orm_migration::prelude::*;

use crate::db::{group, session};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let schema = Schema::new(backend);

        manager
            .create_table(schema.create_table_from_entity(group::Entity))
            .await?;

        manager
            .create_table(schema.create_table_from_entity(session::Entity))
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_status_expires")
                    .table(session::Entity)
                    .col(session::Column::Status)
                    .col(session::Column::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_user_chat")
                    .table(session::Entity)
                    .col(session::Column::UserId)
                    .col(session::Column::ChatId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_sessions_user_chat")
                    .table(session::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_sessions_status_expires")
                    .table(session::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(session::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(group::Entity).to_owned())
            .await?;
        Ok(())
    }
}
