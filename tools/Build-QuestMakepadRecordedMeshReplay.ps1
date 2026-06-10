param(
    [Parameter(Mandatory=$true)]
    [string]$GlbPath,
    [string]$OutDir = "local-artifacts\recorded-mesh-replay",
    [string]$MatterRoot = $env:RUSTY_MATTER_ROOT,
    [int[]]$MeshIndex = @(0, 1),
    [int]$PrimitiveIndex = 0,
    [int]$AnimationIndex = 0,
    [int]$FrameCount = 120,
    [switch]$SkipAdapterSmoke
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

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$ResolvedMatterRoot = if ([string]::IsNullOrWhiteSpace($MatterRoot)) {
    (Resolve-Path (Join-Path $RepoRoot "..\rusty-matter")).Path
} else {
    (Resolve-Path $MatterRoot).Path
}
$ResolvedGlb = (Resolve-Path $GlbPath).Path
$ResolvedOutDir = if ([System.IO.Path]::IsPathRooted($OutDir)) {
    $OutDir
} else {
    Join-Path $RepoRoot $OutDir
}
New-Item -ItemType Directory -Path $ResolvedOutDir -Force | Out-Null
$ResolvedOutDir = (Resolve-Path $ResolvedOutDir).Path
$SequenceDir = Join-Path $ResolvedOutDir "mesh-replay"
New-Item -ItemType Directory -Path $SequenceDir -Force | Out-Null
$SequenceDir = (Resolve-Path $SequenceDir).Path

$ReportOut = Join-Path $ResolvedOutDir "recorded-meta-quest-hand-sequence-report.json"

$SequenceReports = @()
foreach ($CurrentMeshIndex in $MeshIndex) {
    $SequenceOut = Join-Path $SequenceDir ("recorded-meta-quest-hand-sequence-mesh{0}.json" -f $CurrentMeshIndex)

    Invoke-Checked "Matter recorded hand GLB sequence extraction mesh $CurrentMeshIndex" "powershell" @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", "tools\Invoke-HandMeshGlbSequenceSmoke.ps1",
        "-GlbPath", $ResolvedGlb,
        "-Output", $SequenceOut,
        "-MeshIndex", "$CurrentMeshIndex",
        "-PrimitiveIndex", "$PrimitiveIndex",
        "-AnimationIndex", "$AnimationIndex",
        "-FrameCount", "$FrameCount"
    ) -WorkingDirectory $ResolvedMatterRoot

    $AdapterSmokeRan = $false
    if (-not $SkipAdapterSmoke) {
        $previousSequencePath = [Environment]::GetEnvironmentVariable(
            "RUSTY_QUEST_MAKEPAD_RECORDED_SEQUENCE_JSON",
            "Process"
        )
        try {
            $env:RUSTY_QUEST_MAKEPAD_RECORDED_SEQUENCE_JSON = $SequenceOut
            Invoke-Checked "Quest Makepad recorded replay adapter smoke mesh $CurrentMeshIndex" "cargo" @(
                "test",
                "-p", "rusty-quest-makepad-matter-surface",
                "external_recorded_sequence_steps_through_source_frame_when_configured",
                "--",
                "--nocapture"
            ) -WorkingDirectory $RepoRoot
            $AdapterSmokeRan = $true
        } finally {
            if ($null -eq $previousSequencePath) {
                Remove-Item Env:\RUSTY_QUEST_MAKEPAD_RECORDED_SEQUENCE_JSON -ErrorAction SilentlyContinue
            } else {
                $env:RUSTY_QUEST_MAKEPAD_RECORDED_SEQUENCE_JSON = $previousSequencePath
            }
        }
    }

    $Sequence = Get-Content -Path $SequenceOut -Raw | ConvertFrom-Json
    $SequenceFile = Get-Item -LiteralPath $SequenceOut
    $SequenceReports += [ordered]@{
        path = $SequenceOut
        size_bytes = $SequenceFile.Length
        schema_id = [string]$Sequence.schema_id
        sequence_id = [string]$Sequence.sequence_id
        mesh_index = $CurrentMeshIndex
        primitive_index = $PrimitiveIndex
        mesh_name = [string]$Sequence.mesh_name
        animation_name = [string]$Sequence.animation_name
        duration_seconds = [double]$Sequence.duration_seconds
        frame_count = @($Sequence.frames).Count
        vertex_count = [int]$Sequence.vertex_count
        triangle_count = @($Sequence.triangles).Count
        topology_index_hash = [string]$Sequence.topology_index_hash
        adapter_smoke_ran = $AdapterSmokeRan
    }
}

$Report = [ordered]@{
    schema = "rusty.quest.makepad.recorded_mesh_replay_build.v1"
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    source_kind = "recorded-meta-quest-hand-glb"
    glb_path = $ResolvedGlb
    matter_root = $ResolvedMatterRoot
    output_dir = $ResolvedOutDir
    sequence_dir = $SequenceDir
    requested_frame_count = $FrameCount
    mesh_indices = @($MeshIndex)
    primitive_index = $PrimitiveIndex
    animation_index = $AnimationIndex
    sequence_count = @($SequenceReports).Count
    sequences = $SequenceReports
    boundary = [ordered]@{
        settings_payload = "source selection only"
        high_rate_frame_payload_location = "local-artifacts"
        matter_authority = $true
        wasm_runtime_used = $false
        committed_capture_asset = $false
    }
}

$Report | ConvertTo-Json -Depth 8 | Set-Content -Path $ReportOut -Encoding UTF8
foreach ($SequenceReport in $SequenceReports) {
    Write-Output "Quest Makepad recorded mesh replay sequence: $($SequenceReport.path)"
}
Write-Output "Quest Makepad recorded mesh replay report: $ReportOut"
