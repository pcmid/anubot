use sea_orm_migration::prelude::*;

mod m20260418_000001_init;
mod m20260429_000002_spam;
mod m20260501_000003_verify_token;
mod m20260516_000004_spam_decisions;
mod schema;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260418_000001_init::Migration),
            Box::new(m20260429_000002_spam::Migration),
            Box::new(m20260501_000003_verify_token::Migration),
            Box::new(m20260516_000004_spam_decisions::Migration),
        ]
    }
}
