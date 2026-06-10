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
the Quest Makepad surface slice. It consumes mesh replay frames, steps
`rusty-matter-surface-runtime`, packages bounded Makepad-facing rows for
distance slices, collision contacts, and particles, and uses Optics crates for
renderer-neutral visuals. It is not simulation authority.

`rusty-quest-makepad-camera-shell` is the app-facing adapter slice. It consumes
the canonical effective-settings report for the published camera-shell surface
and configures mesh replay, render scale, collision, SDF/ADF overlay, and
particle toggles without depending on the previous source repo or hand-authored
launch values.

## Validation

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

See `docs/PROVENANCE.md` for extraction boundaries.
See `docs/MIGRATION.md` for the remaining app-shell extraction map.
