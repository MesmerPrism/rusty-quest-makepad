# Recorded Hand Replay And Particle Runs

Use this note for local recorded hand-mesh replay, billboard-particle visual
smoke, and density/performance sweeps.

## Recorded Full Hand-Mesh Replay

The committed `public-synthetic-hand-sequence` fixture is only an eight-vertex
smoke replay. For browser-parity recorded hand-mesh validation, keep the large
recorded GLB/sequence as a local artifact and build it through Matter:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRuntimeBundle.ps1 -BundlePath fixtures\profiles\mesh-replay-recorded-left.bundle.json -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRecordedMeshReplay.ps1 -GlbPath S:\Work\tmp\quest-handmesh-matter-full-20260601-123844\quest-handmesh-1780310333778406776.glb -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left -FrameCount 120
```

The tools write a staging-ready local bundle under
`local-artifacts\quest-makepad-runtime-bundle-recorded-left`: effective settings
at the root and generated replay JSON under `mesh-replay`. The builder sets
`RUSTY_QUEST_MAKEPAD_RECORDED_SEQUENCE_JSON` only for the adapter smoke test
and proves each generated hand sequence enters the same Matter source-frame
boundary as bundled replay. Do not commit generated sequences or put high-rate
recorded frames into settings/control JSON.

For live-input equivalent recorded-hand source-frame validation, pass
`-CaptureDir <recorded-hand-capture-dir>` to
`Build-QuestMakepadRecordedMeshReplay.ps1`; the script copies
`left/right.rig.json` and `left/right.clip.jsonl` beside the effective settings
as local data-plane assets and records the staging in its report.

## Particle Visual Smoke

For the current recorded replay plus billboard-particle headset inspection
profile, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRuntimeBundle.ps1 -BundlePath fixtures\profiles\mesh-replay-recorded-left-particles.bundle.json -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left-particles
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRecordedMeshReplay.ps1 -GlbPath S:\Work\tmp\quest-handmesh-matter-full-20260601-123844\quest-handmesh-1780310333778406776.glb -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left-particles -FrameCount 120
```

That profile keeps camera streaming, collision probes, and SDF debug slices off
and uses `makepad.particles.count=64` /
`makepad.particles.render.draw_limit=64`. Evidence should show
`renderThreadBlocking=false`, `distanceSamplerRefit=true` after the first
matching-topology frame, and `particleDistanceRefreshPolicy=step-only`.
Larger particle and collision budgets should be measured in separate runs.

The nonblocking boundary lives in `rusty-quest-makepad-matter-surface` as
`QuestMakepadMatterSurfaceWorker`. Headset apps submit source-frame requests to
that worker and render the latest completed payload. Evidence should include
`RUSTY_QUEST_MAKEPAD_MATTER_SURFACE_WORKER mode=latest-wins workerThread=true
renderThreadBlocking=false` plus the normal Matter runtime marker. Do not move
the worker's high-rate frame payloads into settings/control JSON.

## Density Sweeps

For density/performance experiments, patch generated local effective settings
or a local bundle copy rather than changing committed smoke defaults. Safe
density knobs are:

- `makepad.particles.count`
- `makepad.particles.render.draw_limit`
- `makepad.particles.render.animation_mode=static-ring`
- `makepad.particles.render.size_scale=0.2`
- `makepad.particles.simulation.max_frame_delta_seconds`
- `makepad.particles.distance_refresh_policy=disabled`

The current Makepad world-particle draw path caps visible instances at `8192`,
so higher counts are Matter compute evidence plus capped visual proof.
Runtime markers distinguish `particleCount` and `particleSourceRows` from the
capped visual `particleRows` and `particleVisualRowLimit`.

Rayon remains an explicit high-density experiment path. Build Hostess with
`matter-particles-parallel`, then request
`makepad.particles.execution.backend=rayon` and
`makepad.particles.execution.max_threads` through generated/local effective
settings. Keep serial as the committed default and treat future density work
as bounded-cadence/drop-backlog work before GPU compute.

