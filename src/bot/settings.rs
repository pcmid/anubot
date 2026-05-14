use std::sync::Arc;

use teloxide::types::{
    CallbackQuery, ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, UserId,
};
use url::Url;

use crate::app::AppState;
use crate::bot::text::*;
use crate::bot::{HandlerError, ai};
use crate::db::group::{self, AiConfig};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiField {
    Provider,
    ApiBase,
    ApiKey,
    Model,
    SpamMessageLimit,
    SpamWindowHours,
    SpamDeleteScore,
    SpamKickScore,
    SpamKickThreshold,
}

impl AiField {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "provider" => Some(Self::Provider),
            "url" | "api_base" => Some(Self::ApiBase),
            "key" | "api_key" => Some(Self::ApiKey),
            "model" => Some(Self::Model),
            "limit" | "spam_message_limit" => Some(Self::SpamMessageLimit),
            "window" | "spam_window_hours" => Some(Self::SpamWindowHours),
            "delete_score" | "spam_delete_score" => Some(Self::SpamDeleteScore),
            "kick_score" | "spam_kick_score" => Some(Self::SpamKickScore),
            "kick" | "spam_kick_threshold" | "spam_kick_count" => Some(Self::SpamKickThreshold),
            _ => None,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::ApiBase => "url",
            Self::ApiKey => "key",
            Self::Model => "model",
            Self::SpamMessageLimit => "limit",
            Self::SpamWindowHours => "window",
            Self::SpamDeleteScore => "delete_score",
            Self::SpamKickScore => "kick_score",
            Self::SpamKickThreshold => "kick",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SettingsTarget {
    pub chat_id: i64,
    pub field: AiField,
}

pub async fn on_settings_main(
    msg: Message,
    chat_id: i64,
    state: Arc<AppState>,
) -> Result<(), HandlerError> {
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    if !is_chat_admin(&state, chat_id, from.id).await {
        state
            .telegram
            .send_dm(from.id.0 as i64, SETTINGS_NOT_ADMIN, None)
            .await?;
        return Ok(());
    }
    let (text, kb) = render_main_menu(&state, chat_id).await?;
    state
        .telegram
        .send_dm(from.id.0 as i64, &text, Some(kb))
        .await?;
    Ok(())
}

pub async fn on_settings_callback(
    query: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), HandlerError> {
    let Some(data) = query.data.as_deref() else {
        return Ok(());
    };

    if let Some(action) = parse_callback(data) {
        match action {
            CallbackAction::OpenField(target) => {
                state.telegram.answer_callback(query.id.clone()).await?;
                if !is_chat_admin(&state, target.chat_id, query.from.id).await {
                    state
                        .telegram
                        .send_dm(query.from.id.0 as i64, SETTINGS_NOT_ADMIN, None)
                        .await?;
                    return Ok(());
                }
                handle_open_field(target, query.from.id.0 as i64, &state).await?;
            }
            CallbackAction::SetProvider { chat_id, value } => {
                let toast = format!("{}{}", SETTINGS_PROVIDER_SELECTED_PREFIX, value);
                state
                    .telegram
                    .answer_callback_with_text(query.id.clone(), &toast)
                    .await?;
                if !is_chat_admin(&state, chat_id, query.from.id).await {
                    state
                        .telegram
                        .send_dm(query.from.id.0 as i64, SETTINGS_NOT_ADMIN, None)
                        .await?;
                    return Ok(());
                }
                update_ai_config(&state, chat_id, AiField::Provider, Some(&value)).await?;
                let (text, kb) = render_main_menu(&state, chat_id).await?;
                state
                    .telegram
                    .send_dm(query.from.id.0 as i64, &text, Some(kb))
                    .await?;
            }
            CallbackAction::Test { chat_id } => {
                handle_test(query, chat_id, state).await?;
            }
        }
    } else {
        state.telegram.answer_callback(query.id.clone()).await?;
    }
    Ok(())
}

async fn handle_test(
    query: CallbackQuery,
    chat_id: i64,
    state: Arc<AppState>,
) -> Result<(), HandlerError> {
    state
        .telegram
        .answer_callback_with_text(query.id.clone(), SETTINGS_TEST_PENDING)
        .await?;
    let user_id = query.from.id.0 as i64;
    if !is_chat_admin(&state, chat_id, query.from.id).await {
        state
            .telegram
            .send_dm(user_id, SETTINGS_NOT_ADMIN, None)
            .await?;
        return Ok(());
    }
    let cfg = group::get_ai_config(&state.db, chat_id).await?;
    let Some((provider, base, key, model)) = cfg.ready() else {
        state
            .telegram
            .send_dm(user_id, SETTINGS_TEST_MISSING_CONFIG, None)
            .await?;
        return Ok(());
    };
    let provider = provider.to_string();
    let base = base.to_string();
    let key = key.to_string();
    let model = model.to_string();
    let state2 = state.clone();
    tokio::spawn(async move {
        let text = match ai::check_spam(&provider, &base, &key, &model, "hello").await {
            Ok(_) => SETTINGS_TEST_OK.to_string(),
            Err(e) => format!("{SETTINGS_TEST_FAILED_PREFIX}{e}"),
        };
        if let Err(e) = state2.telegram.send_dm(user_id, &text, None).await {
            tracing::warn!(error = %e, user_id, "send test result DM failed");
        }
    });
    Ok(())
}

async fn handle_open_field(
    target: SettingsTarget,
    user_id: i64,
    state: &AppState,
) -> Result<(), HandlerError> {
    match target.field {
        AiField::Provider => {
            let kb = render_provider_picker(target.chat_id);
            state
                .telegram
                .send_dm(user_id, SETTINGS_PROMPT_PICK_PROVIDER, Some(kb))
                .await?;
        }
        AiField::ApiBase
        | AiField::ApiKey
        | AiField::Model
        | AiField::SpamMessageLimit
        | AiField::SpamWindowHours
        | AiField::SpamDeleteScore
        | AiField::SpamKickScore
        | AiField::SpamKickThreshold => {
            let body = match target.field {
                AiField::ApiBase => SETTINGS_PROMPT_API_BASE,
                AiField::ApiKey => SETTINGS_PROMPT_API_KEY,
                AiField::Model => SETTINGS_PROMPT_MODEL,
                AiField::SpamMessageLimit => SETTINGS_PROMPT_SPAM_MESSAGE_LIMIT,
                AiField::SpamWindowHours => SETTINGS_PROMPT_SPAM_WINDOW_HOURS,
                AiField::SpamDeleteScore => SETTINGS_PROMPT_SPAM_DELETE_SCORE,
                AiField::SpamKickScore => SETTINGS_PROMPT_SPAM_KICK_SCORE,
                AiField::SpamKickThreshold => SETTINGS_PROMPT_SPAM_KICK_THRESHOLD,
                AiField::Provider => unreachable!(),
            };
            let text = format!(
                "{}{}",
                body,
                settings_tag(target.chat_id, target.field.tag())
            );
            state.telegram.send_force_reply(user_id, &text).await?;
        }
    }
    Ok(())
}

pub async fn on_settings_reply(
    msg: Message,
    target: SettingsTarget,
    state: Arc<AppState>,
) -> Result<(), HandlerError> {
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    if !is_reply_to_this_bot(&msg, state.identity.user_id) {
        return Ok(());
    }
    if !is_chat_admin(&state, target.chat_id, from.id).await {
        state
            .telegram
            .send_dm(from.id.0 as i64, SETTINGS_NOT_ADMIN, None)
            .await?;
        return Ok(());
    }

    let value = msg.text().unwrap_or("").trim();
    if value.is_empty() {
        state
            .telegram
            .send_dm(from.id.0 as i64, SETTINGS_EMPTY_VALUE, None)
            .await?;
        return Ok(());
    }
    if matches!(target.field, AiField::ApiBase) && Url::parse(value).is_err() {
        state
            .telegram
            .send_dm(from.id.0 as i64, SETTINGS_INVALID_URL, None)
            .await?;
        return Ok(());
    }
    if !validate_settings_value(target.field, value) {
        state
            .telegram
            .send_dm(from.id.0 as i64, SETTINGS_INVALID_NUMBER, None)
            .await?;
        return Ok(());
    }

    update_ai_config(&state, target.chat_id, target.field, Some(value)).await?;

    let priv_chat_id = msg.chat.id.0;
    if let Err(e) = state
        .telegram
        .delete_message(priv_chat_id, msg.id.0 as i64)
        .await
    {
        tracing::warn!(error = %e, "delete user reply message failed");
    }
    if let Some(prompt) = msg.reply_to_message()
        && let Err(e) = state
            .telegram
            .delete_message(priv_chat_id, prompt.id.0 as i64)
            .await
    {
        tracing::warn!(error = %e, "delete prompt message failed");
    }

    let (text, kb) = render_main_menu(&state, target.chat_id).await?;
    state
        .telegram
        .send_dm(from.id.0 as i64, &text, Some(kb))
        .await?;
    Ok(())
}

pub fn extract_settings_tag(msg: &Message) -> Option<SettingsTarget> {
    let reply = msg.reply_to_message()?;
    let text = reply.text()?;
    parse_tag(text)
}

fn parse_tag(text: &str) -> Option<SettingsTarget> {
    let start = text.rfind("[set:")?;
    let rest = &text[start + 5..];
    let end = rest.find(']')?;
    let inner = &rest[..end];
    let (chat_id_str, field_str) = inner.split_once(':')?;
    let chat_id = chat_id_str.parse::<i64>().ok()?;
    let field = AiField::parse(field_str)?;
    Some(SettingsTarget { chat_id, field })
}

fn is_reply_to_this_bot(msg: &Message, bot_user_id: UserId) -> bool {
    msg.reply_to_message()
        .and_then(|reply| reply.from.as_ref())
        .is_some_and(|from| from.id == bot_user_id)
}

fn validate_settings_value(field: AiField, value: &str) -> bool {
    match field {
        AiField::SpamMessageLimit | AiField::SpamWindowHours | AiField::SpamKickThreshold => {
            value.parse::<i64>().is_ok_and(|n| n > 0)
        }
        AiField::SpamDeleteScore | AiField::SpamKickScore => {
            value.parse::<i64>().is_ok_and(|n| (0..=100).contains(&n))
        }
        AiField::Provider | AiField::ApiBase | AiField::ApiKey | AiField::Model => true,
    }
}

async fn update_ai_config(
    state: &AppState,
    chat_id: i64,
    field: AiField,
    value: Option<&str>,
) -> Result<bool, HandlerError> {
    Ok(group::update_ai_config(&state.db, chat_id, |cfg| {
        apply_ai_config_field(cfg, field, value);
    })
    .await?)
}

fn apply_ai_config_field(cfg: &mut AiConfig, field: AiField, value: Option<&str>) {
    let v = value.map(str::to_string);
    match field {
        AiField::Provider => cfg.provider = v,
        AiField::ApiBase => cfg.api_base = v,
        AiField::ApiKey => cfg.api_key = v,
        AiField::Model => cfg.model = v,
        AiField::SpamMessageLimit => cfg.spam_check_message_limit = v.and_then(|s| s.parse().ok()),
        AiField::SpamWindowHours => cfg.spam_check_window_hours = v.and_then(|s| s.parse().ok()),
        AiField::SpamDeleteScore => cfg.spam_delete_score = v.and_then(|s| s.parse().ok()),
        AiField::SpamKickScore => cfg.spam_kick_score = v.and_then(|s| s.parse().ok()),
        AiField::SpamKickThreshold => cfg.spam_kick_threshold = v.and_then(|s| s.parse().ok()),
    }
}

#[derive(Debug, Clone)]
enum CallbackAction {
    OpenField(SettingsTarget),
    SetProvider { chat_id: i64, value: String },
    Test { chat_id: i64 },
}

fn parse_callback(data: &str) -> Option<CallbackAction> {
    if let Some(rest) = data.strip_prefix("setval:") {
        let mut it = rest.splitn(3, ':');
        let chat_id = it.next()?.parse::<i64>().ok()?;
        let field = it.next()?;
        let value = it.next()?;
        if field != "provider" {
            return None;
        }
        return Some(CallbackAction::SetProvider {
            chat_id,
            value: value.to_string(),
        });
    }
    if let Some(rest) = data.strip_prefix("set:") {
        let (chat_id_str, field_str) = rest.split_once(':')?;
        let chat_id = chat_id_str.parse::<i64>().ok()?;
        let field = AiField::parse(field_str)?;
        return Some(CallbackAction::OpenField(SettingsTarget { chat_id, field }));
    }
    if let Some(rest) = data.strip_prefix("test:") {
        let chat_id = rest.parse::<i64>().ok()?;
        return Some(CallbackAction::Test { chat_id });
    }
    None
}

async fn is_chat_admin(state: &AppState, chat_id: i64, user_id: UserId) -> bool {
    state
        .telegram
        .is_privileged(ChatId(chat_id), user_id)
        .await
        .unwrap_or(false)
}

async fn render_main_menu(
    state: &AppState,
    chat_id: i64,
) -> Result<(String, InlineKeyboardMarkup), HandlerError> {
    let cfg = group::get_ai_config(&state.db, chat_id).await?;
    let limit = cfg.spam_check_message_limit().to_string();
    let window_hours = cfg.spam_check_window_hours().to_string();
    let delete_score = cfg.spam_delete_score().to_string();
    let kick_score = cfg.spam_kick_score().to_string();
    let kick_threshold = cfg.spam_kick_threshold().to_string();
    let text = fill(
        SETTINGS_AI_CONFIG_TEMPLATE,
        &[
            ("chat", &chat_id.to_string()),
            ("provider", &display_field(cfg.provider.as_deref())),
            ("base", &display_field(cfg.api_base.as_deref())),
            ("key", &display_key(cfg.api_key.as_deref())),
            ("model", &display_field(cfg.model.as_deref())),
            ("limit", &limit),
            ("window_hours", &window_hours),
            ("delete_score", &delete_score),
            ("kick_score", &kick_score),
            ("kick_threshold", &kick_threshold),
        ],
    );
    let kb = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(
                BTN_SET_PROVIDER.to_string(),
                format!("set:{}:provider", chat_id),
            ),
            InlineKeyboardButton::callback(
                BTN_SET_API_BASE.to_string(),
                format!("set:{}:url", chat_id),
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                BTN_SET_API_KEY.to_string(),
                format!("set:{}:key", chat_id),
            ),
            InlineKeyboardButton::callback(
                BTN_SET_MODEL.to_string(),
                format!("set:{}:model", chat_id),
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                BTN_SET_SPAM_MESSAGE_LIMIT.to_string(),
                format!("set:{}:limit", chat_id),
            ),
            InlineKeyboardButton::callback(
                BTN_SET_SPAM_WINDOW_HOURS.to_string(),
                format!("set:{}:window", chat_id),
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                BTN_SET_SPAM_DELETE_SCORE.to_string(),
                format!("set:{}:delete_score", chat_id),
            ),
            InlineKeyboardButton::callback(
                BTN_SET_SPAM_KICK_SCORE.to_string(),
                format!("set:{}:kick_score", chat_id),
            ),
            InlineKeyboardButton::callback(
                BTN_SET_SPAM_KICK_THRESHOLD.to_string(),
                format!("set:{}:kick", chat_id),
            ),
        ],
        vec![InlineKeyboardButton::callback(
            BTN_TEST.to_string(),
            format!("test:{}", chat_id),
        )],
    ]);
    Ok((text, kb))
}

