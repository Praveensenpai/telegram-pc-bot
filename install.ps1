<#
.SYNOPSIS
    One-line installer for the Telegram PC control bot on Windows.

.DESCRIPTION
    Downloads the latest release, installs it to %LOCALAPPDATA%\telegram-pc-bot,
    and launches the interactive setup wizard. The wizard asks for the bot token,
    auto-detects who has messaged the bot, and registers autostart at boot.

.EXAMPLE
    irm https://raw.githubusercontent.com/Praveensenpai/telegram-pc-bot/main/install.ps1 | iex
#>

[CmdletBinding()]
param(
    [string]$Repo = 'Praveensenpai/telegram-pc-bot',
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'telegram-pc-bot')
)

$ErrorActionPreference = 'Stop'

$asset = 'telegram-pc-bot-windows-x86_64.exe'
$url = "https://github.com/$Repo/releases/latest/download/$asset"
$target = Join-Path $InstallDir 'telegram-pc-bot.exe'

Write-Host '[*] Installing Telegram PC Control Bot...' -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Write-Host "[*] Downloading $asset..."
Invoke-WebRequest -Uri $url -OutFile $target -UseBasicParsing

Write-Host "[OK] Installed to $target"

# Hand control to the interactive setup wizard, which persists the config and
# registers the boot task.
& $target --setup

Write-Host ''
Write-Host '[OK] Done. The bot will start automatically at boot.' -ForegroundColor Green