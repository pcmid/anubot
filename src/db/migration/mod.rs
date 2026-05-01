use sea_orm_migration::prelude::*;

mod m20260418_000001_init;
mod m20260429_000002_spam;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260418_000001_init::Migration),
            Box::new(m20260429_000002_spam::Migration),
        ]
    }
}
