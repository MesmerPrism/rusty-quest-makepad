use super::*;
use rusty_matter_mesh::{
    HandSkinningMatrixSample, HandSkinningMeshBufferOracle, SurfaceDistanceQueryDiagnostics,
};
use rusty_matter_model::Vec3;
use rusty_matter_surface_runtime::{MatterSurfaceParticleSample, MatterSurfaceParticleSnapshot};
use rusty_quest_makepad_mesh_replay::{MeshReplayConfig, MeshReplayRuntime, MeshReplaySequence};
use std::num::NonZeroUsize;

fn enabled_replay() -> MeshReplayRuntime {
    let mut replay = MeshReplayRuntime::default();
    replay.configure(MeshReplayConfig::normalized(
        true,
        "public-synthetic-hand-sequence".to_owned(),
        1.0,
        0.75,
    ));
    replay.step(0.0);
    replay
}

mod gpu_proofs;
mod surface_runtime;
