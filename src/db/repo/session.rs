use sea_orm::{
    ActiveValue::Set,
    sea_query::{Expr, OnConflict},
};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, QueryFilter};
use sea_orm::{QueryOrder, QuerySelect, SelectColumns};

use crate::db::entity::session::{ActiveModel, Column, Entity, Model, SessionStatus};
use crate::util::time::now_epoch;

#[derive(Debug, Clone)]
pub struct NewSession<'a> {
    pub chat_id: i64,
    pub user_id: i64,
    pub chat_title: &'a str,
    pub user_first_name: &'a str,
    pub verify_msg_id: Option<i64>,
    pub verify_token: &'a str,
    pub expires_at: i64,
}

pub async fn create(db: &DatabaseConnection, s: NewSession<'_>) -> Result<(), DbErr> {
    let row = ActiveModel {
        chat_id: Set(s.chat_id),
        user_id: Set(s.user_id),
        chat_title: Set(s.chat_title.to_owned()),
        user_first_name: Set(s.user_first_name.to_owned()),
        verify_msg_id: Set(s.verify_msg_id),
        verify_token: Set(Some(s.verify_token.to_owned())),
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
                    Column::VerifyToken,
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

pub async fn set_verify_token(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
    token: &str,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::VerifyToken, token.to_owned().into())
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Pending))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}

pub async fn find_active(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
) -> Result<Option<Model>, DbErr> {
    let now = now_epoch();
    Entity::find()
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Pending))
        .filter(Column::ExpiresAt.gt(now))
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

pub async fn mark_expired_if_pending_due(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
    now: i64,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::Status, Expr::value(SessionStatus::Expired))
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Pending))
        .filter(Column::ExpiresAt.lte(now))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}

pub async fn mark_verified_if_pending_unexpired(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
    token: &str,
    verified_at: i64,
) -> Result<bool, DbErr> {
    let res = Entity::update_many()
        .col_expr(Column::Status, Expr::value(SessionStatus::Verified))
        .col_expr(Column::VerifiedAt, verified_at.into())
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::VerifyToken.eq(token))
        .filter(Column::Status.eq(SessionStatus::Pending))
        .filter(Column::ExpiresAt.gt(verified_at))
        .exec(db)
        .await?;
    Ok(res.rows_affected == 1)
}

pub async fn find_due_pending(
    db: &DatabaseConnection,
    now: i64,
    limit: u64,
) -> Result<Vec<Model>, DbErr> {
    Entity::find()
        .filter(Column::Status.eq(SessionStatus::Pending))
        .filter(Column::ExpiresAt.lte(now))
        .order_by_asc(Column::ExpiresAt)
        .limit(limit)
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
    Entity::find()
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Verified))
        .select_only()
        .select_column(Column::SpamCounts)
        .into_tuple()
        .one(db)
        .await
}

pub async fn decrement_spam_count_if_verified(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
) -> Result<Option<i64>, DbErr> {
    use sea_orm::TransactionTrait;
    let txn = db.begin().await?;
    let row = Entity::find()
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Verified))
        .one(&txn)
        .await?;
    let Some(row) = row else {
        txn.rollback().await?;
        return Ok(None);
    };
    let new_count = (row.spam_counts - 1).max(0);
    Entity::update_many()
        .col_expr(Column::SpamCounts, new_count.into())
        .filter(Column::ChatId.eq(chat_id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Status.eq(SessionStatus::Verified))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    Ok(Some(new_count))
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
