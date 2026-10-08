//! Entrypoint for the Telegram PC control bot.
//!
//! The default invocation performs setup on first run and then launches the
//! bot as a **detached background task** that also starts at every boot. The
//! scheduled task itself invokes the binary with `--daemon` (no console), while
//! `--foreground` runs it attached to the current terminal for debugging.

mod api;
mod cli;
mod config;
mod domain;
mod error;
mod infra;

use std::process::ExitCode;
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
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("telegram-pc-bot {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let mode = Mode::parse(&args);
    init_tracing(mode);

    match run(mode).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // `Display` yields the friendly `#[error(...)]` message instead of
            // the `Debug` representation the default `Result` main would print.
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Dispatch the selected [`Mode`].
async fn run(mode: Mode) -> AppResult<()> {
    match mode {
        Mode::Uninstall => uninstall().await,
        Mode::Setup => {
            cli::setup::run().await?;
            launch_background().await
        }
        Mode::Daemon => run_daemon().await,
        Mode::Foreground => {
            if !Config::exists() {
                cli::setup::run().await?;
            }
            run_daemon().await
        }
        Mode::Run => {
            if !Config::exists() {
                cli::setup::run().await?;
            }
            launch_background().await
        }
    }
}

/// Print the command-line help text.
fn print_help() {
    println!(
        "telegram-pc-bot {}\n\n\
        Usage: telegram-pc-bot [OPTION]\n\n\
        With no option, runs setup on first launch and then keeps the bot\n\
        running as a detached background task that starts at every boot.\n\n\
        Options:\n\
          --setup       force the interactive setup wizard\n\
          --foreground  run attached to this terminal (debugging)\n\
          --daemon      run detached (used by the boot task)\n\
          --uninstall   stop the task and delete the stored config\n\
          -h, --help    show this help\n\
          -V, --version show the version",
        env!("CARGO_PKG_VERSION")
    );
}

/// Ensure the boot task is registered and start it in the background.
async fn launch_background() -> AppResult<()> {
    if !autostart::is_installed().await {
        autostart::install().await?;
    }
    // Restart so config changes take effect and only one instance runs.
    let _ = autostart::stop().await;
    autostart::start().await?;

    let log = Config::dir().join("bot.log");
    println!("\n✅ The bot is running in the background and will start automatically at boot.");
    println!("   Logs   : {}", log.display());
    println!("   Stop it: run this program with --uninstall");
    Ok(())
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

/// Stop and remove the boot task, then delete the stored configuration.
async fn uninstall() -> AppResult<()> {
    println!("Stopping and removing the background task…");
    if let Err(error) = autostart::uninstall().await {
        println!("⚠️  {error}");
    }
    Config::remove()?;
    println!("✅ Uninstalled. Delete the executable to finish.");
    Ok(())
}

/// Configure structured logging.
///
/// The detached daemon has no console, so it logs to `<config dir>/bot.log`.
/// Every other mode logs to the terminal, honouring `RUST_LOG`.
fn init_tracing(mode: Mode) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if matches!(mode, Mode::Daemon) {
        let dir = Config::dir();
        let _ = std::fs::create_dir_all(&dir);
        let appender = tracing_appender::rolling::daily(&dir, "bot.log");
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(appender)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}
