# Rusty Quest Makepad Provenance

## Mesh Replay

The `rusty-quest-makepad-mesh-replay` crate extracts the public synthetic mesh
replay parser/runtime from the previous Makepad camera shell lane into this
Quest-specific Makepad repo.

Reference pressure retained:

- input sequence schema remains Matter-owned:
  `rusty.matter.tools.glb_mesh_surface_sequence.v1`;
- Quest adapter marker schema remains
  `rusty.quest.makepad.mesh_replay.v1`;
- the current shader-panel edge overlay exports four representative segments
  as `[x0, y0, x1, y1]` uniforms.

The `rusty-quest-makepad-camera-shell` crate is the first app-facing adapter
slice extracted into this repo. It intentionally consumes the new effective
settings report instead of the previous launch/profile stack, which keeps replay
behavior deterministic and traceable.

Rejected overreach:

- no full app shell move in this slice;
- no full triangle renderer yet;
- no SDF/ADF, collision, or particle simulation authority in this repo;
- no dependency on the legacy source repo.
