use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, sea_query::OnConflict};
use serde::{Deserialize, Serialize};

use crate::util::time::now_epoch;

pub const DEFAULT_TIMEOUT_SECONDS: i64 = 600;
pub const DEFAULT_SPAM_CHECK_MESSAGE_LIMIT: i64 = 3;
pub const DEFAULT_SPAM_CHECK_WINDOW_HOURS: i64 = 24;
pub const DEFAULT_SPAM_KICK_THRESHOLD: i64 = 2;

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
    pub ai_config: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiField {
    Provider,
    ApiBase,
    ApiKey,
    Model,
    SpamMessageLimit,
    SpamWindowHours,
    SpamKickThreshold,
}

impl AiField {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "provider" => Some(Self::Provider),
            "url" | "api_base" => Some(Self::ApiBase),
            "key" | "api_key" => Some(Self::ApiKey),
            "model" => Some(Self::Model),
            "limit" | "spam_message_limit" => Some(Self::SpamMessageLimit),
            "window" | "spam_window_hours" => Some(Self::SpamWindowHours),
            "kick" | "spam_kick_threshold" => Some(Self::SpamKickThreshold),
            _ => None,
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::ApiBase => "url",
            Self::ApiKey => "key",
            Self::Model => "model",
            Self::SpamMessageLimit => "limit",
            Self::SpamWindowHours => "window",
            Self::SpamKickThreshold => "kick",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: Option<String>,
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub spam_check_message_limit: Option<i64>,
    pub spam_check_window_hours: Option<i64>,
    pub spam_kick_threshold: Option<i64>,
}

impl AiConfig {
    pub fn parse(raw: Option<&str>) -> Self {
        raw.and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    pub fn ready(&self) -> Option<(&str, &str, &str, &str)> {
        Some((
            self.provider.as_deref()?,
            self.api_base.as_deref()?,
            self.api_key.as_deref()?,
            self.model.as_deref()?,
        ))
    }

    pub fn spam_check_message_limit(&self) -> i64 {
        self.spam_check_message_limit
            .unwrap_or(DEFAULT_SPAM_CHECK_MESSAGE_LIMIT)
    }

    pub fn spam_check_window_hours(&self) -> i64 {
        self.spam_check_window_hours
            .unwrap_or(DEFAULT_SPAM_CHECK_WINDOW_HOURS)
    }

    pub fn spam_kick_threshold(&self) -> i64 {
        self.spam_kick_threshold
            .unwrap_or(DEFAULT_SPAM_KICK_THRESHOLD)
    }
}

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

pub async fn set_ai_config_field(
    db: &DatabaseConnection,
    chat_id: i64,
    field: AiField,
    value: Option<&str>,
) -> Result<bool, DbErr> {
    let Some(g) = get(db, chat_id).await? else {
        return Ok(false);
    };
    let mut cfg = AiConfig::parse(g.ai_config.as_deref());
    let v = value.map(|s| s.to_string());
    match field {
        AiField::Provider => cfg.provider = v,
        AiField::ApiBase => cfg.api_base = v,
        AiField::ApiKey => cfg.api_key = v,
        AiField::Model => cfg.model = v,
        AiField::SpamMessageLimit => cfg.spam_check_message_limit = v.and_then(|s| s.parse().ok()),
        AiField::SpamWindowHours => cfg.spam_check_window_hours = v.and_then(|s| s.parse().ok()),
        AiField::SpamKickThreshold => cfg.spam_kick_threshold = v.and_then(|s| s.parse().ok()),
    }
    let serialized = serde_json::to_string(&cfg).expect("AiConfig serialize never fails");
    let res = Entity::update_many()
        .col_expr(Column::AiConfig, serialized.into())
        .col_expr(Column::UpdatedAt, now_epoch().into())
        .filter(Column::ChatId.eq(chat_id))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}
