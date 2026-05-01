use sea_orm_migration::prelude::*;

use crate::db::session;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(session::Entity)
                    .add_column(ColumnDef::new(session::Column::VerifyToken).text().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(session::Entity)
                    .drop_column(session::Column::VerifyToken)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
