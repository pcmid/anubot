use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
pub enum Groups {
    Table,
    ChatId,
    Enabled,
    TimeoutSeconds,
    WelcomeText,
    ButtonLabel,
    CreatedAt,
    UpdatedAt,
    AiConfig,
}

#[derive(DeriveIden)]
pub enum Sessions {
    Table,
    ChatId,
    UserId,
    ChatTitle,
    UserFirstName,
    VerifyMsgId,
    VerifyToken,
    Status,
    ExpiresAt,
    CreatedAt,
    VerifiedAt,
    MessageCounts,
    SpamCounts,
}

#[derive(DeriveIden)]
pub enum SpamDecisions {
    Table,
    Id,
    ChatId,
    UserId,
    MsgId,
    Score,
    Action,
    CreatedAt,
}
