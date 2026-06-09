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

