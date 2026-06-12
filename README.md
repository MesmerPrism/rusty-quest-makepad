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

The matter-surface crate root is intentionally a facade. Runtime stepping,
source-frame adaptation, upload rows, ADF debug adaptation, worker execution,
and GPU residency/readback markers live in named modules; see
`docs/ARCHITECTURE.md` before adding new GPU hand-skinning or mesh-to-SDF
adapter code.
The camera-shell crate also keeps the Matter-surface facade in
`src/matter_surface_exports.rs` so future adapter symbols do not expand
`src/lib.rs`.

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
Pass `-CaptureDir <recorded-hand-capture-dir>` when validating the recorded
bind-rig plus compact joint-frame source shape; the tool copies
`left/right.rig.json` and `left/right.clip.jsonl` beside the effective settings
as local data-plane assets and records that staging in the build report.
When installing the Hostess Makepad APK, stage that generated bundle with
`S:\Work\repos\active\rusty-hostess\tools\Stage-HostessMakepadSettings.ps1`.
The helper uses `/data/local/tmp` as the ADB-visible hop and then `run-as` to
copy into Hostess's app-owned `files/hostess-t/settings` directory. Avoid
`/sdcard/Android/data/...` as the handoff path for these files; on current
Quest builds ADB can write it while the app cannot reliably read it.

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
`makepad.adf.debug.error_tolerance`. SDF/ADF debug rebuild cadence is also a
low-rate setting, `makepad.sdf_adf.debug.update_interval_frames`; the default
`1` preserves per-source-frame rebuilds, while larger local sweep values reuse
the last debug payload between rebuilds and are reported as
`sdfAdfDebugSource`, `sdfAdfDebugFrameInterval`, and
`sdfAdfDebugSourceFrameIndex`. This cadence only affects debug visuals; Matter
surface updates, collisions, distance sampling, and particle stepping still use
the current source frame. `combined` remains a gated future mode until a
dedicated slice supports simultaneous SDF slice plus ADF debug output.

The adapter can also project an ADF debug frame into bounded Makepad-facing
world-cell rows with `world_adf_debug_batch_from_frame` or
`QuestMakepadMatterSurfaceFrame::world_adf_debug_batch`. Those rows use
`rusty.quest.makepad.world_adf_debug_batch.v1`, preserve source Optics/Matter
schema IDs, and emit `RUSTY_QUEST_MAKEPAD_WORLD_ADF_DEBUG` markers. They are a
render/data-plane adapter surface for Hostess and other shells, not a new ADF
truth source and not settings JSON.

The first GPU-backed residency slice is render-plane only. Quest-Makepad now
exposes `QuestMakepadGpuResidencyProof` with schema
`rusty.quest.makepad.gpu_residency_proof.v1` for bounded world-particle and ADF
debug batches that are ready for Makepad instanced draw buffers. The proof
preserves Matter's CPU reference behavior, keeps compute kernels out of scope,
keeps high-rate rows out of settings/control JSON, and points Quest evidence to
`RUSTY_MAKEPAD_CADENCE` fields such as
`xrRepaintGeometryUploadBytes`, `xrRepaintInstances`, and `xrRepaintGpuMs`.

Settings changes should be treated as scoped invalidation, not as permission to
reparse or rebuild every runtime subsystem from the frame loop. Generated
bundles staged through Hostess include
`makepad-effective-settings.revision.json` beside the effective settings file.
Hostess runtime checks compare that tiny global/scoped revision sidecar first
and read detailed settings JSON only after a relevant scope hash changes;
path/mtime remains a compatibility fallback for older bundles. Watcher events
and mtimes are hints. High-rate recorded-hand frames, meshes, SDF/ADF fields,
particles, and GPU buffers remain data-plane payloads outside settings/control
JSON.

The compute boundary begins with `QuestMakepadGpuComputePreflight` and
`rusty.quest.makepad.gpu_compute_preflight.v1`. It is emitted only when a
Matter frame has a ready SDF or indexed ADF field-force CPU oracle and records
the future storage-buffer/command-encoder/readback requirements while reporting
`gpuComputeReady=false` and `computeKernel=false`. Mesh-distance and `none`
force modes are intentionally not eligible. The preflight marker preserves
Matter authority and keeps field, particle, mesh, and future GPU-buffer payloads
out of settings/control JSON.

The first real command-buffer checkpoint is
`QuestMakepadGpuStorageProbe` / `rusty.quest.makepad.gpu_storage_probe.v1`.
It wraps a Makepad XR/Vulkan storage-buffer fill/copy/readback probe and is
emitted only from an eligible compute preflight. The marker proves bounded
storage-buffer allocation, queue submission, and readback parity for a fixed
pattern, while still reporting `gpuComputeReady=false`,
`computeKernel=false`, and `highRateJsonPayload=false`. It is a resource-path
proof for the future field/particle kernel, not a GPU field-force
implementation.

The next bounded compute checkpoint is
`QuestMakepadGpuSkinningProbe` / `rusty.quest.makepad.gpu_skinning_probe.v1`.
Recorded hand-capture source frames may carry four joint-matrix skinning
samples derived by Matter from the recorded bind mesh, compact joint frame,
bind poses, weights, and CPU-skinned validation surface. Hostess can submit
those samples to the generic Makepad XR/Vulkan f32 skinning probe and emit a
marker with `weightedDeltaSkinningKernel=false`,
`jointMatrixSkinningKernel=true`,
`meshToSdfKernel=false`, `gpuComputeReady=false`, and
`highRateJsonPayload=false`. This proves bounded skinning arithmetic/readback
against the Matter oracle; it is not the final full-mesh resident skinning or
mesh-to-SDF kernel.

For steady recorded replay, Hostess submits the cached recorded-hand builder
plus the current `RecordedCompactHandJointFrame` to
`QuestMakepadMatterSurfaceWorker::submit_recorded_hand_frame`. The worker
expands that compact frame into Matter's CPU source frame off the app/render
thread. Full skinning-mesh and mesh-to-dense-SDF CPU oracle payloads are
attached only when Hostess requests `gpu_oracle_probes()` for bounded evidence;
ordinary recorded replay uses the Matter-only option.

The bounded mesh-to-dense-SDF proof now reports whether the Makepad XR/Vulkan
backend paid shader/pipeline setup on that submit or reused its
renderer-lifetime program. `programGeneration`, `programReused`,
`shaderCompiledThisSubmit`, and `pipelineCreatedThisSubmit` are adapter
residency evidence fields only. It also reports `sourceMeshBufferGeneration`,
`sourceMeshBuffersResident`, `sourceMeshBuffersReused`,
`sourceVertexBufferBytes`, and `sourceTriangleBufferBytes` so evidence can
separate source mesh storage-buffer allocation from per-submit SDF resources.
Matter remains the CPU oracle for dense-SDF samples and Hostess still only
forwards compact readback markers.

## Validation

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

See `docs/PROVENANCE.md` for extraction boundaries.
See `docs/MIGRATION.md` for the remaining app-shell extraction map.
