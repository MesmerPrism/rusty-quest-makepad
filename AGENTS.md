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
transports live in `rusty-quest`; Matter, Optics, Lattice, and Manifold keep
their source-of-truth lanes.

## Required Skills

Use `$rusty-morphospace-context` for repo-family routing, naming, and
public/private boundary decisions. Use `$system-engineering` for architecture,
contracts, manifests, adapter boundaries, observability, and validation design.
Use `$rust-work-graph` for broad repo inventory, graph snapshots, and
instruction-surface audits. Use `$meta-quest-workflow` only for headset, ADB,
APK, logcat, screenshot, Perfetto, or Wi-Fi ADB work.

## Read Order

1. `README.md`
2. `docs/ARCHITECTURE.md`
3. `docs/VALIDATION.md`
4. `fixtures/README.md`
5. `docs/agent-instructions/README.md`

Open detailed agent runbooks only when they match the task:

- `docs/agent-instructions/hostess-quest-apk.md`: Proven Hostess Quest Makepad
  APK Route, app-private settings staging, generated Quest activity launch, and
  settings invalidation guardrails.
- `docs/agent-instructions/recorded-hand-particles.md`: recorded full
  hand-mesh replay, local data-plane artifacts, visual particle smoke, and
  density sweep rules.
- `docs/agent-instructions/adf-gpu-evidence.md`: ADF/field-force boundaries,
  GPU residency/storage/readback markers, and force-authority promotion gates.

## Validation

Run the repo gate before committing:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

For focused native Matter adapter work, also use:

```powershell
cargo test -p rusty-quest-makepad-mesh-replay
cargo test -p rusty-quest-makepad-matter-surface
cargo test -p rusty-quest-makepad-matter-surface surface_runtime
cargo test -p rusty-quest-makepad-matter-surface gpu_proofs
```

## First-Hop Guardrails

- Do not add legacy Rusty-XR schema IDs, routes, env names, or project names as
  new authority. Preserve legacy lineage in documentation only when needed.
- Do not move high-rate hand meshes, compact joint frames, SDF/ADF cells,
  particles, media frames, or GPU buffers into settings, Android properties,
  runtime profiles, or command/control JSON.
- Keep generated replay sequences, recorded hand captures, GLBs, headset logs,
  and performance evidence under `local-artifacts` or another external
  evidence path; do not commit them.
- Treat `makepad-effective-settings.revision.json` as the settings hotload
  sidecar. Runtime consumers should treat file watch events, mtimes, and
  launch wakeups as hints, compare the global revision/hash, then compare only
  the scoped hashes they own before parsing detailed effective settings.
- Hostess APK validation must use the generated Morphospace Makepad Quest
  manifest and `.MakepadAppXr` Quest activity. Do not add an app-local Android
  manifest template just to remove camera permissions.
- Use `--quest-camera-permissions=false` for the camera-free particle/SDF smoke
  path; camera streaming remains controlled by effective settings.
- Stage Hostess Makepad settings through the Hostess helper, which uses
  `/data/local/tmp` plus `run-as` into `files/hostess-t/settings`. Do not use
  `/sdcard/Android/data/...` as the replay/settings handoff path.
- Matter remains distance, collision, SDF, ADF, particle, and CPU-oracle
  authority. Quest-Makepad adapts those results into Makepad-facing rows,
  runtime markers, and bounded GPU proof markers.
- Optics remains renderer-neutral visual/stimulus authority. Quest-Makepad may
  stage Optics payloads and prove bounded Vulkan readbacks, but it must not
  copy profile bodies into settings or claim runtime-scale GPU readiness.
- GPU residency, storage-buffer, skinning, mesh-SDF, and force-authority
  markers are bounded proof/evidence checkpoints. They do not authorize runtime
  GPU simulation until the explicit force-authority promotion gates are true.
- A GPU force profile gate may report `profileGateSatisfied=true` only when
  settings explicitly request
  `makepad.particles.force.authority=gpu-dense-sdf-field-particle-force`.
  Until residency, freshness, cadence, CPU-oracle comparison, live/recorded
  provider A/B, and rollback evidence are present, Matter CPU remains the
  active force authority.
