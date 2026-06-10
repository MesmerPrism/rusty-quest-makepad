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
