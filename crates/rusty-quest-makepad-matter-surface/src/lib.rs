//! Quest Makepad adapter for native Matter animated-surface runtime output.
//!
//! This crate converts replay frames and Matter runtime payloads into bounded
//! app-facing rows and renderer-neutral Optics visuals. It does not own
//! simulation truth, settings resolution, Android property transport, or
//! Makepad backend resources.

mod adf;
mod adf_world;
mod geometry;
mod gpu_residency;
mod recorded_hand_source;
mod uploads;
mod worker;

use core::fmt;
use std::{num::NonZeroUsize, time::Instant};

use adf::{adf_debug_frame_from_report, build_adf_report};
use rusty_matter_model::Vec3;
use rusty_matter_sdf::{MeshSdfSignMode, MeshToSdfConfig};
use rusty_matter_surface_runtime::{
    MatterSurfaceContactProbeBatch, MatterSurfaceFrameInput, MatterSurfaceParticleSnapshot,
    MatterSurfaceRuntime, MatterSurfaceRuntimeConfig, MatterSurfaceRuntimeError,
    MatterSurfaceRuntimeStats, MatterSurfaceRuntimeUpdate, MatterSurfaceStepDiagnostics,
    ParticleExecutionConfig, DEFAULT_PARTICLE_FORCE_UPDATE_INTERVAL_FRAMES,
    DEFAULT_SURFACE_RUNTIME_PARTICLE_COUNT, DEFAULT_SURFACE_RUNTIME_PARTICLE_SEED,
};
use rusty_optics_mesh::SdfSliceVisual;
use rusty_optics_model::OpticsError;
use rusty_optics_particles::{
    resolve_animated_particle_visual_frame, ParticleVisualAnimationProfile, ParticleVisualFrame,
};
use rusty_quest_makepad_mesh_replay::{MeshReplayError, MeshReplayRuntime};
use uploads::{
    collision_upload_from_batch, distance_slice_upload_from_visual,
    particle_render_payload_for_visual_limit, particle_upload_from_visual_frame,
};

pub use adf::{
    QuestMakepadAdfDebugConfig, QuestMakepadAdfDebugError, QuestMakepadAdfDebugFrame,
    QUEST_MAKEPAD_ADF_DEBUG_SCHEMA_ID, QUEST_MAKEPAD_ADF_DEBUG_VISUAL_ID,
};
pub use adf_world::{
    world_adf_debug_batch_from_frame, QuestMakepadWorldAdfDebugBatch,
    QuestMakepadWorldAdfDebugCell, QuestMakepadWorldAdfDebugPlacement,
    QUEST_MAKEPAD_WORLD_ADF_DEBUG_BATCH_SCHEMA_ID,
    QUEST_MAKEPAD_WORLD_ADF_DEBUG_EVEN_SELECTION_POLICY,
    QUEST_MAKEPAD_WORLD_ADF_DEBUG_MARKER_PREFIX, QUEST_MAKEPAD_WORLD_ADF_DEBUG_RENDER_MODE,
};
pub use gpu_residency::{
    QuestMakepadGpuComputePreflight, QuestMakepadGpuComputeResourceKind,
    QuestMakepadGpuFieldForceProbe, QuestMakepadGpuFieldForceProbeReadback,
    QuestMakepadGpuOracleComputeProbe, QuestMakepadGpuOracleComputeProbeReadback,
    QuestMakepadGpuResidencyPayloadKind, QuestMakepadGpuResidencyProof,
    QuestMakepadGpuStorageProbe, QuestMakepadGpuStorageProbeReadback,
    QUEST_MAKEPAD_ADF_DEBUG_GPU_RESIDENCY_ROW_STRIDE_BYTES,
    QUEST_MAKEPAD_GPU_COMPUTE_DEFAULT_READBACK_PROBE_COUNT,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_BACKEND_STATUS,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_READBACK_POLICY,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_SCHEMA_ID, QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_BACKEND,
    QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_PAYLOAD, QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_BACKEND,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_PAYLOAD,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_SCHEMA_ID, QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS,
    QUEST_MAKEPAD_GPU_RESIDENCY_BACKEND_MAKEPAD_INSTANCED_DRAW,
    QUEST_MAKEPAD_GPU_RESIDENCY_MARKER_PREFIX, QUEST_MAKEPAD_GPU_RESIDENCY_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_RESIDENCY_PROOF_SCHEMA_ID, QUEST_MAKEPAD_GPU_RESIDENCY_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_STORAGE_PROBE_BACKEND, QUEST_MAKEPAD_GPU_STORAGE_PROBE_DEFAULT_BYTES,
    QUEST_MAKEPAD_GPU_STORAGE_PROBE_DEFAULT_PATTERN, QUEST_MAKEPAD_GPU_STORAGE_PROBE_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_STORAGE_PROBE_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_STORAGE_PROBE_RESOURCE_PLANE, QUEST_MAKEPAD_GPU_STORAGE_PROBE_SCHEMA_ID,
    QUEST_MAKEPAD_PARTICLE_GPU_RESIDENCY_ROW_STRIDE_BYTES,
};
pub use rusty_matter_surface_runtime::{
    MatterSurfaceContactProbe, MatterSurfaceParticleDistanceRefreshPolicy,
    MatterSurfaceParticleForceRefresh, MatterSurfaceParticleForceSource,
    MatterSurfaceParticleForceSourceStatus, ParticleExecutionBackend,
};
pub use uploads::{
    world_particle_batch_from_upload, QuestMakepadCollisionRow, QuestMakepadCollisionUpload,
    QuestMakepadDistanceSliceRow, QuestMakepadDistanceSliceUpload, QuestMakepadParticleRow,
    QuestMakepadParticleUpload, QuestMakepadWorldParticleBatch, QuestMakepadWorldParticleInstance,
    QuestMakepadWorldParticlePlacement,
};
pub use worker::{
    QuestMakepadMatterSurfaceWorker, QuestMakepadMatterSurfaceWorkerError,
    QuestMakepadMatterSurfaceWorkerFrame, QuestMakepadMatterSurfaceWorkerOutput,
    QuestMakepadMatterSurfaceWorkerStats, QUEST_MAKEPAD_MATTER_SURFACE_WORKER_MARKER_PREFIX,
    QUEST_MAKEPAD_MATTER_SURFACE_WORKER_SCHEMA_ID,
};

