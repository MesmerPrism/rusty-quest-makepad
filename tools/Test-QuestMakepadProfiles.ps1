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

function Test-RuntimeBundle {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Name,
        [Parameter(Mandatory=$true)]
        [string]$BundlePath,
        [Parameter(Mandatory=$true)]
        [string]$OutDir
    )

    Invoke-Checked $Name "powershell" @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", "tools\Build-QuestMakepadRuntimeBundle.ps1",
        "-BundlePath", $BundlePath,
        "-OutDir", $OutDir
    )

    $reportPath = Join-Path $RepoRoot (Join-Path $OutDir "runtime-bundle-report.json")
    $report = Get-Content -Path $reportPath -Raw | ConvertFrom-Json
    if ($report.schema -ne "rusty.quest.makepad.runtime_bundle_report.v1") {
        throw "$Name runtime bundle report schema mismatch: $($report.schema)"
    }
    if ($report.boundary.legacy_reference_source_used -ne $false) {
        throw "$Name runtime bundle unexpectedly used a legacy reference source"
    }
    if ($report.boundary.device_write_performed -ne $false) {
        throw "$Name runtime bundle test must remain dry-run only"
    }
    if ($report.property_write_plan.set_count -lt 1) {
        throw "$Name runtime bundle did not produce set operations"
    }
}

Test-RuntimeBundle `
    -Name "Quest Makepad smoke runtime bundle" `
    -BundlePath "fixtures\profiles\mesh-replay.bundle.json" `
    -OutDir "local-artifacts\quest-makepad-runtime-bundle"

Test-RuntimeBundle `
    -Name "Quest Makepad recorded-left runtime bundle" `
    -BundlePath "fixtures\profiles\mesh-replay-recorded-left.bundle.json" `
    -OutDir "local-artifacts\quest-makepad-runtime-bundle-recorded-left"

Test-RuntimeBundle `
    -Name "Quest Makepad recorded-left-particles runtime bundle" `
    -BundlePath "fixtures\profiles\mesh-replay-recorded-left-particles.bundle.json" `
    -OutDir "local-artifacts\quest-makepad-runtime-bundle-recorded-left-particles"

Test-RuntimeBundle `
    -Name "Quest Makepad remote-camera Q2Q runtime bundle" `
    -BundlePath "fixtures\profiles\remote-camera-q2q.bundle.json" `
    -OutDir "local-artifacts\quest-makepad-runtime-bundle-remote-camera-q2q"

Test-RuntimeBundle `
    -Name "Quest Makepad stimulus interference runtime bundle" `
    -BundlePath "fixtures\profiles\stimulus-interference.bundle.json" `
    -OutDir "local-artifacts\quest-makepad-runtime-bundle-stimulus-interference"
