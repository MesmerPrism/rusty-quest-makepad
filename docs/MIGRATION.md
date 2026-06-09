# Quest Makepad Migration

This repo is the active Morphospace home for Quest-specific Makepad app
adapters.

## Migrated

- Quest Makepad profile bundles now target the canonical Makepad settings
  surface.
- Mesh replay parser/runtime lives in `rusty-quest-makepad-mesh-replay`.
- The camera shell replay adapter lives in
  `rusty-quest-makepad-camera-shell` and consumes
  `rusty.gui.makepad.effective_settings.v1`.

## Not Copied Wholesale

The previous camera shell mixed several authorities in one app tree. The active
repo should not preserve that coupling by depending on the previous source repo.
Remaining extraction should happen by ownership lane:

- camera source descriptors, projection plans, and homography metrics:
  `rusty-optics`;
- tracked spaces, eye views, poses, and runtime capability snapshots:
  `rusty-lattice`;
- generic Makepad settings resolution, hotload files, and readback reports:
  `rusty-makepad`;
- Quest property transport, Android lifecycle settings, and runtime profiles:
  `rusty-quest`;
- reusable 2D widgets and Android Makepad UI surfaces:
  `rusty-gui` and `rusty-makepad`;
- app-specific Quest headset behavior:
  `rusty-quest-makepad`.

## Acceptance Gate

An active Quest Makepad app crate is migrated only when:

- it consumes the effective settings report, not hand-authored launch values;
- external properties and launch flags are transport entries over the same
  canonical setting ids;
- it has local fixtures or generated evidence for the effective settings it
  applies;
- it has no dependency on the previous source repo;
- its validation runs through `tools/check_all.ps1`.