pub(crate) use geometry::{bounds_max_half_extent, bounds_radius, midpoint, vec3_length};

/// Quest Makepad Matter surface marker schema.
pub const QUEST_MAKEPAD_MATTER_SURFACE_SCHEMA_ID: &str =
    "rusty.quest.makepad.matter_surface_runtime.v1";
/// Quest Makepad Matter surface marker prefix.
pub const QUEST_MAKEPAD_MATTER_SURFACE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_MATTER_SURFACE_RUNTIME";
/// Quest Makepad Matter distance slice upload schema.
pub const QUEST_MAKEPAD_DISTANCE_SLICE_UPLOAD_SCHEMA_ID: &str =
    "rusty.quest.makepad.matter_distance_slice_upload.v1";
/// Quest Makepad Matter collision upload schema.
pub const QUEST_MAKEPAD_COLLISION_UPLOAD_SCHEMA_ID: &str =
    "rusty.quest.makepad.matter_collision_upload.v1";
/// Quest Makepad Matter particle upload schema.
pub const QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID: &str =
    "rusty.quest.makepad.matter_particle_upload.v1";
/// Quest Makepad world-particle batch schema.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID: &str =
    "rusty.quest.makepad.world_particle_batch.v1";
/// Quest Makepad world-particle marker prefix.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_MARKER_PREFIX: &str = "RUSTY_QUEST_MAKEPAD_WORLD_PARTICLES";
/// Start-head-local coordinate space for first-visibility headset smoke tests.
pub const QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE: &str = "makepad-xr-start-head-local";
/// Makepad XR content-local coordinate space.
///
/// Host shells that render inside `XrRoot` should use this space when the root
/// already applies the initial headset-relative content pose.
pub const QUEST_MAKEPAD_CONTENT_LOCAL_SPACE: &str = "makepad-xr-content-local";
/// Initial world-particle render mode.
pub const QUEST_MAKEPAD_CENTER_PROJECTED_BILLBOARD_MODE: &str = "center-projected-billboard";
/// Current Quest Makepad world-particle renderer identity.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_RENDERER_ID: &str =
    "makepad-xr-procedural-ring-billboard";
/// Current Quest Makepad world-particle animation mode.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_MODE: &str = "procedural-morph-ring";
/// Renderer-neutral Optics frame source used by the billboard animation.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_SOURCE: &str =
    "rusty-optics-particle-visual-frame";
/// Reference visual direction borrowed for the current smoke renderer.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_REFERENCE: &str =
    "rusty-viscereality-billboard-ring";
/// Selection policy used when the source particle upload is larger than the
/// current world-object proof renderer can draw.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_EVEN_SELECTION_POLICY: &str = "evenly-spaced-source-rows";

/// Browser-preview cloud-radius multiplier.
pub const DEFAULT_PARTICLE_CLOUD_RADIUS_SCALE: f32 = 2.45;
/// Browser-preview particle-radius multiplier.
pub const DEFAULT_PARTICLE_RADIUS_SCALE: f32 = 0.009;
/// Browser-preview minimum particle radius.
pub const DEFAULT_MIN_PARTICLE_RADIUS: f32 = 0.0012;
/// Default simulated-content center: about 0.5m in front of the initial camera pose.
pub const DEFAULT_WORLD_CONTENT_CENTER: [f32; 3] = [0.0, 0.0, -0.5];
/// Default displayed content radius in Makepad world units.
pub const DEFAULT_WORLD_CONTENT_TARGET_RADIUS: f32 = 0.16;
/// Default Matter particle execution batch size used by Quest Makepad profiles.
pub const DEFAULT_PARTICLE_EXECUTION_BATCH_SIZE: usize = 256;
/// Default SDF/ADF debug-field rebuild interval used by Quest Makepad profiles.
pub const DEFAULT_SDF_ADF_DEBUG_UPDATE_INTERVAL_FRAMES: usize = 1;

