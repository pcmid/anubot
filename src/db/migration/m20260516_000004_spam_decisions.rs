use sea_orm_migration::prelude::*;

use super::schema::SpamDecisions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SpamDecisions::Table)
                    .col(
                        ColumnDef::new(SpamDecisions::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SpamDecisions::ChatId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SpamDecisions::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SpamDecisions::MsgId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SpamDecisions::Score)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SpamDecisions::Action)
                            .string_len(16)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SpamDecisions::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_spam_decisions_chat_user_created")
                    .table(SpamDecisions::Table)
                    .col(SpamDecisions::ChatId)
                    .col(SpamDecisions::UserId)
                    .col(SpamDecisions::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_spam_decisions_chat_user_created")
                    .table(SpamDecisions::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(SpamDecisions::Table).to_owned())
            .await?;
        Ok(())
    }
}
