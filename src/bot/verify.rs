use std::sync::Arc;
use std::time::Duration;

use sea_orm::DbErr;
use teloxide::types::{
    ChatMemberStatus, ChatMemberUpdated, InlineKeyboardButton, InlineKeyboardMarkup, Message,
};
use url::Url;

use crate::bot::text::{
    DEFAULT_BUTTON_LABEL, DEFAULT_DM_BUTTON_LABEL, DEFAULT_DM_VERIFICATION_TEXT,
    DEFAULT_GROUP_VERIFICATION_TEXT, DM_NO_PENDING, fill,
};
use crate::bot::{Bot, HandlerError};
use crate::db::{group, session};
use crate::util::time::now_epoch;

pub fn is_new_member_join(upd: &ChatMemberUpdated) -> bool {
    matches!(
        (upd.old_chat_member.status(), upd.new_chat_member.status()),
        (ChatMemberStatus::Left, ChatMemberStatus::Member)
    )
}

pub async fn on_member_join(upd: ChatMemberUpdated, bot: Arc<Bot>) -> Result<(), HandlerError> {
    let chat_id = upd.chat.id.0;
    let user_id = upd.new_chat_member.user.id.0 as i64;
    let chat_title = upd.chat.title().unwrap_or("").to_string();
    let user_first_name = upd.new_chat_member.user.first_name.clone();
    verification(&bot, chat_id, user_id, &chat_title, &user_first_name).await?;
    Ok(())
}

async fn verification(
    bot: &Arc<Bot>,
    chat_id: i64,
    user_id: i64,
    chat_title: &str,
    user_first_name: &str,
) -> Result<(), DbErr> {
    let Some(group) = group::get(bot.db(), chat_id).await? else {
        return Ok(());
    };
    if !group.enabled {
        return Ok(());
    }

    // 幂等:已有活跃 session 就跳过(比如重复事件)
    if session::find_active(bot.db(), chat_id, user_id)
        .await?
        .is_some()
    {
        return Ok(());
    }

    // 禁言失败(admin / 权限不足)直接放弃,不发消息
    if let Err(err) = bot.restrict_member(chat_id, user_id).await {
        tracing::warn!(
            chat_id, user_id, error = %err,
            "restrict_member failed; skipping verification",
        );
        return Ok(());
    }

    let expires_at = now_epoch() + group.timeout_seconds;
    let template = group
        .welcome_text
        .as_deref()
        .unwrap_or(DEFAULT_GROUP_VERIFICATION_TEXT);
    let button_label = group
        .button_label
        .clone()
        .unwrap_or_else(|| DEFAULT_BUTTON_LABEL.to_string());
    let timeout_minutes = (group.timeout_seconds / 60).to_string();
    let text = fill(
        template,
        &[("chat", chat_title), ("timeout", &timeout_minutes)],
    );
    // Deep link: clicking takes the user to their own private chat with the
    // bot, where Telegram will push `/start <chat_id>` once they tap Start.
    // Only that private chat's recipient (by Telegram-authenticated from.id)
    // can retrieve the actual /app/... URL.
    let verify_url = Url::parse(&format!(
        "https://t.me/{}?start={}",
        bot.bot_username(),
        chat_id,
    ))
    .expect("bot_username + chat_id should produce a valid https URL");

    let msg = match bot
        .send_group_verification_message(
            chat_id,
            user_id,
            user_first_name,
            &text,
            &button_label,
            &verify_url,
        )
        .await
    {
        Ok(m) => m,
        Err(err) => {
            tracing::warn!(
                chat_id, user_id, error = %err,
                "send verification message failed; kicking user",
            );
            let _ = bot.kick_member(chat_id, user_id).await;
            return Ok(());
        }
    };

    session::create(
        bot.db(),
        session::NewSession {
            chat_id,
            user_id,
            chat_title,
            user_first_name,
            verify_msg_id: Some(msg.id.0 as i64),
            expires_at,
        },
    )
    .await?;

    spawn_expiry_task(
        bot.clone(),
        chat_id,
        user_id,
        Some(msg.id.0 as i64),
        expires_at,
    );
    Ok(())
}

