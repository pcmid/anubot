use std::sync::Arc;

use teloxide::prelude::Message;

use crate::app::AppState;
use crate::bot::{HandlerError, ai};
use crate::db::{group, session};
use crate::util::time::now_epoch;

pub async fn on_user_message(msg: Message, state: Arc<AppState>) -> Result<(), HandlerError> {
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

    session::increment_message_count_if_verified(&state.db, chat_id, user_id).await?;

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
    let kick_threshold = cfg.spam_kick_threshold();

    let Some(text) = extract_spam_check_text(&msg) else {
        return Ok(());
    };

    let state2 = state.clone();
    let msg_id = msg.id.0 as i64;
    tokio::spawn(async move {
        match ai::check_spam(&provider, &base, &key, &model, &text).await {
            Ok(true) => handle_spam(&state2, chat_id, user_id, msg_id, kick_threshold).await,
            Ok(false) => {}
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

async fn handle_spam(
    state: &AppState,
    chat_id: i64,
    user_id: i64,
    msg_id: i64,
    kick_threshold: i64,
) {
    if let Err(err) = state.telegram.delete_message(chat_id, msg_id).await {
        tracing::warn!(error = %err, chat_id, msg_id, "delete spam message failed");
    }
    match session::increment_spam_count_if_verified(&state.db, chat_id, user_id).await {
        Ok(Some(new_count)) => {
            tracing::info!(chat_id, user_id, new_count, "spam message detected");
            if new_count >= kick_threshold
                && let Err(err) = state.telegram.kick_member(chat_id, user_id).await
            {
                tracing::warn!(
                    error = %err, chat_id, user_id,
                    "kick spammer failed",
                );
            }
        }
        Ok(None) => {}
        Err(err) => tracing::warn!(
            error = %err, chat_id, user_id,
            "increment_spam_count failed",
        ),
    }
}
