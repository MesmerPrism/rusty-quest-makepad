# Rusty Quest Makepad Architecture

Rusty Quest Makepad owns Quest-specific Makepad app adapters and profile
bundles.

## Ownership

- Quest/OpenXR Makepad shells;
- headset camera and passthrough panels;
- tracked controller and hand input adapters;
- Quest Makepad profile bundles over canonical Makepad settings;
- app-local readback markers.

## Camera Shell Settings Path

The active camera shell consumes
`rusty.gui.makepad.effective_settings.v1` as the single settings boundary. The
generic resolver lives in `rusty-makepad`; Quest runtime property write plans
live in `rusty-quest`; this repo owns only the app adapter that applies the
effective values to Quest Makepad behavior.

The current replay slice is:

1. `rusty-makepad` validates and resolves the settings surface/profile.
2. `rusty-quest` emits a dry-run or device property write plan for transport.
3. `rusty-quest-makepad-camera-shell` reads the effective settings report.
4. `rusty-quest-makepad-mesh-replay` applies the replay config and emits
   app-local markers.
5. `rusty-quest-makepad-matter-surface` can convert replay frames to native
   Matter surfaces, step the Matter-owned distance/collider/particle runtime,
   build optional Matter ADF debug payloads, and package bounded
   Makepad-facing upload rows plus Optics visuals.

`tools\Build-QuestMakepadRuntimeBundle.ps1` is the cohesive operator entry
point for that path. It accepts a Quest Makepad profile bundle, delegates
settings resolution to `rusty-makepad`, delegates platform property planning to
`rusty-quest`, verifies that each planned property points back to the effective
setting that produced it, and writes
`rusty.quest.makepad.runtime_bundle_report.v1` provenance. The report is not a
new settings authority; it is the trace that ties the existing authorities
together for one app launch or hotload preparation.

The camera-shell adapter exposes a full runtime bundle from one effective
settings report: parsed camera-shell settings plus a configured mesh replay
runtime. It also re-exports the mesh replay runtime, uniforms, and marker schema
constants as the app-facing boundary. Hostess and other active Makepad shells
should consume that adapter surface instead of depending on the lower replay
crate directly or reparsing replay settings locally.

The Matter-surface adapter consumes `rusty-matter-surface-runtime`,
`rusty-matter-adf`, and Optics visual crates. It may create app-facing row
buffers and debug payloads for Makepad consumption, but it must not duplicate
Matter distance, collision, SDF, ADF, or particle truth. High-rate particle
rows, SDF slice cells, and ADF leaf-cell data stay on the data/render plane and
do not enter settings, runtime profiles, Android property values, or future
Manifold command JSON. The adapter may cache/reuse SDF/ADF debug visual payloads
between source frames according to the low-rate
`makepad.sdf_adf.debug.update_interval_frames` setting, but that cache is a
renderer/debug cadence policy only. It is not a simulation authority and does
not change the current-frame Matter surface used for collisions, distance
sampling, or particles.

ADF world debug rows follow the same adapter rule. The
`QuestMakepadWorldAdfDebugBatch` boundary converts the existing
`QuestMakepadAdfDebugFrame` / Optics ADF debug visual into bounded cell rows
and compact evidence markers for Makepad shells. It preserves source schema,
field, and grid identity, applies only coordinate/row-limit adaptation, and
does not move ADF cells into the control plane.

GPU residency proofs follow the same ownership rule. The
`QuestMakepadGpuResidencyProof` boundary describes a bounded render-plane
payload that a Hostess or app-shell renderer is submitting to Makepad instanced
draw buffers. It records source schema, renderer id, row counts, dropped rows,
adapter payload bytes, and the selected Makepad instanced-draw backend. It is
not a compute contract, does not introduce renderer resources into Matter, and
does not move particle, SDF, ADF, mesh, or GPU buffers into settings/runtime
profiles/Android properties/command JSON. Quest-side measurement comes from
the app cadence markers that report repaint geometry uploads, instance counts,
and GPU repaint timing.

The camera-shell adapter also consumes `rusty.lattice.display_view_set.v1`
view sets and derives baseline `rusty.optics.video_projection_geometry.v1`
reports. Runtime adapters still own platform event loops and camera homography
inputs; this crate owns the app-facing bridge between clean Lattice/Optics
contracts and Quest Makepad behavior.

## Non-Ownership

- generic Makepad settings resolver;
- generic Makepad widgets and 2D Android apps;
- platform ADB/property writer authority;
- Matter mesh/SDF/collision/particle truth;
- high-rate particle or SDF data-plane authority;
- Manifold command/session authority.
