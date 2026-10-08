use tokio::process::Command;

use crate::domain::models::ActionResult;

/// Hides the flashing console window when spawning processes on Windows.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// PowerShell script collecting a short host status report.
const STATUS_SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
$os = Get-CimInstance Win32_OperatingSystem
$uptime = (Get-Date) - $os.LastBootUpTime
$cpu = (Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average).Average
$totalMb = [math]::Round($os.TotalVisibleMemorySize / 1KB, 1)
$freeMb = [math]::Round($os.FreePhysicalMemory / 1KB, 1)
$usedMb = [math]::Round($totalMb - $freeMb, 1)
$usedPct = if ($totalMb -gt 0) { [math]::Round(($usedMb / $totalMb) * 100, 0) } else { 0 }
$battery = Get-CimInstance Win32_Battery | Select-Object -First 1
$lines = @(
  "Host     : $env:COMPUTERNAME"
  "User     : $env:USERNAME"
  "OS       : $($os.Caption) ($($os.Version))"
  "Uptime   : $($uptime.Days)d $($uptime.Hours)h $($uptime.Minutes)m"
  "CPU load : $cpu%"
  "Memory   : $usedMb MB / $totalMb MB ($usedPct%)"
)
if ($battery) { $lines += "Battery  : $($battery.EstimatedChargeRemaining)%" }
$lines -join "`n"
"#;

/// Collect a short status report from the host.
pub async fn system_status() -> ActionResult {
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", STATUS_SCRIPT]);

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
