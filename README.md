# Rusty Quest Makepad

Rusty Quest Makepad is the Morphospace lane for Quest-specific Makepad headset
apps and adapters.

The initial slice defines the Quest Makepad camera shell settings surface and
profile bundle used to move mesh replay and future SDF/ADF, collision, and
particle controls out of ad hoc launch settings.

Use the runtime bundle builder as the single entry point for this profile:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRuntimeBundle.ps1
```

It resolves the canonical Makepad effective settings, asks `rusty-quest` for the
matching Quest property write plan, and writes one provenance report under
`local-artifacts\quest-makepad-runtime-bundle`.

It also contains `rusty-quest-makepad-mesh-replay`, a reusable parser/runtime
for Matter-owned recorded mesh surface sequences. Replay frames can be exposed
as native Matter `TriangleMeshSurface` values for downstream adapters.

`rusty-quest-makepad-matter-surface` is the native Matter runtime adapter for
the Quest Makepad surface slice. It consumes source frames from smoke replay,
recorded replay, and future realtime hand providers, steps
`rusty-matter-surface-runtime`, packages bounded Makepad-facing rows for
distance slices, ADF debug visuals, collision contacts, and particles, and
uses Optics crates for renderer-neutral visuals. It is not simulation
authority.

The adapter also exposes `QuestMakepadMatterSurfaceWorker`, a nonblocking
latest-wins execution wrapper for headset apps. Hostess and other OpenXR
render loops should submit source-frame/config deltas to this worker and render
the latest completed frame, instead of rebuilding Matter distance/collider and
particle payloads directly on the render cadence. The worker owns scheduling
and evidence counters only; `rusty-matter-surface-runtime` remains the
simulation authority.

For the optional recorded full hand-mesh replay smoke, generate a local Matter
sequence from the external recorded GLB and run the adapter test with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRuntimeBundle.ps1 -BundlePath fixtures\profiles\mesh-replay-recorded-left.bundle.json -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRecordedMeshReplay.ps1 -GlbPath S:\Work\tmp\quest-handmesh-matter-full-20260601-123844\quest-handmesh-1780310333778406776.glb -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left -FrameCount 120
```

The output stays under
`local-artifacts\quest-makepad-runtime-bundle-recorded-left`; do not commit the
generated sequences or route them through settings JSON. The tool extracts mesh
indices `0,1` by default, so the recorded hand pair is represented as two
single-surface replay sequences until Matter owns a multi-surface contract.
Hostess resolves the selected recorded source from a sibling `mesh-replay`
directory beside the effective-settings file.

For headset visual inspection with billboard particles, use the sibling
`mesh-replay-recorded-left-particles.bundle.json` bundle and generate the same
recorded replay data into
`local-artifacts\quest-makepad-runtime-bundle-recorded-left-particles`. That
profile intentionally keeps camera streaming, collision probes, and SDF debug
slices off, with `makepad.particles.count=64` and
`makepad.particles.render.draw_limit=64`, because headset simpleperf showed the
recorded full-mesh proof path is CPU-bound in Matter update/direct particle
sampling unless the worker/latest-snapshot and sampler-refit paths are active.
Runtime markers should show `renderThreadBlocking=false`,
`distanceSamplerRefit=true` after the first matching-topology frame, and
`particleDistanceRefreshPolicy=step-only` for this Quest visual profile. Use a
separate profile/run when collision probe evidence is the measurement target.

For density/performance experiments, keep the same effective-settings path and
patch generated local artifacts rather than adding Hostess overrides. The
camera-shell surface exposes `makepad.particles.render.animation_mode` and
`makepad.particles.render.size_scale`; the 2026-06-10 headset sweep used
`static-ring` plus `0.2` size scale at 1024, 2048, and 4096 particles to keep
billboard render cost low while measuring Matter particle stepping. That sweep
showed render/GPU/upload stayed light and the serial Matter worker became the
bottleneck. A follow-up APK built with Hostess feature
`matter-particles-parallel` validated `backend=rayon` through the same
effective-settings path: at 4096 particles, Rayon/4 reduced mean
`particleStepMs` from the prior serial `658.822` to `294.979`, while
render/upload stayed light. Rayon is therefore an explicit high-density
experiment path, not the committed default. The next density slice should add
bounded simulation cadence or stale-work dropping before GPU compute.

`rusty-quest-makepad-camera-shell` is the app-facing adapter slice. It consumes
the canonical effective-settings report for the published camera-shell surface
and configures mesh replay, render scale, collision, SDF/ADF overlay, and
particle toggles without depending on the previous source repo or hand-authored
launch values.

For the current ADF slice, `makepad.sdf_adf.overlay_mode=adf` enables a
Matter-backed ADF build from the current Matter SDF grid and resolves the
Optics `rusty.optics.adf.debug.visual.v1` payload. Runtime markers report
`adfDebugEnabled`, `adfStatus`, ADF schema IDs, cell/source counts, and
ADF-specific timings. ADF debug build policy is controlled by the same
canonical effective-settings path with `makepad.adf.debug.max_depth`,
`makepad.adf.debug.max_cells`, and
`makepad.adf.debug.error_tolerance`. `combined` remains a gated future mode
until a dedicated slice supports simultaneous SDF slice plus ADF debug output.

## Validation

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

See `docs/PROVENANCE.md` for extraction boundaries.
See `docs/MIGRATION.md` for the remaining app-shell extraction map.
