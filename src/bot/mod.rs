mod bot;
mod commands;
mod text;
mod verify;

pub use bot::Bot;

#[derive(Debug, thiserror::Error)]
pub(crate) enum HandlerError {
    #[error(transparent)]
    Db(#[from] sea_orm::DbErr),
    #[error(transparent)]
    Telegram(#[from] teloxide::RequestError),
}
