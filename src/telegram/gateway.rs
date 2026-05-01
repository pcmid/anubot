use teloxide::payloads::{SendMessageSetters, UnbanChatMemberSetters};
use teloxide::prelude::*;
use teloxide::types::{
    CallbackQueryId, ChatId, ChatPermissions, ForceReply, InlineKeyboardButton,
    InlineKeyboardMarkup, Message, MessageId, ParseMode, ReplyParameters, UserId,
};
use teloxide::{Bot as Teloxide, RequestError};
use url::Url;

use crate::bot::text::{FORCE_REPLY_PLACEHOLDER, fill};

#[derive(Clone)]
pub struct TelegramGateway {
    inner: Teloxide,
}

impl TelegramGateway {
    pub fn new(inner: Teloxide) -> Self {
        Self { inner }
    }

    pub fn client(&self) -> Teloxide {
        self.inner.clone()
    }

    pub async fn restrict_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        self.inner
            .restrict_chat_member(
                ChatId(chat_id),
                to_user_id(user_id),
                ChatPermissions::empty(),
            )
            .await
            .map(|_| ())
    }

    pub async fn unrestrict_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        self.inner
            .restrict_chat_member(ChatId(chat_id), to_user_id(user_id), ChatPermissions::all())
            .await
            .map(|_| ())
    }

    pub async fn kick_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        self.inner
            .kick_chat_member(ChatId(chat_id), to_user_id(user_id))
            .await?;
        self.inner
            .unban_chat_member(ChatId(chat_id), to_user_id(user_id))
            .only_if_banned(true)
            .await
            .map(|_| ())
    }

    pub async fn ban_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        self.inner
            .ban_chat_member(ChatId(chat_id), to_user_id(user_id))
            .revoke_messages(true)
            .await?;

        self.inner
            .kick_chat_member(ChatId(chat_id), to_user_id(user_id))
            .revoke_messages(true)
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
        self.inner
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
        let req = self.inner.send_message(to_user_id(user_id), text);
        match keyboard {
            Some(kb) => req.reply_markup(kb).await,
            None => req.await,
        }
    }

    pub async fn delete_message(&self, chat_id: i64, message_id: i64) -> Result<(), RequestError> {
        self.inner
            .delete_message(ChatId(chat_id), MessageId(message_id as i32))
            .await
            .map(|_| ())
    }

    pub async fn is_privileged(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> Result<bool, RequestError> {
        self.inner
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
        self.inner
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
        self.inner
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
        self.inner
            .send_message(to_user_id(user_id), text)
            .reply_markup(
                ForceReply::new().input_field_placeholder(FORCE_REPLY_PLACEHOLDER.to_string()),
            )
            .await
    }

    pub async fn answer_callback(&self, id: CallbackQueryId) -> Result<(), RequestError> {
        self.inner.answer_callback_query(id).await.map(|_| ())
    }

    pub async fn answer_callback_with_text(
        &self,
        id: CallbackQueryId,
        text: &str,
    ) -> Result<(), RequestError> {
        self.inner
            .answer_callback_query(id)
            .text(text)
            .await
            .map(|_| ())
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
