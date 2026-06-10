//! Quest Makepad adapter for native Matter animated-surface runtime output.
//!
//! This crate converts replay frames and Matter runtime payloads into bounded
//! app-facing rows and renderer-neutral Optics visuals. It does not own
//! simulation truth, settings resolution, Android property transport, or
//! Makepad backend resources.

mod worker;

use core::fmt;
use std::{num::NonZeroUsize, time::Instant};

use rusty_matter_model::Vec3;
use rusty_matter_particles::{ParticleRenderPayload, ParticleSet};
use rusty_matter_sdf::{MeshSdfSignMode, MeshToSdfConfig};
use rusty_matter_surface_runtime::{
    MatterSurfaceContactProbeBatch, MatterSurfaceFrameInput, MatterSurfaceParticleSnapshot,
    MatterSurfaceRuntime, MatterSurfaceRuntimeConfig, MatterSurfaceRuntimeError,
    MatterSurfaceRuntimeStats, MatterSurfaceRuntimeUpdate, MatterSurfaceStepDiagnostics,
    ParticleExecutionConfig, DEFAULT_SURFACE_RUNTIME_PARTICLE_COUNT,
    DEFAULT_SURFACE_RUNTIME_PARTICLE_SEED,
};
use rusty_optics_mesh::SdfSliceVisual;
use rusty_optics_model::OpticsError;
use rusty_optics_particles::{
    resolve_animated_particle_visual_frame, ParticleVisualAnimationProfile, ParticleVisualFrame,
};
use rusty_quest_makepad_mesh_replay::{MeshReplayError, MeshReplayRuntime};

pub use rusty_matter_surface_runtime::{
    MatterSurfaceContactProbe, MatterSurfaceParticleDistanceRefreshPolicy, ParticleExecutionBackend,
};
pub use worker::{
    QuestMakepadMatterSurfaceWorker, QuestMakepadMatterSurfaceWorkerError,
    QuestMakepadMatterSurfaceWorkerFrame, QuestMakepadMatterSurfaceWorkerOutput,
    QuestMakepadMatterSurfaceWorkerStats, QUEST_MAKEPAD_MATTER_SURFACE_WORKER_MARKER_PREFIX,
    QUEST_MAKEPAD_MATTER_SURFACE_WORKER_SCHEMA_ID,
};

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
    /// Whether particles should be stepped and emitted.
    pub particles_enabled: bool,
    /// Particle count for deterministic resets.
    pub particle_count: usize,
    /// Particle reset seed.
    pub particle_seed: u32,
    /// Policy for extra per-particle snapshot distance refreshes.
    pub particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy,
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
            particles_enabled: false,
            particle_count: DEFAULT_SURFACE_RUNTIME_PARTICLE_COUNT,
            particle_seed: DEFAULT_SURFACE_RUNTIME_PARTICLE_SEED,
            particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::StepOnly,
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

/// One packed SDF slice row for Makepad-facing upload paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadDistanceSliceRow {
    /// Plane coordinate and normalized distance as `[u, v, normalized, distance]`.
    pub uv_distance: [f32; 4],
    /// Source position as `[x, y, z, 1]`.
    pub position: [f32; 4],
}

/// Bounded SDF slice upload.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadDistanceSliceUpload {
    /// Schema identifier.
    pub schema_id: String,
    /// Slice width.
    pub width: u32,
    /// Slice height.
    pub height: u32,
    /// Packed rows.
    pub rows: Vec<QuestMakepadDistanceSliceRow>,
}

/// One packed collision row for Makepad-facing upload paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadCollisionRow {
    /// Contact point and distance as `[x, y, z, distance]`.
    pub point_distance: [f32; 4],
    /// Contact normal and overlap flag as `[x, y, z, overlaps]`.
    pub normal_overlap: [f32; 4],
}

/// Bounded collision upload.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadCollisionUpload {
    /// Schema identifier.
    pub schema_id: String,
    /// Packed contact rows.
    pub rows: Vec<QuestMakepadCollisionRow>,
}

/// One packed particle row for Makepad-facing upload paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadParticleRow {
    /// Position and visual radius as `[x, y, z, radius]`.
    pub position_radius: [f32; 4],
    /// RGBA visual color.
    pub color: [f32; 4],
    /// Velocity-derived normal and animation frame as `[x, y, z, frame01]`.
    pub normal_frame: [f32; 4],
    /// Rotation, speed, visual envelope, and flags as `[rotation, aux0, aux1, flags]`.
    pub aux: [f32; 4],
}

/// Bounded particle upload.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadParticleUpload {
    /// Schema identifier.
    pub schema_id: String,
    /// Full Matter source row count before visual-row capping.
    pub source_rows: usize,
    /// Packed particle rows.
    pub rows: Vec<QuestMakepadParticleRow>,
}

/// Placement policy for Matter particles rendered as Makepad world objects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadWorldParticlePlacement {
    /// Replay/Matter content center in the target Makepad/XR coordinate space.
    pub center: [f32; 3],
    /// Target radius for the source replay bounds.
    pub target_radius: f32,
    /// Coordinate space for `center` and emitted instance centers.
    pub coordinate_space: &'static str,
}

