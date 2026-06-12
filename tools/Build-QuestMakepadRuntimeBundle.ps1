param(
    [string]$BundlePath = "fixtures\profiles\mesh-replay.bundle.json",
    [string]$OutDir = "local-artifacts\quest-makepad-runtime-bundle",
    [string]$MakepadRoot = $env:RUSTY_MAKEPAD_ROOT,
    [string]$QuestRoot = $env:RUSTY_QUEST_ROOT
)

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

function Resolve-RepoPath {
    param(
        [Parameter(Mandatory=$true)]
        [string]$RepoRoot,
        [Parameter(Mandatory=$true)]
        [string]$PathValue
    )

    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        return (Resolve-Path $PathValue).Path
    }
    return (Resolve-Path (Join-Path $RepoRoot $PathValue)).Path
}

function Assert-SafeRelativePayloadPath {
    param(
        [Parameter(Mandatory=$true)]
        [string]$PathValue
    )

    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        throw "Payload output path must not be empty"
    }
    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        throw "Payload output path must be relative: $PathValue"
    }
    if ($PathValue.Contains("..")) {
        throw "Payload output path must not contain '..': $PathValue"
    }
}

function Convert-EffectiveValueToPropertyString {
    param($Value)

    if ($Value -is [bool]) {
        return $Value.ToString().ToLowerInvariant()
    }
    if (
        $Value -is [byte] -or
        $Value -is [sbyte] -or
        $Value -is [int16] -or
        $Value -is [uint16] -or
        $Value -is [int] -or
        $Value -is [uint32] -or
        $Value -is [long] -or
        $Value -is [uint64] -or
        $Value -is [single] -or
        $Value -is [double] -or
        $Value -is [decimal]
    ) {
        return [System.Convert]::ToString($Value, [System.Globalization.CultureInfo]::InvariantCulture)
    }
    return [string]$Value
}

function Test-PropertyValueMatchesEffectiveValue {
    param(
        [Parameter(Mandatory=$true)]
        [string]$PropertyValue,
        [Parameter(Mandatory=$true)]
        [string]$EffectiveValue
    )

    if ($PropertyValue -eq $EffectiveValue) {
        return $true
    }

    $numberStyle = [System.Globalization.NumberStyles]::Float
    $culture = [System.Globalization.CultureInfo]::InvariantCulture
    $propertyNumber = [decimal]::Zero
    $effectiveNumber = [decimal]::Zero
    $propertyIsNumber = [decimal]::TryParse($PropertyValue, $numberStyle, $culture, [ref]$propertyNumber)
    $effectiveIsNumber = [decimal]::TryParse($EffectiveValue, $numberStyle, $culture, [ref]$effectiveNumber)

    return $propertyIsNumber -and $effectiveIsNumber -and $propertyNumber -eq $effectiveNumber
}

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$resolvedMakepadRoot = if ([string]::IsNullOrWhiteSpace($MakepadRoot)) {
    (Resolve-Path (Join-Path $RepoRoot "..\rusty-makepad")).Path
} else {
    (Resolve-Path $MakepadRoot).Path
}
$resolvedQuestRoot = if ([string]::IsNullOrWhiteSpace($QuestRoot)) {
    (Resolve-Path (Join-Path $RepoRoot "..\rusty-quest")).Path
} else {
    (Resolve-Path $QuestRoot).Path
}

$resolvedBundle = Resolve-RepoPath -RepoRoot $RepoRoot -PathValue $BundlePath
$bundle = Get-Content -Path $resolvedBundle -Raw | ConvertFrom-Json
if ($bundle.schema -ne "rusty.quest.makepad.runtime_profile.v1") {
    throw "Unsupported Quest Makepad bundle schema: $($bundle.schema)"
}
if ([string]::IsNullOrWhiteSpace([string]$bundle.bundle_id)) {
    throw "Quest Makepad bundle must declare bundle_id"
}
if ([string]::IsNullOrWhiteSpace([string]$bundle.app_id)) {
    throw "Quest Makepad bundle must declare app_id"
}
if (-not ([string]$bundle.effective_settings_marker).StartsWith("RUSTY_QUEST_MAKEPAD_")) {
    throw "effective_settings_marker must use RUSTY_QUEST_MAKEPAD_*"
}