/// One animated hand/surface source frame ready for the native Matter runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadMatterSurfaceSourceFrame {
    /// Stable source identity, for example a recorded replay or realtime hand pair.
    pub source_id: String,
    /// Matter-owned surface frame input.
    pub frame: MatterSurfaceFrameInput,
    /// Source-space bounds minimum for reset/scaling policy.
    pub bounds_min: [f32; 3],
    /// Source-space bounds maximum for reset/scaling policy.
    pub bounds_max: [f32; 3],
    /// Source-space radius used for particle cloud sizing.
    pub bounds_radius: f32,
}

impl QuestMakepadMatterSurfaceSourceFrame {
    /// Creates a source frame from a Matter frame input and source bounds.
    #[must_use]
    pub fn new(
        source_id: impl Into<String>,
        frame: MatterSurfaceFrameInput,
        bounds_min: [f32; 3],
        bounds_max: [f32; 3],
    ) -> Self {
        Self {
            source_id: source_id.into(),
            frame,
            bounds_min,
            bounds_max,
            bounds_radius: bounds_max_half_extent(bounds_min, bounds_max),
        }
    }

    /// Creates a source frame from the current replay frame.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when replay frame conversion
    /// fails.
    pub fn from_replay(replay: &MeshReplayRuntime) -> Result<Self, QuestMakepadMatterSurfaceError> {
        let sequence = replay.sequence();
        let source_id = if replay.config().source.trim().is_empty() {
            sequence.sequence_id().to_owned()
        } else {
            replay.config().source.clone()
        };
        Ok(Self {
            source_id,
            frame: MatterSurfaceFrameInput::new(
                replay.current_frame_index(),
                replay.playback_seconds().max(0.0),
                replay.current_surface()?,
            ),
            bounds_min: sequence.bounds_min(),
            bounds_max: sequence.bounds_max(),
            bounds_radius: sequence.bounds_radius(),
        })
    }

    fn bounds_center(&self) -> Vec3 {
        Vec3::new(
            (self.bounds_min[0] + self.bounds_max[0]) * 0.5,
            (self.bounds_min[1] + self.bounds_max[1]) * 0.5,
            (self.bounds_min[2] + self.bounds_max[2]) * 0.5,
        )
    }

    fn surface_radius(&self) -> f32 {
        self.bounds_radius.max(0.001)
    }
}

/// Quest Makepad native Matter surface adapter config.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadMatterSurfaceConfig {
    /// Whether the native Matter surface runtime is enabled.
    pub enabled: bool,
    /// Whether collision probe rows should be emitted.
    pub collision_enabled: bool,
    /// Whether SDF slice rows should be emitted.
    pub sdf_slice_enabled: bool,
    /// Whether a Matter-backed ADF debug visual should be emitted.
    pub adf_debug_enabled: bool,
    /// ADF debug build configuration.
    pub adf_debug_config: QuestMakepadAdfDebugConfig,
    /// Source-frame interval for SDF/ADF debug-field rebuilds.
    ///
    /// Values above one reuse the last SDF/ADF debug payload between rebuilds.
    /// This affects renderer/debug output only; Matter surface update,
    /// distance sampling, collisions, and particles still consume the current
    /// source frame.
    pub sdf_adf_debug_update_interval_frames: NonZeroUsize,
    /// Whether particles should be stepped and emitted.
    pub particles_enabled: bool,
    /// Particle count for deterministic resets.
    pub particle_count: usize,
    /// Particle reset seed.
    pub particle_seed: u32,
    /// Policy for extra per-particle snapshot distance refreshes.
    pub particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy,
    /// Selected particle force source.
    pub particle_force_source: MatterSurfaceParticleForceSource,
    /// Frame interval for refreshing the selected particle force source.
    pub particle_force_update_interval_frames: NonZeroUsize,
    /// Bounded compare-probe count for future mesh/field diagnostics.
    pub particle_force_compare_probe_count: usize,
    /// Low-rate Matter particle execution backend.
    pub particle_execution_backend: ParticleExecutionBackend,
    /// Logical Matter particle execution batch size.
    pub particle_execution_batch_size: NonZeroUsize,
    /// Optional Matter particle worker cap; `None` lets the backend choose.
    pub particle_execution_max_threads: Option<usize>,
    /// Optional cap for elapsed time simulated by one particle frame.
    pub particle_max_frame_delta_seconds: Option<f32>,
    /// Optional cap for Optics/Makepad visual rows derived from Matter particles.
    ///
    /// This does not change Matter particle truth or simulation count. It only
    /// bounds renderer-facing projection work for current Makepad draw caps.
    pub particle_visual_row_limit: Option<usize>,
    /// Maximum triangles in a surface-distance leaf.
    pub leaf_triangle_count: usize,
    /// SDF slice voxel size when slice output is enabled.
    pub sdf_voxel_size: f32,
    /// SDF slice padding voxels.
    pub sdf_padding_voxels: u32,
    /// SDF slice maximum voxel budget.
    pub sdf_max_voxels: usize,
}

impl Default for QuestMakepadMatterSurfaceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            collision_enabled: false,
            sdf_slice_enabled: false,
            adf_debug_enabled: false,
            adf_debug_config: QuestMakepadAdfDebugConfig::default(),
            sdf_adf_debug_update_interval_frames: NonZeroUsize::new(
                DEFAULT_SDF_ADF_DEBUG_UPDATE_INTERVAL_FRAMES,
            )
            .expect("default SDF/ADF debug update interval is non-zero"),
            particles_enabled: false,
            particle_count: DEFAULT_SURFACE_RUNTIME_PARTICLE_COUNT,
            particle_seed: DEFAULT_SURFACE_RUNTIME_PARTICLE_SEED,
            particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::StepOnly,
            particle_force_source: MatterSurfaceParticleForceSource::MeshDistance,
            particle_force_update_interval_frames: NonZeroUsize::new(
                DEFAULT_PARTICLE_FORCE_UPDATE_INTERVAL_FRAMES,
            )
            .expect("default particle force update interval is non-zero"),
            particle_force_compare_probe_count: 0,
            particle_execution_backend: ParticleExecutionBackend::Serial,
            particle_execution_batch_size: NonZeroUsize::new(DEFAULT_PARTICLE_EXECUTION_BATCH_SIZE)
                .expect("default particle execution batch size is non-zero"),
            particle_execution_max_threads: None,
            particle_max_frame_delta_seconds: None,
            particle_visual_row_limit: None,
            leaf_triangle_count: 8,
            sdf_voxel_size: 0.05,
            sdf_padding_voxels: 1,
            sdf_max_voxels: 65_536,
        }
    }
}

impl QuestMakepadMatterSurfaceConfig {
    /// Creates a Matter runtime config for this adapter config.
    #[must_use]
    pub fn to_matter_config(&self) -> MatterSurfaceRuntimeConfig {
        let mut config = MatterSurfaceRuntimeConfig::default();
        config.runtime_id = "quest.makepad.matter_surface".to_owned();
        config.distance_sampler.leaf_triangle_count = self.leaf_triangle_count;
        config.collider.enabled = self.collision_enabled;
        config.particle_distance_refresh_policy = self.particle_distance_refresh_policy;
        config.particle_force_source = self.particle_force_source;
        config.particle_force_update_interval_frames = self.particle_force_update_interval_frames;
        config.particle_force_compare_probe_count = self.particle_force_compare_probe_count;
        config.particle_force_sdf = self.sdf_config();
        config.particle_force_adf = self.adf_debug_config.to_matter_config();
        config.particles.execution = ParticleExecutionConfig {
            backend: self.particle_execution_backend,
            batch_size: self.particle_execution_batch_size,
            max_threads: self.particle_execution_max_threads,
        };
        config.particles.max_frame_delta_seconds = self.particle_max_frame_delta_seconds;
        config
    }

    /// SDF builder config used for optional slice output.
    #[must_use]
    pub fn sdf_config(&self) -> MeshToSdfConfig {
        MeshToSdfConfig {
            voxel_size: self.sdf_voxel_size,
            padding_voxels: self.sdf_padding_voxels,
            max_voxels: self.sdf_max_voxels,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        }
    }
}

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

/// Quest Makepad Matter surface adapter runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadMatterSurfaceRuntime {
    config: QuestMakepadMatterSurfaceConfig,
    matter: MatterSurfaceRuntime,
    particle_profile: ParticleVisualAnimationProfile,
    particles_initialized: bool,
    sdf_adf_debug_frame_counter: usize,
    sdf_adf_debug_cache: Option<QuestMakepadSdfAdfDebugCache>,
}

impl QuestMakepadMatterSurfaceRuntime {
    /// Creates an adapter runtime.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when the Matter runtime
    /// config is invalid.
    pub fn new(
        config: QuestMakepadMatterSurfaceConfig,
    ) -> Result<Self, QuestMakepadMatterSurfaceError> {
        let matter = MatterSurfaceRuntime::new(config.to_matter_config())?;
        Ok(Self {
            config,
            matter,
            particle_profile: ParticleVisualAnimationProfile::new(
                "quest.makepad.particles.browser_parity",
            ),
            particles_initialized: false,
            sdf_adf_debug_frame_counter: 0,
            sdf_adf_debug_cache: None,
        })
    }

