use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveValue::Set,
    PaginatorTrait,
    sea_query::{Expr, OnConflict},
};
use sea_orm::{QuerySelect, SelectColumns};

use crate::util::time::now_epoch;

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum SessionStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "verified")]
    Verified,
    #[sea_orm(string_value = "expired")]
    Expired,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub chat_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,
    pub chat_title: String,
    pub user_first_name: String,
    pub verify_msg_id: Option<i64>,
    pub status: SessionStatus,
    pub expires_at: i64,
    pub created_at: i64,
    pub verified_at: Option<i64>,
    pub message_counts: i64,
    pub spam_counts: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Clone)]
pub struct NewSession<'a> {
    pub chat_id: i64,
    pub user_id: i64,
    pub chat_title: &'a str,
    pub user_first_name: &'a str,
    pub verify_msg_id: Option<i64>,
    pub expires_at: i64,
}

pub async fn create(db: &DatabaseConnection, s: NewSession<'_>) -> Result<(), DbErr> {
    let row = ActiveModel {
        chat_id: Set(s.chat_id),
        user_id: Set(s.user_id),
        chat_title: Set(s.chat_title.to_owned()),
        user_first_name: Set(s.user_first_name.to_owned()),
        verify_msg_id: Set(s.verify_msg_id),
        status: Set(SessionStatus::Pending),
        expires_at: Set(s.expires_at),
        created_at: Set(now_epoch()),
        verified_at: Set(None),
        message_counts: Set(0),
        spam_counts: Set(0),
    };
    Entity::insert(row)
        .on_conflict(
            OnConflict::columns([Column::ChatId, Column::UserId])
                .update_columns([
                    Column::ChatTitle,
                    Column::UserFirstName,
                    Column::VerifyMsgId,
                    Column::Status,
                    Column::ExpiresAt,
                    Column::CreatedAt,
                    Column::VerifiedAt,
                    Column::MessageCounts,
                    Column::SpamCounts,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

pub async fn find_active(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
) -> Result<Option<Model>, DbErr> {
    Entity::find()
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Pending))
        .one(db)
        .await
}

pub async fn find_verified(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
) -> Result<Option<Model>, DbErr> {
    Entity::find()
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Verified))
        .one(db)
        .await
}

pub async fn mark_expired_if_pending(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::Status, Expr::value(SessionStatus::Expired))
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Pending))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}

pub async fn mark_verified(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
    verified_at: i64,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::Status, Expr::value(SessionStatus::Verified))
        .col_expr(Column::VerifiedAt, verified_at.into())
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Pending))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}

pub async fn find_all_active(db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
    Entity::find()
        .filter(Column::Status.eq(SessionStatus::Pending))
        .all(db)
        .await
}

pub async fn increment_message_count_if_verified(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(
            Column::MessageCounts,
            Expr::col(Column::MessageCounts).add(1),
        )
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Verified))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}

pub async fn increment_spam_count_if_verified(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
) -> Result<Option<i64>, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::SpamCounts, Expr::col(Column::SpamCounts).add(1))
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Verified))
        .exec(db)
        .await?;
    if res.rows_affected != 1 {
        return Ok(None);
    }
    let new_value: Option<i64> = Entity::find()
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Verified))
        .select_only()
        .select_column(Column::SpamCounts)
        .into_tuple()
        .one(db)
        .await?;
    Ok(new_value)
}

pub async fn count_by_status_since(
    db: &DatabaseConnection,
    chat_id: i64,
    status: SessionStatus,
    since: i64,
) -> Result<i64, DbErr> {
    let count = Entity::find()
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::Status.eq(status))
        .filter(Column::CreatedAt.gte(since))
        .count(db)
        .await?;
    Ok(count as i64)
}
