use std::sync::Arc;

use sea_orm::{DatabaseConnection, DbErr};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::payloads::{SendMessageSetters, SetMyCommandsSetters, UnbanChatMemberSetters};
use teloxide::prelude::*;
use teloxide::types::{
    BotCommand, BotCommandScope, ChatId, ChatMemberUpdated, ChatPermissions, InlineKeyboardButton,
    InlineKeyboardMarkup, Message, MessageId, ParseMode, ReplyParameters, Update, UserId,
};
use teloxide::{Bot as Teloxide, RequestError, dptree};
use url::Url;

use crate::bot::commands;
use crate::bot::text::fill;
use crate::bot::verify;
use crate::config::Config;

pub struct Bot {
    tg: Teloxide,
    db: DatabaseConnection,
    public_url: String,
    bot_username: String,
}

impl Bot {
    pub async fn new(db: DatabaseConnection, cfg: &Config) -> Result<Self, RequestError> {
        let tg = Teloxide::new(&cfg.bot_token);
        let bot_username = tg
            .get_me()
            .await?
            .user
            .username
            .clone()
            .expect("bot account must have a username");

        let admin_commands = vec![
            BotCommand::new("enable", "启用本群的人机验证"),
            BotCommand::new("disable", "停用本群的人机验证"),
            BotCommand::new("set_timeout", "设置验证超时(秒,60-3600)"),
            BotCommand::new("set_welcome", "自定义欢迎语(清空则恢复默认)"),
            BotCommand::new("set_button", "自定义按钮文字(清空则恢复默认)"),
            BotCommand::new("status", "查看本群验证状态"),
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

    /// Drive the teloxide dispatcher. Resumes any per-session expiry
    /// coroutines first so a restart doesn't leave sessions stranded.
    pub async fn run(self: Arc<Self>) {
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
                    .filter_map(|m: Message| m.text().and_then(verify::parse_start_payload))
                    .endpoint(verify::on_start_dm),
            )
            .branch(
                Update::filter_chat_member()
                    .filter(|upd: ChatMemberUpdated| verify::is_new_member_join(&upd))
                    .endpoint(verify::on_member_join),
            );

        Dispatcher::builder(self.tg.clone(), handler)
            .dependencies(dptree::deps![self.clone()])
            .default_handler(|_upd| async {})
            .build()
            .dispatch()
            .await
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

    /// Send a private message to `user_id`. `keyboard` is optional;
    /// pass `None` for plain text (e.g. the "no pending verification" reply).
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
}

// -------- helpers -----------------------------------------------------

fn to_user_id(id: i64) -> UserId {
    UserId(id.max(0) as u64)
}

/// Escape the five XML/HTML characters that Telegram's HTML parse mode
/// interprets. Only used inside `<a href="...">{name}</a>` where `name` is
/// user-controlled.
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
