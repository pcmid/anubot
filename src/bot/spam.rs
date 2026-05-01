use std::sync::Arc;

use teloxide::prelude::Message;

use crate::bot::{Bot, HandlerError, ai};
use crate::db::{group, session};
use crate::util::time::now_epoch;

pub async fn on_user_message(msg: Message, bot: Arc<Bot>) -> Result<(), HandlerError> {
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    let chat_id = msg.chat.id.0;
    let user_id = from.id.0 as i64;

    let Some(s) = session::find_verified(bot.db(), chat_id, user_id).await? else {
        return Ok(());
    };

    let Some(g) = group::get(bot.db(), chat_id).await? else {
        return Ok(());
    };
    let cfg = group::AiConfig::parse(g.ai_config.as_deref());
    let within_message_limit = s.message_counts < cfg.spam_check_message_limit();
    let verified_at = s.verified_at.unwrap_or(s.created_at);
    let window_secs = cfg.spam_check_window_hours() * 60 * 60;
    let within_time_window = now_epoch().saturating_sub(verified_at) <= window_secs;

    session::increment_message_count_if_verified(bot.db(), chat_id, user_id).await?;

    if !within_message_limit || !within_time_window {
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

    let text = match msg.text() {
        Some(t) if !t.trim().is_empty() => t.to_string(),
        _ => return Ok(()),
    };

    let bot2 = bot.clone();
    let msg_id = msg.id.0 as i64;
    tokio::spawn(async move {
        match ai::check_spam(&provider, &base, &key, &model, &text).await {
            Ok(true) => handle_spam(&bot2, chat_id, user_id, msg_id, kick_threshold).await,
            Ok(false) => {}
            Err(err) => tracing::warn!(
                error = %err, chat_id, user_id,
                "AI spam check failed; allowing message",
            ),
        }
    });

    Ok(())
}

async fn handle_spam(bot: &Bot, chat_id: i64, user_id: i64, msg_id: i64, kick_threshold: i64) {
    if let Err(err) = bot.delete_message(chat_id, msg_id).await {
        tracing::warn!(error = %err, chat_id, msg_id, "delete spam message failed");
    }
    match session::increment_spam_count_if_verified(bot.db(), chat_id, user_id).await {
        Ok(Some(new_count)) => {
            tracing::info!(chat_id, user_id, new_count, "spam message detected");
            if new_count >= kick_threshold
                && let Err(err) = bot.kick_member(chat_id, user_id).await
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
