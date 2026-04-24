use std::sync::Arc;

use sea_orm::{DatabaseConnection, DbErr};
use teloxide::types::{Message, MessageEntityKind};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Enable,
    Disable,
    SetTimeout(i64),
    SetWelcome(Option<String>),
    SetButton(Option<String>),
    Status,
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

// -------- reply rendering ---------------------------------------------

const REPLY_OK: &str = "操作成功。";
const REPLY_INVALID_TIMEOUT: &str = "超时时长无效,允许范围 60-3600 秒。";

fn render_status(
    enabled: bool,
    timeout_seconds: i64,
    verified_24h: i64,
    declined_24h: i64,
) -> String {
    let state = if enabled { "已启用" } else { "已停用" };
    format!(
        "验证状态:{state}\n超时设置:{timeout_seconds} 秒\n\
         过去 24 小时已验证:{verified_24h}\n过去 24 小时已拒绝:{declined_24h}",
    )
}

pub fn render_reply(reply: CommandReply, bot_username: &str) -> String {
    match reply {
        CommandReply::Ok => REPLY_OK.to_string(),
        CommandReply::NotRegistered => {
            format!("本群尚未启用验证,请先执行 /enable@{bot_username}。")
        }
        CommandReply::NotSupergroup => {
            "本群是普通群组,无法对成员做限制,需要先升级为超级群组 (supergroup)。".to_string()
        }
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
