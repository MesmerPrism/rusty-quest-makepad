$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Name,
        [Parameter(Mandatory=$true)]
        [string]$File,
        [string[]]$Arguments = @()
    )

    & $File @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Push-Location $RepoRoot
try {
    New-Item -ItemType Directory -Path "local-artifacts" -Force | Out-Null
    Invoke-Checked "cargo fmt" "cargo" @("fmt", "--all", "--check")
    Invoke-Checked "cargo test" "cargo" @("test", "--workspace")
    Invoke-Checked "Quest Makepad profiles" "powershell" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "tools\Test-QuestMakepadProfiles.ps1")
    Invoke-Checked "Quest Makepad boundary scan" "python" @("tools\check_quest_makepad_boundaries.py")
} finally {
    Pop-Location
}

