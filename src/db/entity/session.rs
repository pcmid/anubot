use sea_orm::entity::prelude::*;

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
    pub verify_token: Option<String>,
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
