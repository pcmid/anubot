use sea_orm::{DatabaseConnection, DbErr};
use std::sync::Arc;
use teloxide::dispatching::{Dispatcher, ShutdownToken, UpdateFilterExt};
use teloxide::payloads::{SendMessageSetters, SetMyCommandsSetters, UnbanChatMemberSetters};
use teloxide::prelude::*;
use teloxide::types::{
    BotCommand, BotCommandScope, CallbackQueryId, ChatId, ChatMemberUpdated, ChatPermissions,
    ForceReply, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageId, ParseMode,
    ReplyParameters, Update, UserId,
};
use teloxide::{Bot as Teloxide, RequestError, dptree};
use url::Url;

use crate::bot::text::*;
use crate::bot::{commands, settings, spam, verify};
use crate::config::Config;

pub struct Bot {
    tg: Teloxide,
    db: DatabaseConnection,
    public_url: String,
    bot_user_id: UserId,
    bot_username: String,
}

impl Bot {
    pub async fn new(db: DatabaseConnection, cfg: &Config) -> Result<Self, RequestError> {
        let tg = Teloxide::new(&cfg.bot_token);
        let me = tg.get_me().await?.user;
        let bot_user_id = me.id;
        let bot_username = me
            .username
            .clone()
            .expect("bot account must have a username");

        let admin_commands = vec![
            BotCommand::new("enable", CMD_ENABLE),
            BotCommand::new("disable", CMD_DISABLE),
            BotCommand::new("set_timeout", CMD_SET_TIMEOUT),
            BotCommand::new("set_welcome", CMD_SET_WELCOME),
            BotCommand::new("set_button", CMD_SET_BUTTON),
            BotCommand::new("status", CMD_STATUS),
            BotCommand::new("settings", CMD_SETTINGS),
        ];
        if let Err(err) = tg
            .set_my_commands(admin_commands)
            .scope(BotCommandScope::AllChatAdministrators)
            .await
        {
            tracing::warn!(
                error = %err,
                "set_my_commands failed; /-autocomplete may not appear",
            );
        }

        Ok(Self {
            tg,
            db,
            public_url: cfg.public_url.clone(),
            bot_user_id,
            bot_username,
        })
    }

    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    pub fn public_url(&self) -> &str {
        &self.public_url
    }

    pub fn bot_username(&self) -> &str {
        &self.bot_username
    }

    pub fn bot_user_id(&self) -> UserId {
        self.bot_user_id
    }

    pub async fn run(self: Arc<Self>) -> BotDispatcher {
        if let Err(err) = self.resume_pending_sessions().await {
            tracing::error!(
                error = %err,
                "resume_pending_sessions failed; continuing without resume",
            );
        }

        let handler = dptree::entry()
            .branch(
                Update::filter_message()
                    .filter_map_async(commands::filter_admin_command)
                    .endpoint(commands::on_command),
            )
            .branch(
                Update::filter_message()
                    .filter(|m: Message| m.chat.is_private())
                    .filter_map(|m: Message| settings::extract_settings_tag(&m))
                    .endpoint(settings::on_settings_reply),
            )
            .branch(
                Update::filter_message()
                    .filter(|m: Message| m.chat.is_private())
                    .filter_map(|m: Message| m.text().and_then(verify::parse_start_payload))
                    .endpoint(verify::on_start_dm),
            )
            .branch(Update::filter_callback_query().endpoint(settings::on_settings_callback))
            .branch(
                Update::filter_chat_member()
                    .filter(|upd: ChatMemberUpdated| verify::is_new_member_join(&upd))
                    .endpoint(verify::on_member_join),
            )
            .branch(
                Update::filter_message()
                    .filter(|m: Message| !m.chat.is_private())
                    .endpoint(spam::on_user_message),
            );

        let mut dispatcher = Dispatcher::builder(self.tg.clone(), handler)
            .dependencies(dptree::deps![self.clone()])
            .default_handler(|_upd| async {})
            .build();

        let shutdown_token = dispatcher.shutdown_token();
        let dispatcher_task = tokio::spawn(async move {
            dispatcher.dispatch().await;
        });

        BotDispatcher {
            shutdown_token,
            dispatcher_task,
        }
    }

    pub async fn resume_pending_sessions(self: &Arc<Self>) -> Result<(), DbErr> {
        verify::resume_pending(self).await
    }

