use crate::domain::models::PowerAction;

/// Delay applied when the user does not supply one.
pub const DEFAULT_DELAY_SECONDS: u32 = 10;
/// Upper bound for any user supplied delay (24 hours).
pub const MAX_DELAY_SECONDS: u32 = 86_400;
/// Prefix used for every inline keyboard callback payload.
pub const CALLBACK_PREFIX: &str = "act";

/// A decoded inline keyboard callback request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackRequest {
    /// The user confirmed an action with the given delay.
    Confirm {
        /// The action to run.
        action: PowerAction,
        /// Delay in seconds before the action takes effect.
        delay: u32,
    },
    /// The user dismissed the confirmation prompt.
    Cancel,
}

/// Parse an optional delay argument, clamping it to a safe range.
///
/// An empty or malformed value falls back to [`DEFAULT_DELAY_SECONDS`].
#[must_use]
pub fn parse_delay(raw: &str) -> u32 {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return DEFAULT_DELAY_SECONDS;
    }
    match trimmed.parse::<i64>() {
        Ok(value) => value.clamp(0, i64::from(MAX_DELAY_SECONDS)) as u32,
        Err(_) => DEFAULT_DELAY_SECONDS,
    }
}

/// Build the callback payload for a confirmation button.
#[must_use]
pub fn confirm_payload(action: PowerAction, delay: u32) -> String {
    format!("{CALLBACK_PREFIX}:{}:{delay}", action.callback_id())
}

/// Build the callback payload for the cancel button.
#[must_use]
pub fn cancel_payload() -> String {
    format!("{CALLBACK_PREFIX}:cancel:0")
}

/// Decode an inline keyboard callback payload.
#[must_use]
pub fn parse_callback(data: &str) -> Option<CallbackRequest> {
    let mut parts = data.split(':');
    let prefix = parts.next()?;
    let action = parts.next()?;
    let delay = parts.next()?;
    if prefix != CALLBACK_PREFIX || parts.next().is_some() {
        return None;
    }
    if action == "cancel" {
        return Some(CallbackRequest::Cancel);
    }
    let action = PowerAction::from_callback_id(action)?;
    // Clamp here too: never trust a payload to carry an unbounded delay.
    let delay = delay.parse::<u32>().ok()?.min(MAX_DELAY_SECONDS);
    Some(CallbackRequest::Confirm { action, delay })
}

/// Escape text for safe inclusion in an HTML formatted Telegram message.
#[must_use]
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_delay_uses_default() {
        assert_eq!(parse_delay(""), DEFAULT_DELAY_SECONDS);
    }

    #[test]
    fn explicit_delay_is_kept() {
        assert_eq!(parse_delay("30"), 30);
    }

    #[test]
    fn negative_delay_clamps_to_zero() {
        assert_eq!(parse_delay("-5"), 0);
    }

    #[test]
    fn huge_delay_clamps_to_max() {
        assert_eq!(parse_delay("999999"), MAX_DELAY_SECONDS);
    }

    #[test]
    fn garbage_delay_uses_default() {
        assert_eq!(parse_delay("abc"), DEFAULT_DELAY_SECONDS);
    }

    #[test]
    fn callback_round_trips() {
        let payload = confirm_payload(PowerAction::Reboot, 15);
        assert_eq!(
            parse_callback(&payload),
            Some(CallbackRequest::Confirm {
                action: PowerAction::Reboot,
                delay: 15,
            })
        );
    }

    #[test]
    fn cancel_payload_decodes() {
        assert_eq!(
            parse_callback(&cancel_payload()),
            Some(CallbackRequest::Cancel)
        );
    }

    #[test]
    fn callback_delay_is_clamped() {
        assert_eq!(
            parse_callback("act:reboot:4294967295"),
            Some(CallbackRequest::Confirm {
                action: PowerAction::Reboot,
                delay: MAX_DELAY_SECONDS,
            })
        );
    }

    #[test]
    fn malformed_payload_is_none() {
        assert_eq!(parse_callback("act:reboot"), None);
        assert_eq!(parse_callback("wrong:reboot:5"), None);
        assert_eq!(parse_callback("act:unknown:5"), None);
        assert_eq!(parse_callback("act:reboot:x"), None);
    }

    #[test]
    fn html_is_escaped() {
        assert_eq!(escape_html("<b>a & b</b>"), "&lt;b&gt;a &amp; b&lt;/b&gt;");
    }
}
