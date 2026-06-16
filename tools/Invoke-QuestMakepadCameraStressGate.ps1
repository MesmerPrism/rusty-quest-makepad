param(
    [string]$BundlePath = "fixtures\profiles\camera-hwb-live.bundle.json",
    [string]$OutDir = "local-artifacts\quest-makepad-camera-lifecycle-stress",
    [string]$Apk,
    [string]$Adb = $env:RUSTY_QUEST_ADB,
    [string]$Serial = $env:RUSTY_QUEST_SERIAL,
    [string]$PackageName = "io.github.mesmerprism.rustyquest.makepad.camera",
    [string]$Activity = ".MakepadAppXr",
    [ValidateRange(1, 12)]
    [int]$Cycles = 3,
    [ValidateRange(10, 180)]
    [int]$LogSeconds = 35,
    [ValidateRange(1, 30)]
    [int]$PauseSeconds = 3,
    [ValidateRange(1, 30)]
    [int]$ResumeSeconds = 6,
    [switch]$SkipInstall,
    [switch]$SkipOculusPerformanceProfile
)

$ErrorActionPreference = "Stop"

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

function Count-LinesContaining {
    param(
        [string[]]$Lines,
        [string]$Needle
    )
    return @($Lines | Where-Object { $_ -like "*$Needle*" }).Count
}

function Count-LinesMatching {
    param(
        [string[]]$Lines,
        [string]$Pattern
    )
    return @($Lines | Where-Object { $_ -match $Pattern }).Count
}

