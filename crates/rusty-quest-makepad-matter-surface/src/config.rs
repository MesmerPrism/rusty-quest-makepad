use std::num::NonZeroUsize;

use rusty_matter_particles::SurfaceParticleRuntimeConfig;
use rusty_matter_sdf::{MeshSdfSignMode, MeshToSdfConfig};
use rusty_matter_surface_runtime::{
    MatterSurfaceParticleDistanceRefreshPolicy, MatterSurfaceParticleForceSource,
    MatterSurfaceRuntimeConfig, ParticleExecutionBackend, ParticleExecutionConfig,
    DEFAULT_PARTICLE_FORCE_UPDATE_INTERVAL_FRAMES, DEFAULT_SURFACE_RUNTIME_PARTICLE_COUNT,
    DEFAULT_SURFACE_RUNTIME_PARTICLE_SEED,
};

use crate::{
    QuestMakepadAdfDebugConfig, DEFAULT_PARTICLE_EXECUTION_BATCH_SIZE,
    DEFAULT_SDF_ADF_DEBUG_UPDATE_INTERVAL_FRAMES,
};

/// Compact Matter particle-force equation coefficients used for bounded GPU proofs.
///
/// This is evidence/config data only. Matter remains the authority for particle
/// semantics and CPU reference behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadMatterParticleForceOracleConfig {
    /// Target surface distance as a multiplier of particle radius.
    pub target_distance_radius_scale: f32,
    /// Minimum target distance in mesh units.
    pub minimum_target_distance: f32,
    /// Acceleration scale applied toward the target surface band.
    pub attraction_strength: f32,
}

impl QuestMakepadMatterParticleForceOracleConfig {
    /// Captures Matter particle-force coefficients from the runtime config.
    #[must_use]
    pub fn from_matter_config(config: &SurfaceParticleRuntimeConfig) -> Self {
        Self {
            target_distance_radius_scale: config.target_distance_radius_scale,
            minimum_target_distance: config.minimum_target_distance,
            attraction_strength: config.attraction_strength,
        }
    }

    /// Returns the Matter target distance for a particle radius.
    #[must_use]
    pub fn target_distance_for_radius(self, radius: f32) -> f32 {
        (radius * self.target_distance_radius_scale).max(self.minimum_target_distance)
    }
}

impl Default for QuestMakepadMatterParticleForceOracleConfig {
    fn default() -> Self {
        Self::from_matter_config(&SurfaceParticleRuntimeConfig::default())
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

    /// Compact Matter particle-force equation coefficients for bounded GPU proofs.
    #[must_use]
    pub fn particle_force_oracle_config(&self) -> QuestMakepadMatterParticleForceOracleConfig {
        let matter_config = self.to_matter_config();
        QuestMakepadMatterParticleForceOracleConfig::from_matter_config(&matter_config.particles)
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