    pub async fn restrict_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        self.tg
            .restrict_chat_member(
                ChatId(chat_id),
                to_user_id(user_id),
                ChatPermissions::empty(),
            )
            .await
            .map(|_| ())
    }

    pub async fn unrestrict_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        self.tg
            .restrict_chat_member(ChatId(chat_id), to_user_id(user_id), ChatPermissions::all())
            .await
            .map(|_| ())
    }

    pub async fn kick_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        self.tg
            .ban_chat_member(ChatId(chat_id), to_user_id(user_id))
            .await?;
        self.tg
            .unban_chat_member(ChatId(chat_id), to_user_id(user_id))
            .only_if_banned(true)
            .await
            .map(|_| ())
    }

    pub async fn send_group_verification_message(
        &self,
        chat_id: i64,
        user_id: i64,
        user_first_name: &str,
        text_template: &str,
        button_label: &str,
        verify_url: &Url,
    ) -> Result<Message, RequestError> {
        let mention = format!(
            r#"<a href="tg://user?id={id}">{name}</a>"#,
            id = user_id,
            name = html_escape(user_first_name),
        );
        let text = fill(text_template, &[("user", &mention)]);
        let keyboard = InlineKeyboardMarkup::new([[InlineKeyboardButton::url(
            button_label.to_string(),
            verify_url.clone(),
        )]]);
        self.tg
            .send_message(ChatId(chat_id), text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await
    }

    pub async fn send_dm(
        &self,
        user_id: i64,
        text: &str,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<Message, RequestError> {
        let req = self.tg.send_message(to_user_id(user_id), text);
        match keyboard {
            Some(kb) => req.reply_markup(kb).await,
            None => req.await,
        }
    }

    pub async fn delete_message(&self, chat_id: i64, message_id: i64) -> Result<(), RequestError> {
        self.tg
            .delete_message(ChatId(chat_id), MessageId(message_id as i32))
            .await
            .map(|_| ())
    }

    pub async fn is_privileged(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> Result<bool, RequestError> {
        self.tg
            .get_chat_member(chat_id, user_id)
            .await
            .map(|m| m.is_privileged())
    }

    pub async fn reply_to(
        &self,
        chat: ChatId,
        reply_to: MessageId,
        text: &str,
    ) -> Result<Message, RequestError> {
        self.tg
            .send_message(chat, text)
            .reply_parameters(ReplyParameters::new(reply_to))
            .await
    }

    pub async fn reply_with_keyboard(
        &self,
        chat: ChatId,
        reply_to: MessageId,
        text: &str,
        keyboard: InlineKeyboardMarkup,
    ) -> Result<Message, RequestError> {
        self.tg
            .send_message(chat, text)
            .reply_parameters(ReplyParameters::new(reply_to))
            .reply_markup(keyboard)
            .await
    }

    pub async fn send_force_reply(
        &self,
        user_id: i64,
        text: &str,
    ) -> Result<Message, RequestError> {
        self.tg
            .send_message(to_user_id(user_id), text)
            .reply_markup(
                ForceReply::new().input_field_placeholder(FORCE_REPLY_PLACEHOLDER.to_string()),
            )
            .await
    }

    pub async fn answer_callback(&self, id: CallbackQueryId) -> Result<(), RequestError> {
        self.tg.answer_callback_query(id).await.map(|_| ())
    }

    pub async fn answer_callback_with_text(
        &self,
        id: CallbackQueryId,
        text: &str,
    ) -> Result<(), RequestError> {
        self.tg
            .answer_callback_query(id)
            .text(text)
            .await
            .map(|_| ())
    }
}

pub struct BotDispatcher {
    shutdown_token: ShutdownToken,
    dispatcher_task: tokio::task::JoinHandle<()>,
}

impl BotDispatcher {
    pub async fn shutdown(self) {
        match self.shutdown_token.shutdown() {
            Ok(done) => done.await,
            Err(_) => {
                tracing::debug!("dispatcher was not running during shutdown");
                self.dispatcher_task.abort();
            }
        }

        if let Err(err) = self.dispatcher_task.await {
            tracing::warn!(error = %err, "dispatcher task failed during shutdown");
        }
    }
}

fn to_user_id(id: i64) -> UserId {
    UserId(id.max(0) as u64)
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
