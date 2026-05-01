use sea_orm_migration::prelude::*;

use crate::db::{group, session};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(session::Entity)
                    .add_column(
                        ColumnDef::new(session::Column::MessageCounts)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(session::Entity)
                    .add_column(
                        ColumnDef::new(session::Column::SpamCounts)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(group::Entity)
                    .add_column(ColumnDef::new(group::Column::AiConfig).text().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(group::Entity)
                    .drop_column(group::Column::AiConfig)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(session::Entity)
                    .drop_column(session::Column::SpamCounts)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(session::Entity)
                    .drop_column(session::Column::MessageCounts)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