/// Parse the payload of a `/start <chat_id>` message. Accepts `/start@BotName`
/// as well as plain `/start`. Returns the chat_id as i64 on success.
pub fn parse_start_payload(text: &str) -> Option<i64> {
    let text = text.trim();
    let (head, rest) = text.split_once(char::is_whitespace)?;
    let head = head.split_once('@').map(|(c, _)| c).unwrap_or(head);
    if head != "/start" {
        return None;
    }
    rest.trim().parse::<i64>().ok()
}

/// Private-chat `/start <chat_id>` handler. Only the user who was actually
/// restricted on `chat_id` has a matching pending session (Telegram guarantees
/// `from.id` authenticity), so the /app/... URL we send back is visible only
/// to the legitimate verifier.
pub async fn on_start_dm(msg: Message, chat_id: i64, bot: Arc<Bot>) -> Result<(), HandlerError> {
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    let user_id = from.id.0 as i64;

    match session::find_active(bot.db(), chat_id, user_id).await? {
        None => {
            bot.send_dm(user_id, DM_NO_PENDING, None).await?;
        }
        Some(session) => {
            let verify_url = Url::parse(&format!(
                "{}/app/{}/{}",
                bot.public_url().trim_end_matches('/'),
                chat_id,
                user_id,
            ))
            .expect("public_url + chat_id + user_id should produce a valid URL");

            let text = fill(
                DEFAULT_DM_VERIFICATION_TEXT,
                &[
                    ("user", session.user_first_name.as_str()),
                    ("chat", session.chat_title.as_str()),
                ],
            );
            let keyboard = InlineKeyboardMarkup::new([[InlineKeyboardButton::url(
                DEFAULT_DM_BUTTON_LABEL.to_string(),
                verify_url,
            )]]);
            bot.send_dm(user_id, &text, Some(keyboard)).await?;
        }
    }
    Ok(())
}

/// 启动恢复:把 DB 里仍处于 Pending 的 session 重新挂上超时协程。
pub async fn resume_pending(bot: &Arc<Bot>) -> Result<(), DbErr> {
    let rows = session::find_all_active(bot.db()).await?;
    for row in rows {
        spawn_expiry_task(
            bot.clone(),
            row.chat_id,
            row.user_id,
            row.verify_msg_id,
            row.expires_at,
        );
    }
    Ok(())
}

/// 睡到 `expires_at` 然后原子把 session 状态从 Pending 翻成 Expired,
/// 翻成功了才执行 kick + delete。中途 web 端验证成功会把状态改成 Verified,
/// 此时原子 UPDATE 匹配不到行,协程静默退出。
pub fn spawn_expiry_task(
    bot: Arc<Bot>,
    chat_id: i64,
    user_id: i64,
    verify_msg_id: Option<i64>,
    expires_at: i64,
) {
    tokio::spawn(async move {
        let delay = (expires_at - now_epoch()).max(0) as u64;
        tokio::time::sleep(Duration::from_secs(delay)).await;

        match session::mark_expired_if_pending(bot.db(), chat_id, user_id).await {
            Ok(true) => {
                if let Err(err) = bot.kick_member(chat_id, user_id).await {
                    tracing::warn!(
                        chat_id, user_id, error = %err,
                        "expiry: kick_member failed",
                    );
                }
                if let Some(msg_id) = verify_msg_id
                    && let Err(err) = bot.delete_message(chat_id, msg_id).await
                {
                    tracing::warn!(
                        chat_id, msg_id, error = %err,
                        "expiry: delete_message failed",
                    );
                }
            }
            Ok(false) => {}
            Err(err) => tracing::warn!(
                chat_id, user_id, error = %err,
                "expiry: mark_expired_if_pending failed",
            ),
        }
    });
}
