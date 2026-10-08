/// Command-line handling and the interactive setup wizard.
pub mod setup;

/// A parsed command-line invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Run the bot (using an existing config, or setup first if none exists).
    Run,
    /// Force the interactive setup wizard, then run.
    Setup,
    /// Remove the boot task and the stored configuration.
    Uninstall,
}

impl Mode {
    /// Parse the process arguments into a [`Mode`].
    #[must_use]
    pub fn parse<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for arg in args {
            match arg.as_ref() {
                "--setup" | "setup" => return Self::Setup,
                "--uninstall" | "uninstall" => return Self::Uninstall,
                _ => {}
            }
        }
        Self::Run
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_run() {
        assert_eq!(Mode::parse(["bot"]), Mode::Run);
    }

    #[test]
    fn detects_setup() {
        assert_eq!(Mode::parse(["bot", "--setup"]), Mode::Setup);
    }

    #[test]
    fn detects_uninstall() {
        assert_eq!(Mode::parse(["bot", "uninstall"]), Mode::Uninstall);
    }
}
