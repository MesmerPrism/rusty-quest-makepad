param(
    [string]$BundlePath = "fixtures\profiles\camera-hwb-live.bundle.json",
    [string]$BundleOutDir = "local-artifacts\quest-makepad-camera-readiness\runtime-bundle",
    [string]$OutDir = "local-artifacts\quest-makepad-camera-readiness",
    [string]$Apk,
    [string]$Adb = $env:RUSTY_QUEST_ADB,
    [string]$Serial = $env:RUSTY_QUEST_SERIAL,
    [string]$PackageName = "io.github.mesmerprism.rustyquest.makepad.camera",
    [string]$Activity = ".MakepadAppXr",
    [int]$LogSeconds = 45,
    [ValidateRange(0, 5)]
    [int]$OculusCpuLevel = 4,
    [ValidateRange(0, 5)]
    [int]$OculusGpuLevel = 4,
    [ValidateRange(0, 4)]
    [int]$OculusFoveationLevel = 0,
    [ValidateSet("true", "false")]
    [string]$OculusFoveationDynamic = "false",
    [switch]$SkipOculusPerformanceProfile,
    [switch]$SkipInstall,
    [switch]$SkipLaunch
)

$ErrorActionPreference = "Stop"

function Resolve-RepoPath {
    param(
        [Parameter(Mandatory=$true)]
        [string]$RepoRoot,
        [Parameter(Mandatory=$true)]
        [string]$PathValue
    )

    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        return (Resolve-Path -LiteralPath $PathValue).Path
    }
    return (Resolve-Path -LiteralPath (Join-Path $RepoRoot $PathValue)).Path
}

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