impl Default for QuestMakepadWorldParticlePlacement {
    fn default() -> Self {
        Self {
            center: DEFAULT_WORLD_CONTENT_CENTER,
            target_radius: DEFAULT_WORLD_CONTENT_TARGET_RADIUS,
            coordinate_space: QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE,
        }
    }
}

impl QuestMakepadWorldParticlePlacement {
    /// Creates a placement for Makepad XR content-local rendering.
    #[must_use]
    pub const fn content_local(center: [f32; 3], target_radius: f32) -> Self {
        Self {
            center,
            target_radius,
            coordinate_space: QUEST_MAKEPAD_CONTENT_LOCAL_SPACE,
        }
    }
}

/// One Makepad-facing world particle instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadWorldParticleInstance {
    /// World-space center and radius as `[x, y, z, radius]`.
    pub center_radius: [f32; 4],
    /// RGBA visual color.
    pub color: [f32; 4],
    /// Source normal and animation frame as `[x, y, z, frame01]`.
    pub normal_frame: [f32; 4],
    /// Renderer-neutral visual animation metadata as `[rotation, aux0, aux1, flags]`.
    pub aux: [f32; 4],
}

/// Bounded Makepad-facing world particle batch.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadWorldParticleBatch {
    /// Schema identifier.
    pub schema_id: String,
    /// Source upload schema identifier.
    pub source_schema_id: String,
    /// Coordinate space for the instance centers.
    pub coordinate_space: String,
    /// Renderer-facing primitive mode.
    pub render_mode: String,
    /// Replay/Matter content center in the target coordinate space.
    pub content_center: [f32; 3],
    /// Replay/Matter content radius in the target coordinate space.
    pub content_radius: f32,
    /// Scale from replay-local units to Makepad world units.
    pub replay_to_world_scale: f32,
    /// Source particle rows before the batch bound.
    pub source_rows: usize,
    /// Rows dropped by `max_instances`.
    pub dropped_rows: usize,
    /// World-space instances.
    pub instances: Vec<QuestMakepadWorldParticleInstance>,
}

impl QuestMakepadWorldParticleBatch {
    /// Builds a compact evidence marker without logging high-rate particle rows.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        let spread = instance_spread_token(&self.instances);
        format!(
            "{} schema={} phase={} status={} renderMode={} coordinateSpace={} sourceSchema={} sourceRows={} instanceRows={} droppedRows={} selectionPolicy={} contentCenter={} contentRadius={:.6} replayToWorldScale={:.6} contentCenterDistanceMeters={:.3} instanceSpread={} dataPlane=makepad-world-particle-instances",
            QUEST_MAKEPAD_WORLD_PARTICLE_MARKER_PREFIX,
            QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID,
            sanitize_marker_value(phase),
            if self.instances.is_empty() { "empty" } else { "ready" },
            sanitize_marker_value(&self.render_mode),
            sanitize_marker_value(&self.coordinate_space),
            sanitize_marker_value(&self.source_schema_id),
            self.source_rows,
            self.instances.len(),
            self.dropped_rows,
            QUEST_MAKEPAD_WORLD_PARTICLE_EVEN_SELECTION_POLICY,
            vec3_marker_token(self.content_center),
            self.content_radius,
            self.replay_to_world_scale,
            vec3_length(self.content_center),
            spread,
        )
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
}

