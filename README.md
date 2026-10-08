# Telegram PC Control Bot

A Telegram bot that lets an authorized user control a Windows PC remotely:
**reboot, shutdown, cancel a pending shutdown, lock, suspend and hibernate**,
plus a `/status` report (uptime, CPU load, memory, battery).

Written in Rust with [`teloxide`](https://github.com/teloxide/teloxide).

## Install (Windows) — one line

Open **PowerShell as Administrator** and run:

```powershell
irm https://raw.githubusercontent.com/Praveensenpai/telegram-pc-bot/main/install.ps1 | iex
```

> [!IMPORTANT]
> Administrator rights are required so the bot can run `shutdown`.

## First-run setup

There is **no `.env` to edit** — the bot asks for everything the first time.

1. Paste your **bot token** from [@BotFather](https://t.me/BotFather) (input is
   hidden). It is validated live against Telegram.
2. Ask the person who should control the PC to send **`/start`** to your bot.
3. The wizard **auto-detects everyone who has messaged the bot** and shows a
   checklist — toggle the accounts you want to authorize (all are pre-selected).
4. Not in the list? Choose **"Add another Telegram id manually"** and type the
   numeric id (get it from [@userinfobot](https://t.me/userinfobot)).

That's it. The wizard exits, and the bot is **launched in the background** — your
terminal is free, and the boot task is registered automatically. You'll see:

```
✅ The bot is running in the background and will start automatically at boot.
   Logs   : %ProgramData%\telegram-pc-bot\bot.log
   Stop it: run this program with --uninstall
```

The config is saved to `%ProgramData%\telegram-pc-bot\config.json`, so the
auto-started task and your user account read the same file. If you want to
reconfigure, run the installer again and pass `--setup`.

## Commands

| Command | Description |
| :--- | :--- |
| `/start`, `/help` | Show help |
| `/status` | Uptime, CPU load, memory, battery |
| `/reboot [seconds]` | Restart the PC |
| `/shutdown [seconds]` | Power the PC off |
| `/cancel` | Abort a pending shutdown/restart |
| `/lock` | Lock the session |
| `/suspend` | Suspend the PC |
| `/hibernate` | Hibernate the PC |

Destructive actions (`reboot`, `shutdown`, `suspend`, `hibernate`) show a
**Confirm / Cancel** keyboard first.

## Features

- **Interactive setup wizard** — no config files to hand-edit.
- **Auto-detects message senders** — pick authorized accounts from a checklist,
  or enter ids manually.
- **Runs detached in the background** and **auto-starts at boot** (Windows `ONSTART` task as
  `SYSTEM`, highest privileges).
- Numeric **user-ID allowlist** — every update from a stranger is dropped.
- **Confirm / Cancel inline keyboard** before any destructive action.
- **Delay clamping** (0 – 86 400 s, default 10 s) so a typo can't schedule a
  reboot a week out.
- **No panics**: bad config or an invalid token produces a typed error with a
  readable message and a non-zero exit code.
- **Atomic config writes** — a crash mid-save can never corrupt `config.json`.
- **Hardened token file** — the config ACL is restricted to `SYSTEM` and
  `Administrators` on Windows.
- **Rotating logs** — the daemon writes a new `bot.log` daily instead of growing
  without bound.
- Runs silently (no console window flash).

## Notes

- `/lock` is **session-aware**: even though the boot task runs as `SYSTEM`
  (session 0), the bot resolves the active console session and locks *that*
  desktop via `CreateProcessAsUserW`. If no user is logged in it reports so
  instead of silently doing nothing.
- Re-running setup automatically stops a running instance first, so the wizard's
  `get_updates` call never collides with the live poller.

## Building from source

Requires [Rust](https://rustup.rs) (stable).

```bash
cargo build --release
```

Then run the binary with no arguments — the setup wizard starts automatically:

```bat
target\release\telegram-pc-bot.exe
```

## CLI

| Flag | Description |
| :--- | :--- |
| *(none)* | Set up if needed, then launch the bot **in the background** |
| `--setup` | Force the interactive setup wizard, then launch in the background |
| `--foreground` | Run attached to this terminal (for debugging) |
| `--daemon` | Detached mode used by the boot task (logs to `bot.log`) |
| `--uninstall` | Stop and remove the background task + stored configuration |
| `-h`, `--help` | Show usage |
| `-V`, `--version` | Show the version |

Once launched, the bot keeps running **detached from your terminal** and
starts automatically at every boot. Its logs live next to the config in
`bot.log`.

## Project layout

```
src/
├── main.rs              Entrypoint: wizard / daemon / uninstall
├── error.rs             Centralized AppError enum
├── config.rs            JSON config load/save/validate + user-id parsing
├── cli.rs               Argument parsing, setup module declaration
├── cli/
│   └── setup.rs         Interactive wizard (token, auto-detect, autostart)
├── domain/              Pure logic (no I/O)
│   ├── models.rs        PowerAction, ActionResult
│   └── command.rs       Delay/callback parsing, HTML escaping
├── infra/               OS interaction
│   ├── power.rs         shutdown / rundll32 wrappers
│   ├── session_lock.rs  Session-aware lock via CreateProcessAsUserW (Windows)
│   ├── status.rs        PowerShell status query
│   └── autostart.rs     Windows scheduled-task install/uninstall
└── api/
    └── telegram.rs      Handlers, authorization guard, keyboards
```

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

## Releases

Pushing a `v*` tag triggers [`.github/workflows/release.yml`](.github/workflows/release.yml),
which builds the **Windows x86_64** binary, attaches it to a GitHub release, and
publishes `checksums.txt`. The one-line installer always pulls the latest asset.

## Security

The bot can power the machine on and off. Treat the token like a password:

- Only whitelist accounts you control.
- The config lives in `%ProgramData%\telegram-pc-bot\config.json` — restrict who
  can read it.
- Revoke the token via @BotFather if it ever leaks.

## License

MIT