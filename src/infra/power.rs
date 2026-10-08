use tokio::process::Command;

use crate::domain::models::{ActionResult, PowerAction};

/// Hides the flashing console window when spawning processes on Windows.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Execute a power action on the host, returning its outcome.
pub async fn execute(action: PowerAction, delay: u32) -> ActionResult {
    match action {
        PowerAction::Reboot => reboot(delay).await,
        PowerAction::Shutdown => power_off(delay).await,
        PowerAction::Suspend => suspend().await,
        PowerAction::Hibernate => hibernate().await,
        PowerAction::Lock => lock_workstation().await,
    }
}

/// Restart the machine after `delay` seconds.
pub async fn reboot(delay: u32) -> ActionResult {
    let delay = delay.to_string();
    run(
        "shutdown",
        &["/r", "/t", delay.as_str(), "/f", "/c", "System"],
    )
    .await
}

/// Power the machine off after `delay` seconds.
pub async fn power_off(delay: u32) -> ActionResult {
    let delay = delay.to_string();
    run(
        "shutdown",
        &["/s", "/t", delay.as_str(), "/f", "/c", "System"],
    )
    .await
}

/// Cancel a pending shutdown or restart.
pub async fn abort_shutdown() -> ActionResult {
    run("shutdown", &["/a"]).await
}

/// Lock the current session.
pub async fn lock_workstation() -> ActionResult {
    run("rundll32.exe", &["user32.dll,LockWorkStation"]).await
}

/// Suspend the machine (hibernates instead when hibernation is enabled).
pub async fn suspend() -> ActionResult {
    run("rundll32.exe", &["powrprof.dll,SetSuspendState", "0,1,0"]).await
}

/// Hibernate the machine; requires hibernation to be enabled.
pub async fn hibernate() -> ActionResult {
    run("shutdown", &["/h"]).await
}

/// Spawn a host command and capture its output without ever panicking.
async fn run(program: &str, args: &[&str]) -> ActionResult {
    let mut command = Command::new(program);
    command.args(args);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    match command.output().await {
        Ok(output) => {
            let mut text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if text.is_empty() {
                text = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            }
            if !output.status.success() && text.is_empty() {
                text = format!("command exited with status {}", output.status);
            }
            ActionResult {
                ok: output.status.success(),
                output: text,
            }
        }
        Err(error) => ActionResult::failure(error.to_string()),
    }
}
