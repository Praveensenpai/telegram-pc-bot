use crate::error::{AppError, AppResult};

/// Name of the Windows scheduled task that starts the bot at boot.
pub const TASK_NAME: &str = "TelegramPcBot";

/// Register the running executable as a boot-time task.
///
/// On Windows this creates an `ONSTART` scheduled task running as `SYSTEM`
/// with the highest privileges (required by `shutdown`) and starts it
/// immediately. On other platforms this returns a typed error so the wizard
/// can report that autostart is unavailable.
///
/// # Errors
/// Returns [`AppError::Autostart`] when the executable cannot be located or
/// the task cannot be registered.
pub async fn install() -> AppResult<()> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe()
            .map_err(|error| AppError::Autostart(format!("cannot locate executable: {error}")))?;
        let exe = exe.to_string_lossy().to_string();
        let program = format!("\"{exe}\"");

        run_schtasks(&[
            "/Create",
            "/TN",
            TASK_NAME,
            "/TR",
            program.as_str(),
            "/SC",
            "ONSTART",
            "/RU",
            "SYSTEM",
            "/RL",
            "HIGHEST",
            "/F",
        ])
        .await?;
        // Start it now so the user does not have to reboot.
        run_schtasks(&["/Run", "/TN", TASK_NAME]).await?;
        Ok(())
    }

    #[cfg(not(windows))]
    {
        Err(AppError::Autostart(
            "autostart is only supported on Windows".to_owned(),
        ))
    }
}

/// Remove the boot-time task if present.
///
/// # Errors
/// Returns [`AppError::Autostart`] when the task cannot be removed.
pub async fn uninstall() -> AppResult<()> {
    #[cfg(windows)]
    {
        run_schtasks(&["/Delete", "/TN", TASK_NAME, "/F"]).await
    }

    #[cfg(not(windows))]
    {
        Err(AppError::Autostart(
            "autostart is only supported on Windows".to_owned(),
        ))
    }
}

/// Whether the boot-time task is currently registered.
#[must_use]
pub async fn is_installed() -> bool {
    #[cfg(windows)]
    {
        run_schtasks(&["/Query", "/TN", TASK_NAME]).await.is_ok()
    }

    #[cfg(not(windows))]
    {
        false
    }
}

/// Run `schtasks` with the given arguments, mapping a non-zero exit to an
/// [`AppError::Autostart`].
#[cfg(windows)]
async fn run_schtasks(args: &[&str]) -> AppResult<()> {
    use std::process::Stdio;

    use tokio::process::Command;

    /// Hides the flashing console window when spawning processes on Windows.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut command = Command::new("schtasks");
    command.args(args).stdin(Stdio::null());
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command
        .output()
        .await
        .map_err(|error| AppError::Autostart(format!("failed to run schtasks: {error}")))?;

    if output.status.success() {
        return Ok(());
    }

    let mut detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if detail.is_empty() {
        detail = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    }
    Err(AppError::Autostart(if detail.is_empty() {
        format!("schtasks exited with status {}", output.status)
    } else {
        detail
    }))
}
