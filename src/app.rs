use sea_orm::DatabaseConnection;
use teloxide::prelude::Requester;
use teloxide::types::UserId;
use teloxide::{Bot as Teloxide, RequestError};

use crate::config::Config;
use crate::telegram::TelegramGateway;

pub struct AppState {
    pub db: DatabaseConnection,
    pub public_url: String,
    pub identity: BotIdentity,
    pub telegram: TelegramGateway,
}

pub struct BotIdentity {
    pub user_id: UserId,
    pub username: String,
}

impl AppState {
    pub async fn new(db: DatabaseConnection, cfg: &Config) -> Result<Self, RequestError> {
        let tg = Teloxide::new(&cfg.bot_token);
        let me = tg.get_me().await?.user;
        let identity = BotIdentity {
            user_id: me.id,
            username: me.username.expect("bot account must have a username"),
        };

        Ok(Self {
            db,
            public_url: cfg.public_url.clone(),
            identity,
            telegram: TelegramGateway::new(tg),
        })
    }
}
