# Hostess Quest Makepad APK Route

Use this route when validating Rusty Quest Makepad fixtures through the
installable Hostess Makepad APK.

## Build

Build from `rusty-hostess/apps/hostess-t-makepad` through the generated
Morphospace Makepad Quest manifest:

```powershell
& 'S:\Work\tools\Quest\Use-QuestTooling.ps1'
cargo install --path S:\Work\repos\active\makepad-morphospace\tools\cargo_makepad --force
cd S:\Work\repos\active\rusty-hostess\apps\hostess-t-makepad
cargo makepad android --variant=quest --abi=aarch64 --sdk-path="$env:ANDROID_HOME" --package-name=io.github.mesmerprism.rustyhostess.makepad --app-label="Rusty Hostess Makepad" --quest-camera-permissions=false build -p hostess-t-makepad
```

`--variant=quest` is required for `.MakepadAppXr` and OpenXR broker metadata.
`--quest-camera-permissions=false` is the camera-free particle/SDF smoke path;
camera streaming remains controlled by effective settings. Do not create an
app-local `AndroidManifest.xml.template` to remove camera permissions.

## Stage Settings

Before launching the APK, stage the generated effective-settings bundle into
the Hostess app-private path with the Hostess helper:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File S:\Work\repos\active\rusty-hostess\tools\Stage-HostessMakepadSettings.ps1 `
  -BundleDir S:\Work\repos\active\rusty-quest-makepad\local-artifacts\quest-makepad-runtime-bundle-recorded-left-particles
```

The helper uses `/data/local/tmp` as the ADB-visible staging hop, then `run-as`
to copy into `files/hostess-t/settings`. Do not use
`/sdcard/Android/data/...` as the app/ADB handoff path for replay,
recorded-hand, stimulus, or settings payloads.

The helper also writes `makepad-effective-settings.revision.json`. New settings
work should preserve the shared invalidation policy: writers publish a tiny
global/scoped revision sidecar, Hostess runtime hotload checks compare the
sidecar first, and detailed JSON parsing or subsystem rebuilds happen only
after a relevant scope hash changes. Path/mtime remains a fallback for older
bundles.

## Launch And Judge Evidence

Launch headset evidence through the generated Quest/XR activity:

```powershell
adb shell am start -W -n io.github.mesmerprism.rustyhostess.makepad/.MakepadAppXr
```

`$package/.MakepadApp` is the Android launcher activity and may work as a
fallback, but it is not the canonical Quest evidence launch. Do not use legacy
`dev.makepad.android.MakepadApp` for this generated Morphospace package.

Do not interpret an unstaged or `not_configured` Hostess receipt as an adapter
runtime failure until the app-private settings file has been staged.