function Invoke-Adb {
    param(
        [Parameter(Mandatory=$true)]
        [string[]]$Arguments
    )

    $adbArgs = @()
    if (-not [string]::IsNullOrWhiteSpace($Serial)) {
        $adbArgs += @("-s", $Serial)
    }
    $adbArgs += $Arguments
    & $Adb @adbArgs
    if ($LASTEXITCODE -ne 0) {
        throw "adb $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

function Save-Adb {
    param(
        [Parameter(Mandatory=$true)]
        [string[]]$Arguments,
        [Parameter(Mandatory=$true)]
        [string]$Path
    )

    $output = @(Invoke-Adb -Arguments $Arguments)
    $output | Set-Content -Path $Path -Encoding UTF8
    return $output
}

function Find-LatestApk {
    param([Parameter(Mandatory=$true)][string]$RepoRoot)

    $roots = @(
        (Join-Path $RepoRoot "local-artifacts\quest-makepad-camera-apk"),
        (Join-Path $RepoRoot "apps\quest-makepad-camera-shell\target\android")
    )
    $candidates = @()
    foreach ($root in $roots) {
        if (Test-Path -LiteralPath $root) {
            $candidates += Get-ChildItem -LiteralPath $root -Recurse -Filter "*.apk" -File -ErrorAction SilentlyContinue
        }
    }
    $apk = $candidates | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
    if (-not $apk) {
        throw "No APK was provided and no built APK was found under local-artifacts or the app target directory."
    }
    return $apk.FullName
}

function Count-LinesContaining {
    param(
        [string[]]$Lines,
        [string]$Needle
    )
    return @($Lines | Where-Object { $_ -like "*$Needle*" }).Count
}

function Select-LastLineContaining {
    param(
        [string[]]$Lines,
        [string]$Needle
    )
    $matches = @($Lines | Where-Object { $_ -like "*$Needle*" })
    if ($matches.Count -eq 0) {
        return $null
    }
    return [string]$matches[-1]
}

function Get-MarkerValue {
    param(
        [string]$Line,
        [string]$Key
    )
    if ([string]::IsNullOrWhiteSpace($Line)) {
        return $null
    }
    $match = [regex]::Match($Line, "(^|\s)$([regex]::Escape($Key))=([^\s]+)")
    if (-not $match.Success) {
        return $null
    }
    return $match.Groups[2].Value
}

function Select-FinalReadback {
    param(
        [object[]]$Readbacks,
        [string]$Name
    )

    $setMatches = @($Readbacks | Where-Object {
        $_.name -eq $Name -and $_.kind -eq "set"
    })
    if ($setMatches.Count -gt 0) {
        return $setMatches[-1]
    }

    $matches = @($Readbacks | Where-Object { $_.name -eq $Name })
    if ($matches.Count -gt 0) {
        return $matches[-1]
    }

    return $null
}

function ConvertTo-ReadinessInt {
    param($Value)

    if ($null -eq $Value) {
        return 0
    }
    try {
        return [int]$Value
    } catch {
        return 0
    }
}

function ConvertTo-ReadinessDouble {
    param($Value)

    if ($null -eq $Value) {
        return $null
    }
    $parsed = 0.0
    if ([double]::TryParse(
            [string]$Value,
            [System.Globalization.NumberStyles]::Float,
            [System.Globalization.CultureInfo]::InvariantCulture,
            [ref]$parsed
        )) {
        return $parsed
    }
    return $null
}

function Set-DevicePropertyWithReadback {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Name,
        [Parameter(Mandatory=$true)]
        [string]$Value
    )

    Invoke-Adb -Arguments @("shell", "setprop", $Name, $Value) | Out-Null
    $observed = (Invoke-Adb -Arguments @("shell", "getprop", $Name) | Select-Object -First 1).Trim()
    [ordered]@{
        name = $Name
        expected_value = $Value
        observed_value = $observed
        matched = [bool]($observed -eq $Value)
    }
}

function Set-OculusPerformanceProfile {
    if ($SkipOculusPerformanceProfile) {
        return [ordered]@{
            skipped = $true
            properties = @()
            matched = $false
        }
    }

    $properties = @(
        [ordered]@{ name = "debug.oculus.cpuLevel"; value = [string]$OculusCpuLevel },
        [ordered]@{ name = "debug.oculus.gpuLevel"; value = [string]$OculusGpuLevel },
        [ordered]@{ name = "debug.oculus.foveation.level"; value = [string]$OculusFoveationLevel },
        [ordered]@{ name = "debug.oculus.foveation.dynamic"; value = [string]$OculusFoveationDynamic }
    )

    $readbacks = @()
    foreach ($property in $properties) {
        $readbacks += Set-DevicePropertyWithReadback -Name $property.name -Value $property.value
    }

    [ordered]@{
        skipped = $false
        cpu_level = $OculusCpuLevel
        gpu_level = $OculusGpuLevel
        foveation_level = $OculusFoveationLevel
        foveation_dynamic = $OculusFoveationDynamic
        properties = $readbacks
        matched = [bool](@($readbacks | Where-Object { -not $_.matched }).Count -eq 0)
    }
}

function Select-VrApiPerfSummary {
    param([string[]]$Lines)

    $rows = @()
    foreach ($line in $Lines) {
        if ($line -notmatch "\bVrApi\b" -or $line -notmatch "FPS=") {
            continue
        }
        $fpsMatch = [regex]::Match($line, "FPS=([^,\s]+)")
        $fpsCurrent = $null
        $fpsTarget = $null
        if ($fpsMatch.Success -and $fpsMatch.Groups[1].Value -match "^([0-9.]+)/([0-9.]+)$") {
            $fpsCurrent = ConvertTo-ReadinessDouble $Matches[1]
            $fpsTarget = ConvertTo-ReadinessDouble $Matches[2]
        }
        $staleMatch = [regex]::Match($line, "Stale=([0-9]+)")
        $tearMatch = [regex]::Match($line, "Tear=([0-9]+)")
        $appMatch = [regex]::Match($line, "App=([0-9.]+)ms")
        $cpuGpuMatch = [regex]::Match($line, "CPU&GPU=([0-9.]+)ms")
        $rows += [ordered]@{
            line = $line
            fps = if ($fpsMatch.Success) { $fpsMatch.Groups[1].Value } else { $null }
            fps_current = $fpsCurrent
            fps_target = $fpsTarget
            stale = if ($staleMatch.Success) { [int]$staleMatch.Groups[1].Value } else { $null }
            tear = if ($tearMatch.Success) { [int]$tearMatch.Groups[1].Value } else { $null }
            app_ms = if ($appMatch.Success) { ConvertTo-ReadinessDouble $appMatch.Groups[1].Value } else { $null }
            cpu_gpu_ms = if ($cpuGpuMatch.Success) { ConvertTo-ReadinessDouble $cpuGpuMatch.Groups[1].Value } else { $null }
        }
    }

    $recent = @($rows | Select-Object -Last 5)
    $recentStaleSum = 0
    $recentTearSum = 0
    foreach ($row in $recent) {
        if ($null -ne $row.stale) {
            $recentStaleSum += [int]$row.stale
        }
        if ($null -ne $row.tear) {
            $recentTearSum += [int]$row.tear
        }
    }
    $latest = if ($rows.Count -gt 0) { $rows[-1] } else { $null }
    $status = if ($rows.Count -eq 0) {
        "missing"
    } elseif ($recentStaleSum -eq 0 -and $recentTearSum -eq 0) {
        "ok"
    } else {
        "stale"
    }

    [ordered]@{
        status = $status
        row_count = $rows.Count
        recent_window_count = $recent.Count
        recent_stale_sum = $recentStaleSum
        recent_tear_sum = $recentTearSum
        latest = $latest
        recent = $recent
    }
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$questRoot = (Resolve-Path (Join-Path $repoRoot "..\rusty-quest")).Path
$resolvedOutDir = if ([System.IO.Path]::IsPathRooted($OutDir)) {
    $OutDir
} else {
    Join-Path $repoRoot $OutDir
}
New-Item -ItemType Directory -Path $resolvedOutDir -Force | Out-Null
$resolvedOutDir = (Resolve-Path -LiteralPath $resolvedOutDir).Path

if ([string]::IsNullOrWhiteSpace($Adb)) {
    $Adb = "adb"
}
$Adb = (Get-Command $Adb).Source

$resolvedBundle = Resolve-RepoPath -RepoRoot $repoRoot -PathValue $BundlePath
$bundle = Get-Content -Path $resolvedBundle -Raw | ConvertFrom-Json
if ($bundle.schema -ne "rusty.quest.makepad.runtime_profile.v1") {
    throw "Unsupported Quest Makepad bundle schema: $($bundle.schema)"
}

$resolvedBundleOutDir = if ([System.IO.Path]::IsPathRooted($BundleOutDir)) {
    $BundleOutDir
} else {
    Join-Path $repoRoot $BundleOutDir
}
Invoke-Checked "Quest Makepad camera runtime bundle" "powershell" @(
    "-NoProfile", "-ExecutionPolicy", "Bypass",
    "-File", "tools\Build-QuestMakepadRuntimeBundle.ps1",
    "-BundlePath", $resolvedBundle,
    "-OutDir", $resolvedBundleOutDir
) -WorkingDirectory $repoRoot

$questRuntimeProfile = Resolve-RepoPath -RepoRoot $repoRoot -PathValue ([string]$bundle.quest_runtime_profile)
$executedPlanPath = Join-Path $resolvedOutDir "property-write-plan-executed.json"
$applyRuntimeProfileArgs = @(
    "-NoProfile", "-ExecutionPolicy", "Bypass",
    "-File", "tools\Apply-RuntimeProfile.ps1",
    "-ProfilePath", $questRuntimeProfile,
    "-Execute",
    "-Out", $executedPlanPath,
    "-Adb", $Adb
)
if (-not [string]::IsNullOrWhiteSpace($Serial)) {
    $applyRuntimeProfileArgs += @("-Serial", $Serial)
}
Invoke-Checked "Quest camera property staging" "powershell" $applyRuntimeProfileArgs -WorkingDirectory $questRoot

$state = (Invoke-Adb -Arguments @("get-state") | Select-Object -First 1).Trim()
if ($state -ne "device") {
    throw "ADB target is not in device state: $state"
}

$oculusPerformanceProfile = Set-OculusPerformanceProfile
$oculusPerformanceProfile | ConvertTo-Json -Depth 8 |
    Set-Content -Path (Join-Path $resolvedOutDir "oculus-performance-props.json") -Encoding UTF8
if (-not $oculusPerformanceProfile.skipped -and -not $oculusPerformanceProfile.matched) {
    throw "Oculus performance property readback did not match; inspect oculus-performance-props.json."
}

if ([string]::IsNullOrWhiteSpace($Apk)) {
    $Apk = Find-LatestApk -RepoRoot $repoRoot
} else {
    $Apk = (Resolve-Path -LiteralPath $Apk).Path
}

if (-not $SkipInstall) {
    Save-Adb -Arguments @("install", "-r", $Apk) -Path (Join-Path $resolvedOutDir "adb-install.txt") | Out-Null
}

Save-Adb -Arguments @("shell", "am", "force-stop", $PackageName) -Path (Join-Path $resolvedOutDir "adb-force-stop.txt") | Out-Null
Save-Adb -Arguments @("logcat", "-c") -Path (Join-Path $resolvedOutDir "adb-logcat-clear.txt") | Out-Null

if (-not $SkipLaunch) {
    $component = "$PackageName/$Activity"
    Save-Adb -Arguments @(
        "shell", "am", "start", "-W",
        "-a", "android.intent.action.MAIN",
        "-c", "com.oculus.intent.category.VR",
        "-n", $component
    ) -Path (Join-Path $resolvedOutDir "adb-launch.txt") | Out-Null
}

Start-Sleep -Seconds $LogSeconds

$logPath = Join-Path $resolvedOutDir "logcat.txt"
$activityPath = Join-Path $resolvedOutDir "dumpsys-activity.txt"
$windowPath = Join-Path $resolvedOutDir "dumpsys-window.txt"
$packagePath = Join-Path $resolvedOutDir "dumpsys-package.txt"

$log = Save-Adb -Arguments @("logcat", "-d", "-v", "threadtime") -Path $logPath
$activityDump = Save-Adb -Arguments @("shell", "dumpsys", "activity", "activities") -Path $activityPath
$windowDump = Save-Adb -Arguments @("shell", "dumpsys", "window", "windows") -Path $windowPath
$packageDump = Save-Adb -Arguments @("shell", "dumpsys", "package", $PackageName) -Path $packagePath

$executedPlan = Get-Content -Path $executedPlanPath -Raw | ConvertFrom-Json
$readbacks = @($executedPlan.readbacks)
$readbackFailures = @($readbacks | Where-Object { $_.status -ne "matched" })
$requiredProperties = @(
    "debug.rustyquest.makepad.camera.streaming.enabled",
    "debug.rustyquest.makepad.projection.depth.meters",
    "debug.rustyquest.makepad.projection.runtime.resolution.enabled",
    "debug.rustyquest.makepad.xr.render.scale",
    "debug.rustyquest.makepad.display.refresh.rate.hz",
    "debug.rustyquest.makepad.camera.projection.mode",
    "debug.rustyquest.makepad.camera.projection.geometry.profile",
    "debug.rustyquest.makepad.camera.source.sampling.mode",
    "debug.rustyquest.makepad.projection.border.policy",
    "debug.rustyquest.makepad.processing.layer",
    "debug.rustyquest.makepad.projection.sample.mode",
    "debug.rustyquest.makepad.direct.camera.hardware.buffer.external",
    "debug.rustyquest.makepad.native.passthrough.enabled",
    "debug.rustyquest.makepad.projection.target.joystick.controls"
)
$propertyEvidence = @()
foreach ($name in $requiredProperties) {
    $match = Select-FinalReadback -Readbacks $readbacks -Name $name
    $propertyEvidence += [ordered]@{
        name = $name
        matched = [bool]($null -ne $match -and $match.status -eq "matched")
        observed_value = if ($null -ne $match) { [string]$match.observed_value } else { $null }
    }
}

$activityText = ($activityDump -join "`n")
$windowText = ($windowDump -join "`n")
$focusActive = ($activityText -like "*$PackageName*" -and $activityText -like "*MakepadAppXr*") -or
    ($windowText -like "*$PackageName*" -and $windowText -like "*MakepadAppXr*")

$lastCadence = Select-LastLineContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_CADENCE schema=rusty.quest.makepad-cadence.v1 phase=sample"
$lastFrameFlow = Select-LastLineContaining -Lines $log -Needle "phase=xr-end-frame status=submitted"
$lastTextureMetadata = Select-LastLineContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_TEXTURE_METADATA schema=rusty.quest.makepad-texture-metadata.v1"
$lastDescriptorColor = Select-LastLineContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_DESCRIPTOR_COLOR schema=rusty.quest.makepad-descriptor-color.v1"
$lastVideoTextureGate = Select-LastLineContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_VIDEO_TEXTURE_GATE schema=rusty.quest.makepad-video-texture-gate.v1"
$lastShaderLayoutVisualSmoke = Select-LastLineContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_SHADER_LAYOUT_VISUAL_SMOKE schema=rusty.quest.makepad-shader-layout-visual-smoke.v1"
$legacyMakepadForkMarkerPrefix = "RUSTY" + "_XR_MAKEPAD_"
$lastVulkanHardwareBufferCache = Select-LastLineContaining -Lines $log -Needle ($legacyMakepadForkMarkerPrefix + "VULKAN_VIDEO_HARDWARE_BUFFER_CACHE")
$lastVulkanResourceRetire = Select-LastLineContaining -Lines $log -Needle ($legacyMakepadForkMarkerPrefix + "VULKAN_RESOURCE_RETIRE")
$lastVulkanVideoImport = Select-LastLineContaining -Lines $log -Needle ($legacyMakepadForkMarkerPrefix + "VULKAN_VIDEO_IMPORT")
$markerCounts = [ordered]@{
    camera_status = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_CAMERA_STATUS"
    camera2_metadata = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_CAMERA2_METADATA"
    camera2_acquisition = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_CAMERA2_ACQUISITION"
    hardware_buffer_import = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_HARDWARE_BUFFER_IMPORT"
    texture_metadata = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_TEXTURE_METADATA"
    descriptor_color = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_DESCRIPTOR_COLOR"
    video_texture_gate = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_VIDEO_TEXTURE_GATE"
    shader_layout_visual_smoke = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_SHADER_LAYOUT_VISUAL_SMOKE"
    stereo_projection = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_STEREO_PROJECTION"
    cadence = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_CADENCE"
    frame_adoption = Count-LinesContaining -Lines $log -Needle "RUSTY_QUEST_MAKEPAD_FRAME_ADOPTION"
    makepad_frame_flow = Count-LinesContaining -Lines $log -Needle "phase=xr-end-frame status=submitted"
    makepad_vulkan_hardware_buffer_cache = Count-LinesContaining -Lines $log -Needle ($legacyMakepadForkMarkerPrefix + "VULKAN_VIDEO_HARDWARE_BUFFER_CACHE")
    makepad_vulkan_resource_retire = Count-LinesContaining -Lines $log -Needle ($legacyMakepadForkMarkerPrefix + "VULKAN_RESOURCE_RETIRE")
    makepad_vulkan_video_import = Count-LinesContaining -Lines $log -Needle ($legacyMakepadForkMarkerPrefix + "VULKAN_VIDEO_IMPORT")
}
$cadence = [ordered]@{
    last_line = $lastCadence
    left_texture_update_delta = Get-MarkerValue -Line $lastCadence -Key "leftTextureUpdateDelta"
    right_texture_update_delta = Get-MarkerValue -Line $lastCadence -Key "rightTextureUpdateDelta"
    paired_texture_update_delta = Get-MarkerValue -Line $lastCadence -Key "pairedTextureUpdateDelta"
    xr_display_refresh_rate_hz = Get-MarkerValue -Line $lastCadence -Key "xrDisplayRefreshRateHz"
    xr_effective_frame_rate_hz = Get-MarkerValue -Line $lastCadence -Key "xrEffectiveFrameRateHz"
    texture_path = Get-MarkerValue -Line $lastCadence -Key "cameraTexturePath"
    makepad_vulkan_import = Get-MarkerValue -Line $lastCadence -Key "makepadVulkanImport"
    paired_left_right_camera_frames = Get-MarkerValue -Line $lastCadence -Key "pairedLeftRightCameraFrames"
    projection_mapping_ready = Get-MarkerValue -Line $lastCadence -Key "projectionMappingReady"
    aligned_projection = Get-MarkerValue -Line $lastCadence -Key "alignedProjection"
    visible_camera_projection_ready = Get-MarkerValue -Line $lastCadence -Key "visibleCameraProjectionReady"
    texture_ready_marker = Get-MarkerValue -Line $lastCadence -Key "textureReady"
}
$frameFlowPredictedDisplayPeriodNs = Get-MarkerValue -Line $lastFrameFlow -Key "predictedDisplayPeriodNs"
$frameFlowDisplayRefreshHz = $null
$frameFlowPredictedDisplayPeriodNsNumber = ConvertTo-ReadinessDouble $frameFlowPredictedDisplayPeriodNs
if ($null -ne $frameFlowPredictedDisplayPeriodNsNumber -and $frameFlowPredictedDisplayPeriodNsNumber -gt 0.0) {
    $frameFlowDisplayRefreshHz = [Math]::Round(1000000000.0 / $frameFlowPredictedDisplayPeriodNsNumber, 2)
}
$frameFlow = [ordered]@{
    last_line = $lastFrameFlow
    should_render = Get-MarkerValue -Line $lastFrameFlow -Key "shouldRender"
    layer_count = Get-MarkerValue -Line $lastFrameFlow -Key "layerCount"
    result_code = Get-MarkerValue -Line $lastFrameFlow -Key "resultCode"
    render_path = Get-MarkerValue -Line $lastFrameFlow -Key "renderPath"
    predicted_display_period_ns = $frameFlowPredictedDisplayPeriodNs
    display_refresh_rate_hz = $frameFlowDisplayRefreshHz
}

$textureMetadata = [ordered]@{
    last_line = $lastTextureMetadata
    texture_metadata_ready = (Get-MarkerValue -Line $lastTextureMetadata -Key "textureMetadataReady") -eq "true"
    gpu_import_ready = (Get-MarkerValue -Line $lastTextureMetadata -Key "gpuImportReady") -eq "true"
    visual_release_accepted = (Get-MarkerValue -Line $lastTextureMetadata -Key "visualReleaseAccepted") -eq "true"
    camera_id = Get-MarkerValue -Line $lastTextureMetadata -Key "cameraId"
    camera_input_id = Get-MarkerValue -Line $lastTextureMetadata -Key "cameraInputId"
    camera_format_id = Get-MarkerValue -Line $lastTextureMetadata -Key "cameraFormatId"
    source_frame_seq = Get-MarkerValue -Line $lastTextureMetadata -Key "sourceFrameSeq"
    camera_frame_seq = Get-MarkerValue -Line $lastTextureMetadata -Key "cameraFrameSeq"
    camera_timestamp_ns = Get-MarkerValue -Line $lastTextureMetadata -Key "cameraTimestampNs"
    hardware_buffer_id = Get-MarkerValue -Line $lastTextureMetadata -Key "hardwareBufferId"
    import_seq = Get-MarkerValue -Line $lastTextureMetadata -Key "importSeq"
    texture_update_seq = Get-MarkerValue -Line $lastTextureMetadata -Key "textureUpdateSeq"
    descriptor_shape = Get-MarkerValue -Line $lastTextureMetadata -Key "descriptorShape"
    event_resource_path = Get-MarkerValue -Line $lastTextureMetadata -Key "eventResourcePath"
    texture_path = Get-MarkerValue -Line $lastTextureMetadata -Key "cameraTexturePath"
    makepad_vulkan_import = Get-MarkerValue -Line $lastTextureMetadata -Key "makepadVulkanImport"
    texture_update_count = Get-MarkerValue -Line $lastTextureMetadata -Key "textureUpdateCount"
    left_texture_update_count = Get-MarkerValue -Line $lastTextureMetadata -Key "leftTextureUpdateCount"
    right_texture_update_count = Get-MarkerValue -Line $lastTextureMetadata -Key "rightTextureUpdateCount"
    resource_reused = Get-MarkerValue -Line $lastTextureMetadata -Key "resourceReused"
    fallback_active = Get-MarkerValue -Line $lastTextureMetadata -Key "fallbackActive"
}
$descriptorColor = [ordered]@{
    last_line = $lastDescriptorColor
    descriptor_shape_ready = (Get-MarkerValue -Line $lastDescriptorColor -Key "descriptorShapeReady") -eq "true"
    ycbcr_metadata_present = (Get-MarkerValue -Line $lastDescriptorColor -Key "ycbcrMetadataPresent") -eq "true"
    color_conformance_ready = (Get-MarkerValue -Line $lastDescriptorColor -Key "colorConformanceReady") -eq "true"
    expected_descriptor_shape = Get-MarkerValue -Line $lastDescriptorColor -Key "expectedDescriptorShape"
    descriptor_shape = Get-MarkerValue -Line $lastDescriptorColor -Key "descriptorShape"
    color_conversion = Get-MarkerValue -Line $lastDescriptorColor -Key "colorConversion"
    color_reference = Get-MarkerValue -Line $lastDescriptorColor -Key "colorReference"
    visual_color_status = Get-MarkerValue -Line $lastDescriptorColor -Key "visualColorStatus"
    visual_release_accepted = (Get-MarkerValue -Line $lastDescriptorColor -Key "visualReleaseAccepted") -eq "true"
}
$videoTextureGate = [ordered]@{
    last_line = $lastVideoTextureGate
    texture_gate_ready = (Get-MarkerValue -Line $lastVideoTextureGate -Key "textureGateReady") -eq "true"
    equivalent_texture_metadata_ready = (Get-MarkerValue -Line $lastVideoTextureGate -Key "equivalentTextureMetadataReady") -eq "true"
    gpu_import_ready = (Get-MarkerValue -Line $lastVideoTextureGate -Key "gpuImportReady") -eq "true"
    native_video_widget_started = (Get-MarkerValue -Line $lastVideoTextureGate -Key "nativeVideoWidgetStarted") -eq "true"
    open_gl_oes_companion_validated = (Get-MarkerValue -Line $lastVideoTextureGate -Key "openGlOesCompanionValidated") -eq "true"
    direct_hwb_vulkan_validated = (Get-MarkerValue -Line $lastVideoTextureGate -Key "directHwbVulkanValidated") -eq "true"
    defer_broad_media_imports = (Get-MarkerValue -Line $lastVideoTextureGate -Key "deferBroadMediaImports") -eq "true"
    gate_kind = Get-MarkerValue -Line $lastVideoTextureGate -Key "gateKind"
}
$shaderLayoutVisualSmoke = [ordered]@{
    last_line = $lastShaderLayoutVisualSmoke
    shader_layout_visual_smoke_ready = ($null -ne $lastShaderLayoutVisualSmoke) -and
        (Get-MarkerValue -Line $lastShaderLayoutVisualSmoke -Key "status") -eq "ok" -and
        (Get-MarkerValue -Line $lastShaderLayoutVisualSmoke -Key "cameraTextureBinding") -eq "true" -and
        (Get-MarkerValue -Line $lastShaderLayoutVisualSmoke -Key "projectionPanelDrawEnabled") -eq "true"
    shader_layout_fix_scope = Get-MarkerValue -Line $lastShaderLayoutVisualSmoke -Key "shaderLayoutFixScope"
    folded_into = Get-MarkerValue -Line $lastShaderLayoutVisualSmoke -Key "foldedInto"
    visual_release_accepted = (Get-MarkerValue -Line $lastShaderLayoutVisualSmoke -Key "visualReleaseAccepted") -eq "true"
}

$targetedFatal = @($log | Where-Object {
    ($_ -match "FATAL EXCEPTION|Fatal signal|SIGSEGV|SIGABRT|kgsl|GPU page fault|ANR") -and
    ($_ -match [regex]::Escape($PackageName) -or $_ -match "makepad|rustyquest|RUSTY_QUEST_MAKEPAD")
})

$vulkanSurfaceOutOfDate = @($log | Where-Object { $_ -match "VK_ERROR_OUT_OF_DATE_KHR|ERROR_OUT_OF_DATE_KHR|\bout.of.date\b|OUT_OF_DATE" })
$vulkanSurfaceSuboptimal = @($log | Where-Object { $_ -match "VK_SUBOPTIMAL_KHR|SUBOPTIMAL_KHR|\bsuboptimal\b|SUBOPTIMAL" })
$vulkanSurfaceLost = @($log | Where-Object { $_ -match "VK_ERROR_SURFACE_LOST_KHR|ERROR_SURFACE_LOST_KHR|surface lost|SURFACE_LOST" })
$lifecyclePauseResume = @($log | Where-Object {
    ($_ -match "onPause|onResume|APP_CMD_PAUSE|APP_CMD_RESUME|pause|resume") -and
    ($_ -match "makepad|rustyquest|Rusty|MakepadAppXr")
})
$vulkanLifecycle = [ordered]@{
    recovery_ready = $targetedFatal.Count -eq 0
    hardware_buffer_cache_marker_count = $markerCounts.makepad_vulkan_hardware_buffer_cache
    resource_retire_marker_count = $markerCounts.makepad_vulkan_resource_retire
    video_import_marker_count = $markerCounts.makepad_vulkan_video_import
    hardware_buffer_cache_last_line = $lastVulkanHardwareBufferCache
    resource_retire_last_line = $lastVulkanResourceRetire
    video_import_last_line = $lastVulkanVideoImport
    texture_cache_churn_observed = $markerCounts.makepad_vulkan_hardware_buffer_cache -gt 0
    resource_retire_observed = $markerCounts.makepad_vulkan_resource_retire -gt 0
    surface_out_of_date_observed = $vulkanSurfaceOutOfDate.Count -gt 0
    surface_suboptimal_observed = $vulkanSurfaceSuboptimal.Count -gt 0
    surface_lost_observed = $vulkanSurfaceLost.Count -gt 0
    pause_resume_observed = $lifecyclePauseResume.Count -gt 0
    surface_out_of_date_lines = @($vulkanSurfaceOutOfDate | Select-Object -First 20)
    surface_suboptimal_lines = @($vulkanSurfaceSuboptimal | Select-Object -First 20)
    surface_lost_lines = @($vulkanSurfaceLost | Select-Object -First 20)
    pause_resume_lines = @($lifecyclePauseResume | Select-Object -First 20)
}

$metaPerfStale = Select-VrApiPerfSummary -Lines $log

$transportReady = ($readbackFailures.Count -eq 0) -and (@($propertyEvidence | Where-Object { -not $_.matched }).Count -eq 0)
$markerReady = $markerCounts.hardware_buffer_import -gt 0 -and
    $markerCounts.texture_metadata -gt 0 -and
    $markerCounts.descriptor_color -gt 0 -and
    $markerCounts.video_texture_gate -gt 0 -and
    $markerCounts.shader_layout_visual_smoke -gt 0 -and
    $markerCounts.makepad_frame_flow -gt 0
$leftTextureUpdateDelta = ConvertTo-ReadinessInt $cadence.left_texture_update_delta
$rightTextureUpdateDelta = ConvertTo-ReadinessInt $cadence.right_texture_update_delta
$pairedTextureUpdateDelta = ConvertTo-ReadinessInt $cadence.paired_texture_update_delta
$textureReadyFromCadence = ($leftTextureUpdateDelta -gt 0) -and
    ($rightTextureUpdateDelta -gt 0) -and
    ($pairedTextureUpdateDelta -gt 0)
$leftTextureMetadataCount = ConvertTo-ReadinessInt $textureMetadata.left_texture_update_count
$rightTextureMetadataCount = ConvertTo-ReadinessInt $textureMetadata.right_texture_update_count
$textureReadyFromMetadata = ($leftTextureMetadataCount -gt 0) -and ($rightTextureMetadataCount -gt 0)
$textureReady = [bool]($textureReadyFromCadence -or $textureReadyFromMetadata)
$hardwareBufferReadyFromCadence = $cadence.texture_path -eq "direct-camera-hardware-buffer-external" -and
    $cadence.makepad_vulkan_import -eq "true"
$hardwareBufferReadyFromMetadata = $textureMetadata.texture_path -eq "direct-camera-hardware-buffer-external" -and
    $textureMetadata.makepad_vulkan_import -eq "true" -and
    $textureMetadata.gpu_import_ready
$hardwareBufferReady = [bool]($hardwareBufferReadyFromCadence -or $hardwareBufferReadyFromMetadata)
$textureMetadataReady = [bool]($textureMetadata.texture_metadata_ready -and $textureMetadata.gpu_import_ready)
$descriptorColorReady = [bool]($descriptorColor.color_conformance_ready)
$videoTextureGateReady = [bool]($videoTextureGate.texture_gate_ready)
$shaderLayoutVisualSmokeReady = [bool]($shaderLayoutVisualSmoke.shader_layout_visual_smoke_ready)
$frameFlowLayerCount = ConvertTo-ReadinessInt $frameFlow.layer_count
$frameFlowReady = $frameFlow.should_render -eq "true" -and
    $frameFlowLayerCount -gt 0 -and
    $frameFlow.result_code -eq "0"
$projectionReady = $cadence.paired_left_right_camera_frames -eq "true" -and
    $cadence.projection_mapping_ready -eq "true" -and
    $cadence.aligned_projection -eq "true" -and
    $cadence.visible_camera_projection_ready -eq "true"
$cadenceDisplayRefreshHz = ConvertTo-ReadinessDouble $cadence.xr_display_refresh_rate_hz
$cadenceEffectiveFrameRateHz = ConvertTo-ReadinessDouble $cadence.xr_effective_frame_rate_hz
$frameFlowRefreshHz = ConvertTo-ReadinessDouble $frameFlow.display_refresh_rate_hz
$vrApiTargetFrameRateHz = $null
$vrApiCurrentFrameRateHz = $null
if ($null -ne $metaPerfStale.latest) {
    $vrApiTargetFrameRateHz = ConvertTo-ReadinessDouble $metaPerfStale.latest.fps_target
    $vrApiCurrentFrameRateHz = ConvertTo-ReadinessDouble $metaPerfStale.latest.fps_current
}
$displayRefreshHz = $frameFlowRefreshHz
if ($null -eq $displayRefreshHz) {
    $displayRefreshHz = $vrApiTargetFrameRateHz
}
if ($null -eq $displayRefreshHz) {
    $displayRefreshHz = $cadenceDisplayRefreshHz
}
$effectiveFrameRateHz = $cadenceEffectiveFrameRateHz
if ($null -eq $effectiveFrameRateHz) {
    $effectiveFrameRateHz = $vrApiCurrentFrameRateHz
}
$displayRefreshReady = $null -ne $displayRefreshHz -and [Math]::Abs($displayRefreshHz - 72.0) -le 1.0
$effectiveFrameRateReady = $null -ne $effectiveFrameRateHz -and $effectiveFrameRateHz -ge 70.0
$performanceReady = [bool](
    $oculusPerformanceProfile.matched -and
    $displayRefreshReady -and
    $effectiveFrameRateReady -and
    $metaPerfStale.status -eq "ok"
)
$ready = [bool]($transportReady -and
    $focusActive -and
    $markerReady -and
    $textureReady -and
    $hardwareBufferReady -and
    $textureMetadataReady -and
    $descriptorColorReady -and
    $videoTextureGateReady -and
    $shaderLayoutVisualSmokeReady -and
    $frameFlowReady -and
    $targetedFatal.Count -eq 0)

$scorecard = [ordered]@{
    schema = "rusty.quest.makepad.camera_readiness_scorecard.v1"
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    ready = $ready
    package_name = $PackageName
    activity = $Activity
    apk = [ordered]@{
        path = $Apk
        sha256 = (Get-FileHash -LiteralPath $Apk -Algorithm SHA256).Hash.ToLowerInvariant()
        length = (Get-Item -LiteralPath $Apk).Length
    }
    adb = [ordered]@{
        path = $Adb
        serial = if ([string]::IsNullOrWhiteSpace($Serial)) { $null } else { $Serial }
        state = $state
    }
    ownership = [ordered]@{
        apk_build_owner = "rusty-quest-makepad"
        profile_transport_owner = "rusty-quest"
        runtime_adapter_owner = "rusty-quest-makepad"
        legacy_reference_source_used = $false
    }
    bundle = [ordered]@{
        source = $resolvedBundle
        out_dir = $resolvedBundleOutDir
        runtime_report = Join-Path $resolvedBundleOutDir "runtime-bundle-report.json"
        executed_property_plan = $executedPlanPath
    }
    launch = [ordered]@{
        skip_install = [bool]$SkipInstall
        skip_launch = [bool]$SkipLaunch
        package_name = $PackageName
        activity = $Activity
        component = "$PackageName/$Activity"
        focused_makepad_xr_activity = [bool]$focusActive
    }
    profile_transport = [ordered]@{
        readback_count = $readbacks.Count
        readback_failures = @($readbackFailures)
        required_properties = $propertyEvidence
        transport_ready = $transportReady
    }
    performance = [ordered]@{
        performance_ready = $performanceReady
        display_refresh_ready = $displayRefreshReady
        effective_frame_rate_ready = $effectiveFrameRateReady
        oculus_performance_profile = $oculusPerformanceProfile
        meta_perf_stale = $metaPerfStale
        expected_display_refresh_rate_hz = 72.0
        expected_xr_render_scale = 0.90
        observed_display_refresh_rate_hz = $displayRefreshHz
        observed_effective_frame_rate_hz = $effectiveFrameRateHz
        cadence_display_refresh_rate_hz = $cadenceDisplayRefreshHz
        cadence_effective_frame_rate_hz = $cadenceEffectiveFrameRateHz
        frame_flow_display_refresh_rate_hz = $frameFlowRefreshHz
        vr_api_target_frame_rate_hz = $vrApiTargetFrameRateHz
        vr_api_current_frame_rate_hz = $vrApiCurrentFrameRateHz
    }
    route_gates = [ordered]@{
        route_ready = $ready
        marker_ready = $markerReady
        texture_ready = $textureReady
        texture_ready_from_cadence = $textureReadyFromCadence
        texture_ready_from_metadata = $textureReadyFromMetadata
        hardware_buffer_ready = $hardwareBufferReady
        hardware_buffer_ready_from_cadence = $hardwareBufferReadyFromCadence
        hardware_buffer_ready_from_metadata = $hardwareBufferReadyFromMetadata
        texture_metadata_ready = $textureMetadataReady
        descriptor_color_ready = $descriptorColorReady
        video_texture_gate_ready = $videoTextureGateReady
        shader_layout_visual_smoke_ready = $shaderLayoutVisualSmokeReady
        frame_flow_ready = $frameFlowReady
        projection_ready = $projectionReady
        performance_ready = $performanceReady
    }
    texture_metadata = $textureMetadata
    descriptor_color = $descriptorColor
    video_texture_gate = $videoTextureGate
    shader_layout_visual_smoke = $shaderLayoutVisualSmoke
    vulkan_lifecycle = $vulkanLifecycle
    markers = [ordered]@{
        counts = $markerCounts
        marker_ready = $markerReady
        cadence = $cadence
        frame_flow = $frameFlow
        texture_ready = $textureReady
        texture_ready_from_cadence = $textureReadyFromCadence
        texture_ready_from_metadata = $textureReadyFromMetadata
        hardware_buffer_ready = $hardwareBufferReady
        hardware_buffer_ready_from_cadence = $hardwareBufferReadyFromCadence
        hardware_buffer_ready_from_metadata = $hardwareBufferReadyFromMetadata
        texture_metadata_ready = $textureMetadataReady
        descriptor_color_ready = $descriptorColorReady
        video_texture_gate_ready = $videoTextureGateReady
        shader_layout_visual_smoke_ready = $shaderLayoutVisualSmokeReady
        frame_flow_ready = $frameFlowReady
        projection_ready = $projectionReady
    }
    faults = [ordered]@{
        targeted_fatal_count = $targetedFatal.Count
        targeted_fatal_lines = @($targetedFatal | Select-Object -First 50)
    }
    evidence = [ordered]@{
        out_dir = $resolvedOutDir
        logcat = $logPath
        activity_dump = $activityPath
        window_dump = $windowPath
        package_dump = $packagePath
        oculus_performance_props = Join-Path $resolvedOutDir "oculus-performance-props.json"
    }
}

$scorecardPath = Join-Path $resolvedOutDir "readiness-scorecard.json"
$scorecard | ConvertTo-Json -Depth 12 | Set-Content -Path $scorecardPath -Encoding UTF8
Write-Output "Quest Makepad camera readiness scorecard written: $scorecardPath"
if (-not $ready) {
    Write-Warning "Quest Makepad camera readiness did not pass; inspect $scorecardPath and logcat evidence."
}
