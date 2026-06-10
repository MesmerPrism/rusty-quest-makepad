# Rusty Quest Makepad Agent Notes

This is the clean source repository for Quest-specific Makepad apps and
adapters in Rusty Morphospace. Keep committed content self-contained and free
of local-only planning paths and historical naming drift.

Rusty Morphospace is the top-level project/platform umbrella. This repo remains
the Quest-Makepad app lane inside that umbrella: Quest/OpenXR/Makepad shells,
headset camera/passthrough panels, tracked input adapters, Lattice frame/view
binding at the app-adapter boundary, and Quest-specific Makepad runtime
profiles.

Project-owned source in this repo is licensed `AGPL-3.0-or-later`. The upstream
Makepad fork remains an upstream-derived toolkit dependency under its own
license and provenance.

## Purpose

Rusty Quest Makepad owns Quest-specific Makepad app adapters. Generic Makepad
settings and descriptors live in `rusty-makepad`; platform write/readback
transports live in `rusty-quest`.

## Read Order

1. `README.md`
2. `docs/ARCHITECTURE.md`
3. `docs/VALIDATION.md`
4. `fixtures/README.md`

## Validation

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

## Hostess Quest APK Consumer Path

`rusty-quest-makepad` owns the Quest Makepad adapter/profile surface and the
effective-settings fixtures consumed by Hostess. When validating those fixtures
in the installable Hostess Makepad APK, build the Hostess app through the
generated Morphospace Makepad Quest manifest rather than creating a custom
Android manifest template.

Use this downstream APK route from `rusty-hostess` when headset evidence is
needed:

```powershell
& 'S:\Work\tools\Quest\Use-QuestTooling.ps1'
cargo install --path S:\Work\repos\active\makepad-morphospace\tools\cargo_makepad --force
cd S:\Work\repos\active\rusty-hostess\apps\hostess-t-makepad
cargo makepad android --variant=quest --abi=aarch64 --sdk-path="$env:ANDROID_HOME" --package-name=io.github.mesmerprism.rustyhostess.makepad --app-label="Rusty Hostess Makepad" --quest-camera-permissions=false build -p hostess-t-makepad
```

`--variant=quest` is required for `.MakepadAppXr` and OpenXR broker metadata.
`--quest-camera-permissions=false` is the camera-free particle/SDF smoke path;
camera streaming remains controlled by effective settings, and high-rate
particle/SDF data must stay out of settings/control JSON.

## Recorded Full Hand-Mesh Replay Smoke

The committed `public-synthetic-hand-sequence` fixture is only an eight-vertex
smoke replay. For browser-parity recorded hand-mesh validation, keep the large
recorded GLB/sequence as a local artifact and build it through Matter:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRuntimeBundle.ps1 -BundlePath fixtures\profiles\mesh-replay-recorded-left.bundle.json -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRecordedMeshReplay.ps1 -GlbPath S:\Work\tmp\quest-handmesh-matter-full-20260601-123844\quest-handmesh-1780310333778406776.glb -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left -FrameCount 120
```

The tools write a staging-ready local bundle under
`local-artifacts\quest-makepad-runtime-bundle-recorded-left`: effective settings
at the root, and generated replay JSON under `mesh-replay`. The recorded replay
builder sets `RUSTY_QUEST_MAKEPAD_RECORDED_SEQUENCE_JSON` only for the adapter
smoke test and proves each generated hand sequence enters the same Matter
source-frame boundary as the bundled replay. It extracts mesh indices `0,1` by
default for the recorded left/right GLB. Do not commit the generated sequences
or put high-rate recorded frames into settings/control JSON.

For the current recorded replay plus billboard-particle headset inspection
profile, use the sibling bundle/output path:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRuntimeBundle.ps1 -BundlePath fixtures\profiles\mesh-replay-recorded-left-particles.bundle.json -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left-particles
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadRecordedMeshReplay.ps1 -GlbPath S:\Work\tmp\quest-handmesh-matter-full-20260601-123844\quest-handmesh-1780310333778406776.glb -OutDir local-artifacts\quest-makepad-runtime-bundle-recorded-left-particles -FrameCount 120
```

That profile keeps camera streaming, collision probes, and SDF debug slices off
and uses `makepad.particles.count=64` /
`makepad.particles.render.draw_limit=64`. Headset simpleperf showed this slice
was CPU-bound in Matter distance sampler rebuild/direct particle sampling before
the worker/latest-snapshot and sampler-refit paths landed. Current evidence
should show `renderThreadBlocking=false`, `distanceSamplerRefit=true` after
the first matching-topology frame, and
`particleDistanceRefreshPolicy=step-only` for the Quest visual profile. Larger
particle and collision budgets should still be measured in separate runs.

The nonblocking boundary lives in `rusty-quest-makepad-matter-surface` as
`QuestMakepadMatterSurfaceWorker`. Headset apps submit source-frame requests to
that worker and render the latest completed payload. Evidence should include
`RUSTY_QUEST_MAKEPAD_MATTER_SURFACE_WORKER mode=latest-wins workerThread=true
renderThreadBlocking=false` plus the normal Matter runtime marker. Do not move
the worker's high-rate frame payloads into settings/control JSON.

