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

The camera-shell adapter re-exports the mesh replay runtime, uniforms, and
marker schema constants as the app-facing boundary. Hostess and other active
Makepad shells should consume that adapter surface instead of depending on the
lower replay crate directly or reparsing replay settings locally.

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
- Manifold command/session authority.
