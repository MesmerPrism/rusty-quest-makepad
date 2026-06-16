# Camera HWB Makepad Quest APK

This runbook is the clean owner route for the camera-enabled Makepad Quest APK.
Use it for APK build, `camera-hwb-live` profile staging, `.MakepadAppXr` launch,
and readiness scoring. Rusty-XR is historical compatibility evidence only; do
not route new camera-enabled Makepad Quest work through Rusty-XR scripts,
package names, or `rusty.xr.*` schemas.

## Owners

- APK build and Makepad runtime adapter: `rusty-quest-makepad`
- Android property/profile transport: `rusty-quest`
- Makepad fork dependency and packager source: `makepad-morphospace`
- Legacy Rusty-XR outputs: reference evidence only

Runtime settings consumed by this app must use the
`debug.rustyquest.makepad.*` Android property namespace. A successful generic
`debug.rustyquest.*` readback proves transport only; it is not app acceptance
evidence unless the Makepad runtime reports the matching effective marker.

## Build

From `S:\Work\repos\active\rusty-quest-makepad`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Build-QuestMakepadCameraApk.ps1 `
  -UseWindowsHost `
  -SdkPath <host-matched-sdk> `
  -MakepadSourceRoot S:\Work\repos\active\makepad-morphospace
```

The wrapper writes provenance to
`local-artifacts/quest-makepad-camera-apk/makepad-camera-apk-build-provenance.json`
and copies the built APK into `local-artifacts/quest-makepad-camera-apk`.

## Profile Bundle

Use `fixtures/profiles/camera-hwb-live.bundle.json`. The profile disables mesh,
collision, SDF/ADF, and particle slices, enables live camera streaming, selects
the direct hardware-buffer external texture path, uses the passthrough-underlay
border policy, keeps the peripheral-stretch projection layer active, and enables
projection-target joystick scale control.

Validate profile surfaces with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Test-QuestMakepadProfiles.ps1
```

## Headset Readiness

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Invoke-QuestMakepadCameraReadiness.ps1 `
  -Adb <adb-path> `
  -Serial <quest-serial>
```

The route-readiness scorecard must show:

- `ownership.legacy_reference_source_used=false`
- final required property readbacks matched
- `io.github.mesmerprism.rustyquest.makepad.camera/.MakepadAppXr` focused
- OpenXR frame submission markers with a nonzero layer count
- direct `cameraTexturePath=direct-camera-hardware-buffer-external`
- `makepadVulkanImport=true`
- paired left/right texture-update cadence
- zero targeted fatal, ANR, GPU page-fault, or app-process signal lines

Projection-mapping and visual-release markers are reported separately under
`markers.projection_ready`. They are parity evidence, not the route-ownership
gate for moving the build/profile/launch path out of Rusty-XR.
