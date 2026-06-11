# Rusty Quest Makepad Validation

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

The validation gate checks the local bundle model and camera-shell adapter,
asks `rusty-makepad` to validate and resolve the settings surface/profile,
asks `rusty-quest` to generate a dry-run property write plan, and scans for
legacy naming.

Focused native Matter adapter checks:

```powershell
cargo test -p rusty-quest-makepad-mesh-replay
cargo test -p rusty-quest-makepad-matter-surface
```

The replay test proves recorded frames can become native Matter
`TriangleMeshSurface` values. The Matter-surface adapter test proves the local
path can step replay frames through Matter distance/collider/particle runtime
and emit bounded Makepad-facing rows without browser Wasm.

The Matter-surface adapter tests also cover
`QuestMakepadMatterSurfaceWorker`, the nonblocking latest-wins wrapper used by
Hostess. Worker evidence in headset logs should include
`RUSTY_QUEST_MAKEPAD_MATTER_SURFACE_WORKER` with
`mode=latest-wins`, `workerThread=true`, and
`renderThreadBlocking=false`, followed by normal
`RUSTY_QUEST_MAKEPAD_MATTER_SURFACE_RUNTIME` markers from the completed
Matter-backed frame. Runtime markers should include compact stage timings such
as `adapterTotalMs`, `matterUpdateMs`, `particleStepMs`, `particleVisualMs`,
and row-packing timings, plus `distanceSamplerRefit`,
`particleDistanceRefreshPolicy`, `particleDistanceSamples`,
`particleSubsteps`, `particleClosestSamples`, `particleSurfaceNodeTests`,
`particleSurfaceLeafTests`, `particleSurfaceTriangleTests`,
`particleRefreshSamples`, `particleRefreshNodeTests`,
`particleRefreshLeafTests`, and `particleRefreshTriangleTests`. Current
particle runs should also identify the authority boundary with
`particleSamplingAuthority=matter-mesh-distance-sampler`,
`particleFieldSource=current-mesh-distance`, and
`sdfAdfDebugParticleAuthority=false`; this proves particles are using Matter's
direct animated mesh surface query path rather than sampling the SDF/ADF debug
visual payload. ADF-enabled runs should also show `adfDebugEnabled=true`,
`adfStatus=ready`,
`adfSchema=rusty.quest.makepad.matter_adf_debug.v1`,
`adfVisualSchema=rusty.optics.adf.debug.visual.v1`, `adfCells`,
`adfSourceSamples`, `adfSplitCount`, `adfMaxLevel`, `adfBuildMs`, and
`adfVisualMs`. Those fields let performance runs separate Matter CPU query
shape, Optics conversion, Makepad-facing packing, upload pressure, ADF build
pressure, and GPU repaint before considering cache or GPU-backend changes.
ADF profile/config sweeps should patch only generated/local effective settings
for `makepad.adf.debug.max_depth`, `makepad.adf.debug.max_cells`, and
`makepad.adf.debug.error_tolerance`; the runtime marker must echo the selected
values as `adfMaxDepth`, `adfMaxCells`, and `adfErrorTolerance`.
SDF/ADF debug-cadence sweeps should use
`makepad.sdf_adf.debug.update_interval_frames`; evidence must show
`sdfAdfDebugSource=fresh|reused`, `sdfAdfDebugFrameInterval`, and
`sdfAdfDebugSourceFrameIndex`. Reused frames should show zero SDF/ADF build
timing for that adapter frame while Matter update/collision/particle timings
continue to reflect the current source frame.

Focused ADF adapter checks:

```powershell
cargo test -p rusty-quest-makepad-matter-surface adf -- --nocapture
cargo test -p rusty-quest-makepad-camera-shell adf -- --nocapture
cargo test -p rusty-quest-makepad-camera-shell sdf_adf_debug_update_interval -- --nocapture
```

Optional recorded full hand-mesh replay smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRuntimeBundle.ps1 -BundlePath fixtures\profiles\mesh-replay-recorded-left.bundle.json -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRecordedMeshReplay.ps1 -GlbPath S:\Work\tmp\quest-handmesh-matter-full-20260601-123844\quest-handmesh-1780310333778406776.glb -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left -FrameCount 120
```

The first command writes recorded-left effective settings and a dry-run Quest
property plan. The second calls
`rusty-matter\tools\Invoke-HandMeshGlbSequenceSmoke.ps1`, writes generated
Matter sequences under
`local-artifacts\quest-makepad-runtime-bundle-recorded-left\mesh-replay`, and runs
`external_recorded_sequence_steps_through_source_frame_when_configured` with
`RUSTY_QUEST_MAKEPAD_RECORDED_SEQUENCE_JSON` pointing at that local artifact.
By default the tool extracts mesh indices `0,1`, matching the recorded
left/right GLB layout as separate single-surface Matter replay sequences. The
GLB/sequences remain external high-rate data-plane artifacts and are not
settings payloads or committed fixtures.

For Hostess/APK staging, push
`local-artifacts\quest-makepad-runtime-bundle-recorded-left\effective-settings.json`
as `makepad-effective-settings.json` and copy the sibling `mesh-replay`
directory into the same app-private settings directory. Hostess resolves
`makepad.mesh_replay.source=recorded-meta-quest-hand-left` against that sibling
data-plane directory.

For the current recorded replay plus billboard-particle headset profile, build
and stage `mesh-replay-recorded-left-particles.bundle.json` into
`local-artifacts\quest-makepad-runtime-bundle-recorded-left-particles` instead.
That profile is a visual-inspection profile, not a collision benchmark:
camera streaming, collision probes, and SDF debug slices are off, and both the
Matter particle count and drawn billboard limit are `64`. The Quest adapter
uses `particleDistanceRefreshPolicy=step-only` for this visual path so it does
not refresh per-particle visual distances once before and once after every
particle step; Matter's default native facade policy remains the exact
surface-update-and-step refresh behavior.

For density-only measurement, patch a generated effective-settings bundle
instead of changing the committed visual smoke defaults. Use matching
`makepad.particles.count` and `makepad.particles.render.draw_limit`, set
`makepad.particles.render.animation_mode=static-ring`, and set
`makepad.particles.render.size_scale=0.2`. The 2026-06-10 Quest sweep at 1024,
2048, and 4096 static small billboards showed `xrEffectiveFrameRateHz=90`,
`xrRepaintTextureUploadBytes=0`, and low GPU repaint time through 4096, while
serial Matter `particleStepMs`/worker latency grew into hundreds of
milliseconds. This indicates the current density bottleneck is Matter CPU
stepping plus fixed-step backlog, not billboard rendering or CPU-GPU upload.
To validate the opt-in Rayon path, build the Hostess APK with
`--features matter-particles-parallel`, then patch only generated/local
effective settings with `makepad.particles.execution.backend=rayon` and a
positive `makepad.particles.execution.max_threads`. The 2026-06-10 Rayon/4
Quest run kept render/upload light and reduced 4096-particle mean
`particleStepMs` from the prior serial `658.822` to `294.979`, but still
showed worker backlog. The next density validation should add bounded
catch-up/drop policy before considering a GPU compute backend.

The profile validation runs through
`tools\Build-QuestMakepadRuntimeBundle.ps1`, which also checks that each Quest
property set operation references an effective setting and carries the same
value. This keeps profile, property, and app readback preparation on one
deterministic surface instead of a hand-edited ADB launch sequence.
