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
New-Item -ItemType Directory -Path (Join-Path $RepoRoot "local-artifacts") -Force | Out-Null

Invoke-Checked "Quest Makepad runtime bundle" "powershell" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "tools\Build-QuestMakepadRuntimeBundle.ps1")

$reportPath = Join-Path $RepoRoot "local-artifacts\quest-makepad-runtime-bundle\runtime-bundle-report.json"
$report = Get-Content -Path $reportPath -Raw | ConvertFrom-Json
if ($report.schema -ne "rusty.quest.makepad.runtime_bundle_report.v1") {
    throw "runtime bundle report schema mismatch: $($report.schema)"
}
if ($report.boundary.legacy_reference_source_used -ne $false) {
    throw "runtime bundle unexpectedly used a legacy reference source"
}
if ($report.boundary.device_write_performed -ne $false) {
    throw "runtime bundle test must remain dry-run only"
}
if ($report.property_write_plan.set_count -lt 1) {
    throw "runtime bundle did not produce set operations"
}
