//! Entrypoint for the Telegram PC control bot.
//!
//! On first run (or with `--setup`) an interactive wizard collects the bot
//! token and authorized users, persists them, and registers a boot-time task.
//! Subsequent launches (including the auto-started one) read the stored config
//! and run as a daemon.

mod api;
mod cli;
mod config;
mod domain;
mod error;
mod infra;

use std::sync::Arc;

use teloxide::dptree;
use teloxide::prelude::*;
use teloxide::types::Update;
use tracing_subscriber::EnvFilter;

use crate::api::telegram;
use crate::cli::Mode;
use crate::config::Config;
use crate::error::AppResult;
use crate::infra::autostart;

#[tokio::main]
async fn main() -> AppResult<()> {
    init_tracing();

    match Mode::parse(std::env::args().skip(1)) {
        Mode::Uninstall => uninstall().await,
        Mode::Setup => {
            cli::setup::run().await?;
            run_daemon().await
        }
        Mode::Run => {
            if !Config::exists() {
                cli::setup::run().await?;
            }
            run_daemon().await
        }
    }
}

/// Load the stored configuration and serve updates until interrupted.
async fn run_daemon() -> AppResult<()> {
    let config = Arc::new(Config::load()?);
    let bot = Bot::new(config.bot_token());

    // Validate the token before building the dispatcher, so a bad token
    // produces a typed error instead of a panic inside teloxide.
    let me = bot.get_me().await?;
    tracing::info!(
        "Bot online as @{}. Whitelist contains {} user(s).",
        me.username(),
        config.allowed_user_count()
    );

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(telegram::handle_message))
        .branch(Update::filter_callback_query().endpoint(telegram::handle_callback));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

/// Remove the boot task and the stored configuration.
async fn uninstall() -> AppResult<()> {
    println!("Removing boot task and stored configuration…");
    if let Err(error) = autostart::uninstall().await {
        println!("⚠️  {error}");
    }
    Config::remove()?;
    println!("✅ Uninstalled. Delete the executable to finish.");
    Ok(())
}

/// Configure structured logging, honouring `RUST_LOG`.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