For particle-density sweeps, keep the committed recorded-left-particles profile
as the 64-particle visual smoke and patch only the generated effective settings
or a local bundle copy. Safe density knobs are
`makepad.particles.count`, `makepad.particles.render.draw_limit`,
`makepad.particles.render.animation_mode=static-ring`, and
`makepad.particles.render.size_scale=0.2`. The 2026-06-10 headset sweep at
1024, 2048, and 4096 static small billboards showed light Makepad render/GPU
cost and zero texture upload, while the serial Matter particle worker developed
large backlog. A separate APK built with Hostess feature
`matter-particles-parallel` can request
`makepad.particles.execution.backend=rayon` and
`makepad.particles.execution.max_threads` through the same generated
effective-settings path. The 2026-06-10 Rayon/4 run cut 4096-particle mean
`particleStepMs` from the prior serial `658.822` to `294.979`, but still
developed backlog. Keep serial as the committed default and treat the next
density implementation as bounded-cadence/drop-backlog work before GPU compute.

The bounded-cadence density path now has a canonical setting:
`makepad.particles.simulation.max_frame_delta_seconds`, where committed
profiles use `0` for unbounded behavior and local density sweeps can set a
positive cap such as `0.022`. Evidence should include
`particleInputDeltaSeconds`, `particleSimulatedDeltaSeconds`, and
`particleDroppedDeltaSeconds` in
`RUSTY_QUEST_MAKEPAD_MATTER_SURFACE_RUNTIME`. The 2026-06-10 bounded Rayon/4
sweep at `1024`, `2048`, `4096`, `8192`, and `16384` source particles used
static-ring billboards at `size_scale=0.2` with camera/collision/SDF off.
Render cadence stayed `90.0` Hz, texture upload stayed `0`, and GPU repaint
stayed about `0.40`-`1.19 ms`; Matter worker time remained the limit. The
current Makepad world-particle draw path caps visible instances at `8192`, so
the `16384` run is compute evidence only: Matter emitted `16384` rows while
draw markers reported `drawnInstances=8192` and `droppedRows=8192`.
For compute-focused sweeps, also set
`makepad.particles.distance_refresh_policy=disabled` in generated/local
effective settings. This skips only the extra snapshot/debug
`last_surface_distance` refresh pass; Matter integration still samples the
animated hand surface and markers should show `particleClosestSamples` equal to
the source particle count. The 2026-06-10 disabled-refresh Rayon/4 sweep with
static-ring billboards at `size_scale=0.2` reached `1024`, `2048`, `4096`,
`8192`, `16384`, and `32768` source particles. Evidence showed
`particleDistanceSamples=0`, `particleRefreshSamples=0`, texture upload bytes
`0`, and app-owned cadence at `90.0` Hz. Step means were about `14.3`,
`26.3`, `48.6`, `101.0`, `188.9`, and `428.7 ms` respectively; above `8192`
the current renderer still draws only `8192` instances, so higher counts are
Matter compute evidence plus capped visual proof. The adapter now applies the
existing `makepad.particles.render.draw_limit` before Optics visual-frame
resolution and Makepad row packing. In runtime markers, `particleCount` and
`particleSourceRows` are full Matter source counts, while `particleRows` is the
capped visual row count and `particleVisualRowLimit` is the effective cap.
World draw markers still report full `sourceRows`, drawn instances, and
`droppedRows`. A focused 2026-06-11 headset run at `32768` source particles
with draw limit `8192` confirmed the split: `particlePayloadMs` mean dropped
from the previous `29.228` to `9.823`, `particleVisualMs` from `41.535` to
`10.474`, and `particleUploadMs` from `1.230` to `0.280`, while
`particleStepMs` stayed about the same because Matter still simulated all
source particles. Evidence:
`S:\Work\tmp\quest-makepad-visual-row-cap-density-20260611-0013`.
A follow-up Matter hot-path allocation cleanup, validated in
`S:\Work\tmp\quest-makepad-hotpath-allocation-density-20260611-0044`,
reduced the same `32768`/`8192` profile's `particleStepMs` mean from
`433.741` to `404.871` without changing particle truth or visual cap markers.

Before launching the APK, stage
`fixtures\effective-settings\mesh-replay.effective-settings.json` into the
Hostess app-private path:

```powershell
$adb = $env:RUSTY_XR_ADB
$package = 'io.github.mesmerprism.rustyhostess.makepad'
& $adb push S:\Work\repos\active\rusty-quest-makepad\fixtures\effective-settings\mesh-replay.effective-settings.json /data/local/tmp/makepad-effective-settings.json
& $adb shell "run-as $package sh -c 'mkdir -p files/hostess-t/settings && cp /data/local/tmp/makepad-effective-settings.json files/hostess-t/settings/makepad-effective-settings.json'"
& $adb shell am start -W -n "$package/.MakepadAppXr"
```

Do not interpret an unstaged or `not_configured` Hostess receipt as an adapter
runtime failure until this app-private settings file has been staged.
