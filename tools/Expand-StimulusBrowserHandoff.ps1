param(
    [Parameter(Mandatory=$true)]
    [string]$HandoffPath,
    [string]$OutDir = "local-artifacts\quest-makepad-browser-stimulus-handoff"
)

$ErrorActionPreference = "Stop"

function Assert-SafeRelativePayloadPath {
    param([string]$PathValue)

    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        throw "Payload path must not be empty"
    }
    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        throw "Payload path must be relative: $PathValue"
    }
    if ($PathValue.Contains("..")) {
        throw "Payload path must not contain '..': $PathValue"
    }
}

function Get-Sha256HexForText {
    param([string]$Text)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        return (($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString("x2") }) -join "")
    } finally {
        $sha.Dispose()
    }
}

function Write-TextPayload {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Root,
        [Parameter(Mandatory=$true)]
        [string]$RelativePath,
        [Parameter(Mandatory=$true)]
        [string]$Text
    )

    Assert-SafeRelativePayloadPath -PathValue $RelativePath
    $out = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Path (Split-Path -Parent $out) -Force | Out-Null
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($out, $Text, $utf8NoBom)
    return (Resolve-Path $out).Path
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$resolvedHandoff = (Resolve-Path $HandoffPath).Path
$resolvedOutDir = if ([System.IO.Path]::IsPathRooted($OutDir)) {
    $OutDir
} else {
    Join-Path $repoRoot $OutDir
}
New-Item -ItemType Directory -Path $resolvedOutDir -Force | Out-Null
$resolvedOutDir = (Resolve-Path $resolvedOutDir).Path

$handoffText = Get-Content -LiteralPath $resolvedHandoff -Raw
$handoff = $handoffText | ConvertFrom-Json
if ($handoff.schema -ne "rusty.optics.stimulus.quest_handoff.v1") {
    throw "Unsupported stimulus handoff schema: $($handoff.schema)"
}
if ($handoff.effective_settings.schema -ne "rusty.gui.makepad.effective_settings.v1") {
    throw "Unsupported effective settings schema: $($handoff.effective_settings.schema)"
}

$effectiveSettingsPath = Write-TextPayload `
    -Root $resolvedOutDir `
    -RelativePath "effective-settings.json" `
    -Text (($handoff.effective_settings | ConvertTo-Json -Depth 100) + "`n")

$profileRelativePath = if ($handoff.files.stimulus_profile) {
    [string]$handoff.files.stimulus_profile
} else {
    "stimulus/stimulus-profile.json"
}
$profileJson = if ($handoff.stimulus_profile_json) {
    [string]$handoff.stimulus_profile_json
} else {
    $handoff.stimulus_profile | ConvertTo-Json -Depth 100 -Compress
}
$profilePath = Write-TextPayload `
    -Root $resolvedOutDir `
    -RelativePath $profileRelativePath `
    -Text $profileJson
$profileSha = Get-Sha256HexForText $profileJson
if ($handoff.stimulus_profile_sha256 -and $profileSha -ne [string]$handoff.stimulus_profile_sha256) {
    throw "Stimulus profile SHA-256 mismatch: expected $($handoff.stimulus_profile_sha256), got $profileSha"
}

$tuningPath = $null
$tuningSha = $null
if ($handoff.stimulus_tuning_json) {
    $tuningRelativePath = if ($handoff.files.stimulus_tuning) {
        [string]$handoff.files.stimulus_tuning
    } else {
        "stimulus/stimulus-tuning.json"
    }
    $tuningJson = [string]$handoff.stimulus_tuning_json
    $tuningPath = Write-TextPayload `
        -Root $resolvedOutDir `
        -RelativePath $tuningRelativePath `
        -Text $tuningJson
    $tuningSha = Get-Sha256HexForText $tuningJson
    if ($handoff.stimulus_tuning_sha256 -and $tuningSha -ne [string]$handoff.stimulus_tuning_sha256) {
        throw "Stimulus tuning SHA-256 mismatch: expected $($handoff.stimulus_tuning_sha256), got $tuningSha"
    }
}

$report = [ordered]@{
    schema = "rusty.quest.makepad.stimulus_handoff_expand_report.v1"
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    source_handoff_path = $resolvedHandoff
    out_dir = $resolvedOutDir
    effective_settings_path = $effectiveSettingsPath
    stimulus_profile_path = $profilePath
    stimulus_profile_sha256 = $profileSha
    stimulus_tuning_path = $tuningPath
    stimulus_tuning_sha256 = $tuningSha
    next_stage_command = "powershell -NoProfile -ExecutionPolicy Bypass -File S:\Work\repos\active\rusty-hostess\tools\Stage-HostessMakepadSettings.ps1 -BundleDir $resolvedOutDir"
    boundary = [ordered]@{
        settings_payload = "low-rate stimulus enable/path/schema/hash/presentation controls"
        stimulus_profile_payload = "renderer-neutral Optics profile staged as sibling JSON"
        high_rate_json_payload = $false
    }
}

$reportPath = Join-Path $resolvedOutDir "stimulus-handoff-expand-report.json"
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding UTF8
Write-Output "Stimulus browser handoff expanded: $reportPath"