fn render_provider_picker(chat_id: i64) -> InlineKeyboardMarkup {
    let rows: Vec<Vec<InlineKeyboardButton>> = PROVIDER_BUTTONS
        .chunks(2)
        .map(|pair| {
            pair.iter()
                .map(|(label, value)| {
                    InlineKeyboardButton::callback(
                        label.to_string(),
                        format!("setval:{}:provider:{}", chat_id, value),
                    )
                })
                .collect()
        })
        .collect();
    InlineKeyboardMarkup::new(rows)
}

fn display_field(v: Option<&str>) -> String {
    v.unwrap_or(SETTINGS_UNSET).to_string()
}

fn display_key(v: Option<&str>) -> String {
    match v {
        None => SETTINGS_UNSET.to_string(),
        Some(s) if s.len() <= 8 => "***".to_string(),
        Some(s) => format!("{}...{}", &s[..4], &s[s.len() - 4..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag_finds_trailing_marker() {
        let t = parse_tag("请回复 API URL\n\n[set:-1001234567890:url]").unwrap();
        assert_eq!(t.chat_id, -1001234567890);
        assert!(matches!(t.field, AiField::ApiBase));
    }

    #[test]
    fn parse_tag_returns_none_when_missing() {
        assert!(parse_tag("nothing here").is_none());
    }

    #[test]
    fn parse_callback_open_field_roundtrip() {
        let CallbackAction::OpenField(t) = parse_callback("set:42:model").unwrap() else {
            panic!("expected OpenField");
        };
        assert_eq!(t.chat_id, 42);
        assert!(matches!(t.field, AiField::Model));
    }

    #[test]
    fn parse_callback_setval_provider_roundtrip() {
        let CallbackAction::SetProvider { chat_id, value } =
            parse_callback("setval:-1001:provider:anthropic").unwrap()
        else {
            panic!("expected SetProvider");
        };
        assert_eq!(chat_id, -1001);
        assert_eq!(value, "anthropic");
    }

    #[test]
    fn parse_callback_setval_non_provider_field_rejected() {
        assert!(parse_callback("setval:42:url:https://x").is_none());
    }

    #[test]
    fn parse_callback_test_roundtrip() {
        let CallbackAction::Test { chat_id } = parse_callback("test:-1003900460608").unwrap()
        else {
            panic!("expected Test");
        };
        assert_eq!(chat_id, -1003900460608);
    }
}
