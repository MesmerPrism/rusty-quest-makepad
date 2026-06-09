# Rusty Quest Makepad

Rusty Quest Makepad is the Morphospace lane for Quest-specific Makepad headset
apps and adapters.

The initial slice defines the Quest Makepad camera shell settings surface and
profile bundle used to move mesh replay and future SDF/ADF, collision, and
particle controls out of ad hoc launch settings.

It also contains `rusty-quest-makepad-mesh-replay`, a reusable parser/runtime
for Matter-owned recorded mesh surface sequences.

`rusty-quest-makepad-camera-shell` is the app-facing adapter slice. It consumes
canonical effective settings and configures the mesh replay runtime without
depending on the previous source repo or hand-authored launch values.

## Validation

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\check_all.ps1
```

See `docs/PROVENANCE.md` for extraction boundaries.
See `docs/MIGRATION.md` for the remaining app-shell extraction map.
