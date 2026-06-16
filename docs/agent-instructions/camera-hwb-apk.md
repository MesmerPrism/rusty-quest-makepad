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

The live headset performance envelope is part of the clean route, not a
Rusty-XR script dependency. `camera-hwb-live` stages
`debug.rustyquest.makepad.display.refresh.rate.hz=72.0` and
`debug.rustyquest.makepad.xr.render.scale=0.90`; the readiness command also
sets the Quest performance props to CPU 4, GPU 4, fixed foveation level 0, and
dynamic foveation disabled unless explicitly skipped. A full render scale of
`1.0` is a stress/parity run, not the default live HWB camera route.
If cadence reports a native `xrDisplayRefreshRateHz` that differs from the
requested frame cadence, the performance gate follows OpenXR
`predictedDisplayPeriodNs` and VrApi `FPS=current/target` evidence while still
recording the cadence value as a diagnostic.

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
- `route_gates.texture_metadata_ready=true`, with source frame, camera id,
  AHardwareBuffer id, import sequence, descriptor shape, and texture-update
  cadence reported under `texture_metadata`
- `route_gates.descriptor_color_ready=true`, with descriptor shape and YCbCr
  sampler-conversion metadata reported under `descriptor_color`
- `route_gates.video_texture_gate_ready=true` for the direct-HWB Vulkan
  equivalent video-texture gate; `open_gl_oes_companion_validated=false` remains
  explicit until a dedicated native/OES video run proves it
- `route_gates.shader_layout_visual_smoke_ready=true`, folded into the camera
  projection panel runtime smoke for the Makepad shader-layout fix
- `performance.performance_ready=true` with `FPS=72/72`, zero recent stale and
  tear samples, and observed display/effective frame rates near 72 Hz
- zero targeted fatal, ANR, GPU page-fault, or app-process signal lines

Projection-mapping and visual-release markers are reported separately under
`markers.projection_ready`. They are parity evidence, not the route-ownership
gate for moving the build/profile/launch path out of Rusty-XR.

## Lifecycle Stress Gate

Before trusting broader camera/video/HWB imports, run repeated launch/stop plus
pause/resume evidence through the clean script:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\Invoke-QuestMakepadCameraStressGate.ps1 `
  -Adb <adb-path> `
  -Serial <quest-serial> `
  -Cycles 3
```

The stress scorecard is
`local-artifacts/quest-makepad-camera-lifecycle-stress/lifecycle-stress-scorecard.json`.
It requires every cycle's route, metadata, descriptor/color, video-texture,
shader-layout visual-smoke, and performance gates to pass with no targeted
fatal lines. It reports imported hardware-buffer cache markers, retire markers,
video-import markers, pause/resume evidence, repeated force-stop/launch cycles,
and any observed out-of-date, suboptimal, or surface-lost Vulkan events.

Surface out-of-date/suboptimal/surface-lost events are opportunistic unless a
future test deliberately induces them; absence is recorded as not observed, not
as proof that recovery was exercised.

## Import Boundaries

Keep upstream Makepad XR and historical Rusty-XR branches as source maps and
mechanics references only. The clean APK/profile/readiness/stress route is
owned by `rusty-quest-makepad`, `rusty-quest`, and `makepad-morphospace`.

Do not take broad media/video imports until the lifecycle stress and
video-texture gates are clean. The OpenGL OES companion path remains a separate
validation target even when direct Vulkan HWB texture metadata is green.
