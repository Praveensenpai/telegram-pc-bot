use std::fmt;

/// A power or session action that can be performed on the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    /// Restart the machine.
    Reboot,
    /// Power the machine off.
    Shutdown,
    /// Suspend the machine.
    Suspend,
    /// Hibernate the machine.
    Hibernate,
    /// Lock the current session.
    Lock,
}

impl PowerAction {
    /// Stable identifier used inside Telegram callback payloads.
    #[must_use]
    pub fn callback_id(self) -> &'static str {
        match self {
            Self::Reboot => "reboot",
            Self::Shutdown => "shutdown",
            Self::Suspend => "suspend",
            Self::Hibernate => "hibernate",
            Self::Lock => "lock",
        }
    }

    /// Human readable label shown to the user.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Reboot => "reboot",
            Self::Shutdown => "shutdown",
            Self::Suspend => "suspend",
            Self::Hibernate => "hibernate",
            Self::Lock => "lock",
        }
    }

    /// Recover an action from its callback identifier.
    #[must_use]
    pub fn from_callback_id(id: &str) -> Option<Self> {
        match id {
            "reboot" => Some(Self::Reboot),
            "shutdown" => Some(Self::Shutdown),
            "suspend" => Some(Self::Suspend),
            "hibernate" => Some(Self::Hibernate),
            "lock" => Some(Self::Lock),
            _ => None,
        }
    }
}

impl fmt::Display for PowerAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Outcome of a host command.
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Whether the command reported success.
    pub ok: bool,
    /// Captured stdout/stderr or an error description.
    pub output: String,
}

impl ActionResult {
    /// Build a failed result.
    #[must_use]
    pub fn failure(output: impl Into<String>) -> Self {
        Self {
            ok: false,
            output: output.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_id_round_trips() {
        for action in [
            PowerAction::Reboot,
            PowerAction::Shutdown,
            PowerAction::Suspend,
            PowerAction::Hibernate,
            PowerAction::Lock,
        ] {
            assert_eq!(
                PowerAction::from_callback_id(action.callback_id()),
                Some(action)
            );
        }
    }

    #[test]
    fn unknown_callback_id_is_none() {
        assert_eq!(PowerAction::from_callback_id("nope"), None);
    }

    #[test]
    fn display_matches_label() {
        assert_eq!(PowerAction::Reboot.to_string(), "reboot");
    }
}
