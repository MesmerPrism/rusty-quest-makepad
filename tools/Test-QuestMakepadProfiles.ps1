$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Name,
        [Parameter(Mandatory=$true)]
        [string]$File,
        [string[]]$Arguments = @(),
        [string]$WorkingDirectory = (Get-Location).Path
    )

    Push-Location $WorkingDirectory
    try {
        & $File @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "$Name failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$MakepadRoot = if ($env:RUSTY_MAKEPAD_ROOT) { Resolve-Path $env:RUSTY_MAKEPAD_ROOT } else { Resolve-Path (Join-Path $RepoRoot "..\rusty-makepad") }
$QuestRoot = if ($env:RUSTY_QUEST_ROOT) { Resolve-Path $env:RUSTY_QUEST_ROOT } else { Resolve-Path (Join-Path $RepoRoot "..\rusty-quest") }

$surface = Join-Path $RepoRoot "fixtures\settings\quest-makepad-camera-shell.settings.json"
$settingsProfile = Join-Path $RepoRoot "fixtures\profiles\mesh-replay.settings-profile.json"
$questProfile = Join-Path $RepoRoot "fixtures\profiles\mesh-replay.quest-runtime-profile.json"
$effectiveOut = Join-Path $RepoRoot "local-artifacts\quest-makepad-effective-settings.json"

New-Item -ItemType Directory -Path (Join-Path $RepoRoot "local-artifacts") -Force | Out-Null

Invoke-Checked "Quest Makepad settings surface" "cargo" @("run", "-p", "rusty-makepad-settings-cli", "--", "validate-surface", "--surface", $surface, "--profile", $settingsProfile) -WorkingDirectory $MakepadRoot
Invoke-Checked "Quest Makepad effective settings" "cargo" @("run", "-p", "rusty-makepad-settings-cli", "--", "resolve", "--surface", $surface, "--profile", $settingsProfile, "--out", $effectiveOut) -WorkingDirectory $MakepadRoot
Invoke-Checked "Quest Makepad runtime profile dry-run" "powershell" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "tools\Apply-RuntimeProfile.ps1", "-ProfilePath", $questProfile, "-DryRun", "-Out", "local-artifacts\quest-makepad-property-write-plan.json") -WorkingDirectory $QuestRoot

