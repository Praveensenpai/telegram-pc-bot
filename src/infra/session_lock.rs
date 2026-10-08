//! Session-aware workstation lock for Windows.
//!
//! The boot task runs as `SYSTEM` in session 0, where a plain
//! `LockWorkStation` call only locks the non-interactive services session. To
//! actually lock the logged-in user we obtain the token of the active console
//! session and launch `rundll32 user32.dll,LockWorkStation` inside it with
//! `CreateProcessAsUserW`.

use std::ffi::c_void;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
use windows_sys::Win32::System::RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken};
use windows_sys::Win32::System::Threading::{
    CreateProcessAsUserW, CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, PROCESS_INFORMATION,
    STARTUPINFOW,
};

/// Lock the workstation of the active console session.
///
/// # Errors
/// Returns a human readable reason when there is no active session or any
/// underlying Win32 call fails.
pub fn lock() -> Result<(), String> {
    // SAFETY: this is the documented service-to-session launch sequence. Every
    // handle is released on all paths before returning.
    unsafe {
        let session_id = WTSGetActiveConsoleSessionId();
        if session_id == u32::MAX {
            return Err("no active console session".to_owned());
        }

        let mut token: HANDLE = std::ptr::null_mut();
        if WTSQueryUserToken(session_id, &mut token) == 0 {
            return Err("could not obtain the session user token".to_owned());
        }

        let mut env: *mut c_void = std::ptr::null_mut();
        let have_env = CreateEnvironmentBlock(&mut env, token, 0) != 0;

        // `CreateProcessAsUserW` may modify the command line, so hand it a
        // writable, null-terminated UTF-16 buffer.
        let mut command: Vec<u16> = "rundll32.exe user32.dll,LockWorkStation\0"
            .encode_utf16()
            .collect();

        let mut startup: STARTUPINFOW = std::mem::zeroed();
        startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut info: PROCESS_INFORMATION = std::mem::zeroed();

        let created = CreateProcessAsUserW(
            token,
            std::ptr::null(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
            if have_env { env } else { std::ptr::null() },
            std::ptr::null(),
            &startup,
            &mut info,
        );

        if have_env {
            DestroyEnvironmentBlock(env);
        }
        if !info.hProcess.is_null() {
            CloseHandle(info.hProcess);
        }
        if !info.hThread.is_null() {
            CloseHandle(info.hThread);
        }
        CloseHandle(token);

        if created == 0 {
            return Err("failed to launch the lock command in the session".to_owned());
        }
        Ok(())
    }
}