function Read-LogLines {
    param([string[]]$Paths)

    $lines = @()
    foreach ($path in $Paths) {
        if (Test-Path -LiteralPath $path) {
            $lines += Get-Content -LiteralPath $path
        }
    }
    return $lines
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
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

$cycleResults = @()
$allLogPaths = @()

for ($cycle = 1; $cycle -le $Cycles; $cycle += 1) {
    $cycleName = "cycle-{0:D2}" -f $cycle
    $cycleOutDir = Join-Path $resolvedOutDir $cycleName
    $cycleBundleOutDir = Join-Path $cycleOutDir "runtime-bundle"
    New-Item -ItemType Directory -Path $cycleOutDir -Force | Out-Null

    $readinessArgs = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", "tools\Invoke-QuestMakepadCameraReadiness.ps1",
        "-BundlePath", $BundlePath,
        "-BundleOutDir", $cycleBundleOutDir,
        "-OutDir", $cycleOutDir,
        "-Adb", $Adb,
        "-PackageName", $PackageName,
        "-Activity", $Activity,
        "-LogSeconds", ([string]$LogSeconds)
    )
    if (-not [string]::IsNullOrWhiteSpace($Serial)) {
        $readinessArgs += @("-Serial", $Serial)
    }
    if (-not [string]::IsNullOrWhiteSpace($Apk)) {
        $readinessArgs += @("-Apk", $Apk)
    }
    if ($SkipOculusPerformanceProfile) {
        $readinessArgs += "-SkipOculusPerformanceProfile"
    }
    if ($SkipInstall -or $cycle -gt 1) {
        $readinessArgs += "-SkipInstall"
    }

    & powershell @readinessArgs
    if ($LASTEXITCODE -ne 0) {
        throw "readiness cycle $cycle failed with exit code $LASTEXITCODE"
    }

    $scorecardPath = Join-Path $cycleOutDir "readiness-scorecard.json"
    $scorecard = Get-Content -LiteralPath $scorecardPath -Raw | ConvertFrom-Json
    $allLogPaths += Join-Path $cycleOutDir "logcat.txt"

    Save-Adb -Arguments @("shell", "input", "keyevent", "KEYCODE_HOME") `
        -Path (Join-Path $cycleOutDir "adb-home.txt") | Out-Null
    Start-Sleep -Seconds $PauseSeconds
    Save-Adb -Arguments @(
        "shell", "am", "start", "-W",
        "-a", "android.intent.action.MAIN",
        "-c", "com.oculus.intent.category.VR",
        "-n", "$PackageName/$Activity"
    ) -Path (Join-Path $cycleOutDir "adb-resume-launch.txt") | Out-Null
    Start-Sleep -Seconds $ResumeSeconds
    $resumeLogPath = Join-Path $cycleOutDir "logcat-after-resume.txt"
    Save-Adb -Arguments @("logcat", "-d", "-v", "threadtime") -Path $resumeLogPath | Out-Null
    $allLogPaths += $resumeLogPath
    Save-Adb -Arguments @("shell", "am", "force-stop", $PackageName) `
        -Path (Join-Path $cycleOutDir "adb-final-force-stop.txt") | Out-Null

    $cycleLogs = Read-LogLines -Paths @((Join-Path $cycleOutDir "logcat.txt"), $resumeLogPath)
    $cycleFatalCount = Count-LinesMatching -Lines $cycleLogs `
        -Pattern "FATAL EXCEPTION|Fatal signal|SIGSEGV|SIGABRT|kgsl|GPU page fault|ANR"

    $cycleResults += [ordered]@{
        cycle = $cycle
        out_dir = $cycleOutDir
        readiness_scorecard = $scorecardPath
        ready = [bool]$scorecard.ready
        performance_ready = [bool]$scorecard.performance.performance_ready
        projection_ready = [bool]$scorecard.route_gates.projection_ready
        texture_metadata_ready = [bool]$scorecard.route_gates.texture_metadata_ready
        descriptor_color_ready = [bool]$scorecard.route_gates.descriptor_color_ready
        video_texture_gate_ready = [bool]$scorecard.route_gates.video_texture_gate_ready
        shader_layout_visual_smoke_ready = [bool]$scorecard.route_gates.shader_layout_visual_smoke_ready
        lifecycle_recovery_ready = [bool]$scorecard.vulkan_lifecycle.recovery_ready
        targeted_fatal_count = $cycleFatalCount
        pause_resume_log = $resumeLogPath
        force_stop_completed = $true
    }
}

$allLogs = Read-LogLines -Paths $allLogPaths
$fatalCount = Count-LinesMatching -Lines $allLogs `
    -Pattern "FATAL EXCEPTION|Fatal signal|SIGSEGV|SIGABRT|kgsl|GPU page fault|ANR"
$surfaceOutOfDateCount = Count-LinesMatching -Lines $allLogs `
    -Pattern "VK_ERROR_OUT_OF_DATE_KHR|ERROR_OUT_OF_DATE_KHR|\bout.of.date\b|OUT_OF_DATE"
$surfaceSuboptimalCount = Count-LinesMatching -Lines $allLogs `
    -Pattern "VK_SUBOPTIMAL_KHR|SUBOPTIMAL_KHR|\bsuboptimal\b|SUBOPTIMAL"
$surfaceLostCount = Count-LinesMatching -Lines $allLogs `
    -Pattern "VK_ERROR_SURFACE_LOST_KHR|ERROR_SURFACE_LOST_KHR|surface lost|SURFACE_LOST"
$pauseResumeCount = Count-LinesMatching -Lines $allLogs `
    -Pattern "onPause|onResume|APP_CMD_PAUSE|APP_CMD_RESUME|pause|resume"
$legacyMakepadForkMarkerPrefix = "RUSTY" + "_XR_MAKEPAD_"

$allRouteReady = @($cycleResults | Where-Object { -not $_.ready }).Count -eq 0
$allPerformanceReady = @($cycleResults | Where-Object { -not $_.performance_ready }).Count -eq 0
$allTextureMetadataReady = @($cycleResults | Where-Object { -not $_.texture_metadata_ready }).Count -eq 0
$allDescriptorColorReady = @($cycleResults | Where-Object { -not $_.descriptor_color_ready }).Count -eq 0
$allVideoTextureGateReady = @($cycleResults | Where-Object { -not $_.video_texture_gate_ready }).Count -eq 0
$allShaderLayoutVisualSmokeReady = @($cycleResults | Where-Object { -not $_.shader_layout_visual_smoke_ready }).Count -eq 0
$completedCycles = @($cycleResults).Count
$stressReady = [bool](
    $completedCycles -eq $Cycles -and
    $allRouteReady -and
    $allPerformanceReady -and
    $allTextureMetadataReady -and
    $allDescriptorColorReady -and
    $allVideoTextureGateReady -and
    $allShaderLayoutVisualSmokeReady -and
    $fatalCount -eq 0
)

$scorecard = [ordered]@{
    schema = "rusty.quest.makepad.camera_lifecycle_stress_scorecard.v1"
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    stress_ready = $stressReady
    cycles_requested = $Cycles
    cycles_completed = $completedCycles
    package_name = $PackageName
    activity = $Activity
    adb = [ordered]@{
        path = $Adb
        serial = if ([string]::IsNullOrWhiteSpace($Serial)) { $null } else { $Serial }
    }
    gates = [ordered]@{
        all_route_ready = $allRouteReady
        all_performance_ready = $allPerformanceReady
        all_texture_metadata_ready = $allTextureMetadataReady
        all_descriptor_color_ready = $allDescriptorColorReady
        all_video_texture_gate_ready = $allVideoTextureGateReady
        all_shader_layout_visual_smoke_ready = $allShaderLayoutVisualSmokeReady
        repeated_launch_stop_cycles_ready = $completedCycles -eq $Cycles
        pause_resume_exercised = $pauseResumeCount -gt 0
        targeted_fatal_count = $fatalCount
    }
    lifecycle = [ordered]@{
        surface_out_of_date_observed = $surfaceOutOfDateCount -gt 0
        surface_out_of_date_count = $surfaceOutOfDateCount
        surface_suboptimal_observed = $surfaceSuboptimalCount -gt 0
        surface_suboptimal_count = $surfaceSuboptimalCount
        surface_lost_observed = $surfaceLostCount -gt 0
        surface_lost_count = $surfaceLostCount
        pause_resume_marker_count = $pauseResumeCount
        hardware_buffer_cache_marker_count = Count-LinesContaining -Lines $allLogs -Needle ($legacyMakepadForkMarkerPrefix + "VULKAN_VIDEO_HARDWARE_BUFFER_CACHE")
        resource_retire_marker_count = Count-LinesContaining -Lines $allLogs -Needle ($legacyMakepadForkMarkerPrefix + "VULKAN_RESOURCE_RETIRE")
        video_import_marker_count = Count-LinesContaining -Lines $allLogs -Needle ($legacyMakepadForkMarkerPrefix + "VULKAN_VIDEO_IMPORT")
        surface_recovery_events_are_opportunistic = $true
    }
    caveats = @(
        "Surface out-of-date, suboptimal, and surface-lost recovery is reported when observed; this script does not force the compositor to produce those events.",
        "OpenGL OES companion playback remains unvalidated unless a native-video-widget gate reports a dedicated pass.",
        "Projection visual acceptance remains separate from route, metadata, descriptor, and performance readiness."
    )
    cycles = $cycleResults
    evidence = [ordered]@{
        out_dir = $resolvedOutDir
        log_paths = $allLogPaths
    }
}

$scorecardPath = Join-Path $resolvedOutDir "lifecycle-stress-scorecard.json"
$scorecard | ConvertTo-Json -Depth 14 | Set-Content -Path $scorecardPath -Encoding UTF8
Write-Output "Quest Makepad camera lifecycle stress scorecard written: $scorecardPath"
if (-not $stressReady) {
    Write-Warning "Quest Makepad camera lifecycle stress gate did not pass; inspect $scorecardPath."
}
