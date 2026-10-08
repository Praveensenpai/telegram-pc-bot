//! Interactive first-run setup wizard.
//!
//! Guides the user through entering a bot token, auto-detects everyone who has
//! messaged the bot, lets the user pick which of them are authorized (or type
//! ids manually) and persists the configuration. Launching the background task
//! is handled by `main` once the wizard returns.

use std::collections::BTreeMap;

use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Password};
use teloxide::prelude::*;
use teloxide::types::Update;

use crate::config::{parse_user_ids, Config};
use crate::error::{AppError, AppResult};

/// A Telegram user detected from recent bot updates.
#[derive(Debug, Clone)]
struct DetectedUser {
    id: u64,
    label: String,
}

/// Run the full setup wizard.
///
/// # Errors
/// Returns a typed error when input, token validation or config persistence
/// fails.
pub async fn run() -> AppResult<()> {
    let theme = ColorfulTheme::default();
    print_banner();

    let token = prompt_token(&theme)?;
    let bot = Bot::new(token.clone());

    let username = validate_token(&bot).await?;
    println!("\n✅ Token valid — connected to @{username}.\n");

    let detected = detect_users(&bot).await?;
    let allowed_ids = select_users(&theme, &detected)?;

    let config = Config {
        bot_token: token,
        allowed_user_ids: allowed_ids,
    };
    config.save()?;
    println!("\n💾 Configuration saved to {}", Config::path().display());

    Ok(())
}

/// Print the wizard banner.
fn print_banner() {
    println!();
    println!("╭──────────────────────────────────────────────╮");
    println!("│   Telegram PC Control Bot — First-run setup  │");
    println!("╰──────────────────────────────────────────────╯");
    println!();
}

/// Ask for the bot token (hidden input).
fn prompt_token(theme: &ColorfulTheme) -> AppResult<String> {
    let token = Password::with_theme(theme)
        .with_prompt("Paste your bot token from @BotFather")
        .interact()?;
    let token = token.trim().to_owned();
    if token.is_empty() {
        return Err(AppError::Input("bot token cannot be empty".to_owned()));
    }
    Ok(token)
}

/// Validate the token and return the bot username.
async fn validate_token(bot: &Bot) -> AppResult<String> {
    let me = bot
        .get_me()
        .await
        .map_err(|error| AppError::Input(format!("Telegram rejected the token: {error}")))?;
    Ok(me.username().to_owned())
}

/// Poll recent updates and collect everyone who messaged the bot.
async fn detect_users(bot: &Bot) -> AppResult<Vec<DetectedUser>> {
    println!("🔍 Looking for people who have messaged the bot…");

    let updates: Vec<Update> = bot.get_updates().await?;
    let mut users: BTreeMap<u64, String> = BTreeMap::new();

    for update in &updates {
        if let Some(user) = update.from() {
            users.entry(user.id.0).or_insert_with(|| format_user(user));
        }
    }

    if users.is_empty() {
        println!(
            "   No recent messages found. Ask the person to send /start to the bot,\n   then re-run setup."
        );
    } else {
        println!("   Found {} sender(s).", users.len());
    }

    Ok(users
        .into_iter()
        .map(|(id, label)| DetectedUser { id, label })
        .collect())
}

/// Build a readable label for a detected user.
fn format_user(user: &teloxide::types::User) -> String {
    let name = user.full_name();
    match user.username.as_deref() {
        Some(username) => format!("{name} (@{username})"),
        None => name,
    }
}

/// Let the user choose which detected users are authorized.
///
/// Always offers a "enter ids manually" path and merges both selections.
fn select_users(theme: &ColorfulTheme, detected: &[DetectedUser]) -> AppResult<Vec<u64>> {
    let mut selected: Vec<u64> = Vec::new();

    if !detected.is_empty() {
        let labels: Vec<String> = detected
            .iter()
            .map(|user| format!("{} — id {}", user.label, user.id))
            .collect();
        let defaults = vec![true; labels.len()];
        let picks = MultiSelect::with_theme(theme)
            .with_prompt("Select the accounts allowed to control this PC (space to toggle)")
            .items(&labels)
            .defaults(&defaults)
            .interact()?;
        selected.extend(picks.into_iter().map(|index| detected[index].id));
    }

    let manual = Confirm::with_theme(theme)
        .with_prompt("Add another Telegram id manually?")
        .default(false)
        .interact()?;
    if manual {
        selected.extend(prompt_manual_ids(theme)?);
    }

    selected.sort_unstable();
    selected.dedup();
    if selected.is_empty() {
        return Err(AppError::Input(
            "at least one authorized user id is required".to_owned(),
        ));
    }
    Ok(selected)
}

/// Prompt for a comma/semicolon separated list of ids.
fn prompt_manual_ids(theme: &ColorfulTheme) -> AppResult<Vec<u64>> {
    let raw: String = Input::with_theme(theme)
        .with_prompt("Enter id(s), separated by commas")
        .interact_text()?;
    parse_user_ids(&raw)
}
