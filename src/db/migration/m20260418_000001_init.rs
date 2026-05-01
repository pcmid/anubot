use sea_orm_migration::prelude::*;

use crate::db::{group, session};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(group::Entity)
                    .col(
                        ColumnDef::new(group::Column::ChatId)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(group::Column::Enabled).boolean().not_null())
                    .col(
                        ColumnDef::new(group::Column::TimeoutSeconds)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(group::Column::WelcomeText).text().null())
                    .col(ColumnDef::new(group::Column::ButtonLabel).text().null())
                    .col(
                        ColumnDef::new(group::Column::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(group::Column::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(session::Entity)
                    .col(
                        ColumnDef::new(session::Column::ChatId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(session::Column::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(session::Column::ChatTitle).text().not_null())
                    .col(
                        ColumnDef::new(session::Column::UserFirstName)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(session::Column::VerifyMsgId)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(session::Column::Status)
                            .string_len(16)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(session::Column::ExpiresAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(session::Column::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(session::Column::VerifiedAt)
                            .big_integer()
                            .null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(session::Column::ChatId)
                            .col(session::Column::UserId),
                    )
                    .to_owned(),
            )
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
