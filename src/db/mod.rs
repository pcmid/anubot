pub(crate) mod group;
pub(crate) mod session;

mod migration;
use crate::db::migration::Migrator;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

pub(crate) async fn connect(url: &str) -> Result<DatabaseConnection, DbErr> {
    let opts = ConnectOptions::new(url.to_owned());
    let db = Database::connect(opts).await?;
    Migrator::up(&db, None).await?;
    Ok(db)
}
