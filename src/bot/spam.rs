use std::sync::Arc;

use teloxide::prelude::Message;

use crate::app::AppState;
use crate::bot::{HandlerError, ai};
use crate::db::{group, session};
use crate::util::time::now_epoch;

pub async fn on_user_message(msg: Message, state: Arc<AppState>) -> Result<(), HandlerError> {
    check_message(msg, state, MessageKind::New).await
}

pub async fn on_user_edited_message(
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), HandlerError> {
    check_message(msg, state, MessageKind::Edited).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageKind {
    New,
    Edited,
}

async fn check_message(
    msg: Message,
    state: Arc<AppState>,
    kind: MessageKind,
) -> Result<(), HandlerError> {
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    let chat_id = msg.chat.id.0;
    let user_id = from.id.0 as i64;

    let Some(s) = session::find_verified(&state.db, chat_id, user_id).await? else {
        return Ok(());
    };

    if group::get(&state.db, chat_id).await?.is_none() {
        return Ok(());
    }
    let cfg = group::get_ai_config(&state.db, chat_id).await?;
    let within_message_limit = s.message_counts < cfg.spam_check_message_limit();
    let verified_at = s.verified_at.unwrap_or(s.created_at);
    let window_secs = cfg.spam_check_window_hours() * 60 * 60;
    let within_time_window = now_epoch().saturating_sub(verified_at) <= window_secs;

    if kind == MessageKind::New {
        session::increment_message_count_if_verified(&state.db, chat_id, user_id).await?;
    }

    if !within_message_limit && !within_time_window {
        return Ok(());
    }

    let Some((provider, base, key, model)) = cfg.ready() else {
        return Ok(());
    };
    let provider = provider.to_string();
    let base = base.to_string();
    let key = key.to_string();
    let model = model.to_string();
    let delete_score = cfg.spam_delete_score();
    let kick_score = cfg.spam_kick_score();
    let kick_threshold = cfg.spam_kick_threshold();

    let Some(text) = extract_spam_check_text(&msg) else {
        return Ok(());
    };

    let msg_id = msg.id.0 as i64;
    tokio::spawn(async move {
        match ai::check_spam(&provider, &base, &key, &model, &text).await {
            Ok(score) if score >= delete_score => {
                tracing::debug!(chat_id, user_id, score, "spam message detected",);
                delete_spam_message(&state, chat_id, msg_id).await;

                if score >= kick_score
                    || spam_count(&state, chat_id, user_id).await >= kick_threshold
                {
                    kick_spammer(&state, chat_id, user_id).await;
                }
            }
            Ok(_) => {}
            Err(err) => tracing::warn!(
                error = %err, chat_id, user_id,
                "AI spam check failed; allowing message",
            ),
        }
    });

    Ok(())
}

fn extract_spam_check_text(msg: &Message) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(reply) = msg.reply_to_message()
        && let Some(text) = message_text(reply)
    {
        parts.push(text.to_string());
    }
    if let Some(text) = message_text(msg) {
        parts.push(text.to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

fn message_text(msg: &Message) -> Option<&str> {
    msg.text()
        .or_else(|| msg.caption())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

async fn delete_spam_message(state: &AppState, chat_id: i64, msg_id: i64) {
    if let Err(err) = state.telegram.delete_message(chat_id, msg_id).await {
        tracing::warn!(error = %err, chat_id, msg_id, "delete spam message failed");
    }
}

async fn spam_count(state: &AppState, chat_id: i64, user_id: i64) -> i64 {
    match session::increment_spam_count_if_verified(&state.db, chat_id, user_id).await {
        Ok(Some(new_count)) => new_count,
        Ok(None) => 0,
        Err(err) => {
            tracing::warn!(
                error = %err, chat_id, user_id,
                "increment_spam_count failed",
            );
            0
        }
    }
}

async fn kick_spammer(state: &AppState, chat_id: i64, user_id: i64) {
    tracing::debug!(chat_id, user_id, "kicking spammer",);
    if let Err(err) = state.telegram.kick_member(chat_id, user_id).await {
        tracing::warn!(
            error = %err, chat_id, user_id,
            "kick spammer failed",
        );
    }
}
