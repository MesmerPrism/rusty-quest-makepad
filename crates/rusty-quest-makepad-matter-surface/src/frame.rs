use rusty_matter_surface_runtime::{
    MatterSurfaceContactProbeBatch, MatterSurfaceParticleSnapshot, MatterSurfaceRuntimeStats,
    MatterSurfaceRuntimeUpdate, MatterSurfaceStepDiagnostics,
};
use rusty_optics_mesh::SdfSliceVisual;
use rusty_optics_particles::ParticleVisualFrame;

use crate::{
    world_adf_debug_batch_from_frame, world_particle_batch_from_upload, QuestMakepadAdfDebugFrame,
    QuestMakepadCollisionUpload, QuestMakepadDistanceSliceUpload, QuestMakepadGpuMeshSdfProbeInput,
    QuestMakepadGpuSkinningMeshProbeInput, QuestMakepadGpuSkinningProbeInput,
    QuestMakepadMatterSurfaceProviderShape, QuestMakepadParticleUpload,
    QuestMakepadWorldAdfDebugBatch, QuestMakepadWorldAdfDebugPlacement,
    QuestMakepadWorldParticleBatch, QuestMakepadWorldParticlePlacement,
};

/// Adapter-stage timings for one Matter-backed surface frame.
///
/// These are compact evidence fields for performance classification. They are
/// not a simulation contract and they do not carry high-rate particle or mesh
/// data.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadMatterSurfaceStageTimings {
    /// Total adapter wall-clock time for the frame.
    pub total_ms: f32,
    /// Native Matter frame update time, including current distance sampler work.
    pub matter_update_ms: f32,
    /// Particle reset time when a reset was needed.
    pub particle_reset_ms: f32,
    /// Native Matter particle step time.
    pub particle_step_ms: f32,
    /// Collision probe time.
    pub collision_probe_ms: f32,
    /// Collision row packing time.
    pub collision_upload_ms: f32,
    /// SDF grid and debug visual build time.
    pub sdf_build_ms: f32,
    /// SDF row packing time.
    pub sdf_upload_ms: f32,
    /// ADF build time after the source SDF grid exists.
    pub adf_build_ms: f32,
    /// Optics ADF debug visual conversion time.
    pub adf_visual_ms: f32,
    /// Particle snapshot readout time.
    pub particle_snapshot_ms: f32,
    /// Matter particle render payload build time.
    pub particle_payload_ms: f32,
    /// Optics particle visual frame conversion time.
    pub particle_visual_ms: f32,
    /// Particle row packing time.
    pub particle_upload_ms: f32,
}

/// Adapter frame output.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadMatterSurfaceFrame {
    /// Stable source identity for this frame.
    pub source_id: String,
    /// Provider shape used before this frame entered Matter.
    pub source_provider_shape: QuestMakepadMatterSurfaceProviderShape,
    /// Source-space bounds minimum for render placement and evidence consumers.
    pub source_bounds_min: [f32; 3],
    /// Source-space bounds maximum for render placement and evidence consumers.
    pub source_bounds_max: [f32; 3],
    /// Source-space radius used by Matter particle reset policy.
    pub source_bounds_radius: f32,
    /// Runtime update for the current source surface.
    pub matter_update: MatterSurfaceRuntimeUpdate,
    /// Runtime stats after the frame step.
    pub stats: MatterSurfaceRuntimeStats,
    /// Collision probe batch.
    pub collision: MatterSurfaceContactProbeBatch,
    /// Collision upload rows.
    pub collision_upload: QuestMakepadCollisionUpload,
    /// Optional renderer-neutral SDF slice visual.
    pub sdf_slice: Option<SdfSliceVisual>,
    /// Optional SDF slice upload rows.
    pub sdf_slice_upload: Option<QuestMakepadDistanceSliceUpload>,
    /// Optional Matter-backed Optics ADF debug visual.
    pub adf_debug: Option<QuestMakepadAdfDebugFrame>,
    /// Whether SDF/ADF debug payloads were reused from an earlier source frame.
    pub sdf_adf_debug_reused: bool,
    /// Source frame index that produced the current SDF/ADF debug payloads.
    pub sdf_adf_debug_source_frame_index: Option<usize>,
    /// Configured SDF/ADF debug rebuild interval in source frames.
    pub sdf_adf_debug_update_interval_frames: usize,
    /// Typed particle snapshot with latest surface distances.
    pub particle_snapshot: MatterSurfaceParticleSnapshot,
    /// Matter-owned particle step diagnostics, if particles were stepped.
    pub particle_step: Option<MatterSurfaceStepDiagnostics>,
    /// Optional renderer-neutral particle visual frame.
    pub particle_visual_frame: Option<ParticleVisualFrame>,
    /// Optional particle upload rows.
    pub particle_upload: Option<QuestMakepadParticleUpload>,
    /// Optional bounded recorded-hand GPU skinning probe input.
    pub gpu_skinning_probe: Option<QuestMakepadGpuSkinningProbeInput>,
    /// Optional full recorded-hand GPU skinning mesh residency probe input.
    pub gpu_skinning_mesh_probe: Option<QuestMakepadGpuSkinningMeshProbeInput>,
    /// Optional bounded GPU mesh-to-dense-SDF probe input.
    pub gpu_mesh_sdf_probe: Option<QuestMakepadGpuMeshSdfProbeInput>,
    /// Adapter timing evidence for this frame.
    pub stage_timings: QuestMakepadMatterSurfaceStageTimings,
}

impl QuestMakepadMatterSurfaceFrame {
    /// Builds a world-object particle batch from this frame's particle upload.
    #[must_use]
    pub fn world_particle_batch(
        &self,
        bounds_min: [f32; 3],
        bounds_max: [f32; 3],
        placement: QuestMakepadWorldParticlePlacement,
        max_instances: usize,
    ) -> Option<QuestMakepadWorldParticleBatch> {
        let upload = self.particle_upload.as_ref()?;
        Some(world_particle_batch_from_upload(
            upload,
            bounds_min,
            bounds_max,
            placement,
            max_instances,
        ))
    }

    /// Builds a world-object ADF debug batch from this frame's ADF visual.
    #[must_use]
    pub fn world_adf_debug_batch(
        &self,
        placement: QuestMakepadWorldAdfDebugPlacement,
        max_cells: usize,
    ) -> Option<QuestMakepadWorldAdfDebugBatch> {
        let adf_debug = self.adf_debug.as_ref()?;
        Some(world_adf_debug_batch_from_frame(
            adf_debug, placement, max_cells,
        ))
    }
}
