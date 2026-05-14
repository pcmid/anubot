use std::sync::Arc;
use std::time::Duration;

use rand::RngCore;
use sea_orm::DbErr;
use teloxide::types::{
    ChatJoinRequest, ChatMemberUpdated, InlineKeyboardButton, InlineKeyboardMarkup, Message,
};
use url::Url;

use crate::app::AppState;
use crate::bot::HandlerError;
use crate::bot::text::*;
use crate::db::{group, session};
use crate::util::time::now_epoch;

const EXPIRY_POLL_SECONDS: u64 = 60;
const EXPIRY_BATCH_LIMIT: u64 = 100;

pub async fn on_chat_member_joined(
    update: ChatMemberUpdated,
    state: Arc<AppState>,
) -> Result<(), HandlerError> {
    let user = update.new_chat_member.user.clone();
    if user.id == state.identity.user_id {
        return Ok(());
    }

    let chat_id = update.chat.id.0;
    let chat_title = update.chat.title().unwrap_or("").to_string();
    tracing::info!(chat_id, user_id = user.id.0, "user joined group");
    tracing::debug!(
        chat_id,
        chat_title = %chat_title,
        user_id = user.id.0,
        user_first_name = %user.first_name,
        "user joined group (details)",
    );
    verification(
        &state,
        chat_id,
        user.id.0 as i64,
        &chat_title,
        &user.first_name,
    )
    .await?;
    Ok(())
}

pub async fn on_chat_join_request(
    request: ChatJoinRequest,
    state: Arc<AppState>,
) -> Result<(), HandlerError> {
    let user = request.from.clone();
    if user.id == state.identity.user_id {
        return Ok(());
    }

    let chat_id = request.chat.id.0;
    let user_id = user.id.0 as i64;
    let chat_title = request.chat.title().unwrap_or("").to_string();

    tracing::info!(chat_id, user_id, "join_request: received");

    let Some(group) = group::get(&state.db, chat_id).await? else {
        tracing::info!(chat_id, user_id, "join_request: group unknown — ignoring");
        return Ok(());
    };
    if !group.enabled {
        tracing::info!(
            chat_id,
            user_id,
            "join_request: verification disabled for this chat — ignoring",
        );
        return Ok(());
    }

    tracing::info!(chat_id, user_id, "join_request: step=restrict");
    if let Err(err) = state.telegram.restrict_member(chat_id, user_id).await {
        tracing::error!(
            chat_id,
            user_id,
            error = %err,
            "join_request: restrict failed before approve; bot likely lacks restrict-member permission — not approving",
        );
        return Ok(());
    }

    tracing::info!(chat_id, user_id, "join_request: step=approve");
    if let Err(err) = state.telegram.approve_join_request(chat_id, user_id).await {
        tracing::error!(
            chat_id,
            user_id,
            error = %err,
            "join_request: approve failed after retries; bot likely lacks invite-users permission",
        );
        return Ok(());
    }
    tracing::info!(chat_id, user_id, "join_request: approved");
    tracing::debug!(
        chat_id,
        chat_title = %chat_title,
        user_id,
        user_first_name = %user.first_name,
        "join_request: approved (details)",
    );

    Ok(())
}

async fn verification(
    state: &Arc<AppState>,
    chat_id: i64,
    user_id: i64,
    chat_title: &str,
    user_first_name: &str,
) -> Result<(), DbErr> {
    let Some(group) = group::get(&state.db, chat_id).await? else {
        return Ok(());
    };
    if !group.enabled {
        return Ok(());
    }

    if session::find_active(&state.db, chat_id, user_id)
        .await?
        .is_some()
    {
        return Ok(());
    }

    if let Err(err) = state.telegram.restrict_member(chat_id, user_id).await {
        tracing::error!(
            chat_id, user_id, error = %err,
            "restrict_member failed after retries; bot likely lacks restrict-member permission in this chat — skipping verification",
        );
        return Ok(());
    }

    let expires_at = now_epoch() + group.timeout_seconds;
    let verify_token = generate_verify_token();
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
    let verify_url = Url::parse(&format!(
        "https://t.me/{}?start={}",
        state.identity.username, chat_id,
    ))
    .expect("bot_username + chat_id should produce a valid https URL");

    let msg = match state
        .telegram
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
        Err(send_err) => {
            tracing::warn!(
                chat_id, user_id, error = %send_err,
                "send verification message failed; trying to kick",
            );
            if let Err(kick_err) = state.telegram.kick_member(chat_id, user_id).await {
                tracing::error!(
                    chat_id, user_id,
                    send_error = %send_err, kick_error = %kick_err,
                    "send AND kick both failed; attempting unrestrict so the member is not stuck muted",
                );
                if let Err(unrestrict_err) =
                    state.telegram.unrestrict_member(chat_id, user_id).await
                {
                    tracing::error!(
                        chat_id, user_id, error = %unrestrict_err,
                        "unrestrict also failed; member is muted with no session — manual intervention required",
                    );
                }
            }
            return Ok(());
        }
    };

    session::create(
        &state.db,
        session::NewSession {
            chat_id,
            user_id,
            chat_title,
            user_first_name,
            verify_msg_id: Some(msg.id.0 as i64),
            verify_token: &verify_token,
            expires_at,
        },
    )
    .await?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPayload {
    Verify(i64),
    Settings(i64),
}

