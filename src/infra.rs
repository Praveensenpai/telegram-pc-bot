/// Operating-system interaction: power control, status and autostart.
pub mod autostart;
/// Power control commands (shutdown, reboot, lock, suspend).
pub mod power;
/// Session-aware workstation lock (Windows only).
#[cfg(windows)]
pub mod session_lock;
/// Host status collection via PowerShell.
pub mod status;