$settingsSurface = Resolve-RepoPath -RepoRoot $RepoRoot -PathValue ([string]$bundle.settings_surface)
$settingsProfile = Resolve-RepoPath -RepoRoot $RepoRoot -PathValue ([string]$bundle.settings_profile)
$questRuntimeProfile = Resolve-RepoPath -RepoRoot $RepoRoot -PathValue ([string]$bundle.quest_runtime_profile)

$resolvedOutDir = if ([System.IO.Path]::IsPathRooted($OutDir)) {
    $OutDir
} else {
    Join-Path $RepoRoot $OutDir
}
New-Item -ItemType Directory -Path $resolvedOutDir -Force | Out-Null
$resolvedOutDir = (Resolve-Path $resolvedOutDir).Path

$effectiveOut = Join-Path $resolvedOutDir "effective-settings.json"
$propertyPlanOut = Join-Path $resolvedOutDir "property-write-plan.json"
$reportOut = Join-Path $resolvedOutDir "runtime-bundle-report.json"
$payloads = @()

if ($bundle.PSObject.Properties.Name -contains "stimulus_profile") {
    $stimulusProfileSource = Resolve-RepoPath -RepoRoot $RepoRoot -PathValue ([string]$bundle.stimulus_profile)
    $stimulusProfileOutRelative = if ($bundle.PSObject.Properties.Name -contains "stimulus_profile_out") {
        [string]$bundle.stimulus_profile_out
    } else {
        "stimulus/stimulus-profile.json"
    }
    Assert-SafeRelativePayloadPath -PathValue $stimulusProfileOutRelative
    $stimulusProfileOut = Join-Path $resolvedOutDir $stimulusProfileOutRelative
    New-Item -ItemType Directory -Path (Split-Path -Parent $stimulusProfileOut) -Force | Out-Null
    Copy-Item -LiteralPath $stimulusProfileSource -Destination $stimulusProfileOut -Force
    $payloads += [ordered]@{
        role = "stimulus-profile"
        source = $stimulusProfileSource
        relative_path = $stimulusProfileOutRelative -replace "\\", "/"
        out = $stimulusProfileOut
        sha256 = (Get-FileHash -LiteralPath $stimulusProfileOut -Algorithm SHA256).Hash.ToLowerInvariant()
        size_bytes = (Get-Item -LiteralPath $stimulusProfileOut).Length
    }
}

Invoke-Checked "Quest Makepad settings surface" "cargo" @(
    "run", "-p", "rusty-makepad-settings-cli", "--",
    "validate-surface",
    "--surface", $settingsSurface,
    "--profile", $settingsProfile
) -WorkingDirectory $resolvedMakepadRoot

Invoke-Checked "Quest Makepad effective settings" "cargo" @(
    "run", "-p", "rusty-makepad-settings-cli", "--",
    "resolve",
    "--surface", $settingsSurface,
    "--profile", $settingsProfile,
    "--out", $effectiveOut
) -WorkingDirectory $resolvedMakepadRoot

$questPlanRelative = "local-artifacts\quest-makepad-runtime-bundle-property-write-plan.json"
Invoke-Checked "Quest Makepad runtime profile dry-run" "powershell" @(
    "-NoProfile", "-ExecutionPolicy", "Bypass",
    "-File", "tools\Apply-RuntimeProfile.ps1",
    "-ProfilePath", $questRuntimeProfile,
    "-DryRun",
    "-Out", $questPlanRelative
) -WorkingDirectory $resolvedQuestRoot

$questPlanSource = Join-Path $resolvedQuestRoot $questPlanRelative
Copy-Item -LiteralPath $questPlanSource -Destination $propertyPlanOut -Force

$effective = Get-Content -Path $effectiveOut -Raw | ConvertFrom-Json
$propertyPlan = Get-Content -Path $propertyPlanOut -Raw | ConvertFrom-Json
if ($effective.schema -ne "rusty.gui.makepad.effective_settings.v1") {
    throw "Unexpected effective settings schema: $($effective.schema)"
}
if ($effective.app_id -ne $bundle.app_id) {
    throw "Effective settings app_id $($effective.app_id) does not match bundle $($bundle.app_id)"
}
if ($propertyPlan.schema -ne "rusty.quest.property_write_plan.v1") {
    throw "Unexpected Quest property write plan schema: $($propertyPlan.schema)"
}