/// Quest Makepad Matter surface adapter runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadMatterSurfaceRuntime {
    config: QuestMakepadMatterSurfaceConfig,
    matter: MatterSurfaceRuntime,
    particle_profile: ParticleVisualAnimationProfile,
    particles_initialized: bool,
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

        let (sdf_slice, sdf_slice_upload) = if self.config.enabled && self.config.sdf_slice_enabled
        {
            let started_at = Instant::now();
            let grid = self.matter.build_sdf_grid(self.config.sdf_config())?;
            let slice = SdfSliceVisual::middle_z("quest.makepad.sdf_slice.middle_z", &grid)?;
            stage_timings.sdf_build_ms = elapsed_ms(started_at);
            let started_at = Instant::now();
            let upload = distance_slice_upload_from_visual(&slice);
            stage_timings.sdf_upload_ms = elapsed_ms(started_at);
            (Some(slice), Some(upload))
        } else {
            (None, None)
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
        format!(
            "{} schema={} phase={} status={} nativeMatterRuntime=true wasmRuntimeUsed=false shaderScaffoldUsed=false proceduralParticleOverlayUsed=false proceduralSdfOverlayUsed=false proceduralCollisionOverlayUsed=false dataPlane=makepad-compact-uniform-rows sourceId={} sourceSchema={} frameIndex={} vertexCount={} triangleCount={} particleCount={} particleDistanceRefreshPolicy={} particleDistanceSamples={} particleInputDeltaSeconds={:.6} particleSimulatedDeltaSeconds={:.6} particleDroppedDeltaSeconds={:.6} particleSubsteps={} particleClosestSamples={} particleSurfaceNodeTests={} particleSurfaceLeafTests={} particleSurfaceTriangleTests={} particleRefreshSamples={} particleRefreshNodeTests={} particleRefreshLeafTests={} particleRefreshTriangleTests={} particleExecutionBackend={} particleExecutionBatchSize={} particleExecutionChunks={} particleExecutionWorkers={} particleExecutionElapsedMicros={} collisionRows={} particleSourceRows={} particleRows={} particleVisualRowLimit={} sdfRows={} leafTriangleCount={} distanceSamplerRefit={} adapterTotalMs={:.3} matterUpdateMs={:.3} particleResetMs={:.3} particleStepMs={:.3} collisionProbeMs={:.3} collisionUploadMs={:.3} sdfBuildMs={:.3} sdfUploadMs={:.3} particleSnapshotMs={:.3} particlePayloadMs={:.3} particleVisualMs={:.3} particleUploadMs={:.3}",
            QUEST_MAKEPAD_MATTER_SURFACE_MARKER_PREFIX,
            QUEST_MAKEPAD_MATTER_SURFACE_SCHEMA_ID,
            sanitize_marker_value(phase),
            if self.config.enabled { "ready" } else { "disabled" },
            sanitize_marker_value(&frame.source_id),
            frame.matter_update.topology_key.schema_id,
            frame.matter_update.frame_index.unwrap_or(0),
            frame.matter_update.vertex_count,
            frame.matter_update.triangle_count,
            frame.stats.particle_count,
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
            frame.stage_timings.particle_snapshot_ms,
            frame.stage_timings.particle_payload_ms,
            frame.stage_timings.particle_visual_ms,
            frame.stage_timings.particle_upload_ms,
        )
    }
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
    /// Optics visual payload failed.
    Optics(OpticsError),
}

impl fmt::Display for QuestMakepadMatterSurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MeshReplay(error) => write!(formatter, "{error}"),
            Self::Matter(error) => write!(formatter, "{error}"),
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

impl From<OpticsError> for QuestMakepadMatterSurfaceError {
    fn from(value: OpticsError) -> Self {
        Self::Optics(value)
    }
}

fn distance_slice_upload_from_visual(visual: &SdfSliceVisual) -> QuestMakepadDistanceSliceUpload {
    let rows = visual
        .cells
        .iter()
        .map(|cell| QuestMakepadDistanceSliceRow {
            uv_distance: [
                cell.plane[0] as f32,
                cell.plane[1] as f32,
                cell.normalized_distance,
                cell.distance,
            ],
            position: [cell.position.x, cell.position.y, cell.position.z, 1.0],
        })
        .collect();
    QuestMakepadDistanceSliceUpload {
        schema_id: QUEST_MAKEPAD_DISTANCE_SLICE_UPLOAD_SCHEMA_ID.to_owned(),
        width: visual.width,
        height: visual.height,
        rows,
    }
}

fn collision_upload_from_batch(
    batch: &MatterSurfaceContactProbeBatch,
) -> QuestMakepadCollisionUpload {
    let rows = batch
        .results
        .iter()
        .filter_map(|result| {
            let contact = result.contact.as_ref()?;
            Some(QuestMakepadCollisionRow {
                point_distance: [
                    contact.point.x,
                    contact.point.y,
                    contact.point.z,
                    contact.distance,
                ],
                normal_overlap: [
                    contact.normal.x,
                    contact.normal.y,
                    contact.normal.z,
                    if result.overlaps { 1.0 } else { 0.0 },
                ],
            })
        })
        .collect();
    QuestMakepadCollisionUpload {
        schema_id: QUEST_MAKEPAD_COLLISION_UPLOAD_SCHEMA_ID.to_owned(),
        rows,
    }
}

fn particle_render_payload_for_visual_limit(
    matter: &MatterSurfaceRuntime,
    payload_id: &'static str,
    visual_row_limit: Option<usize>,
) -> Result<ParticleRenderPayload, QuestMakepadMatterSurfaceError> {
    let Some(limit) = visual_row_limit else {
        return matter
            .particle_render_payload(payload_id)
            .map_err(Into::into);
    };

    let source_particles = matter.particle_runtime().particles();
    let source_count = source_particles.particles.len();
    let visual_count = source_count.min(limit);
    if visual_count == source_count {
        return matter
            .particle_render_payload(payload_id)
            .map_err(Into::into);
    }

    let mut sampled = ParticleSet::with_capacity(source_particles.set_id.clone(), visual_count);
    sampled.time_seconds = source_particles.time_seconds;
    for selection_index in 0..visual_count {
        if let Some(source_index) =
            evenly_spaced_source_index(selection_index, visual_count, source_count)
        {
            if let Some(particle) = source_particles.particles.get(source_index) {
                sampled.push(particle.clone());
            }
        }
    }

    ParticleRenderPayload::from_particle_set(payload_id, &sampled)
        .map_err(MatterSurfaceRuntimeError::from)
        .map_err(Into::into)
}