pub fn parse_start_payload(text: &str) -> Option<StartPayload> {
    let text = text.trim();
    let (head, rest) = text.split_once(char::is_whitespace)?;
    let head = head.split_once('@').map(|(c, _)| c).unwrap_or(head);
    if head != "/start" {
        return None;
    }
    let rest = rest.trim();
    if let Some(suffix) = rest.strip_prefix("settings_") {
        return suffix.parse::<i64>().ok().map(StartPayload::Settings);
    }
    rest.parse::<i64>().ok().map(StartPayload::Verify)
}

pub async fn on_start_dm(
    msg: Message,
    payload: StartPayload,
    state: Arc<AppState>,
) -> Result<(), HandlerError> {
    match payload {
        StartPayload::Verify(chat_id) => on_verify_start(msg, chat_id, state).await,
        StartPayload::Settings(chat_id) => {
            crate::bot::settings::on_settings_main(msg, chat_id, state).await
        }
    }
}

async fn on_verify_start(
    msg: Message,
    chat_id: i64,
    state: Arc<AppState>,
) -> Result<(), HandlerError> {
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    let user_id = from.id.0 as i64;

    match session::find_active(&state.db, chat_id, user_id).await? {
        None => {
            state.telegram.send_dm(user_id, DM_NO_PENDING, None).await?;
        }
        Some(session) => {
            let verify_token = match session.verify_token {
                Some(token) => token,
                None => {
                    let token = generate_verify_token();
                    let persisted =
                        session::set_verify_token(&state.db, chat_id, user_id, &token).await?;
                    if !persisted {
                        state.telegram.send_dm(user_id, DM_NO_PENDING, None).await?;
                        return Ok(());
                    }
                    token
                }
            };
            let verify_url = Url::parse(&format!(
                "{}/app/{}/{}/{}",
                state.public_url.trim_end_matches('/'),
                chat_id,
                user_id,
                verify_token,
            ))
            .expect("public_url + chat_id + user_id + token should produce a valid URL");

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
            state
                .telegram
                .send_dm(user_id, &text, Some(keyboard))
                .await?;
        }
    }
    Ok(())
}

fn generate_verify_token() -> String {
    let mut bytes = [0_u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn spawn_expiry_worker(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(EXPIRY_POLL_SECONDS));
        loop {
            interval.tick().await;
            if let Err(err) = expire_due_sessions(&state).await {
                tracing::warn!(error = %err, "expiry worker failed");
            }
        }
    });
}

async fn expire_due_sessions(state: &Arc<AppState>) -> Result<(), DbErr> {
    loop {
        let now = now_epoch();
        let rows = session::find_due_pending(&state.db, now, EXPIRY_BATCH_LIMIT).await?;
        if rows.is_empty() {
            return Ok(());
        }

        let row_count = rows.len() as u64;
        for row in rows {
            let expired =
                session::mark_expired_if_pending_due(&state.db, row.chat_id, row.user_id, now)
                    .await?;
            if !expired {
                continue;
            }

            if let Err(err) = state.telegram.kick_member(row.chat_id, row.user_id).await {
                tracing::error!(
                    chat_id = row.chat_id, user_id = row.user_id, error = %err,
                    "expiry: kick_member failed after retries; bot likely lacks ban-users permission in this chat",
                );
            }

            if let Some(msg_id) = row.verify_msg_id
                && let Err(err) = state.telegram.delete_message(row.chat_id, msg_id).await
            {
                tracing::warn!(
                    chat_id = row.chat_id, msg_id, error = %err,
                    "expiry: delete_message failed",
                );
            }
        }

        if row_count < EXPIRY_BATCH_LIMIT {
            return Ok(());
        }
    }
}