$effectiveById = @{}
foreach ($setting in @($effective.settings)) {
    $effectiveById[[string]$setting.setting_id] = $setting.value
}

foreach ($payload in @($payloads | Where-Object { $_.role -eq "stimulus-profile" })) {
    $profilePathSetting = "makepad.stimulus.profile_path"
    $profileShaSetting = "makepad.stimulus.profile_sha256"
    if (-not $effectiveById.ContainsKey($profilePathSetting)) {
        throw "Stimulus profile payload is present but $profilePathSetting is missing"
    }
    if (-not $effectiveById.ContainsKey($profileShaSetting)) {
        throw "Stimulus profile payload is present but $profileShaSetting is missing"
    }
    if ([string]$effectiveById[$profilePathSetting] -ne [string]$payload.relative_path) {
        throw "Stimulus profile payload path $($payload.relative_path) does not match effective setting $profilePathSetting=$($effectiveById[$profilePathSetting])"
    }
    if ([string]$effectiveById[$profileShaSetting] -ne [string]$payload.sha256) {
        throw "Stimulus profile payload SHA $($payload.sha256) does not match effective setting $profileShaSetting=$($effectiveById[$profileShaSetting])"
    }
}

$sourceLinks = @()
foreach ($operation in @($propertyPlan.operations)) {
    if ($operation.kind -ne "set") {
        continue
    }
    $sourceSettingId = [string]$operation.source_setting_id
    if (-not $effectiveById.ContainsKey($sourceSettingId)) {
        throw "Property $($operation.name) references missing effective setting $sourceSettingId"
    }
    $effectiveValue = Convert-EffectiveValueToPropertyString $effectiveById[$sourceSettingId]
    if (-not (Test-PropertyValueMatchesEffectiveValue -PropertyValue ([string]$operation.value) -EffectiveValue $effectiveValue)) {
        throw "Property $($operation.name) value $($operation.value) does not match effective setting $sourceSettingId value $effectiveValue"
    }
    $sourceLinks += [ordered]@{
        android_property = [string]$operation.name
        source_setting_id = $sourceSettingId
        effective_value = $effectiveById[$sourceSettingId]
        property_value = [string]$operation.value
    }
}

$operations = @($propertyPlan.operations)
$setOperations = @($operations | Where-Object { $_.kind -eq "set" })
$clearOperations = @($operations | Where-Object { $_.kind -eq "clear" })
$report = [ordered]@{
    schema = "rusty.quest.makepad.runtime_bundle_report.v1"
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    bundle_id = [string]$bundle.bundle_id
    app_id = [string]$bundle.app_id
    dry_run = [bool]$propertyPlan.dry_run
    source_bundle_path = $resolvedBundle
    settings_surface_path = $settingsSurface
    settings_profile_path = $settingsProfile
    quest_runtime_profile_path = $questRuntimeProfile
    makepad_root = $resolvedMakepadRoot
    quest_root = $resolvedQuestRoot
    effective_settings = [ordered]@{
        schema = [string]$effective.schema
        path = $effectiveOut
        revision = $effective.revision
        setting_count = @($effective.settings).Count
        marker = [string]$bundle.effective_settings_marker
    }
    payloads = @($payloads)
    property_write_plan = [ordered]@{
        schema = [string]$propertyPlan.schema
        path = $propertyPlanOut
        profile_id = [string]$propertyPlan.profile_id
        operation_count = $operations.Count
        clear_count = $clearOperations.Count
        set_count = $setOperations.Count
    }
    source_links = $sourceLinks
    boundary = [ordered]@{
        settings_authority = "rusty-makepad"
        platform_transport_authority = "rusty-quest"
        app_adapter_authority = "rusty-quest-makepad"
        legacy_reference_source_used = $false
        device_write_performed = $false
    }
}

$report | ConvertTo-Json -Depth 12 | Set-Content -Path $reportOut -Encoding UTF8
Write-Output "Quest Makepad runtime bundle report written: $reportOut"
