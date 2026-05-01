use std::sync::Arc;

use sea_orm::{DatabaseConnection, DbErr};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageEntityKind};
use url::Url;

use crate::bot::text::*;
use crate::bot::{Bot, HandlerError};
use crate::db::session::SessionStatus;
use crate::db::{group, session};
use crate::util::time::now_epoch;

pub async fn filter_admin_command(msg: Message, bot: Arc<Bot>) -> Option<Command> {
    let from = msg.from.as_ref()?;
    if !bot
        .is_privileged(msg.chat.id, from.id)
        .await
        .unwrap_or(false)
    {
        return None;
    }
    let entities = msg.parse_entities()?;
    let first = entities.first()?;
    if first.kind() != &MessageEntityKind::BotCommand {
        return None;
    }
    let cmd_text = first.text();
    let (cmd, at) = cmd_text.split_once('@')?;
    if !at.eq_ignore_ascii_case(bot.bot_username()) {
        return None;
    }
    let full = msg.text()?;
    let rest = full[first.range().end..].trim();
    parse_admin_command(cmd, rest)
}

pub async fn on_command(msg: Message, cmd: Command, bot: Arc<Bot>) -> Result<(), HandlerError> {
    if matches!(cmd, Command::Settings) {
        return on_settings_command(msg, bot).await;
    }
    let reply = if matches!(cmd, Command::Enable) && !msg.chat.is_supergroup() {
        CommandReply::NotSupergroup
    } else {
        handle_command(bot.db(), msg.chat.id.0, cmd).await?
    };
    bot.reply_to(
        msg.chat.id,
        msg.id,
        &render_reply(reply, bot.bot_username()),
    )
    .await?;
    Ok(())
}

async fn on_settings_command(msg: Message, bot: Arc<Bot>) -> Result<(), HandlerError> {
    let chat_id = msg.chat.id.0;
    if group::get(bot.db(), chat_id).await?.is_none() {
        bot.reply_to(msg.chat.id, msg.id, SETTINGS_GROUP_NOT_REGISTERED)
            .await?;
        return Ok(());
    }
    let link = format!(
        "https://t.me/{}?start=settings_{}",
        bot.bot_username(),
        chat_id
    );
    let url = Url::parse(&link).expect("bot_username + chat_id should produce a valid URL");
    let keyboard = InlineKeyboardMarkup::new([[InlineKeyboardButton::url(
        SETTINGS_LINK_LABEL.to_string(),
        url,
    )]]);
    bot.reply_with_keyboard(msg.chat.id, msg.id, SETTINGS_COMMAND_PROMPT, keyboard)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Enable,
    Disable,
    SetTimeout(i64),
    SetWelcome(Option<String>),
    SetButton(Option<String>),
    Status,
    Settings,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommandReply {
    Ok,
    NotRegistered,
    NotSupergroup,
    InvalidTimeout,
    Status {
        enabled: bool,
        timeout_seconds: i64,
        verified_24h: i64,
        declined_24h: i64,
    },
}

pub const TIMEOUT_MIN: i64 = 60;
pub const TIMEOUT_MAX: i64 = 3600;
pub const STATUS_WINDOW_SECONDS: i64 = 86_400;

pub async fn handle_command(
    db: &DatabaseConnection,
    chat_id: i64,
    cmd: Command,
) -> Result<CommandReply, DbErr> {
    if matches!(cmd, Command::Enable) {
        group::upsert_enabled(db, chat_id).await?;
        return Ok(CommandReply::Ok);
    }

    let Some(group) = group::get(db, chat_id).await? else {
        return Ok(CommandReply::NotRegistered);
    };

    match cmd {
        Command::Enable => {
            group::set_enabled(db, chat_id, true).await?;
            Ok(CommandReply::Ok)
        }
        Command::Disable => {
            group::set_enabled(db, chat_id, false).await?;
            Ok(CommandReply::Ok)
        }
        Command::SetTimeout(seconds) => {
            if !(TIMEOUT_MIN..=TIMEOUT_MAX).contains(&seconds) {
                return Ok(CommandReply::InvalidTimeout);
            }
            group::set_timeout(db, chat_id, seconds).await?;
            Ok(CommandReply::Ok)
        }
        Command::SetWelcome(text) => {
            group::set_welcome(db, chat_id, text.as_deref()).await?;
            Ok(CommandReply::Ok)
        }
        Command::SetButton(text) => {
            group::set_button(db, chat_id, text.as_deref()).await?;
            Ok(CommandReply::Ok)
        }
        Command::Settings => Ok(CommandReply::Ok),
        Command::Status => {
            let since = now_epoch() - STATUS_WINDOW_SECONDS;
            let verified_24h =
                session::count_by_status_since(db, chat_id, SessionStatus::Verified, since).await?;
            let expired_24h =
                session::count_by_status_since(db, chat_id, SessionStatus::Expired, since).await?;
            Ok(CommandReply::Status {
                enabled: group.enabled,
                timeout_seconds: group.timeout_seconds,
                verified_24h,
                declined_24h: expired_24h,
            })
        }
    }
}

fn render_status(
    enabled: bool,
    timeout_seconds: i64,
    verified_24h: i64,
    declined_24h: i64,
) -> String {
    let state = if enabled {
        REPLY_STATUS_ENABLED
    } else {
        REPLY_STATUS_DISABLED
    };
    fill(
        REPLY_STATUS_TEMPLATE,
        &[
            ("state", state),
            ("timeout_seconds", &timeout_seconds.to_string()),
            ("verified_24h", &verified_24h.to_string()),
            ("declined_24h", &declined_24h.to_string()),
        ],
    )
}

pub fn render_reply(reply: CommandReply, bot_username: &str) -> String {
    match reply {
        CommandReply::Ok => REPLY_OK.to_string(),
        CommandReply::NotRegistered => fill(
            REPLY_NOT_REGISTERED_TEMPLATE,
            &[("bot_username", bot_username)],
        ),
        CommandReply::NotSupergroup => REPLY_NOT_SUPERGROUP.to_string(),
        CommandReply::InvalidTimeout => REPLY_INVALID_TIMEOUT.to_string(),
        CommandReply::Status {
            enabled,
            timeout_seconds,
            verified_24h,
            declined_24h,
        } => render_status(enabled, timeout_seconds, verified_24h, declined_24h),
    }
}

pub fn parse_admin_command(cmd: &str, rest: &str) -> Option<Command> {
    match (cmd, rest) {
        ("/enable", "") => Some(Command::Enable),
        ("/disable", "") => Some(Command::Disable),
        ("/set_timeout", r) => r.parse::<i64>().ok().map(Command::SetTimeout),
        ("/set_welcome", r) => Some(Command::SetWelcome(opt_string(r))),
        ("/set_button", r) => Some(Command::SetButton(opt_string(r))),
        ("/status", "") => Some(Command::Status),
        ("/settings", "") => Some(Command::Settings),
        _ => None,
    }
}

fn opt_string(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
