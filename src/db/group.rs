use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, sea_query::OnConflict};

use crate::util::time::now_epoch;

pub const DEFAULT_TIMEOUT_SECONDS: i64 = 600;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "groups")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub chat_id: i64,
    pub enabled: bool,
    pub timeout_seconds: i64,
    pub welcome_text: Option<String>,
    pub button_label: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub async fn get(db: &DatabaseConnection, chat_id: i64) -> Result<Option<Model>, DbErr> {
    Entity::find_by_id(chat_id).one(db).await
}

pub async fn upsert_enabled(db: &DatabaseConnection, chat_id: i64) -> Result<(), DbErr> {
    let now = now_epoch();
    let new = ActiveModel {
        chat_id: Set(chat_id),
        enabled: Set(true),
        timeout_seconds: Set(DEFAULT_TIMEOUT_SECONDS),
        welcome_text: Set(None),
        button_label: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    Entity::insert(new)
        .on_conflict(
            OnConflict::column(Column::ChatId)
                .update_columns([Column::Enabled, Column::UpdatedAt])
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

pub async fn set_enabled(
    db: &DatabaseConnection,
    chat_id: i64,
    enabled: bool,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::Enabled, enabled.into())
        .col_expr(Column::UpdatedAt, now_epoch().into())
        .filter(Column::ChatId.eq(chat_id))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}

pub async fn set_timeout(
    db: &DatabaseConnection,
    chat_id: i64,
    seconds: i64,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::TimeoutSeconds, seconds.into())
        .col_expr(Column::UpdatedAt, now_epoch().into())
        .filter(Column::ChatId.eq(chat_id))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}

pub async fn set_welcome(
    db: &DatabaseConnection,
    chat_id: i64,
    text: Option<&str>,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::WelcomeText, text.map(|s| s.to_owned()).into())
        .col_expr(Column::UpdatedAt, now_epoch().into())
        .filter(Column::ChatId.eq(chat_id))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}

pub async fn set_button(
    db: &DatabaseConnection,
    chat_id: i64,
    text: Option<&str>,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::ButtonLabel, text.map(|s| s.to_owned()).into())
        .col_expr(Column::UpdatedAt, now_epoch().into())
        .filter(Column::ChatId.eq(chat_id))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}