    /// Returns the adapter config.
    #[must_use]
    pub fn config(&self) -> &QuestMakepadMatterSurfaceConfig {
        &self.config
    }

    /// Returns the underlying Matter runtime.
    #[must_use]
    pub fn matter_runtime(&self) -> &MatterSurfaceRuntime {
        &self.matter
    }

    /// Steps from the current mesh replay frame.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when replay, Matter, or
    /// Optics payload construction fails.
    pub fn step_from_replay(
        &mut self,
        replay: &MeshReplayRuntime,
        delta_seconds: f32,
        probes: &[MatterSurfaceContactProbe],
    ) -> Result<QuestMakepadMatterSurfaceFrame, QuestMakepadMatterSurfaceError> {
        self.step_from_source_frame(
            QuestMakepadMatterSurfaceSourceFrame::from_replay(replay)?,
            delta_seconds,
            probes,
        )
    }

    /// Steps from a source frame that already carries a Matter surface.
    ///
    /// Recorded replay and future realtime Quest hand-mesh providers should
    /// converge here so Matter remains the only SDF/collider/particle authority.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when Matter or Optics payload
    /// construction fails.
    pub fn step_from_source_frame(
        &mut self,
        source_frame: QuestMakepadMatterSurfaceSourceFrame,
        delta_seconds: f32,
        probes: &[MatterSurfaceContactProbe],
    ) -> Result<QuestMakepadMatterSurfaceFrame, QuestMakepadMatterSurfaceError> {
        let total_started_at = Instant::now();
        let mut stage_timings = QuestMakepadMatterSurfaceStageTimings::default();
        let center = source_frame.bounds_center();
        let surface_radius = source_frame.surface_radius();
        let cloud_radius = surface_radius * DEFAULT_PARTICLE_CLOUD_RADIUS_SCALE;
        let particle_radius =
            (surface_radius * DEFAULT_PARTICLE_RADIUS_SCALE).max(DEFAULT_MIN_PARTICLE_RADIUS);
        let source_id = source_frame.source_id.clone();

        let started_at = Instant::now();
        let matter_update = self.matter.update_frame(source_frame.frame)?;
        stage_timings.matter_update_ms = elapsed_ms(started_at);

        let mut particle_step = None;
        if self.config.enabled && self.config.particles_enabled {
            if !self.particles_initialized
                || self.matter.stats().particle_count != self.config.particle_count
            {
                let started_at = Instant::now();
                self.matter.reset_particles(
                    center,
                    self.config.particle_count,
                    cloud_radius,
                    particle_radius,
                    surface_radius,
                    self.config.particle_seed,
                )?;
                stage_timings.particle_reset_ms += elapsed_ms(started_at);
                self.particles_initialized = true;
            }
            let started_at = Instant::now();
            particle_step = Some(self.matter.step_particles(
                surface_radius,
                center,
                cloud_radius,
                delta_seconds.max(0.0),
            )?);
            stage_timings.particle_step_ms = elapsed_ms(started_at);
        }

        let started_at = Instant::now();
        let collision = if self.config.enabled && self.config.collision_enabled {
            self.matter.probe_contacts(probes)
        } else {
            self.matter.probe_contacts(&[])
        };
        stage_timings.collision_probe_ms = elapsed_ms(started_at);
        let started_at = Instant::now();
        let collision_upload = collision_upload_from_batch(&collision);
        stage_timings.collision_upload_ms = elapsed_ms(started_at);

        let needs_sdf_adf_debug =
            self.config.enabled && (self.config.sdf_slice_enabled || self.config.adf_debug_enabled);
        let (sdf_slice, sdf_slice_upload, adf_debug, sdf_adf_debug_reused, sdf_adf_debug_frame) =
            if needs_sdf_adf_debug {
                let interval = self.config.sdf_adf_debug_update_interval_frames.get();
                let should_rebuild = self.sdf_adf_debug_cache.is_none()
                    || self.sdf_adf_debug_frame_counter % interval == 0;
                self.sdf_adf_debug_frame_counter =
                    self.sdf_adf_debug_frame_counter.saturating_add(1);
                if should_rebuild {
                    let debug_frame = self
                        .build_sdf_adf_debug_frame(&mut stage_timings, matter_update.frame_index)?;
                    self.sdf_adf_debug_cache = Some(debug_frame.clone());
                    (
                        debug_frame.sdf_slice,
                        debug_frame.sdf_slice_upload,
                        debug_frame.adf_debug,
                        false,
                        debug_frame.source_frame_index,
                    )
                } else {
                    let debug_frame = self
                        .sdf_adf_debug_cache
                        .as_ref()
                        .expect("SDF/ADF debug cache exists when rebuild is skipped")
                        .clone();
                    (
                        debug_frame.sdf_slice,
                        debug_frame.sdf_slice_upload,
                        debug_frame.adf_debug,
                        true,
                        debug_frame.source_frame_index,
                    )
                }
            } else {
                (None, None, None, false, None)
            };

        let started_at = Instant::now();
        let particle_snapshot = self.matter.particle_snapshot();
        stage_timings.particle_snapshot_ms = elapsed_ms(started_at);
        let particle_source_rows = self.matter.stats().particle_count;
        let (particle_visual_frame, particle_upload) =
            if self.config.enabled && self.config.particles_enabled {
                let started_at = Instant::now();
                let payload = particle_render_payload_for_visual_limit(
                    &self.matter,
                    "quest.makepad.particles.current",
                    self.config.particle_visual_row_limit,
                )?;
                stage_timings.particle_payload_ms = elapsed_ms(started_at);
                let started_at = Instant::now();
                let frame = resolve_animated_particle_visual_frame(
                    "quest.makepad.particles.visual.current",
                    &payload,
                    &self.particle_profile,
                )?;
                stage_timings.particle_visual_ms = elapsed_ms(started_at);
                let started_at = Instant::now();
                let upload = particle_upload_from_visual_frame(&frame, particle_source_rows);
                stage_timings.particle_upload_ms = elapsed_ms(started_at);
                (Some(frame), Some(upload))
            } else {
                (None, None)
            };
        stage_timings.total_ms = elapsed_ms(total_started_at);

        Ok(QuestMakepadMatterSurfaceFrame {
            source_id,
            matter_update,
            stats: self.matter.stats(),
            collision,
            collision_upload,
            sdf_slice,
            sdf_slice_upload,
            adf_debug,
            sdf_adf_debug_reused,
            sdf_adf_debug_source_frame_index: sdf_adf_debug_frame,
            sdf_adf_debug_update_interval_frames: self
                .config
                .sdf_adf_debug_update_interval_frames
                .get(),
            particle_snapshot,
            particle_step,
            particle_visual_frame,
            particle_upload,
            stage_timings,
        })
    }

