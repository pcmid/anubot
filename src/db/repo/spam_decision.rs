use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};

use crate::db::entity::spam_decision::{ActiveModel, Entity, SpamAction};
use crate::util::time::now_epoch;

pub async fn record(
    db: &DatabaseConnection,
    chat_id: i64,
    user_id: i64,
    msg_id: i64,
    score: i64,
    action: SpamAction,
) -> Result<(), DbErr> {
    let row = ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        chat_id: Set(chat_id),
        user_id: Set(user_id),
        msg_id: Set(msg_id),
        score: Set(score),
        action: Set(action),
        created_at: Set(now_epoch()),
    };
    Entity::insert(row).exec(db).await?;
    Ok(())
}
