mod ai;
mod commands;
pub(crate) mod routes;
mod settings;
mod spam;
pub(crate) mod text;
mod verify;

#[derive(Debug, thiserror::Error)]
pub(crate) enum HandlerError {
    #[error(transparent)]
    Db(#[from] sea_orm::DbErr),
    #[error(transparent)]
    Telegram(#[from] teloxide::RequestError),
}