fn particle_upload_from_visual_frame(
    frame: &ParticleVisualFrame,
    source_rows: usize,
) -> QuestMakepadParticleUpload {
    let rows = frame
        .samples
        .iter()
        .map(|sample| QuestMakepadParticleRow {
            position_radius: [
                sample.position.x,
                sample.position.y,
                sample.position.z,
                sample.radius,
            ],
            color: [
                sample.color.r,
                sample.color.g,
                sample.color.b,
                sample.color.a,
            ],
            normal_frame: [
                sample.normal.x,
                sample.normal.y,
                sample.normal.z,
                sample.frame01,
            ],
            aux: [
                sample.rotation_radians,
                sample.aux0,
                sample.aux1,
                sample.flags as f32,
            ],
        })
        .collect();
    QuestMakepadParticleUpload {
        schema_id: QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID.to_owned(),
        source_rows,
        rows,
    }
}

/// Converts a particle upload into bounded Makepad world-particle instances.
#[must_use]
pub fn world_particle_batch_from_upload(
    upload: &QuestMakepadParticleUpload,
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    placement: QuestMakepadWorldParticlePlacement,
    max_instances: usize,
) -> QuestMakepadWorldParticleBatch {
    let bounds_center = midpoint(bounds_min, bounds_max);
    let bounds_radius = bounds_radius(bounds_min, bounds_max).max(0.001);
    let placement_radius = placement.target_radius.max(0.001);
    let scale = placement_radius / bounds_radius;
    let upload_rows = upload.rows.len();
    let instance_count = upload_rows.min(max_instances);
    let instances = (0..instance_count)
        .filter_map(|selection_index| {
            let source_index =
                evenly_spaced_source_index(selection_index, instance_count, upload_rows)?;
            upload.rows.get(source_index)
        })
        .map(|row| {
            let source = [
                row.position_radius[0],
                row.position_radius[1],
                row.position_radius[2],
            ];
            let centered = [
                source[0] - bounds_center[0],
                source[1] - bounds_center[1],
                source[2] - bounds_center[2],
            ];
            QuestMakepadWorldParticleInstance {
                center_radius: [
                    placement.center[0] + centered[0] * scale,
                    placement.center[1] + centered[1] * scale,
                    placement.center[2] + centered[2] * scale,
                    row.position_radius[3].max(0.001) * scale,
                ],
                color: row.color,
                normal_frame: row.normal_frame,
                aux: row.aux,
            }
        })
        .collect::<Vec<_>>();
    QuestMakepadWorldParticleBatch {
        schema_id: QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID.to_owned(),
        source_schema_id: upload.schema_id.clone(),
        coordinate_space: placement.coordinate_space.to_owned(),
        render_mode: QUEST_MAKEPAD_CENTER_PROJECTED_BILLBOARD_MODE.to_owned(),
        content_center: placement.center,
        content_radius: placement_radius,
        replay_to_world_scale: scale,
        source_rows: upload.source_rows,
        dropped_rows: upload.source_rows.saturating_sub(instances.len()),
        instances,
    }
}

fn evenly_spaced_source_index(
    selection_index: usize,
    selection_count: usize,
    source_count: usize,
) -> Option<usize> {
    if source_count == 0 || selection_count == 0 || selection_index >= selection_count {
        return None;
    }
    if selection_count >= source_count {
        return Some(selection_index);
    }
    if selection_count == 1 {
        return Some(source_count / 2);
    }
    let numerator = selection_index
        .saturating_mul(source_count.saturating_sub(1))
        .saturating_add((selection_count - 1) / 2);
    Some((numerator / (selection_count - 1)).min(source_count - 1))
}

fn instance_spread_token(instances: &[QuestMakepadWorldParticleInstance]) -> String {
    let Some(first) = instances.first() else {
        return "0.000000,0.000000,0.000000".to_owned();
    };
    let mut minimum = [
        first.center_radius[0],
        first.center_radius[1],
        first.center_radius[2],
    ];
    let mut maximum = minimum;
    for instance in instances.iter().skip(1) {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(instance.center_radius[axis]);
            maximum[axis] = maximum[axis].max(instance.center_radius[axis]);
        }
    }
    vec3_marker_token([
        maximum[0] - minimum[0],
        maximum[1] - minimum[1],
        maximum[2] - minimum[2],
    ])
}

