use sea_orm_migration::prelude::*;

use super::schema::{Groups, Sessions};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Groups::Table)
                    .col(
                        ColumnDef::new(Groups::ChatId)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Groups::Enabled).boolean().not_null())
                    .col(
                        ColumnDef::new(Groups::TimeoutSeconds)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Groups::WelcomeText).text().null())
                    .col(ColumnDef::new(Groups::ButtonLabel).text().null())
                    .col(ColumnDef::new(Groups::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Groups::UpdatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .col(ColumnDef::new(Sessions::ChatId).big_integer().not_null())
                    .col(ColumnDef::new(Sessions::UserId).big_integer().not_null())
                    .col(ColumnDef::new(Sessions::ChatTitle).text().not_null())
                    .col(ColumnDef::new(Sessions::UserFirstName).text().not_null())
                    .col(ColumnDef::new(Sessions::VerifyMsgId).big_integer().null())
                    .col(ColumnDef::new(Sessions::Status).string_len(16).not_null())
                    .col(ColumnDef::new(Sessions::ExpiresAt).big_integer().not_null())
                    .col(ColumnDef::new(Sessions::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Sessions::VerifiedAt).big_integer().null())
                    .primary_key(Index::create().col(Sessions::ChatId).col(Sessions::UserId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_status_expires")
                    .table(Sessions::Table)
                    .col(Sessions::Status)
                    .col(Sessions::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_user_chat")
                    .table(Sessions::Table)
                    .col(Sessions::UserId)
                    .col(Sessions::ChatId)
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
                    .table(Sessions::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_sessions_status_expires")
                    .table(Sessions::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Groups::Table).to_owned())
            .await?;
        Ok(())
    }
}
