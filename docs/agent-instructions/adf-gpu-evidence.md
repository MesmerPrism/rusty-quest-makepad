# ADF And GPU Evidence Boundaries

Use this note when changing ADF, field-force, GPU residency, storage/readback,
or force-authority promotion behavior.

## ADF And Field-Force Boundaries

The `makepad.sdf_adf.overlay_mode` setting has these runtime meanings:

- `sdf` enables the Matter-backed SDF slice path.
- `adf` builds a Matter ADF from the current Matter SDF grid and resolves the
  Optics ADF debug visual.
- `combined` remains a gated future mode.

Evidence for `adf` should be compact marker fields such as
`adfDebugEnabled=true`, `adfStatus=ready`,
`adfSchema=rusty.quest.makepad.matter_adf_debug.v1`,
`adfVisualSchema=rusty.optics.adf.debug.visual.v1`, `adfCells`,
`adfSourceSamples`, `adfBuildMs`, and `adfVisualMs`. Do not copy ADF leaf-cell
arrays into settings, runtime profiles, Android properties, or command JSON.

Particle stepping can use Matter's animated mesh surface directly through the
mesh-distance/surface-sampler path or Matter-owned CPU reference fields through
`sdf-field` / `adf-field`. Evidence should include
`particleSamplingAuthority=matter-mesh-distance-sampler`,
`matter-sdf-field-sampler`, or `matter-adf-field-sampler` and the matching
`particleFieldSource`. `sdfAdfDebugParticleAuthority=false` must remain true
for field-force modes because particles sample Matter-owned runtime fields,
not Quest-Makepad ADF/SDF debug visual payloads.

Normal profiles select exactly one runtime force authority.
`makepad.particles.force.source` selects the Matter CPU oracle/fallback
(`mesh-distance`, `none`, `sdf-field`, or `adf-field`).
`makepad.particles.force.authority` is the Quest-Makepad adapter profile gate
and currently defaults to `matter-cpu`; the only GPU-backed request token is
`gpu-dense-sdf-field-particle-force`.

## GPU Proof Checkpoints

`RUSTY_QUEST_MAKEPAD_GPU_RESIDENCY` is render-plane evidence only. It must
report `computeKernel=false`, `matterCpuReferencePreserved=true`, and
`highRateJsonPayload=false`. Treat `RUSTY_MAKEPAD_CADENCE` fields such as
`xrRepaintGeometryUploadBytes`, `xrRepaintInstances`, and `xrRepaintGpuMs` as
the headset measurement companion.

`RUSTY_QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT` is a preflight marker, not a
compute claim. Emit it only for Matter frames with exactly one ready field-force
CPU oracle: `sdf-field` or `adf-field`. It must report
`gpuComputeReady=false`, `computeKernel=false`,
`makepadComputeBackend=makepad-command-encoder-pending`,
`cpuOraclePreserved=true`, and `highRateJsonPayload=false`.

`RUSTY_QUEST_MAKEPAD_GPU_STORAGE_PROBE` proves a resource path, not GPU
particle force semantics. It may be emitted only from an eligible compute
preflight plus a Makepad XR/Vulkan storage-buffer readback result. It must keep
the selected CPU oracle and report
`resourcePlane=vulkan-storage-buffer-command-readback`,
`storageProbeBackend=makepad-vulkan-queue-submit-fill-copy-readback`,
`readbackMatched=true`, `commandEncoderSubmitted=true`,
`storageBufferResident=true`, and `gpuCommandExecuted=true`, while keeping
`gpuComputeReady=false`, `computeKernel=false`, and
`highRateJsonPayload=false`.

Recorded-hand skinning and mesh-SDF markers are bounded readback proofs. They
must keep `gpuComputeReady=false` until full runtime residency, reuse, cadence,
and CPU-oracle comparison evidence are present. Mesh-SDF evidence should carry
`programGeneration`, `programReused`, first-use shader/pipeline fields,
source-mesh buffer residency/reuse fields, derived-buffer residency/reuse
fields, and a current bounded `sampleCount` of at least `8`.

## Force-Authority Gates

`RUSTY_QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE` may be emitted only from a
ready resident dense-SDF particle-force proof that matches the Matter CPU
oracle.

`RUSTY_QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE` records the next adapter
boundary: candidate eligible, explicit profile gate required, runtime
selection not permitted, exactly one Matter CPU force authority still active,
and Matter CPU fallback ready. `profileGateSatisfied` and
`gpuForceAuthorityProfileEnabled` may become `true` only when
`makepad.particles.force.authority=gpu-dense-sdf-field-particle-force`.

`RUSTY_QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY` is the promotion health
receipt after that gate. It must keep `activeForceAuthorityKind=matter-cpu`,
`activeForceAuthorityCount=1`, `runtimeSelectionPermitted=false`,
`boundedProofOnly=true`, and `matterCpuFallbackReady=true` until every
promotion gate is true. Steady-state residency and cadence may become true
first, but freshness, expanded CPU-oracle comparison, and live-vs-recorded
provider A/B must still keep rollback/fallback on Matter CPU.

Use
`fixtures\profiles\mesh-replay-recorded-left-particles-gpu-force.bundle.json`
for explicit profile-gate evidence; it keeps `sdf-field` as the Matter CPU
oracle/fallback while requesting the GPU-backed force authority.

The indexed ADF pre-GPU sweep under
`S:\Work\tmp\quest-makepad-indexed-adf-pre-gpu-sweep-20260611-141903` is the
current force-mode evidence baseline. Treat it as the stop point for default
CPU ADF micro-tuning before GPU-backed residency work unless a correctness or
evidence marker bug appears.