fn vec3_length(value: [f32; 3]) -> f32 {
    (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
}

fn midpoint(minimum: [f32; 3], maximum: [f32; 3]) -> [f32; 3] {
    [
        (minimum[0] + maximum[0]) * 0.5,
        (minimum[1] + maximum[1]) * 0.5,
        (minimum[2] + maximum[2]) * 0.5,
    ]
}

fn bounds_max_half_extent(minimum: [f32; 3], maximum: [f32; 3]) -> f32 {
    let extent_x = maximum[0] - minimum[0];
    let extent_y = maximum[1] - minimum[1];
    let extent_z = maximum[2] - minimum[2];
    extent_x.max(extent_y).max(extent_z).max(0.0) * 0.5
}

fn bounds_radius(minimum: [f32; 3], maximum: [f32; 3]) -> f32 {
    let center = midpoint(minimum, maximum);
    let dx = (maximum[0] - center[0])
        .abs()
        .max((minimum[0] - center[0]).abs());
    let dy = (maximum[1] - center[1])
        .abs()
        .max((minimum[1] - center[1]).abs());
    let dz = (maximum[2] - center[2])
        .abs()
        .max((minimum[2] - center[2]).abs());
    (dx.mul_add(dx, dy.mul_add(dy, dz * dz))).sqrt()
}

fn sanitize_marker_value(value: &str) -> String {
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

fn vec3_marker_token(value: [f32; 3]) -> String {
    format!("{:.6},{:.6},{:.6}", value[0], value[1], value[2])
}

fn optional_usize_marker_token(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_matter_model::Vec3;
    use rusty_quest_makepad_mesh_replay::{MeshReplayConfig, MeshReplaySequence};

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

    #[test]
    fn adapter_steps_matter_surface_from_replay() {
        let replay = enabled_replay();
        let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
            enabled: true,
            collision_enabled: true,
            sdf_slice_enabled: true,
            particles_enabled: true,
            particle_count: 16,
            particle_execution_batch_size: NonZeroUsize::new(4).unwrap(),
            sdf_voxel_size: 0.12,
            sdf_max_voxels: 4_096,
            ..QuestMakepadMatterSurfaceConfig::default()
        })
        .expect("runtime builds");

        let frame = runtime
            .step_from_replay(
                &replay,
                1.0 / 30.0,
                &[MatterSurfaceContactProbe::sphere(
                    "probe.center",
                    Vec3::new(0.0, 0.0, 0.0),
                    0.25,
                )],
            )
            .expect("adapter frame builds");

        assert_eq!(
            frame.matter_update.vertex_count,
            replay.sequence().vertex_count()
        );
        assert_eq!(
            frame.matter_update.triangle_count,
            replay.sequence().triangle_count()
        );
        assert_eq!(frame.particle_snapshot.samples.len(), 16);
        assert_eq!(
            frame
                .particle_step
                .as_ref()
                .unwrap()
                .refreshed_distance_samples,
            16
        );
        let upload = frame.particle_upload.as_ref().unwrap();
        assert_eq!(upload.source_rows, 16);
        assert_eq!(upload.rows.len(), 16);
        let world_batch = frame
            .world_particle_batch(
                replay.sequence().bounds_min(),
                replay.sequence().bounds_max(),
                QuestMakepadWorldParticlePlacement::default(),
                8,
            )
            .expect("world particle batch builds");
        assert_eq!(world_batch.source_rows, 16);
        assert_eq!(world_batch.instances.len(), 8);
        assert_eq!(world_batch.dropped_rows, 8);
        assert_eq!(world_batch.content_center, DEFAULT_WORLD_CONTENT_CENTER);
        assert_eq!(
            world_batch.coordinate_space,
            QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE
        );
        assert!(!frame.collision_upload.rows.is_empty());
        assert!(frame.sdf_slice_upload.as_ref().unwrap().rows.len() > 1);

        let marker = runtime.marker_line("unit-test", &frame);
        assert!(marker.contains("nativeMatterRuntime=true"));
        assert!(marker.contains("sourceId=public-synthetic-hand-sequence"));
        assert!(marker.contains("wasmRuntimeUsed=false"));
        assert!(marker.contains("shaderScaffoldUsed=false"));
        assert!(marker.contains("proceduralParticleOverlayUsed=false"));
        assert!(marker.contains("dataPlane=makepad-compact-uniform-rows"));
        assert!(marker.contains("distanceSamplerRefit=false"));
        assert!(marker.contains("particleDistanceRefreshPolicy=step-only"));
        assert!(marker.contains("particleDistanceSamples=16"));
        assert!(marker.contains("particleInputDeltaSeconds=0.033333"));
        assert!(marker.contains("particleSimulatedDeltaSeconds=0.033333"));
        assert!(marker.contains("particleDroppedDeltaSeconds=0.000000"));
        assert!(marker.contains("particleSubsteps="));
        assert!(marker.contains("particleClosestSamples="));
        assert!(marker.contains("particleSurfaceNodeTests="));
        assert!(marker.contains("particleSurfaceLeafTests="));
        assert!(marker.contains("particleSurfaceTriangleTests="));
        assert!(marker.contains("particleRefreshSamples=16"));
        assert!(marker.contains("particleRefreshNodeTests="));
        assert!(marker.contains("particleRefreshLeafTests="));
        assert!(marker.contains("particleRefreshTriangleTests="));
        assert!(marker.contains("particleExecutionBackend=serial"));
        assert!(marker.contains("particleExecutionBatchSize=4"));
        assert!(marker.contains("particleExecutionChunks="));
        assert!(marker.contains("particleExecutionWorkers=1"));
        assert!(marker.contains("particleExecutionElapsedMicros="));
        assert!(marker.contains("particleSourceRows=16"));
        assert!(marker.contains("particleRows=16"));
        assert!(marker.contains("particleVisualRowLimit=none"));
        assert!(marker.contains("adapterTotalMs="));
        assert!(marker.contains("matterUpdateMs="));
        assert!(marker.contains("particleStepMs="));
        assert!(marker.contains("particleVisualMs="));
        assert!(frame.stage_timings.total_ms >= frame.stage_timings.matter_update_ms);
        assert!(!marker.contains("rusty.xr"));
        assert!(!marker.contains("RUSTY_XR"));

        let world_marker = world_batch.marker_line("unit-test");
        assert!(world_marker.contains(QUEST_MAKEPAD_WORLD_PARTICLE_MARKER_PREFIX));
        assert!(world_marker.contains("renderMode=center-projected-billboard"));
        assert!(world_marker.contains("selectionPolicy=evenly-spaced-source-rows"));
        assert!(world_marker.contains("instanceSpread="));
        assert!(world_marker.contains("contentCenterDistanceMeters=0.500"));
        assert!(!world_marker.contains("rusty.xr"));
        assert!(!world_marker.contains("RUSTY_XR"));
    }

    #[test]
    fn adapter_can_bound_particle_simulation_delta() {
        let replay = enabled_replay();
        let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
            enabled: true,
            particles_enabled: true,
            particle_count: 16,
            particle_max_frame_delta_seconds: Some(1.0 / 60.0),
            ..QuestMakepadMatterSurfaceConfig::default()
        })
        .expect("runtime builds");

        let frame = runtime
            .step_from_replay(&replay, 0.25, &[])
            .expect("adapter frame builds");
        let diagnostics = frame
            .particle_step
            .as_ref()
            .expect("particles step when enabled");

        assert_eq!(diagnostics.particles.input_delta_seconds, 0.25);
        assert!((diagnostics.particles.simulated_delta_seconds - 1.0 / 60.0).abs() < 1.0e-6);
        assert!((diagnostics.particles.dropped_delta_seconds - (0.25 - 1.0 / 60.0)).abs() < 1.0e-6);
        let marker = runtime.marker_line("unit-test", &frame);
        assert!(marker.contains("particleInputDeltaSeconds=0.250000"));
        assert!(marker.contains("particleSimulatedDeltaSeconds=0.016667"));
        assert!(marker.contains("particleDroppedDeltaSeconds=0.233333"));
    }

    #[test]
    fn adapter_caps_particle_visual_rows_without_changing_matter_count() {
        let replay = enabled_replay();
        let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
            enabled: true,
            particles_enabled: true,
            particle_count: 32,
            particle_visual_row_limit: Some(8),
            ..QuestMakepadMatterSurfaceConfig::default()
        })
        .expect("runtime builds");

        let frame = runtime
            .step_from_replay(&replay, 1.0 / 60.0, &[])
            .expect("adapter frame builds");

        assert_eq!(frame.stats.particle_count, 32);
        assert_eq!(frame.particle_snapshot.samples.len(), 32);
        assert_eq!(
            frame
                .particle_visual_frame
                .as_ref()
                .expect("visual frame")
                .samples
                .len(),
            8
        );
        let upload = frame.particle_upload.as_ref().expect("particle upload");
        assert_eq!(upload.source_rows, 32);
        assert_eq!(upload.rows.len(), 8);

        let world_batch = frame
            .world_particle_batch(
                replay.sequence().bounds_min(),
                replay.sequence().bounds_max(),
                QuestMakepadWorldParticlePlacement::default(),
                8,
            )
            .expect("world particle batch builds");
        assert_eq!(world_batch.source_rows, 32);
        assert_eq!(world_batch.instances.len(), 8);
        assert_eq!(world_batch.dropped_rows, 24);

        let marker = runtime.marker_line("unit-test", &frame);
        assert!(marker.contains("particleCount=32"));
        assert!(marker.contains("particleSourceRows=32"));
        assert!(marker.contains("particleRows=8"));
        assert!(marker.contains("particleVisualRowLimit=8"));
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn adapter_reports_parallel_particle_execution_when_feature_enabled() {
        let replay = enabled_replay();
        let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
            enabled: true,
            particles_enabled: true,
            particle_count: 64,
            particle_execution_backend: ParticleExecutionBackend::Parallel,
            particle_execution_batch_size: NonZeroUsize::new(8).unwrap(),
            particle_execution_max_threads: Some(2),
            ..QuestMakepadMatterSurfaceConfig::default()
        })
        .expect("parallel runtime builds");

        let frame = runtime
            .step_from_replay(&replay, 1.0 / 30.0, &[])
            .expect("adapter frame builds");
        let diagnostics = frame
            .particle_step
            .as_ref()
            .expect("particles step when enabled");

        assert_eq!(
            diagnostics.particles.execution.backend,
            ParticleExecutionBackend::Parallel
        );
        assert_eq!(diagnostics.particles.execution.batch_size, 8);
        assert_eq!(diagnostics.particles.execution.worker_count, 2);
        let marker = runtime.marker_line("unit-test", &frame);
        assert!(marker.contains("particleExecutionBackend=rayon"));
        assert!(marker.contains("particleExecutionWorkers=2"));
    }

    #[test]
    fn adapter_steps_generic_source_frame_like_replay_frame() {
        let replay = enabled_replay();
        let source_frame = QuestMakepadMatterSurfaceSourceFrame::from_replay(&replay)
            .expect("source frame builds");

        assert_eq!(source_frame.source_id, "public-synthetic-hand-sequence");
        assert_eq!(source_frame.frame.frame_index, replay.current_frame_index());
        assert_eq!(source_frame.bounds_min, replay.sequence().bounds_min());
        assert_eq!(source_frame.bounds_max, replay.sequence().bounds_max());
        assert_eq!(
            source_frame.bounds_radius,
            replay.sequence().bounds_radius()
        );

        let config = QuestMakepadMatterSurfaceConfig {
            enabled: true,
            collision_enabled: true,
            sdf_slice_enabled: false,
            particles_enabled: false,
            ..QuestMakepadMatterSurfaceConfig::default()
        };
        let mut source_runtime =
            QuestMakepadMatterSurfaceRuntime::new(config.clone()).expect("runtime builds");
        let mut replay_runtime =
            QuestMakepadMatterSurfaceRuntime::new(config).expect("runtime builds");

        let probes = [MatterSurfaceContactProbe::sphere(
            "probe.center",
            Vec3::new(0.0, 0.0, 0.0),
            0.25,
        )];
        let from_source = source_runtime
            .step_from_source_frame(source_frame, 1.0 / 60.0, &probes)
            .expect("source frame steps");
        let from_replay = replay_runtime
            .step_from_replay(&replay, 1.0 / 60.0, &probes)
            .expect("replay frame steps");

        assert_eq!(from_source.source_id, from_replay.source_id);
        assert_eq!(
            from_source.matter_update.frame_index,
            from_replay.matter_update.frame_index
        );
        assert_eq!(
            from_source.matter_update.vertex_count,
            from_replay.matter_update.vertex_count
        );
        assert_eq!(
            from_source.matter_update.triangle_count,
            from_replay.matter_update.triangle_count
        );
        assert_eq!(
            from_source.collision_upload.rows.len(),
            from_replay.collision_upload.rows.len()
        );

        let marker = source_runtime.marker_line("unit-test", &from_source);
        assert!(marker.contains("sourceId=public-synthetic-hand-sequence"));
        assert!(!marker.contains("rusty.xr"));
        assert!(!marker.contains("RUSTY_XR"));
    }

    #[test]
    fn external_recorded_sequence_steps_through_source_frame_when_configured() {
        let Ok(sequence_path) = std::env::var("RUSTY_QUEST_MAKEPAD_RECORDED_SEQUENCE_JSON") else {
            return;
        };
        let sequence_json =
            std::fs::read_to_string(&sequence_path).expect("recorded sequence JSON reads");
        let sequence =
            MeshReplaySequence::from_json_str(&sequence_json).expect("recorded sequence parses");
        assert!(sequence.vertex_count() > 8);
        assert!(sequence.triangle_count() > 6);
        assert!(sequence.frame_count() > 1);

        let mut replay = MeshReplayRuntime::from_sequence(
            sequence,
            MeshReplayConfig::normalized(
                true,
                "recorded-meta-quest-hand-sequence".to_owned(),
                1.0,
                1.0,
            ),
        );
        replay.step(0.0);

        let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
            enabled: true,
            collision_enabled: true,
            sdf_slice_enabled: false,
            particles_enabled: false,
            ..QuestMakepadMatterSurfaceConfig::default()
        })
        .expect("runtime builds");
        let frame = runtime
            .step_from_source_frame(
                QuestMakepadMatterSurfaceSourceFrame::from_replay(&replay)
                    .expect("source frame builds"),
                1.0 / 60.0,
                &[MatterSurfaceContactProbe::sphere(
                    "probe.center",
                    replay.sequence().bounds_center(),
                    replay.sequence().bounds_radius().max(0.01),
                )],
            )
            .expect("recorded source frame steps");

        assert_eq!(frame.source_id, "recorded-meta-quest-hand-sequence");
        assert_eq!(
            frame.matter_update.vertex_count,
            replay.sequence().vertex_count()
        );
        assert_eq!(
            frame.matter_update.triangle_count,
            replay.sequence().triangle_count()
        );
        assert_eq!(frame.collision_upload.rows.len(), 1);
        let marker = runtime.marker_line("external-recorded-sequence", &frame);
        assert!(marker.contains("nativeMatterRuntime=true"));
        assert!(marker.contains("sourceId=recorded-meta-quest-hand-sequence"));
        assert!(marker.contains("wasmRuntimeUsed=false"));
        assert!(marker.contains("shaderScaffoldUsed=false"));
    }

    #[test]
    fn world_particle_billboard_renderer_identity_is_morphospace_scoped() {
        let values = [
            QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_RENDERER_ID,
            QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_MODE,
            QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_SOURCE,
            QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_REFERENCE,
        ];

        for value in values {
            assert!(!value.contains("rusty.xr"));
            assert!(!value.contains("rustyxr"));
            assert!(!value.contains("RUSTY_XR"));
        }
        assert_eq!(
            QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_RENDERER_ID,
            "makepad-xr-procedural-ring-billboard"
        );
        assert_eq!(
            QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_SOURCE,
            "rusty-optics-particle-visual-frame"
        );
    }

    #[test]
    fn adapter_can_update_surface_without_high_rate_payloads_enabled() {
        let replay = enabled_replay();
        let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
            enabled: true,
            collision_enabled: false,
            sdf_slice_enabled: false,
            particles_enabled: false,
            ..QuestMakepadMatterSurfaceConfig::default()
        })
        .expect("runtime builds");

        let frame = runtime
            .step_from_replay(&replay, 1.0 / 60.0, &[])
            .expect("adapter frame builds");

        assert_eq!(frame.matter_update.vertex_count, 8);
        assert_eq!(frame.collision_upload.rows.len(), 0);
        assert!(frame.sdf_slice_upload.is_none());
        assert!(frame.particle_upload.is_none());
        assert!(frame.particle_step.is_none());
        assert_eq!(frame.particle_snapshot.samples.len(), 0);
    }

    #[test]
    fn world_particle_batch_places_content_center_half_meter_in_front() {
        let upload = QuestMakepadParticleUpload {
            schema_id: QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID.to_owned(),
            source_rows: 2,
            rows: vec![
                QuestMakepadParticleRow {
                    position_radius: [0.0, 0.0, 0.0, 0.02],
                    color: [0.2, 0.8, 1.0, 1.0],
                    normal_frame: [0.0, 0.0, 1.0, 0.5],
                    aux: [0.25, 0.0, 0.0, 0.0],
                },
                QuestMakepadParticleRow {
                    position_radius: [1.0, 0.0, 0.0, 0.02],
                    color: [1.0, 0.5, 0.2, 1.0],
                    normal_frame: [1.0, 0.0, 0.0, 0.25],
                    aux: [0.75, 0.0, 0.0, 0.0],
                },
            ],
        };

        let batch = world_particle_batch_from_upload(
            &upload,
            [-1.0, -1.0, -1.0],
            [1.0, 1.0, 1.0],
            QuestMakepadWorldParticlePlacement::default(),
            16,
        );

        assert_eq!(batch.instances.len(), 2);
        assert_eq!(
            [
                batch.instances[0].center_radius[0],
                batch.instances[0].center_radius[1],
                batch.instances[0].center_radius[2],
            ],
            DEFAULT_WORLD_CONTENT_CENTER
        );
        assert!(
            (batch.instances[0].center_radius[3] - (0.02 * batch.replay_to_world_scale)).abs()
                < 0.000_001
        );
        assert_eq!(batch.dropped_rows, 0);
    }

    #[test]
    fn world_particle_batch_samples_across_source_rows() {
        let upload = QuestMakepadParticleUpload {
            schema_id: QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID.to_owned(),
            source_rows: 10,
            rows: (0..10)
                .map(|index| QuestMakepadParticleRow {
                    position_radius: [index as f32, index as f32 * 0.5, index as f32 * -0.25, 0.02],
                    color: [0.2, 0.8, 1.0, 1.0],
                    normal_frame: [0.0, 0.0, 1.0, 0.5],
                    aux: [index as f32 * 0.01, 0.0, 0.0, 0.0],
                })
                .collect(),
        };

        let batch = world_particle_batch_from_upload(
            &upload,
            [0.0, 0.0, -3.0],
            [9.0, 4.5, 0.0],
            QuestMakepadWorldParticlePlacement::default(),
            4,
        );

        assert_eq!(batch.instances.len(), 4);
        assert_eq!(batch.source_rows, 10);
        assert_eq!(batch.dropped_rows, 6);
        assert!(batch.instances[0].center_radius[0] < batch.instances[1].center_radius[0]);
        assert!(batch.instances[3].center_radius[0] > batch.instances[2].center_radius[0]);
        let marker = batch.marker_line("unit-test");
        assert!(marker.contains("selectionPolicy=evenly-spaced-source-rows"));
        assert!(marker.contains("instanceSpread="));
    }

    #[test]
    fn world_particle_placement_can_target_makepad_content_local_space() {
        let placement = QuestMakepadWorldParticlePlacement::content_local(
            [0.0, 0.58, -0.22],
            DEFAULT_WORLD_CONTENT_TARGET_RADIUS,
        );

        assert_eq!(
            placement.coordinate_space,
            QUEST_MAKEPAD_CONTENT_LOCAL_SPACE
        );
        assert_eq!(placement.center, [0.0, 0.58, -0.22]);

        let batch = QuestMakepadWorldParticleBatch {
            schema_id: QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID.to_owned(),
            source_schema_id: QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID.to_owned(),
            coordinate_space: placement.coordinate_space.to_owned(),
            render_mode: QUEST_MAKEPAD_CENTER_PROJECTED_BILLBOARD_MODE.to_owned(),
            content_center: placement.center,
            content_radius: placement.target_radius,
            replay_to_world_scale: 1.0,
            source_rows: 0,
            dropped_rows: 0,
            instances: Vec::new(),
        };
        assert!(batch
            .marker_line("unit-test")
            .contains("contentCenterDistanceMeters=0.620"));
    }
}
