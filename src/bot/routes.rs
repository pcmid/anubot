use std::sync::Arc;

use teloxide::dispatching::{Dispatcher, UpdateFilterExt};
use teloxide::payloads::SetMyCommandsSetters;
use teloxide::prelude::*;
use teloxide::types::{BotCommand, BotCommandScope, Message, Update};
use teloxide::{RequestError, dptree};

use crate::app::AppState;
use crate::bot::text::*;
use crate::bot::{commands, settings, spam, verify};

pub async fn run_dispatcher(state: Arc<AppState>) {
    if let Err(err) = register_commands(&state).await {
        tracing::warn!(
            error = %err,
            "set_my_commands failed; /-autocomplete may not appear",
        );
    }

    verify::spawn_expiry_worker(state.clone());

    let private_messages = dptree::entry()
        .filter(|m: Message| m.chat.is_private())
        .branch(
            dptree::entry()
                .filter_map(|m: Message| settings::extract_settings_tag(&m))
                .endpoint(settings::on_settings_reply),
        )
        .branch(
            dptree::entry()
                .filter_map(|m: Message| m.text().and_then(verify::parse_start_payload))
                .endpoint(verify::on_start_dm),
        );

    let group_messages = dptree::entry()
        .filter(|m: Message| !m.chat.is_private())
        .branch(
            dptree::entry()
                .filter_map(verify::extract_new_chat_members)
                .endpoint(verify::on_new_chat_members),
        )
        .branch(dptree::entry().endpoint(spam::on_user_message));

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .branch(
                    dptree::entry()
                        .filter_map_async(commands::filter_admin_command)
                        .endpoint(commands::on_command),
                )
                .branch(private_messages)
                .branch(group_messages),
        )
        .branch(Update::filter_callback_query().endpoint(settings::on_settings_callback));

    let mut dispatcher = Dispatcher::builder(state.telegram.client(), handler)
        .dependencies(dptree::deps![state])
        .default_handler(|_upd| async {})
        .build();

    tokio::spawn(async move {
        dispatcher.dispatch().await;
    });
}

async fn register_commands(state: &AppState) -> Result<(), RequestError> {
    let admin_commands = vec![
        BotCommand::new("enable", CMD_ENABLE),
        BotCommand::new("disable", CMD_DISABLE),
        BotCommand::new("set_timeout", CMD_SET_TIMEOUT),
        BotCommand::new("set_welcome", CMD_SET_WELCOME),
        BotCommand::new("set_button", CMD_SET_BUTTON),
        BotCommand::new("status", CMD_STATUS),
        BotCommand::new("settings", CMD_SETTINGS),
        BotCommand::new("ban", CMD_BAN),
    ];
    state
        .telegram
        .client()
        .set_my_commands(admin_commands)
        .scope(BotCommandScope::AllChatAdministrators)
        .await
        .map(|_| ())
}
