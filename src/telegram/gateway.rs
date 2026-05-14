use std::future::Future;
use std::time::Duration;

use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{
    CallbackQueryId, ChatId, ChatPermissions, ForceReply, InlineKeyboardButton,
    InlineKeyboardMarkup, Message, MessageId, ParseMode, ReplyParameters, UserId,
};
use teloxide::{Bot as Teloxide, RequestError};
use url::Url;

use crate::bot::text::{FORCE_REPLY_PLACEHOLDER, fill};
use crate::util::html;

const TG_MAX_ATTEMPTS: u32 = 3;
const TG_INITIAL_BACKOFF: Duration = Duration::from_millis(200);

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
        with_retries("restrict_member", || async {
            self.inner
                .restrict_chat_member(
                    ChatId(chat_id),
                    to_user_id(user_id),
                    ChatPermissions::empty(),
                )
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn unrestrict_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        with_retries("unrestrict_member", || async {
            self.inner
                .restrict_chat_member(ChatId(chat_id), to_user_id(user_id), ChatPermissions::all())
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn kick_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        with_retries("ban_chat_member", || async {
            self.inner
                .ban_chat_member(ChatId(chat_id), to_user_id(user_id))
                .await
                .map(|_| ())
        })
        .await?;
        with_retries("unban_chat_member", || async {
            self.inner
                .unban_chat_member(ChatId(chat_id), to_user_id(user_id))
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn ban_member(&self, chat_id: i64, user_id: i64) -> Result<(), RequestError> {
        with_retries("ban_member", || async {
            self.inner
                .ban_chat_member(ChatId(chat_id), to_user_id(user_id))
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn approve_join_request(
        &self,
        chat_id: i64,
        user_id: i64,
    ) -> Result<(), RequestError> {
        with_retries("approve_join_request", || async {
            self.inner
                .approve_chat_join_request(ChatId(chat_id), to_user_id(user_id))
                .await
                .map(|_| ())
        })
        .await
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
            name = html::escape(user_first_name),
        );
        let text = fill(text_template, &[("user", &mention)]);
        let keyboard = InlineKeyboardMarkup::new([[InlineKeyboardButton::url(
            button_label.to_string(),
            verify_url.clone(),
        )]]);
        with_retries("send_group_verification_message", || async {
            self.inner
                .send_message(ChatId(chat_id), text.clone())
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard.clone())
                .await
        })
        .await
    }

    pub async fn send_dm(
        &self,
        user_id: i64,
        text: &str,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<Message, RequestError> {
        with_retries("send_dm", || async {
            let req = self
                .inner
                .send_message(to_user_id(user_id), text.to_string());
            match keyboard.clone() {
                Some(kb) => req.reply_markup(kb).await,
                None => req.await,
            }
        })
        .await
    }

    pub async fn delete_message(&self, chat_id: i64, message_id: i64) -> Result<(), RequestError> {
        with_retries("delete_message", || async {
            self.inner
                .delete_message(ChatId(chat_id), MessageId(message_id as i32))
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn is_privileged(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> Result<bool, RequestError> {
        with_retries("get_chat_member", || async {
            self.inner
                .get_chat_member(chat_id, user_id)
                .await
                .map(|m| m.is_privileged())
        })
        .await
    }

    pub async fn reply_to(
        &self,
        chat: ChatId,
        reply_to: MessageId,
        text: &str,
    ) -> Result<Message, RequestError> {
        with_retries("reply_to", || async {
            self.inner
                .send_message(chat, text.to_string())
                .reply_parameters(ReplyParameters::new(reply_to))
                .await
        })
        .await
    }

    pub async fn reply_with_keyboard(
        &self,
        chat: ChatId,
        reply_to: MessageId,
        text: &str,
        keyboard: InlineKeyboardMarkup,
    ) -> Result<Message, RequestError> {
        with_retries("reply_with_keyboard", || async {
            self.inner
                .send_message(chat, text.to_string())
                .reply_parameters(ReplyParameters::new(reply_to))
                .reply_markup(keyboard.clone())
                .await
        })
        .await
    }

    pub async fn send_force_reply(
        &self,
        user_id: i64,
        text: &str,
    ) -> Result<Message, RequestError> {
        with_retries("send_force_reply", || async {
            self.inner
                .send_message(to_user_id(user_id), text.to_string())
                .reply_markup(
                    ForceReply::new().input_field_placeholder(FORCE_REPLY_PLACEHOLDER.to_string()),
                )
                .await
        })
        .await
    }

    pub async fn answer_callback(&self, id: CallbackQueryId) -> Result<(), RequestError> {
        with_retries("answer_callback", || async {
            self.inner
                .answer_callback_query(id.clone())
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn answer_callback_with_text(
        &self,
        id: CallbackQueryId,
        text: &str,
    ) -> Result<(), RequestError> {
        with_retries("answer_callback_with_text", || async {
            self.inner
                .answer_callback_query(id.clone())
                .text(text.to_string())
                .await
                .map(|_| ())
        })
        .await
    }
}

fn to_user_id(id: i64) -> UserId {
    UserId(id.max(0) as u64)
}

fn is_transient(err: &RequestError) -> bool {
    matches!(
        err,
        RequestError::RetryAfter(_) | RequestError::Network(_) | RequestError::Io(_)
    )
}

async fn with_retries<F, Fut, T>(op: &'static str, mut f: F) -> Result<T, RequestError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, RequestError>>,
{
    let mut delay = TG_INITIAL_BACKOFF;
    let mut attempt: u32 = 1;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(err) if is_transient(&err) && attempt < TG_MAX_ATTEMPTS => {
                tracing::warn!(
                    op,
                    attempt,
                    error = %err,
                    wait_ms = delay.as_millis() as u64,
                    "TG transient error; retrying",
                );
                tokio::time::sleep(delay).await;
                delay = delay.saturating_mul(2);
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}