    /// Builds an evidence marker for a frame.
    #[must_use]
    pub fn marker_line(&self, phase: &str, frame: &QuestMakepadMatterSurfaceFrame) -> String {
        let particle_step = frame.particle_step.as_ref();
        let particle_surface_node_tests =
            particle_step.map_or(0, |diagnostics| diagnostics.particles.surface_node_tests);
        let particle_surface_leaf_tests =
            particle_step.map_or(0, |diagnostics| diagnostics.particles.surface_leaf_tests);
        let particle_surface_triangle_tests = particle_step.map_or(0, |diagnostics| {
            diagnostics.particles.surface_triangle_tests
        });
        let particle_refresh_node_tests = particle_step.map_or(0, |diagnostics| {
            diagnostics.refreshed_distance_diagnostics.node_tests
        });
        let particle_refresh_leaf_tests = particle_step.map_or(0, |diagnostics| {
            diagnostics.refreshed_distance_diagnostics.leaf_tests
        });
        let particle_refresh_triangle_tests = particle_step.map_or(0, |diagnostics| {
            diagnostics.refreshed_distance_diagnostics.triangle_tests
        });
        let adf_debug = frame.adf_debug.as_ref();
        let adf_status = match (self.config.adf_debug_enabled, adf_debug.is_some()) {
            (false, _) => "disabled",
            (true, true) => "ready",
            (true, false) => "empty",
        };
        let sdf_adf_debug_source = match (
            self.config.sdf_slice_enabled || self.config.adf_debug_enabled,
            frame.sdf_adf_debug_reused,
        ) {
            (false, _) => "disabled",
            (true, false) => "fresh",
            (true, true) => "reused",
        };
        let particle_force_source = frame.stats.particle_force_source;
        let particle_force_source_status = particle_step.map_or(
            if self.config.particles_enabled {
                "not-stepped"
            } else {
                "disabled"
            },
            |diagnostics| diagnostics.particle_force_source_status.marker_value(),
        );
        let particle_force_refresh = particle_step.map_or(
            if self.config.particles_enabled {
                "not-stepped"
            } else {
                "disabled"
            },
            |diagnostics| diagnostics.particle_force_refresh.marker_value(),
        );
        let sdf_adf_debug_particle_authority =
            particle_step.is_some_and(|diagnostics| diagnostics.sdf_adf_debug_particle_authority);
        format!(
            "{} schema={} phase={} status={} nativeMatterRuntime=true wasmRuntimeUsed=false shaderScaffoldUsed=false proceduralParticleOverlayUsed=false proceduralSdfOverlayUsed=false proceduralCollisionOverlayUsed=false dataPlane=makepad-compact-uniform-rows sourceId={} sourceSchema={} frameIndex={} vertexCount={} triangleCount={} sdfAdfDebugSource={} sdfAdfDebugFrameInterval={} sdfAdfDebugSourceFrameIndex={} particleCount={} particleForceSource={} particleForceSourceStatus={} particleForceRefresh={} particleForceUpdateIntervalFrames={} particleForceCompareProbeCount={} particleSamplingAuthority={} particleFieldSource={} sdfAdfDebugParticleAuthority={} particleDistanceRefreshPolicy={} particleDistanceSamples={} particleInputDeltaSeconds={:.6} particleSimulatedDeltaSeconds={:.6} particleDroppedDeltaSeconds={:.6} particleSubsteps={} particleClosestSamples={} particleSurfaceNodeTests={} particleSurfaceLeafTests={} particleSurfaceTriangleTests={} particleRefreshSamples={} particleRefreshNodeTests={} particleRefreshLeafTests={} particleRefreshTriangleTests={} particleExecutionBackend={} particleExecutionBatchSize={} particleExecutionChunks={} particleExecutionWorkers={} particleExecutionElapsedMicros={} collisionRows={} particleSourceRows={} particleRows={} particleVisualRowLimit={} sdfRows={} adfDebugEnabled={} adfStatus={} adfSchema={} adfVisualSchema={} adfCells={} adfSourceSamples={} adfSplitCount={} adfMaxLevel={} adfMaxDepth={} adfMaxCells={} adfErrorTolerance={:.6} leafTriangleCount={} distanceSamplerRefit={} adapterTotalMs={:.3} matterUpdateMs={:.3} particleResetMs={:.3} particleStepMs={:.3} collisionProbeMs={:.3} collisionUploadMs={:.3} sdfBuildMs={:.3} sdfUploadMs={:.3} adfBuildMs={:.3} adfVisualMs={:.3} particleSnapshotMs={:.3} particlePayloadMs={:.3} particleVisualMs={:.3} particleUploadMs={:.3}",
            QUEST_MAKEPAD_MATTER_SURFACE_MARKER_PREFIX,
            QUEST_MAKEPAD_MATTER_SURFACE_SCHEMA_ID,
            sanitize_marker_value(phase),
            if self.config.enabled { "ready" } else { "disabled" },
            sanitize_marker_value(&frame.source_id),
            frame.matter_update.topology_key.schema_id,
            frame.matter_update.frame_index.unwrap_or(0),
            frame.matter_update.vertex_count,
            frame.matter_update.triangle_count,
            sdf_adf_debug_source,
            frame.sdf_adf_debug_update_interval_frames,
            optional_usize_marker_token(frame.sdf_adf_debug_source_frame_index),
            frame.stats.particle_count,
            particle_force_source.marker_value(),
            particle_force_source_status,
            particle_force_refresh,
            frame.stats.particle_force_update_interval_frames,
            frame.stats.particle_force_compare_probe_count,
            particle_force_source.sampling_authority_marker(),
            particle_force_source.field_source_marker(),
            sdf_adf_debug_particle_authority,
            frame.stats.particle_distance_refresh_policy.marker_value(),
            frame.stats.particle_distance_samples,
            frame
                .particle_step
                .as_ref()
                .map_or(0.0, |diagnostics| diagnostics.particles.input_delta_seconds),
            frame
                .particle_step
                .as_ref()
                .map_or(0.0, |diagnostics| diagnostics.particles.simulated_delta_seconds),
            frame
                .particle_step
                .as_ref()
                .map_or(0.0, |diagnostics| diagnostics.particles.dropped_delta_seconds),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.substeps),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.closest_samples),
            particle_surface_node_tests,
            particle_surface_leaf_tests,
            particle_surface_triangle_tests,
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.refreshed_distance_samples),
            particle_refresh_node_tests,
            particle_refresh_leaf_tests,
            particle_refresh_triangle_tests,
            frame.particle_step.as_ref().map_or("none", |diagnostics| {
                diagnostics.particles.execution.backend.marker_value()
            }),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.execution.batch_size),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.execution.chunk_count),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.execution.worker_count),
            frame.particle_step.as_ref().map_or(0, |diagnostics| {
                diagnostics.particles.execution.elapsed_micros
            }),
            frame.collision_upload.rows.len(),
            frame
                .particle_upload
                .as_ref()
                .map_or(0, |upload| upload.source_rows),
            frame
                .particle_upload
                .as_ref()
                .map_or(0, |upload| upload.rows.len()),
            optional_usize_marker_token(self.config.particle_visual_row_limit),
            frame
                .sdf_slice_upload
                .as_ref()
                .map_or(0, |upload| upload.rows.len()),
            self.config.adf_debug_enabled,
            adf_status,
            adf_debug.map_or("none", |frame| frame.schema_id.as_str()),
            adf_debug.map_or("none", |frame| frame.visual_schema_id.as_str()),
            adf_debug.map_or(0, |frame| frame.visual.cell_count),
            adf_debug.map_or(0, |frame| frame.diagnostics.source_sample_count),
            adf_debug.map_or(0, |frame| frame.diagnostics.split_count),
            adf_debug.map_or(0, |frame| frame.diagnostics.max_level),
            self.config.adf_debug_config.max_depth,
            self.config.adf_debug_config.max_cells,
            self.config.adf_debug_config.error_tolerance,
            frame
                .stats
                .distance_sampler
                .as_ref()
                .map_or(0, |stats| stats.leaf_triangle_count),
            frame.matter_update.distance_sampler_refit,
            frame.stage_timings.total_ms,
            frame.stage_timings.matter_update_ms,
            frame.stage_timings.particle_reset_ms,
            frame.stage_timings.particle_step_ms,
            frame.stage_timings.collision_probe_ms,
            frame.stage_timings.collision_upload_ms,
            frame.stage_timings.sdf_build_ms,
            frame.stage_timings.sdf_upload_ms,
            frame.stage_timings.adf_build_ms,
            frame.stage_timings.adf_visual_ms,
            frame.stage_timings.particle_snapshot_ms,
            frame.stage_timings.particle_payload_ms,
            frame.stage_timings.particle_visual_ms,
            frame.stage_timings.particle_upload_ms,
        )
    }

    fn build_sdf_adf_debug_frame(
        &self,
        stage_timings: &mut QuestMakepadMatterSurfaceStageTimings,
        source_frame_index: Option<usize>,
    ) -> Result<QuestMakepadSdfAdfDebugCache, QuestMakepadMatterSurfaceError> {
        let started_at = Instant::now();
        let grid = self.matter.build_sdf_grid(self.config.sdf_config())?;
        stage_timings.sdf_build_ms = elapsed_ms(started_at);
        let (sdf_slice, sdf_slice_upload) = if self.config.sdf_slice_enabled {
            let slice = SdfSliceVisual::middle_z("quest.makepad.sdf_slice.middle_z", &grid)?;
            let started_at = Instant::now();
            let upload = distance_slice_upload_from_visual(&slice);
            stage_timings.sdf_upload_ms = elapsed_ms(started_at);
            (Some(slice), Some(upload))
        } else {
            (None, None)
        };
        let adf_debug = if self.config.adf_debug_enabled {
            let started_at = Instant::now();
            let report = build_adf_report(&grid, self.config.adf_debug_config)?;
            stage_timings.adf_build_ms = elapsed_ms(started_at);
            let started_at = Instant::now();
            let frame = adf_debug_frame_from_report(report)?;
            stage_timings.adf_visual_ms = elapsed_ms(started_at);
            Some(frame)
        } else {
            None
        };
        Ok(QuestMakepadSdfAdfDebugCache {
            source_frame_index,
            sdf_slice,
            sdf_slice_upload,
            adf_debug,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct QuestMakepadSdfAdfDebugCache {
    source_frame_index: Option<usize>,
    sdf_slice: Option<SdfSliceVisual>,
    sdf_slice_upload: Option<QuestMakepadDistanceSliceUpload>,
    adf_debug: Option<QuestMakepadAdfDebugFrame>,
}

impl Default for QuestMakepadMatterSurfaceRuntime {
    fn default() -> Self {
        Self::new(QuestMakepadMatterSurfaceConfig::default()).expect("default config is valid")
    }
}

/// Adapter failure.
#[derive(Clone, Debug, PartialEq)]
pub enum QuestMakepadMatterSurfaceError {
    /// Replay frame conversion failed.
    MeshReplay(MeshReplayError),
    /// Matter runtime failed.
    Matter(MatterSurfaceRuntimeError),
    /// ADF debug payload failed.
    Adf(QuestMakepadAdfDebugError),
    /// Optics visual payload failed.
    Optics(OpticsError),
}

impl fmt::Display for QuestMakepadMatterSurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MeshReplay(error) => write!(formatter, "{error}"),
            Self::Matter(error) => write!(formatter, "{error}"),
            Self::Adf(error) => write!(formatter, "{error}"),
            Self::Optics(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for QuestMakepadMatterSurfaceError {}

impl From<MeshReplayError> for QuestMakepadMatterSurfaceError {
    fn from(value: MeshReplayError) -> Self {
        Self::MeshReplay(value)
    }
}

impl From<MatterSurfaceRuntimeError> for QuestMakepadMatterSurfaceError {
    fn from(value: MatterSurfaceRuntimeError) -> Self {
        Self::Matter(value)
    }
}

impl From<QuestMakepadAdfDebugError> for QuestMakepadMatterSurfaceError {
    fn from(value: QuestMakepadAdfDebugError) -> Self {
        Self::Adf(value)
    }
}

impl From<OpticsError> for QuestMakepadMatterSurfaceError {
    fn from(value: OpticsError) -> Self {
        Self::Optics(value)
    }
}

pub(crate) fn sanitize_marker_value(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn elapsed_ms(started_at: Instant) -> f32 {
    started_at.elapsed().as_secs_f32() * 1000.0
}

pub(crate) fn vec3_marker_token(value: [f32; 3]) -> String {
    format!("{:.6},{:.6},{:.6}", value[0], value[1], value[2])
}

fn optional_usize_marker_token(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

#[cfg(test)]
mod tests;
