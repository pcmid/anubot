use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use sea_orm::{ColumnTrait, sea_query::OnConflict};

use crate::db::entity::group::{ActiveModel, Column, Entity, Model};
use crate::db::group::{AiConfig, DEFAULT_TIMEOUT_SECONDS};
use crate::util::time::now_epoch;

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
        ai_config: Set(None),
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

pub async fn get_ai_config(db: &DatabaseConnection, chat_id: i64) -> Result<AiConfig, DbErr> {
    Ok(get(db, chat_id)
        .await?
        .and_then(|g| g.ai_config)
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default())
}

pub async fn set_ai_config(
    db: &DatabaseConnection,
    chat_id: i64,
    cfg: &AiConfig,
) -> Result<bool, DbErr> {
    if get(db, chat_id).await?.is_none() {
        return Ok(false);
    }
    let value = serde_json::to_value(cfg).expect("AiConfig serialize never fails");
    let res = Entity::update_many()
        .col_expr(Column::AiConfig, value.into())
        .col_expr(Column::UpdatedAt, now_epoch().into())
        .filter(Column::ChatId.eq(chat_id))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}
