//! Telegram bot handlers: authorization guard, commands and callbacks.

use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

use crate::config::Config;
use crate::domain::command::{
    cancel_payload, confirm_payload, escape_html, parse_callback, parse_delay, CallbackRequest,
};
use crate::domain::models::{ActionResult, PowerAction};
use crate::infra::{power, status};

/// Result alias used by every handler endpoint.
type HandlerResult = ResponseResult<()>;

/// Help text shown for `/start` and `/help`.
const HELP_TEXT: &str = "\
<b>PC control bot</b>

/status – uptime, CPU and memory
/reboot [seconds] – restart the PC
/shutdown [seconds] – power the PC off
/cancel – abort a pending shutdown/restart
/lock – lock the session
/suspend – suspend the PC
/hibernate – hibernate the PC

If no delay is given, 10s is used.";

/// Handle every incoming text message.
pub async fn handle_message(bot: Bot, msg: Message, config: Arc<Config>) -> HandlerResult {
    if !authorized(msg.from.as_ref().map(|user| user.id.0), &config) {
        return Ok(());
    }
    let Some(text) = msg.text() else {
        return Ok(());
    };
    let (command, argument) = split_command(text);
    match command.as_str() {
        "/start" | "/help" => {
            bot.send_message(msg.chat.id, HELP_TEXT)
                .parse_mode(ParseMode::Html)
                .await?;
        }
        "/status" => send_status(&bot, msg.chat.id).await?,
        "/reboot" => ask_confirmation(&bot, msg.chat.id, PowerAction::Reboot, argument).await?,
        "/shutdown" => {
            ask_confirmation(&bot, msg.chat.id, PowerAction::Shutdown, argument).await?;
        }
        "/suspend" => ask_confirmation(&bot, msg.chat.id, PowerAction::Suspend, argument).await?,
        "/hibernate" => {
            ask_confirmation(&bot, msg.chat.id, PowerAction::Hibernate, argument).await?;
        }
        "/lock" => {
            let result = power::lock_workstation().await;
            reply_result(&bot, msg.chat.id, PowerAction::Lock, &result).await?;
        }
        "/cancel" => {
            let result = power::abort_shutdown().await;
            reply_cancel(&bot, msg.chat.id, &result).await?;
        }
        _ => {}
    }
    Ok(())
}

/// Handle inline keyboard callbacks (confirm / cancel).
pub async fn handle_callback(bot: Bot, query: CallbackQuery, config: Arc<Config>) -> HandlerResult {
    bot.answer_callback_query(query.id.clone()).await?;
    if !authorized(Some(query.from.id.0), &config) {
        return Ok(());
    }
    let Some(data) = query.data.as_deref() else {
        return Ok(());
    };
    let Some(request) = parse_callback(data) else {
        return Ok(());
    };
    let Some(message) = query.regular_message() else {
        return Ok(());
    };
    let chat_id = message.chat.id;
    let message_id = message.id;

    match request {
        CallbackRequest::Cancel => {
            bot.edit_message_text(chat_id, message_id, "Cancelled.")
                .await?;
        }
        CallbackRequest::Confirm { action, delay } => {
            let pending = format!("⏳ Executing <b>{}</b>…", action.label());
            bot.edit_message_text(chat_id, message_id, pending)
                .parse_mode(ParseMode::Html)
                .await?;
            let result = power::execute(action, delay).await;
            bot.edit_message_text(chat_id, message_id, format_result(action, &result))
                .parse_mode(ParseMode::Html)
                .await?;
        }
    }
    Ok(())
}

/// Whether the given Telegram user id may control the host.
fn authorized(user_id: Option<u64>, config: &Config) -> bool {
    user_id.is_some_and(|id| config.is_allowed(id))
}

/// Split a raw message into a command and its trailing argument.
fn split_command(text: &str) -> (String, &str) {
    let trimmed = text.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let command = parts.next().unwrap_or_default();
    // Strip the optional `@botname` suffix Telegram adds in group chats.
    let command = command.split('@').next().unwrap_or_default().to_lowercase();
    let argument = parts.next().unwrap_or_default().trim();
    (command, argument)
}

/// Send the host status report.
async fn send_status(bot: &Bot, chat_id: ChatId) -> HandlerResult {
    let result = status::system_status().await;
    let text = if result.output.is_empty() {
        "⚠️ Could not read system status.".to_owned()
    } else {
        format!("<pre>{}</pre>", escape_html(&result.output))
    };
    bot.send_message(chat_id, text)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

/// Ask the user to confirm a destructive action.
async fn ask_confirmation(
    bot: &Bot,
    chat_id: ChatId,
    action: PowerAction,
    argument: &str,
) -> HandlerResult {
    let delay = parse_delay(argument);
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("✅ Confirm", confirm_payload(action, delay)),
        InlineKeyboardButton::callback("❌ Cancel", cancel_payload()),
    ]]);
    let prompt = format!("Confirm <b>{}</b> (delay {delay}s)?", action.label());
    bot.send_message(chat_id, prompt)
        .reply_markup(keyboard)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

/// Reply with the outcome of a single action.
async fn reply_result(
    bot: &Bot,
    chat_id: ChatId,
    action: PowerAction,
    result: &ActionResult,
) -> HandlerResult {
    bot.send_message(chat_id, format_result(action, result))
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

/// Reply with the outcome of a cancel request.
async fn reply_cancel(bot: &Bot, chat_id: ChatId, result: &ActionResult) -> HandlerResult {
    let icon = if result.ok { "✅" } else { "⚠️" };
    let detail = if result.output.is_empty() {
        "done".to_owned()
    } else {
        escape_html(&result.output)
    };
    let text = format!("{icon} <b>cancel</b>\n<code>{detail}</code>");
    bot.send_message(chat_id, text)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

/// Render an action result as an HTML message.
fn format_result(action: PowerAction, result: &ActionResult) -> String {
    let icon = if result.ok { "✅" } else { "⚠️" };
    let detail = if result.output.is_empty() {
        "done".to_owned()
    } else {
        escape_html(&result.output)
    };
    format!("{icon} <b>{}</b>\n<code>{detail}</code>", action.label())
}
