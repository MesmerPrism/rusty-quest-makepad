//! Profile-driven Quest Makepad camera shell adapter.

mod gpu_force_promotion;
mod matter_surface_exports;
mod mesh_replay_source;
mod stimulus_volume_gpu;
mod stimulus_volume_image_preview;
mod stimulus_volume_raymarch_preview;

use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

pub use gpu_force_promotion::*;
pub use matter_surface_exports::*;
use rusty_lattice_model::{validate_display_view_set, DisplayViewSet};
use rusty_optics_model::{
    ProjectionGeometryReport, Rect2, VideoProjectionMapping, IDENTITY_HOMOGRAPHY,
    STIMULUS_PROFILE_SCHEMA_ID as OPTICS_STIMULUS_PROFILE_SCHEMA_ID,
    STIMULUS_VOLUME_SCHEMA_ID as OPTICS_STIMULUS_VOLUME_SCHEMA_ID,
};
use rusty_optics_stimulus::{
    StimulusProfile as OpticsStimulusProfile,
    StimulusVolumeProfileSummary as OpticsStimulusVolumeProfileSummary,
};
use rusty_quest_makepad_mesh_replay::MeshReplayConfig;
pub use rusty_quest_makepad_mesh_replay::{
    MeshReplayRuntime, MeshReplayUniforms, RecordedCompactHandJointFrame, RecordedHandRig,
    REPLAY_MARKER_PREFIX, REPLAY_SCHEMA_ID, SELECTED_SEGMENT_COUNT,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
pub use stimulus_volume_gpu::*;
pub use stimulus_volume_image_preview::*;
pub use stimulus_volume_raymarch_preview::*;

use mesh_replay_source::mesh_replay_runtime_from_config;
pub use mesh_replay_source::{
    MESH_REPLAY_SOURCE_PUBLIC_SYNTHETIC_HAND_SEQUENCE,
    MESH_REPLAY_SOURCE_RECORDED_META_QUEST_HAND_LEFT,
    MESH_REPLAY_SOURCE_RECORDED_META_QUEST_HAND_RIGHT, RECORDED_META_QUEST_HAND_LEFT_SEQUENCE_FILE,
    RECORDED_META_QUEST_HAND_RIGHT_SEQUENCE_FILE,
};

/// Canonical camera shell app id.
pub const CAMERA_SHELL_APP_ID: &str = "rusty-quest-makepad.camera-shell";
/// Effective-settings schema consumed by the shell.
pub const EFFECTIVE_SETTINGS_SCHEMA_ID: &str = "rusty.gui.makepad.effective_settings.v1";
/// Replay enable setting id.
pub const SETTING_MESH_REPLAY_ENABLED: &str = "makepad.mesh_replay.enabled";
/// Replay source setting id.
pub const SETTING_MESH_REPLAY_SOURCE: &str = "makepad.mesh_replay.source";
/// Replay speed setting id.
pub const SETTING_MESH_REPLAY_SPEED: &str = "makepad.mesh_replay.speed";
/// Replay opacity setting id.
pub const SETTING_MESH_REPLAY_OPACITY: &str = "makepad.mesh_replay.opacity";
/// Render scale setting id.
pub const SETTING_RENDER_SCALE: &str = "makepad.render.scale";
/// Camera streaming enable setting id.
pub const SETTING_CAMERA_STREAMING_ENABLED: &str = "makepad.camera.streaming.enabled";
/// Procedural stimulus enable setting id.
pub const SETTING_STIMULUS_ENABLED: &str = "makepad.stimulus.enabled";
/// App-private relative path to the staged Optics stimulus profile.
pub const SETTING_STIMULUS_PROFILE_PATH: &str = "makepad.stimulus.profile_path";
/// SHA-256 digest of the staged Optics stimulus profile file.
pub const SETTING_STIMULUS_PROFILE_SHA256: &str = "makepad.stimulus.profile_sha256";
/// Schema id expected for the staged Optics stimulus profile.
pub const SETTING_STIMULUS_PROFILE_SCHEMA: &str = "makepad.stimulus.profile_schema";
/// App-private relative path to the staged browser-tuning sidecar.
pub const SETTING_STIMULUS_TUNING_PATH: &str = "makepad.stimulus.tuning_path";
/// SHA-256 digest of the staged browser-tuning sidecar.
pub const SETTING_STIMULUS_TUNING_SHA256: &str = "makepad.stimulus.tuning_sha256";
/// Presentation mode requested by the staged stimulus profile.
pub const SETTING_STIMULUS_PRESENTATION_MODE: &str = "makepad.stimulus.presentation_mode";
/// Remote camera session enable setting id.
pub const SETTING_REMOTE_CAMERA_ENABLED: &str = "quest.remote_camera.enabled";
/// Remote camera session id setting id.
pub const SETTING_REMOTE_CAMERA_SESSION_ID: &str = "quest.remote_camera.session_id";
/// Remote camera topology id setting id.
pub const SETTING_REMOTE_CAMERA_TOPOLOGY_ID: &str = "quest.remote_camera.topology_id";
/// Remote camera endpoint device id setting id.
pub const SETTING_REMOTE_CAMERA_ENDPOINT_DEVICE_ID: &str = "quest.remote_camera.endpoint_device_id";
/// Remote camera endpoint device kind setting id.
pub const SETTING_REMOTE_CAMERA_ENDPOINT_DEVICE_KIND: &str =
    "quest.remote_camera.endpoint_device_kind";
/// Remote camera endpoint role setting id.
pub const SETTING_REMOTE_CAMERA_ENDPOINT_ROLE: &str = "quest.remote_camera.endpoint_role";
/// Remote camera privacy tier setting id.
pub const SETTING_REMOTE_CAMERA_PRIVACY_TIER: &str = "quest.remote_camera.privacy_tier";
/// Remote camera endpoint lane count setting id.
pub const SETTING_REMOTE_CAMERA_LANE_COUNT: &str = "quest.remote_camera.lane_count";
/// Remote camera endpoint incoming lane count setting id.
pub const SETTING_REMOTE_CAMERA_INCOMING_LANE_COUNT: &str =
    "quest.remote_camera.incoming_lane_count";
/// Remote camera endpoint outgoing lane count setting id.
pub const SETTING_REMOTE_CAMERA_OUTGOING_LANE_COUNT: &str =
    "quest.remote_camera.outgoing_lane_count";
/// Remote camera transport kind setting id.
pub const SETTING_REMOTE_CAMERA_TRANSPORT_KIND: &str = "quest.remote_camera.transport_kind";
/// Collision enable setting id.
pub const SETTING_COLLISION_ENABLED: &str = "makepad.collision.enabled";
/// SDF/ADF overlay mode setting id.
pub const SETTING_SDF_ADF_OVERLAY_MODE: &str = "makepad.sdf_adf.overlay_mode";
/// Particle overlay enable setting id.
pub const SETTING_PARTICLES_ENABLED: &str = "makepad.particles.enabled";
/// Particle renderer draw-limit setting id.
pub const SETTING_PARTICLE_RENDER_DRAW_LIMIT: &str = "makepad.particles.render.draw_limit";
/// Particle renderer animation-mode setting id.
pub const SETTING_PARTICLE_RENDER_ANIMATION_MODE: &str = "makepad.particles.render.animation_mode";
/// Particle renderer size-scale setting id.
pub const SETTING_PARTICLE_RENDER_SIZE_SCALE: &str = "makepad.particles.render.size_scale";
/// Native Matter surface-runtime leaf triangle count setting id.
pub const SETTING_MATTER_SURFACE_LEAF_TRIANGLE_COUNT: &str =
    "makepad.matter.surface_runtime.leaf_triangle_count";
/// Native Matter particle count setting id.
pub const SETTING_MATTER_PARTICLE_COUNT: &str = "makepad.particles.count";
/// Native Matter particle seed setting id.
pub const SETTING_MATTER_PARTICLE_SEED: &str = "makepad.particles.seed";
/// Native Matter particle execution backend setting id.
pub const SETTING_MATTER_PARTICLE_EXECUTION_BACKEND: &str = "makepad.particles.execution.backend";
/// Native Matter particle execution batch-size setting id.
pub const SETTING_MATTER_PARTICLE_EXECUTION_BATCH_SIZE: &str =
    "makepad.particles.execution.batch_size";
/// Native Matter particle execution max-thread setting id; zero means no cap.
pub const SETTING_MATTER_PARTICLE_EXECUTION_MAX_THREADS: &str =
    "makepad.particles.execution.max_threads";
/// Native Matter particle maximum simulated frame delta; zero means unbounded.
pub const SETTING_MATTER_PARTICLE_MAX_FRAME_DELTA_SECONDS: &str =
    "makepad.particles.simulation.max_frame_delta_seconds";
/// Native Matter particle snapshot-distance refresh policy setting id.
pub const SETTING_MATTER_PARTICLE_DISTANCE_REFRESH_POLICY: &str =
    "makepad.particles.distance_refresh_policy";
/// Native Matter particle force-source setting id.
pub const SETTING_MATTER_PARTICLE_FORCE_SOURCE: &str = "makepad.particles.force.source";
/// Adapter-level particle force-authority setting id.
pub const SETTING_PARTICLE_FORCE_AUTHORITY: &str = "makepad.particles.force.authority";
/// Native Matter particle force-source refresh interval setting id.
pub const SETTING_MATTER_PARTICLE_FORCE_UPDATE_INTERVAL_FRAMES: &str =
    "makepad.particles.force.update_interval_frames";
/// Native Matter particle bounded compare-probe count setting id.
pub const SETTING_MATTER_PARTICLE_FORCE_COMPARE_PROBE_COUNT: &str =
    "makepad.particles.force.compare_probe_count";
/// Native Matter SDF slice voxel-size setting id.
pub const SETTING_MATTER_SDF_SLICE_VOXEL_SIZE: &str = "makepad.sdf.slice.voxel_size";
/// Native Matter SDF slice max-cell setting id.
pub const SETTING_MATTER_SDF_SLICE_MAX_CELLS: &str = "makepad.sdf.slice.max_cells";
/// Native Matter ADF debug maximum subdivision depth setting id.
pub const SETTING_MATTER_ADF_DEBUG_MAX_DEPTH: &str = "makepad.adf.debug.max_depth";
/// Native Matter ADF debug maximum leaf-cell setting id.
pub const SETTING_MATTER_ADF_DEBUG_MAX_CELLS: &str = "makepad.adf.debug.max_cells";
/// Native Matter ADF debug distance-range tolerance setting id.
pub const SETTING_MATTER_ADF_DEBUG_ERROR_TOLERANCE: &str = "makepad.adf.debug.error_tolerance";
/// Native Matter SDF/ADF debug rebuild interval setting id.
pub const SETTING_MATTER_SDF_ADF_DEBUG_UPDATE_INTERVAL_FRAMES: &str =
    "makepad.sdf_adf.debug.update_interval_frames";
/// Default world-particle draw cap for current Quest Makepad billboard smoke.
pub const DEFAULT_PARTICLE_RENDER_DRAW_LIMIT: usize = 96;
/// Default particle renderer animation mode.
pub const DEFAULT_PARTICLE_RENDER_ANIMATION_MODE: ParticleRenderAnimationMode =
    ParticleRenderAnimationMode::ProceduralMorphRing;
/// Default particle renderer size scale.
pub const DEFAULT_PARTICLE_RENDER_SIZE_SCALE: f32 = 1.0;
/// Default projection footprint sample grid for app-shell contract smoke tests.
pub const DEFAULT_PROJECTION_FOOTPRINT_GRID: usize = 8;
/// Default staged Optics stimulus profile schema.
pub const DEFAULT_STIMULUS_PROFILE_SCHEMA_ID: &str = OPTICS_STIMULUS_PROFILE_SCHEMA_ID;
/// Optics stimulus volume descriptor schema accepted by this adapter boundary.
pub const STIMULUS_VOLUME_SCHEMA_ID: &str = OPTICS_STIMULUS_VOLUME_SCHEMA_ID;
/// Marker name reserved for future Quest runtime volume-profile adoption evidence.
pub const STIMULUS_VOLUME_ADOPTION_MARKER: &str = "RUSTY_QUEST_MAKEPAD_STIMULUS_VOLUME_ADOPTION";
/// Default full-screen stimulus presentation mode.
pub const DEFAULT_STIMULUS_PRESENTATION_MODE: StimulusPresentationMode =
    StimulusPresentationMode::StereoEyeField;

/// Render-side particle animation policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParticleRenderAnimationMode {
    /// Use the current animated procedural ring visual.
    ProceduralMorphRing,
    /// Keep the ring static so density tests can reduce visual animation cost.
    StaticRing,
}

impl ParticleRenderAnimationMode {
    /// Parse a stable settings token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "procedural-morph-ring" => Some(Self::ProceduralMorphRing),
            "static-ring" => Some(Self::StaticRing),
            _ => None,
        }
    }

    /// Stable marker/settings token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProceduralMorphRing => "procedural-morph-ring",
            Self::StaticRing => "static-ring",
        }
    }

    /// Whether renderer time/frame animation should be active.
    #[must_use]
    pub const fn uses_frame_animation(self) -> bool {
        matches!(self, Self::ProceduralMorphRing)
    }
}

impl Default for ParticleRenderAnimationMode {
    fn default() -> Self {
        DEFAULT_PARTICLE_RENDER_ANIMATION_MODE
    }
}

/// Full-screen stimulus presentation policy selected through settings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StimulusPresentationMode {
    /// Generate one full-viewport field and bind it to both XR eye views.
    #[default]
    StereoEyeField,
}

impl StimulusPresentationMode {
    /// Parse a stable settings token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "StereoEyeField" => Some(Self::StereoEyeField),
            _ => None,
        }
    }

    /// Stable marker/settings token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StereoEyeField => "StereoEyeField",
        }
    }
}

/// Low-rate selection for a staged Optics procedural stimulus profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StimulusEffectiveConfig {
    /// Whether the app adapter should present the staged stimulus profile.
    pub enabled: bool,
    /// Relative path under the app-private settings root.
    pub profile_path: String,
    /// Expected SHA-256 digest for the profile file.
    pub profile_sha256: String,
    /// Expected Optics profile schema id.
    pub profile_schema: String,
    /// Relative path to the browser-tuning sidecar, if staged.
    pub tuning_path: String,
    /// Expected SHA-256 digest for the tuning sidecar, if staged.
    pub tuning_sha256: String,
    /// Full-screen presentation mode.
    pub presentation_mode: StimulusPresentationMode,
}

impl StimulusEffectiveConfig {
    /// Disabled default for profiles that do not opt into stimulus playback.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            profile_path: "none".to_string(),
            profile_sha256: "none".to_string(),
            profile_schema: DEFAULT_STIMULUS_PROFILE_SCHEMA_ID.to_string(),
            tuning_path: "none".to_string(),
            tuning_sha256: "none".to_string(),
            presentation_mode: DEFAULT_STIMULUS_PRESENTATION_MODE,
        }
    }
}

/// Complete camera-shell subset of the canonical effective-settings report.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraShellEffectiveConfig {
    /// Replay settings.
    pub replay: CameraShellReplayConfig,
    /// Render scale for the Makepad/XR runtime.
    pub render_scale: f32,
    /// Whether the app shell should acquire direct or broker camera frames.
    pub camera_streaming_enabled: bool,
    /// Staged full-screen Optics stimulus profile selection.
    pub stimulus: StimulusEffectiveConfig,
    /// Remote camera session handoff settings for this Quest endpoint.
    pub remote_camera: RemoteCameraEffectiveConfig,
    /// Whether collision behavior is enabled.
    pub collision_enabled: bool,
    /// SDF/ADF overlay mode.
    pub sdf_adf_overlay_mode: SdfAdfOverlayMode,
    /// Whether particle behavior is enabled.
    pub particles_enabled: bool,
    /// Adapter-level force authority requested by the profile.
    pub particle_force_authority: QuestMakepadForceAuthorityMode,
    /// Low-rate receipt for the live-vs-recorded provider A/B promotion gate.
    pub gpu_force_provider_ab_receipt: QuestMakepadGpuForceProviderAbReceipt,
    /// Makepad-side particle render draw cap; does not change Matter truth.
    pub particle_render_draw_limit: usize,
    /// Render-side particle animation mode; does not change Matter truth.
    pub particle_render_animation_mode: ParticleRenderAnimationMode,
    /// Render-side particle size scale; does not change Matter truth.
    pub particle_render_size_scale: f32,
    /// Native Matter surface runtime config derived from effective settings.
    pub matter_surface: QuestMakepadMatterSurfaceConfig,
}

impl CameraShellEffectiveConfig {
    /// Parse the app-facing camera-shell config from canonical effective
    /// settings.
    pub fn from_effective_settings_json(json: &str) -> Result<Self, CameraShellConfigError> {
        let value: Value =
            serde_json::from_str(json).map_err(|_| CameraShellConfigError::MalformedJson)?;
        validate_header(&value)?;
        let settings = settings_array(&value)?;
        let replay = CameraShellReplayConfig::from_settings(settings)?;
        let render_scale = parse_f32_setting(settings, SETTING_RENDER_SCALE)?;
        let camera_streaming_enabled =
            parse_bool_setting(settings, SETTING_CAMERA_STREAMING_ENABLED)?;
        let stimulus = parse_stimulus_config(settings)?;
        let remote_camera = parse_remote_camera_config(settings)?;
        let collision_enabled = parse_bool_setting(settings, SETTING_COLLISION_ENABLED)?;
        let sdf_adf_overlay_mode = parse_sdf_adf_overlay_mode(settings)?;
        let particles_enabled = parse_bool_setting(settings, SETTING_PARTICLES_ENABLED)?;
        let particle_force_authority = parse_particle_force_authority_or_default(
            settings,
            SETTING_PARTICLE_FORCE_AUTHORITY,
            QuestMakepadForceAuthorityMode::default(),
        )?;
        let gpu_force_provider_ab_receipt = parse_gpu_force_provider_ab_receipt_or_default(
            settings,
            SETTING_GPU_FORCE_LIVE_RECORDED_PROVIDER_AB_RECEIPT,
            QuestMakepadGpuForceProviderAbReceipt::default(),
        )?;
        let particle_render_draw_limit = parse_usize_setting_or_default(
            settings,
            SETTING_PARTICLE_RENDER_DRAW_LIMIT,
            DEFAULT_PARTICLE_RENDER_DRAW_LIMIT,
        )?;
        let particle_render_animation_mode = parse_particle_render_animation_mode_or_default(
            settings,
            SETTING_PARTICLE_RENDER_ANIMATION_MODE,
            DEFAULT_PARTICLE_RENDER_ANIMATION_MODE,
        )?;
        let particle_render_size_scale = parse_positive_f32_setting_or_default(
            settings,
            SETTING_PARTICLE_RENDER_SIZE_SCALE,
            DEFAULT_PARTICLE_RENDER_SIZE_SCALE,
        )?;
        let matter_surface = parse_matter_surface_config(
            settings,
            replay.enabled,
            collision_enabled,
            sdf_adf_overlay_mode,
            particles_enabled,
            particle_render_draw_limit,
        )?;

        Ok(Self {
            replay,
            render_scale,
            camera_streaming_enabled,
            stimulus,
            remote_camera,
            collision_enabled,
            sdf_adf_overlay_mode,
            particles_enabled,
            particle_force_authority,
            gpu_force_provider_ab_receipt,
            particle_render_draw_limit,
            particle_render_animation_mode,
            particle_render_size_scale,
            matter_surface,
        })
    }
}

/// Remote camera low-rate session handoff consumed by the Quest Makepad shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteCameraEffectiveConfig {
    /// Whether the app adapter should prepare a remote camera session.
    pub enabled: bool,
    /// Quest-owned remote camera session id.
    pub session_id: String,
    /// Topology id from the Quest remote-camera plan.
    pub topology_id: String,
    /// Endpoint device id represented by this app instance.
    pub endpoint_device_id: String,
    /// Endpoint device kind.
    pub endpoint_device_kind: String,
    /// Derived endpoint role.
    pub endpoint_role: String,
    /// Privacy tier from the validated Quest plan.
    pub privacy_tier: String,
    /// Total lanes involving this endpoint.
    pub lane_count: usize,
    /// Incoming lanes for this endpoint.
    pub incoming_lane_count: usize,
    /// Outgoing lanes for this endpoint.
    pub outgoing_lane_count: usize,
    /// Transport kind declared by the Quest profile.
    pub transport_kind: String,
}

impl RemoteCameraEffectiveConfig {
    /// Disabled default for older bundles or profiles that do not opt in.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            session_id: "none".to_string(),
            topology_id: "none".to_string(),
            endpoint_device_id: "none".to_string(),
            endpoint_device_kind: "none".to_string(),
            endpoint_role: "disabled".to_string(),
            privacy_tier: "local_lan_diagnostic".to_string(),
            lane_count: 0,
            incoming_lane_count: 0,
            outgoing_lane_count: 0,
            transport_kind: "none".to_string(),
        }
    }
}

/// App-facing runtime bundle built from one canonical effective-settings
/// report.
#[derive(Debug)]
pub struct CameraShellRuntimeBundle {
    /// Full effective settings config consumed by the camera shell.
    pub effective_config: CameraShellEffectiveConfig,
    /// Mesh replay runtime configured from the replay subset.
    pub mesh_replay_runtime: MeshReplayRuntime,
    /// Native Matter surface runtime configured from the feature subset.
    pub matter_surface_runtime: QuestMakepadMatterSurfaceRuntime,
}

/// Build the full app-facing runtime bundle from canonical effective settings
/// JSON.
pub fn camera_shell_runtime_bundle_from_effective_settings_json(
    json: &str,
) -> Result<CameraShellRuntimeBundle, CameraShellConfigError> {
    let effective_config = CameraShellEffectiveConfig::from_effective_settings_json(json)?;
    build_camera_shell_runtime_bundle(effective_config, None)
}

/// Build the full app-facing runtime bundle from canonical effective settings
/// JSON and a directory that may contain external replay data-plane assets.
pub fn camera_shell_runtime_bundle_from_effective_settings_json_with_replay_asset_dir(
    json: &str,
    replay_asset_dir: &Path,
) -> Result<CameraShellRuntimeBundle, CameraShellConfigError> {
    let effective_config = CameraShellEffectiveConfig::from_effective_settings_json(json)?;
    build_camera_shell_runtime_bundle(effective_config, Some(replay_asset_dir))
}

fn build_camera_shell_runtime_bundle(
    effective_config: CameraShellEffectiveConfig,
    replay_asset_dir: Option<&Path>,
) -> Result<CameraShellRuntimeBundle, CameraShellConfigError> {
    let mesh_replay_runtime =
        mesh_replay_runtime_from_config(&effective_config.replay, replay_asset_dir)?;
    let matter_surface_runtime =
        QuestMakepadMatterSurfaceRuntime::new(effective_config.matter_surface.clone())
            .map_err(|error| CameraShellConfigError::MatterSurfaceRuntime(error.to_string()))?;
    Ok(CameraShellRuntimeBundle {
        effective_config,
        mesh_replay_runtime,
        matter_surface_runtime,
    })
}

/// SDF/ADF overlay modes exposed by the camera shell settings surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdfAdfOverlayMode {
    /// No SDF/ADF overlay.
    Off,
    /// SDF overlay.
    Sdf,
    /// ADF overlay.
    Adf,
    /// Combined SDF and ADF overlay.
    Combined,
}

impl SdfAdfOverlayMode {
    /// Stable setting value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Sdf => "sdf",
            Self::Adf => "adf",
            Self::Combined => "combined",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "sdf" => Some(Self::Sdf),
            "adf" => Some(Self::Adf),
            "combined" => Some(Self::Combined),
            _ => None,
        }
    }
}

/// Runtime-gated Matter SDF/ADF mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdfAdfRuntimeMode {
    /// No Matter SDF output.
    Off,
    /// Matter-backed SDF output.
    Sdf,
    /// Matter-backed ADF debug output.
    Adf,
    /// Combined SDF/ADF was requested but simultaneous output is not wired yet.
    UnsupportedCombined,
}

impl SdfAdfRuntimeMode {
    /// Builds the runtime-gated mode from the user-facing setting.
    #[must_use]
    pub const fn from_overlay_mode(mode: SdfAdfOverlayMode) -> Self {
        match mode {
            SdfAdfOverlayMode::Off => Self::Off,
            SdfAdfOverlayMode::Sdf => Self::Sdf,
            SdfAdfOverlayMode::Adf => Self::Adf,
            SdfAdfOverlayMode::Combined => Self::UnsupportedCombined,
        }
    }

    /// Returns whether Matter-backed SDF slice output is allowed.
    #[must_use]
    pub const fn matter_sdf_enabled(self) -> bool {
        matches!(self, Self::Sdf)
    }

    /// Returns whether Matter-backed ADF debug output is allowed.
    #[must_use]
    pub const fn matter_adf_enabled(self) -> bool {
        matches!(self, Self::Adf)
    }

    /// Returns whether this mode is an unsupported future ADF placeholder.
    #[must_use]
    pub const fn is_unsupported_adf_placeholder(self) -> bool {
        matches!(self, Self::UnsupportedCombined)
    }

    /// Stable marker status label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Sdf => "sdf",
            Self::Adf => "adf",
            Self::UnsupportedCombined => "unsupported_combined",
        }
    }
}

/// Replay subset of the camera shell effective settings.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraShellReplayConfig {
    /// Whether replay is enabled.
    pub enabled: bool,
    /// Replay source id or path label.
    pub source: String,
    /// Playback speed multiplier.
    pub speed: f32,
    /// Overlay opacity.
    pub opacity: f32,
}

impl CameraShellReplayConfig {
    /// Parse the replay config from a canonical effective-settings report.
    pub fn from_effective_settings_json(json: &str) -> Result<Self, CameraShellConfigError> {
        let value: Value =
            serde_json::from_str(json).map_err(|_| CameraShellConfigError::MalformedJson)?;
        validate_header(&value)?;
        let settings = settings_array(&value)?;
        Self::from_settings(settings)
    }

    fn from_settings(settings: &[Value]) -> Result<Self, CameraShellConfigError> {
        let enabled = setting_value(settings, SETTING_MESH_REPLAY_ENABLED)?
            .as_bool()
            .ok_or(CameraShellConfigError::InvalidSettingValue(
                SETTING_MESH_REPLAY_ENABLED,
            ))?;
        let source = setting_value(settings, SETTING_MESH_REPLAY_SOURCE)?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .ok_or(CameraShellConfigError::InvalidSettingValue(
                SETTING_MESH_REPLAY_SOURCE,
            ))?;
        let speed = parse_f32_setting(settings, SETTING_MESH_REPLAY_SPEED)?;
        let opacity = parse_f32_setting(settings, SETTING_MESH_REPLAY_OPACITY)?;

        Ok(Self {
            enabled,
            source,
            speed,
            opacity,
        })
    }

    /// Convert to the reusable mesh replay runtime configuration.
    #[must_use]
    pub fn into_mesh_replay_config(self) -> MeshReplayConfig {
        MeshReplayConfig::normalized(self.enabled, self.source, self.speed, self.opacity)
    }
}

/// Build mesh replay config from canonical effective settings JSON.
pub fn mesh_replay_config_from_effective_settings_json(
    json: &str,
) -> Result<MeshReplayConfig, CameraShellConfigError> {
    CameraShellReplayConfig::from_effective_settings_json(json)
        .map(CameraShellReplayConfig::into_mesh_replay_config)
}

/// Build a mesh replay runtime from canonical effective settings JSON.
pub fn mesh_replay_runtime_from_effective_settings_json(
    json: &str,
) -> Result<MeshReplayRuntime, CameraShellConfigError> {
    camera_shell_runtime_bundle_from_effective_settings_json(json)
        .map(|bundle| bundle.mesh_replay_runtime)
}

/// Build a mesh replay runtime from canonical effective settings and an
/// external replay asset directory.
pub fn mesh_replay_runtime_from_effective_settings_json_with_replay_asset_dir(
    json: &str,
    replay_asset_dir: &Path,
) -> Result<MeshReplayRuntime, CameraShellConfigError> {
    camera_shell_runtime_bundle_from_effective_settings_json_with_replay_asset_dir(
        json,
        replay_asset_dir,
    )
    .map(|bundle| bundle.mesh_replay_runtime)
}

/// Verified staged Optics stimulus profile payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StimulusProfilePayload {
    /// Effective low-rate stimulus selection.
    pub config: StimulusEffectiveConfig,
    /// Resolved app-private profile path.
    pub profile_path: PathBuf,
    /// Profile id read from the staged JSON.
    pub profile_id: String,
    /// Profile schema id read from the staged JSON.
    pub profile_schema: String,
    /// Raw staged profile JSON.
    pub profile_json: String,
    /// Verified SHA-256 digest of the raw staged profile JSON.
    pub profile_sha256: String,
    /// Compact volume/compute summary extracted from the Optics profile.
    pub volume_summary: StimulusVolumeProfileSummary,
}

/// Compact adapter-facing summary of Optics stimulus volume intent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StimulusVolumeProfileSummary {
    /// Whether the profile declares a volume descriptor.
    pub volume_present: bool,
    /// Volume descriptor schema id.
    pub volume_schema: Option<String>,
    /// Volume id read from the staged profile.
    pub volume_id: Option<String>,
    /// Optics volume field kind.
    pub field_kind: Option<String>,
    /// Intended renderer storage class.
    pub storage_hint: Option<String>,
    /// Bounded descriptor grid dimensions.
    pub grid_dimensions: Option<[u64; 3]>,
    /// Bounded raymarch/probe step count.
    pub step_count: Option<u64>,
    /// Kernel ABI id selected by the profile.
    pub kernel_abi_id: Option<String>,
    /// Count of declared compute passes in the Optics ABI.
    pub compute_pass_count: usize,
    /// Bounded readback samples declared by the volume probe pass.
    pub volume_readback_probe_samples: Option<u64>,
    /// Output layers declared by the stereo raymarch pass.
    pub stereo_field_output_layers: Option<u64>,
}

impl From<OpticsStimulusVolumeProfileSummary> for StimulusVolumeProfileSummary {
    fn from(summary: OpticsStimulusVolumeProfileSummary) -> Self {
        Self {
            volume_present: summary.volume_present,
            volume_schema: summary.volume_schema,
            volume_id: summary.volume_id,
            field_kind: summary.field_kind,
            storage_hint: summary.storage_hint,
            grid_dimensions: summary.grid_dimensions,
            step_count: summary.step_count,
            kernel_abi_id: summary.kernel_abi_id,
            compute_pass_count: summary.compute_pass_count,
            volume_readback_probe_samples: summary.volume_readback_probe_samples,
            stereo_field_output_layers: summary.stereo_field_output_layers,
        }
    }
}

/// Load and verify the staged Optics stimulus profile selected by effective
/// settings. The JSON body stays a renderer-neutral Optics payload; this helper
/// only proves identity and full-screen stereo intent before a Quest renderer
/// adapter lowers it to Vulkan/Makepad resources.
pub fn stimulus_profile_payload_from_effective_settings_json_with_root(
    json: &str,
    settings_root: &Path,
) -> Result<Option<StimulusProfilePayload>, CameraShellConfigError> {
    let effective_config = CameraShellEffectiveConfig::from_effective_settings_json(json)?;
    let config = effective_config.stimulus;
    if !config.enabled {
        return Ok(None);
    }

    let profile_path = settings_root.join(&config.profile_path);
    let profile_json = std::fs::read_to_string(&profile_path)
        .map_err(|error| CameraShellConfigError::StimulusProfileAsset(error.to_string()))?;
    let profile_sha256 = sha256_hex(profile_json.as_bytes());
    if profile_sha256 != config.profile_sha256 {
        return Err(CameraShellConfigError::StimulusProfileAsset(format!(
            "profile sha256 mismatch: expected {}, got {}",
            config.profile_sha256, profile_sha256
        )));
    }
    let profile: Value = serde_json::from_str(&profile_json)
        .map_err(|_| CameraShellConfigError::StimulusProfileAsset("malformed JSON".to_string()))?;
    let profile_schema = profile
        .get("schema_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CameraShellConfigError::StimulusProfileAsset("missing schema_id".to_string())
        })?;
    if profile_schema != config.profile_schema {
        return Err(CameraShellConfigError::StimulusProfileAsset(format!(
            "profile schema mismatch: expected {}, got {}",
            config.profile_schema, profile_schema
        )));
    }
    let presentation = profile.get("presentation").ok_or_else(|| {
        CameraShellConfigError::StimulusProfileAsset("missing presentation".to_string())
    })?;
    let presentation_mode = presentation
        .get("mode")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CameraShellConfigError::StimulusProfileAsset("missing presentation.mode".to_string())
        })?;
    if presentation_mode != config.presentation_mode.as_str() {
        return Err(CameraShellConfigError::StimulusProfileAsset(format!(
            "presentation mode mismatch: expected {}, got {}",
            config.presentation_mode.as_str(),
            presentation_mode
        )));
    }
    if presentation.get("coverage").and_then(Value::as_str) != Some("FullViewport")
        || presentation.get("eye_count").and_then(Value::as_u64) != Some(2)
    {
        return Err(CameraShellConfigError::StimulusProfileAsset(
            "stimulus profile must target full-viewport stereo eyes".to_string(),
        ));
    }
    let profile_id = profile
        .get("profile_id")
        .and_then(Value::as_str)
        .unwrap_or("stimulus.profile.unknown")
        .to_string();
    let volume_summary = stimulus_volume_profile_summary(&profile, &profile_json)?;

    Ok(Some(StimulusProfilePayload {
        config,
        profile_path,
        profile_id,
        profile_schema: profile_schema.to_string(),
        profile_json,
        profile_sha256,
        volume_summary,
    }))
}

fn stimulus_volume_profile_summary(
    profile: &Value,
    profile_json: &str,
) -> Result<StimulusVolumeProfileSummary, CameraShellConfigError> {
    if let Some(summary) = typed_optics_stimulus_volume_profile_summary(profile, profile_json)? {
        return Ok(summary);
    }
    stimulus_volume_profile_summary_legacy(profile)
}

fn typed_optics_stimulus_volume_profile_summary(
    profile: &Value,
    profile_json: &str,
) -> Result<Option<StimulusVolumeProfileSummary>, CameraShellConfigError> {
    match serde_json::from_str::<OpticsStimulusProfile>(profile_json) {
        Ok(optics_profile) => {
            let summary = OpticsStimulusVolumeProfileSummary::from_profile(&optics_profile)
                .map_err(|error| CameraShellConfigError::StimulusProfileAsset(error.to_string()))?;
            if summary.volume_present {
                summary
                    .validate_bounded_stereo_preview(512)
                    .map_err(|error| {
                        CameraShellConfigError::StimulusProfileAsset(error.to_string())
                    })?;
            }
            Ok(Some(summary.into()))
        }
        Err(error) if looks_like_full_optics_profile(profile) => {
            Err(CameraShellConfigError::StimulusProfileAsset(format!(
                "invalid Optics stimulus profile: {error}"
            )))
        }
        Err(_) => Ok(None),
    }
}

fn looks_like_full_optics_profile(profile: &Value) -> bool {
    profile.get("layer_graph").is_some()
        || profile.get("temporal").is_some()
        || profile.get("safety").is_some()
}

fn stimulus_volume_profile_summary_legacy(
    profile: &Value,
) -> Result<StimulusVolumeProfileSummary, CameraShellConfigError> {
    let Some(volume) = profile.get("volume") else {
        return Ok(StimulusVolumeProfileSummary::default());
    };
    let volume_schema = required_profile_string(volume, "volume", "schema_id")?;
    if volume_schema != STIMULUS_VOLUME_SCHEMA_ID {
        return Err(CameraShellConfigError::StimulusProfileAsset(format!(
            "unsupported stimulus volume schema: {volume_schema}"
        )));
    }
    let volume_id = required_profile_string(volume, "volume", "volume_id")?;
    let field_kind = required_profile_string(volume, "volume", "field_kind")?;
    let storage_hint = required_profile_string(volume, "volume", "storage_hint")?;
    let grid_dimensions = required_profile_u64_array3(volume, "volume", "grid_dimensions")?;
    if grid_dimensions
        .iter()
        .any(|value| *value == 0 || *value > 512)
    {
        return Err(CameraShellConfigError::StimulusProfileAsset(
            "volume grid_dimensions must be within 1..=512 per axis".to_string(),
        ));
    }
    let step_count = required_profile_u64(volume, "volume", "step_count")?;
    if !(1..=256).contains(&step_count) {
        return Err(CameraShellConfigError::StimulusProfileAsset(
            "volume step_count must be within 1..=256".to_string(),
        ));
    }

    let kernel_abi = profile.get("kernel_abi").ok_or_else(|| {
        CameraShellConfigError::StimulusProfileAsset(
            "volume profile requires kernel_abi".to_string(),
        )
    })?;
    let kernel_abi_id = required_profile_string(kernel_abi, "kernel_abi", "abi_id")?;
    let compute_passes = kernel_abi
        .get("compute_passes")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CameraShellConfigError::StimulusProfileAsset(
                "kernel_abi.compute_passes must be an array".to_string(),
            )
        })?;

    let volume_readback_probe_samples = compute_passes
        .iter()
        .find(|pass| pass.get("kind").and_then(Value::as_str) == Some("VolumeReadbackProbe"))
        .and_then(|pass| pass.get("bounded_readback_samples").and_then(Value::as_u64));
    let stereo_field_output_layers = compute_passes
        .iter()
        .find(|pass| pass.get("kind").and_then(Value::as_str) == Some("VolumeRaymarchStereoField"))
        .and_then(|pass| pass.get("output_layers").and_then(Value::as_u64));

    if volume_readback_probe_samples
        .map(|samples| samples == 0 || samples > 512)
        .unwrap_or(true)
    {
        return Err(CameraShellConfigError::StimulusProfileAsset(
            "volume profile requires a bounded VolumeReadbackProbe pass".to_string(),
        ));
    }
    if stereo_field_output_layers != Some(2) {
        return Err(CameraShellConfigError::StimulusProfileAsset(
            "volume profile requires a two-layer VolumeRaymarchStereoField pass".to_string(),
        ));
    }

    Ok(StimulusVolumeProfileSummary {
        volume_present: true,
        volume_schema: Some(volume_schema),
        volume_id: Some(volume_id),
        field_kind: Some(field_kind),
        storage_hint: Some(storage_hint),
        grid_dimensions: Some(grid_dimensions),
        step_count: Some(step_count),
        kernel_abi_id: Some(kernel_abi_id),
        compute_pass_count: compute_passes.len(),
        volume_readback_probe_samples,
        stereo_field_output_layers,
    })
}

fn required_profile_string(
    object: &Value,
    context: &str,
    field: &str,
) -> Result<String, CameraShellConfigError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            CameraShellConfigError::StimulusProfileAsset(format!(
                "missing or invalid {context}.{field}"
            ))
        })
}

fn required_profile_u64(
    object: &Value,
    context: &str,
    field: &str,
) -> Result<u64, CameraShellConfigError> {
    object.get(field).and_then(Value::as_u64).ok_or_else(|| {
        CameraShellConfigError::StimulusProfileAsset(format!(
            "missing or invalid {context}.{field}"
        ))
    })
}

fn required_profile_u64_array3(
    object: &Value,
    context: &str,
    field: &str,
) -> Result<[u64; 3], CameraShellConfigError> {
    let values = object.get(field).and_then(Value::as_array).ok_or_else(|| {
        CameraShellConfigError::StimulusProfileAsset(format!(
            "missing or invalid {context}.{field}"
        ))
    })?;
    if values.len() != 3 {
        return Err(CameraShellConfigError::StimulusProfileAsset(format!(
            "{context}.{field} must contain exactly three values"
        )));
    }
    let mut output = [0_u64; 3];
    for (index, value) in values.iter().enumerate() {
        output[index] = value.as_u64().ok_or_else(|| {
            CameraShellConfigError::StimulusProfileAsset(format!(
                "{context}.{field}[{index}] must be an unsigned integer"
            ))
        })?;
    }
    Ok(output)
}

/// Baseline projection reports derived from a Lattice display view set.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraShellProjectionReports {
    /// Source Lattice view set id.
    pub view_set_id: String,
    /// Left view projection report.
    pub left: ProjectionGeometryReport,
    /// Right view projection report.
    pub right: ProjectionGeometryReport,
}

/// Parse a Lattice display view set JSON payload and derive baseline Optics
/// projection geometry reports.
pub fn projection_reports_from_lattice_view_set_json(
    json: &str,
) -> Result<CameraShellProjectionReports, CameraShellConfigError> {
    let view_set: DisplayViewSet = serde_json::from_str(json)
        .map_err(|_| CameraShellConfigError::MalformedDisplayViewSetJson)?;
    projection_reports_from_lattice_view_set(&view_set)
}

/// Derive baseline Optics projection geometry reports from a Lattice view set.
pub fn projection_reports_from_lattice_view_set(
    view_set: &DisplayViewSet,
) -> Result<CameraShellProjectionReports, CameraShellConfigError> {
    if let Err(errors) = validate_display_view_set(view_set) {
        let message = errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(CameraShellConfigError::InvalidDisplayViewSet(message));
    }

    let source_valid_uv_rect = Rect2::UNIT;
    let left = ProjectionGeometryReport::from_homographies(
        format!("{}.left.projection", view_set.view_set_id),
        "left",
        VideoProjectionMapping::FullFrameSurface,
        IDENTITY_HOMOGRAPHY,
        IDENTITY_HOMOGRAPHY,
        source_valid_uv_rect,
        DEFAULT_PROJECTION_FOOTPRINT_GRID,
    )
    .map_err(|error| CameraShellConfigError::ProjectionReport(error.to_string()))?;
    let right = ProjectionGeometryReport::from_homographies(
        format!("{}.right.projection", view_set.view_set_id),
        "right",
        VideoProjectionMapping::FullFrameSurface,
        IDENTITY_HOMOGRAPHY,
        IDENTITY_HOMOGRAPHY,
        source_valid_uv_rect,
        DEFAULT_PROJECTION_FOOTPRINT_GRID,
    )
    .map_err(|error| CameraShellConfigError::ProjectionReport(error.to_string()))?;

    Ok(CameraShellProjectionReports {
        view_set_id: view_set.view_set_id.clone(),
        left,
        right,
    })
}

/// Camera shell effective-settings parsing error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CameraShellConfigError {
    /// JSON could not be parsed.
    MalformedJson,
    /// Required field is missing.
    MissingField(&'static str),
    /// Effective-settings schema id is unexpected.
    UnexpectedSchema(Option<String>),
    /// Effective-settings app id is unexpected.
    UnexpectedApp(Option<String>),
    /// Required setting id is absent.
    MissingSetting(&'static str),
    /// Setting value has the wrong type or range.
    InvalidSettingValue(&'static str),
    /// Lattice display view set JSON could not be parsed.
    MalformedDisplayViewSetJson,
    /// Lattice display view set failed validation.
    InvalidDisplayViewSet(String),
    /// Optics projection report could not be built.
    ProjectionReport(String),
    /// Native Matter surface runtime could not be built.
    MatterSurfaceRuntime(String),
    /// External replay source assets are missing or invalid.
    MeshReplayAsset(String),
    /// Staged Optics stimulus profile assets are missing or invalid.
    StimulusProfileAsset(String),
}

impl std::fmt::Display for CameraShellConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedJson => f.write_str("malformed effective-settings JSON"),
            Self::MissingField(field) => write!(f, "missing field {field}"),
            Self::UnexpectedSchema(found) => write!(
                f,
                "unexpected effective-settings schema {}",
                found.as_deref().unwrap_or("<missing>")
            ),
            Self::UnexpectedApp(found) => write!(
                f,
                "unexpected effective-settings app id {}",
                found.as_deref().unwrap_or("<missing>")
            ),
            Self::MissingSetting(setting_id) => write!(f, "missing setting {setting_id}"),
            Self::InvalidSettingValue(setting_id) => {
                write!(f, "invalid value for setting {setting_id}")
            }
            Self::MalformedDisplayViewSetJson => f.write_str("malformed display view set JSON"),
            Self::InvalidDisplayViewSet(message) => {
                write!(f, "invalid display view set: {message}")
            }
            Self::ProjectionReport(message) => write!(f, "invalid projection report: {message}"),
            Self::MatterSurfaceRuntime(message) => {
                write!(f, "invalid Matter surface runtime: {message}")
            }
            Self::MeshReplayAsset(message) => write!(f, "invalid mesh replay asset: {message}"),
            Self::StimulusProfileAsset(message) => {
                write!(f, "invalid stimulus profile asset: {message}")
            }
        }
    }
}

impl std::error::Error for CameraShellConfigError {}

fn validate_header(value: &Value) -> Result<(), CameraShellConfigError> {
    let schema = value.get("schema").and_then(Value::as_str);
    if schema != Some(EFFECTIVE_SETTINGS_SCHEMA_ID) {
        return Err(CameraShellConfigError::UnexpectedSchema(
            schema.map(str::to_string),
        ));
    }

    let app_id = value.get("app_id").and_then(Value::as_str);
    if app_id != Some(CAMERA_SHELL_APP_ID) {
        return Err(CameraShellConfigError::UnexpectedApp(
            app_id.map(str::to_string),
        ));
    }

    Ok(())
}

fn setting_value<'a>(
    settings: &'a [Value],
    setting_id: &'static str,
) -> Result<&'a Value, CameraShellConfigError> {
    settings
        .iter()
        .find(|setting| {
            setting
                .get("setting_id")
                .and_then(Value::as_str)
                .is_some_and(|candidate| candidate == setting_id)
        })
        .ok_or(CameraShellConfigError::MissingSetting(setting_id))
        .and_then(|setting| {
            setting
                .get("value")
                .ok_or(CameraShellConfigError::MissingField("value"))
        })
}

fn settings_array(value: &Value) -> Result<&[Value], CameraShellConfigError> {
    value
        .get("settings")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or(CameraShellConfigError::MissingField("settings"))
}

fn parse_f32_setting(
    settings: &[Value],
    setting_id: &'static str,
) -> Result<f32, CameraShellConfigError> {
    setting_value(settings, setting_id)?
        .as_f64()
        .filter(|number| number.is_finite())
        .map(|number| number as f32)
        .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id))
}

fn parse_bool_setting(
    settings: &[Value],
    setting_id: &'static str,
) -> Result<bool, CameraShellConfigError> {
    setting_value(settings, setting_id)?
        .as_bool()
        .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id))
}

fn parse_remote_camera_config(
    settings: &[Value],
) -> Result<RemoteCameraEffectiveConfig, CameraShellConfigError> {
    let enabled = parse_bool_setting_or_default(settings, SETTING_REMOTE_CAMERA_ENABLED, false)?;
    let session_id =
        parse_string_setting_or_default(settings, SETTING_REMOTE_CAMERA_SESSION_ID, "none")?;
    let topology_id =
        parse_string_setting_or_default(settings, SETTING_REMOTE_CAMERA_TOPOLOGY_ID, "none")?;
    let endpoint_device_id = parse_string_setting_or_default(
        settings,
        SETTING_REMOTE_CAMERA_ENDPOINT_DEVICE_ID,
        "none",
    )?;
    let endpoint_device_kind = parse_string_setting_or_default(
        settings,
        SETTING_REMOTE_CAMERA_ENDPOINT_DEVICE_KIND,
        "none",
    )?;
    let endpoint_role =
        parse_string_setting_or_default(settings, SETTING_REMOTE_CAMERA_ENDPOINT_ROLE, "disabled")?;
    let privacy_tier = parse_string_setting_or_default(
        settings,
        SETTING_REMOTE_CAMERA_PRIVACY_TIER,
        "local_lan_diagnostic",
    )?;
    let lane_count = parse_usize_setting_or_default(settings, SETTING_REMOTE_CAMERA_LANE_COUNT, 0)?;
    let incoming_lane_count =
        parse_usize_setting_or_default(settings, SETTING_REMOTE_CAMERA_INCOMING_LANE_COUNT, 0)?;
    let outgoing_lane_count =
        parse_usize_setting_or_default(settings, SETTING_REMOTE_CAMERA_OUTGOING_LANE_COUNT, 0)?;
    let transport_kind =
        parse_string_setting_or_default(settings, SETTING_REMOTE_CAMERA_TRANSPORT_KIND, "none")?;

    if enabled {
        if session_id == "none" || topology_id == "none" || endpoint_device_id == "none" {
            return Err(CameraShellConfigError::InvalidSettingValue(
                SETTING_REMOTE_CAMERA_SESSION_ID,
            ));
        }
        if endpoint_role == "disabled" || transport_kind == "none" || lane_count == 0 {
            return Err(CameraShellConfigError::InvalidSettingValue(
                SETTING_REMOTE_CAMERA_ENDPOINT_ROLE,
            ));
        }
        if incoming_lane_count + outgoing_lane_count != lane_count {
            return Err(CameraShellConfigError::InvalidSettingValue(
                SETTING_REMOTE_CAMERA_LANE_COUNT,
            ));
        }
    }

    Ok(RemoteCameraEffectiveConfig {
        enabled,
        session_id,
        topology_id,
        endpoint_device_id,
        endpoint_device_kind,
        endpoint_role,
        privacy_tier,
        lane_count,
        incoming_lane_count,
        outgoing_lane_count,
        transport_kind,
    })
}

fn parse_stimulus_config(
    settings: &[Value],
) -> Result<StimulusEffectiveConfig, CameraShellConfigError> {
    let enabled = parse_bool_setting_or_default(settings, SETTING_STIMULUS_ENABLED, false)?;
    let profile_path =
        parse_string_setting_or_default(settings, SETTING_STIMULUS_PROFILE_PATH, "none")?;
    let profile_sha256 =
        parse_string_setting_or_default(settings, SETTING_STIMULUS_PROFILE_SHA256, "none")?;
    let profile_schema = parse_string_setting_or_default(
        settings,
        SETTING_STIMULUS_PROFILE_SCHEMA,
        DEFAULT_STIMULUS_PROFILE_SCHEMA_ID,
    )?;
    let tuning_path =
        parse_string_setting_or_default(settings, SETTING_STIMULUS_TUNING_PATH, "none")?;
    let tuning_sha256 =
        parse_string_setting_or_default(settings, SETTING_STIMULUS_TUNING_SHA256, "none")?;
    let presentation_mode = parse_stimulus_presentation_mode_or_default(
        settings,
        SETTING_STIMULUS_PRESENTATION_MODE,
        DEFAULT_STIMULUS_PRESENTATION_MODE,
    )?;

    if enabled {
        if is_none_token(&profile_path) || !is_safe_relative_payload_path(&profile_path) {
            return Err(CameraShellConfigError::InvalidSettingValue(
                SETTING_STIMULUS_PROFILE_PATH,
            ));
        }
        if !is_sha256_hex(&profile_sha256) {
            return Err(CameraShellConfigError::InvalidSettingValue(
                SETTING_STIMULUS_PROFILE_SHA256,
            ));
        }
        if profile_schema != DEFAULT_STIMULUS_PROFILE_SCHEMA_ID {
            return Err(CameraShellConfigError::InvalidSettingValue(
                SETTING_STIMULUS_PROFILE_SCHEMA,
            ));
        }
    }
    if !is_none_token(&tuning_path) && !is_safe_relative_payload_path(&tuning_path) {
        return Err(CameraShellConfigError::InvalidSettingValue(
            SETTING_STIMULUS_TUNING_PATH,
        ));
    }
    if !is_none_token(&tuning_sha256) && !is_sha256_hex(&tuning_sha256) {
        return Err(CameraShellConfigError::InvalidSettingValue(
            SETTING_STIMULUS_TUNING_SHA256,
        ));
    }
    if is_none_token(&tuning_path) && !is_none_token(&tuning_sha256) {
        return Err(CameraShellConfigError::InvalidSettingValue(
            SETTING_STIMULUS_TUNING_PATH,
        ));
    }

    Ok(StimulusEffectiveConfig {
        enabled,
        profile_path,
        profile_sha256,
        profile_schema,
        tuning_path,
        tuning_sha256,
        presentation_mode,
    })
}

fn parse_sdf_adf_overlay_mode(
    settings: &[Value],
) -> Result<SdfAdfOverlayMode, CameraShellConfigError> {
    setting_value(settings, SETTING_SDF_ADF_OVERLAY_MODE)?
        .as_str()
        .and_then(SdfAdfOverlayMode::parse)
        .ok_or(CameraShellConfigError::InvalidSettingValue(
            SETTING_SDF_ADF_OVERLAY_MODE,
        ))
}

fn parse_matter_surface_config(
    settings: &[Value],
    replay_enabled: bool,
    collision_enabled: bool,
    overlay_mode: SdfAdfOverlayMode,
    particles_enabled: bool,
    particle_render_draw_limit: usize,
) -> Result<QuestMakepadMatterSurfaceConfig, CameraShellConfigError> {
    let runtime_mode = SdfAdfRuntimeMode::from_overlay_mode(overlay_mode);
    let mut config = QuestMakepadMatterSurfaceConfig::default();
    config.collision_enabled = collision_enabled;
    config.sdf_slice_enabled = runtime_mode.matter_sdf_enabled();
    config.adf_debug_enabled = runtime_mode.matter_adf_enabled();
    config.particles_enabled = particles_enabled;
    config.particle_visual_row_limit = Some(particle_render_draw_limit);
    config.enabled = replay_enabled
        && (config.collision_enabled
            || config.sdf_slice_enabled
            || config.adf_debug_enabled
            || particles_enabled);
    config.leaf_triangle_count = parse_usize_setting_or_default(
        settings,
        SETTING_MATTER_SURFACE_LEAF_TRIANGLE_COUNT,
        config.leaf_triangle_count,
    )?;
    config.particle_count = parse_usize_setting_or_default(
        settings,
        SETTING_MATTER_PARTICLE_COUNT,
        config.particle_count,
    )?;
    config.particle_seed =
        parse_u32_setting_or_default(settings, SETTING_MATTER_PARTICLE_SEED, config.particle_seed)?;
    config.particle_distance_refresh_policy =
        parse_particle_distance_refresh_policy_setting_or_default(
            settings,
            SETTING_MATTER_PARTICLE_DISTANCE_REFRESH_POLICY,
            config.particle_distance_refresh_policy,
        )?;
    config.particle_force_source = parse_particle_force_source_setting_or_default(
        settings,
        SETTING_MATTER_PARTICLE_FORCE_SOURCE,
        config.particle_force_source,
    )?;
    config.particle_force_update_interval_frames = parse_nonzero_usize_setting_or_default(
        settings,
        SETTING_MATTER_PARTICLE_FORCE_UPDATE_INTERVAL_FRAMES,
        config.particle_force_update_interval_frames,
    )?;
    config.particle_force_compare_probe_count = parse_usize_setting_or_default(
        settings,
        SETTING_MATTER_PARTICLE_FORCE_COMPARE_PROBE_COUNT,
        config.particle_force_compare_probe_count,
    )?;
    config.particle_execution_backend = parse_particle_execution_backend_setting_or_default(
        settings,
        SETTING_MATTER_PARTICLE_EXECUTION_BACKEND,
        config.particle_execution_backend,
    )?;
    config.particle_execution_batch_size = parse_nonzero_usize_setting_or_default(
        settings,
        SETTING_MATTER_PARTICLE_EXECUTION_BATCH_SIZE,
        config.particle_execution_batch_size,
    )?;
    let max_threads_default = config.particle_execution_max_threads.unwrap_or(0);
    let max_threads = parse_usize_setting_or_default(
        settings,
        SETTING_MATTER_PARTICLE_EXECUTION_MAX_THREADS,
        max_threads_default,
    )?;
    config.particle_execution_max_threads = if max_threads == 0 {
        None
    } else {
        Some(max_threads)
    };
    let max_frame_delta_seconds_default = config.particle_max_frame_delta_seconds.unwrap_or(0.0);
    let max_frame_delta_seconds = parse_non_negative_f32_setting_or_default(
        settings,
        SETTING_MATTER_PARTICLE_MAX_FRAME_DELTA_SECONDS,
        max_frame_delta_seconds_default,
    )?;
    config.particle_max_frame_delta_seconds = if max_frame_delta_seconds == 0.0 {
        None
    } else {
        Some(max_frame_delta_seconds)
    };
    config.sdf_voxel_size = parse_f32_setting_or_default(
        settings,
        SETTING_MATTER_SDF_SLICE_VOXEL_SIZE,
        config.sdf_voxel_size,
    )?;
    config.sdf_max_voxels = parse_usize_setting_or_default(
        settings,
        SETTING_MATTER_SDF_SLICE_MAX_CELLS,
        config.sdf_max_voxels,
    )?;
    config.adf_debug_config.max_depth = parse_u32_setting_or_default(
        settings,
        SETTING_MATTER_ADF_DEBUG_MAX_DEPTH,
        config.adf_debug_config.max_depth,
    )?;
    config.adf_debug_config.max_cells = parse_usize_setting_or_default(
        settings,
        SETTING_MATTER_ADF_DEBUG_MAX_CELLS,
        config.adf_debug_config.max_cells,
    )?;
    config.adf_debug_config.error_tolerance = parse_positive_f32_setting_or_default(
        settings,
        SETTING_MATTER_ADF_DEBUG_ERROR_TOLERANCE,
        config.adf_debug_config.error_tolerance,
    )?;
    config.sdf_adf_debug_update_interval_frames = parse_nonzero_usize_setting_or_default(
        settings,
        SETTING_MATTER_SDF_ADF_DEBUG_UPDATE_INTERVAL_FRAMES,
        config.sdf_adf_debug_update_interval_frames,
    )?;
    Ok(config)
}

fn parse_particle_execution_backend_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: ParticleExecutionBackend,
) -> Result<ParticleExecutionBackend, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_str()
            .and_then(parse_particle_execution_backend)
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_particle_execution_backend(value: &str) -> Option<ParticleExecutionBackend> {
    match value.trim().to_ascii_lowercase().as_str() {
        "serial" => Some(ParticleExecutionBackend::Serial),
        #[cfg(feature = "parallel")]
        "rayon" => Some(ParticleExecutionBackend::Parallel),
        _ => None,
    }
}

fn parse_particle_distance_refresh_policy_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: MatterSurfaceParticleDistanceRefreshPolicy,
) -> Result<MatterSurfaceParticleDistanceRefreshPolicy, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_str()
            .and_then(parse_particle_distance_refresh_policy)
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_particle_distance_refresh_policy(
    value: &str,
) -> Option<MatterSurfaceParticleDistanceRefreshPolicy> {
    match value.trim().to_ascii_lowercase().as_str() {
        "surface-update-and-step" => {
            Some(MatterSurfaceParticleDistanceRefreshPolicy::SurfaceUpdateAndStep)
        }
        "step-only" => Some(MatterSurfaceParticleDistanceRefreshPolicy::StepOnly),
        "disabled" => Some(MatterSurfaceParticleDistanceRefreshPolicy::Disabled),
        _ => None,
    }
}

fn parse_particle_force_source_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: MatterSurfaceParticleForceSource,
) -> Result<MatterSurfaceParticleForceSource, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_str()
            .and_then(parse_particle_force_source)
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_particle_force_source(value: &str) -> Option<MatterSurfaceParticleForceSource> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mesh-distance" => Some(MatterSurfaceParticleForceSource::MeshDistance),
        "none" => Some(MatterSurfaceParticleForceSource::None),
        "sdf-field" => Some(MatterSurfaceParticleForceSource::SdfField),
        "adf-field" => Some(MatterSurfaceParticleForceSource::AdfField),
        _ => None,
    }
}

fn parse_particle_force_authority_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: QuestMakepadForceAuthorityMode,
) -> Result<QuestMakepadForceAuthorityMode, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_str()
            .and_then(QuestMakepadForceAuthorityMode::parse)
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_gpu_force_provider_ab_receipt_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: QuestMakepadGpuForceProviderAbReceipt,
) -> Result<QuestMakepadGpuForceProviderAbReceipt, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_str()
            .and_then(QuestMakepadGpuForceProviderAbReceipt::parse)
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_particle_render_animation_mode_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: ParticleRenderAnimationMode,
) -> Result<ParticleRenderAnimationMode, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_str()
            .and_then(ParticleRenderAnimationMode::parse)
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_stimulus_presentation_mode_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: StimulusPresentationMode,
) -> Result<StimulusPresentationMode, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_str()
            .and_then(StimulusPresentationMode::parse)
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_f32_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: f32,
) -> Result<f32, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_f64()
            .filter(|number| number.is_finite())
            .map(|number| number as f32)
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_positive_f32_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: f32,
) -> Result<f32, CameraShellConfigError> {
    let value = parse_f32_setting_or_default(settings, setting_id, default)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(CameraShellConfigError::InvalidSettingValue(setting_id))
    }
}

fn parse_bool_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: bool,
) -> Result<bool, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_bool()
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_string_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: &str,
) -> Result<String, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default.to_string()),
    }
}

fn parse_non_negative_f32_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: f32,
) -> Result<f32, CameraShellConfigError> {
    let value = parse_f32_setting_or_default(settings, setting_id, default)?;
    if value >= 0.0 {
        Ok(value)
    } else {
        Err(CameraShellConfigError::InvalidSettingValue(setting_id))
    }
}

fn parse_nonzero_usize_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: NonZeroUsize,
) -> Result<NonZeroUsize, CameraShellConfigError> {
    let value = parse_usize_setting_or_default(settings, setting_id, default.get())?;
    NonZeroUsize::new(value).ok_or(CameraShellConfigError::InvalidSettingValue(setting_id))
}

fn parse_usize_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: usize,
) -> Result<usize, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_u64()
            .and_then(|number| usize::try_from(number).ok())
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn parse_u32_setting_or_default(
    settings: &[Value],
    setting_id: &'static str,
    default: u32,
) -> Result<u32, CameraShellConfigError> {
    match optional_setting_value(settings, setting_id) {
        Some(value) => value
            .as_u64()
            .and_then(|number| u32::try_from(number).ok())
            .ok_or(CameraShellConfigError::InvalidSettingValue(setting_id)),
        None => Ok(default),
    }
}

fn optional_setting_value<'a>(settings: &'a [Value], setting_id: &str) -> Option<&'a Value> {
    settings
        .iter()
        .find(|setting| {
            setting
                .get("setting_id")
                .and_then(Value::as_str)
                .is_some_and(|candidate| candidate == setting_id)
        })
        .and_then(|setting| setting.get("value"))
}

fn is_none_token(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("none")
}

fn is_safe_relative_payload_path(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('/')
        && !trimmed.starts_with('\\')
        && !trimmed.contains("..")
        && !trimmed.contains(':')
        && !trimmed.contains('\\')
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EFFECTIVE_SETTINGS_FIXTURE: &str =
        include_str!("../../../fixtures/effective-settings/mesh-replay.effective-settings.json");
    const LATTICE_VIEW_SET_FIXTURE: &str =
        include_str!("../../../fixtures/lattice/synthetic-display-view-set.json");

    #[test]
    fn effective_settings_configures_replay_runtime() {
        let config =
            CameraShellReplayConfig::from_effective_settings_json(EFFECTIVE_SETTINGS_FIXTURE)
                .unwrap();
        assert!(config.enabled);
        assert_eq!(config.source, "public-synthetic-hand-sequence");
        assert_eq!(config.speed, 1.5);
        assert_eq!(config.opacity, 0.75);

        let mut runtime =
            mesh_replay_runtime_from_effective_settings_json(EFFECTIVE_SETTINGS_FIXTURE).unwrap();
        let first = runtime.step(0.0);
        assert!(first.enabled);
        assert_eq!(first.frame_index, 0);
        let marker = runtime.config_marker_line("settings-applied");
        assert!(marker.contains("schema=rusty.quest.makepad.mesh_replay.v1"));
        assert!(marker.contains("source=public-synthetic-hand-sequence"));
        assert!(marker.contains("speed=1.500"));
    }

    #[test]
    fn effective_settings_configures_full_camera_shell_surface() {
        let config =
            CameraShellEffectiveConfig::from_effective_settings_json(EFFECTIVE_SETTINGS_FIXTURE)
                .unwrap();

        assert!(config.replay.enabled);
        assert_eq!(config.replay.speed, 1.5);
        assert_eq!(config.render_scale, 0.9);
        assert!(!config.camera_streaming_enabled);
        assert!(!config.stimulus.enabled);
        assert_eq!(config.stimulus.profile_path, "none");
        assert_eq!(
            config.stimulus.profile_schema,
            DEFAULT_STIMULUS_PROFILE_SCHEMA_ID
        );
        assert_eq!(
            config.stimulus.presentation_mode,
            StimulusPresentationMode::StereoEyeField
        );
        assert!(!config.remote_camera.enabled);
        assert_eq!(config.remote_camera.session_id, "none");
        assert!(config.collision_enabled);
        assert_eq!(config.sdf_adf_overlay_mode, SdfAdfOverlayMode::Sdf);
        assert_eq!(config.sdf_adf_overlay_mode.as_str(), "sdf");
        assert!(config.particles_enabled);
        assert_eq!(
            config.particle_force_authority,
            QuestMakepadForceAuthorityMode::MatterCpu
        );
        assert_eq!(
            config.gpu_force_provider_ab_receipt,
            QuestMakepadGpuForceProviderAbReceipt::None
        );
        assert_eq!(config.particle_render_draw_limit, 192);
        assert_eq!(
            config.particle_render_animation_mode,
            ParticleRenderAnimationMode::ProceduralMorphRing
        );
        assert_eq!(config.particle_render_size_scale, 1.0);
        assert!(config.matter_surface.enabled);
        assert!(config.matter_surface.sdf_slice_enabled);
        assert!(!config.matter_surface.adf_debug_enabled);
        assert!(config.matter_surface.particles_enabled);
        assert_eq!(config.matter_surface.leaf_triangle_count, 8);
        assert_eq!(config.matter_surface.particle_count, 1_000);
        assert_eq!(
            config.matter_surface.particle_distance_refresh_policy,
            MatterSurfaceParticleDistanceRefreshPolicy::StepOnly
        );
        assert_eq!(
            config.matter_surface.particle_force_source,
            MatterSurfaceParticleForceSource::MeshDistance
        );
        assert_eq!(
            config
                .matter_surface
                .particle_force_update_interval_frames
                .get(),
            1
        );
        assert_eq!(config.matter_surface.particle_force_compare_probe_count, 0);
        assert_eq!(
            config.matter_surface.particle_execution_backend,
            ParticleExecutionBackend::Serial
        );
        assert_eq!(
            config.matter_surface.particle_execution_batch_size.get(),
            DEFAULT_PARTICLE_EXECUTION_BATCH_SIZE
        );
        assert_eq!(config.matter_surface.particle_execution_max_threads, None);
        assert_eq!(config.matter_surface.particle_max_frame_delta_seconds, None);
        assert_eq!(config.matter_surface.particle_visual_row_limit, Some(192));
        assert_eq!(config.matter_surface.adf_debug_config.max_depth, 4);
        assert_eq!(config.matter_surface.adf_debug_config.max_cells, 4096);
        assert!((config.matter_surface.adf_debug_config.error_tolerance - 0.025).abs() < 0.000_001);
    }

    #[test]
    fn effective_settings_builds_full_runtime_bundle() {
        let mut bundle =
            camera_shell_runtime_bundle_from_effective_settings_json(EFFECTIVE_SETTINGS_FIXTURE)
                .unwrap();

        assert!(bundle.effective_config.replay.enabled);
        assert_eq!(bundle.effective_config.render_scale, 0.9);
        assert!(!bundle.effective_config.camera_streaming_enabled);
        assert!(!bundle.effective_config.stimulus.enabled);
        assert!(!bundle.effective_config.remote_camera.enabled);
        assert!(bundle.effective_config.collision_enabled);
        assert_eq!(
            bundle.effective_config.sdf_adf_overlay_mode,
            SdfAdfOverlayMode::Sdf
        );
        assert!(bundle.effective_config.particles_enabled);
        assert_eq!(
            bundle.effective_config.particle_force_authority,
            QuestMakepadForceAuthorityMode::MatterCpu
        );
        assert_eq!(
            bundle.effective_config.gpu_force_provider_ab_receipt,
            QuestMakepadGpuForceProviderAbReceipt::None
        );
        assert_eq!(bundle.effective_config.particle_render_draw_limit, 192);
        assert_eq!(
            bundle.effective_config.particle_render_animation_mode,
            ParticleRenderAnimationMode::ProceduralMorphRing
        );
        assert_eq!(bundle.effective_config.particle_render_size_scale, 1.0);
        assert!(bundle.effective_config.matter_surface.enabled);
        assert!(!bundle.effective_config.matter_surface.adf_debug_enabled);
        assert_eq!(bundle.matter_surface_runtime.config().particle_count, 1_000);
        assert_eq!(
            bundle
                .matter_surface_runtime
                .config()
                .particle_execution_batch_size
                .get(),
            DEFAULT_PARTICLE_EXECUTION_BATCH_SIZE
        );
        assert_eq!(
            bundle
                .matter_surface_runtime
                .config()
                .particle_visual_row_limit,
            Some(192)
        );

        let step = bundle.mesh_replay_runtime.step(0.0);
        assert!(step.enabled);
        assert_eq!(step.frame_index, 0);
    }

    #[test]
    fn rejects_wrong_schema() {
        let wrong_schema = EFFECTIVE_SETTINGS_FIXTURE.replace(
            EFFECTIVE_SETTINGS_SCHEMA_ID,
            "rusty.gui.makepad.not_effective.v1",
        );
        assert_eq!(
            CameraShellReplayConfig::from_effective_settings_json(&wrong_schema).unwrap_err(),
            CameraShellConfigError::UnexpectedSchema(Some(
                "rusty.gui.makepad.not_effective.v1".to_string()
            ))
        );
    }

    #[test]
    fn rejects_wrong_app_id() {
        let wrong_app = EFFECTIVE_SETTINGS_FIXTURE
            .replace(CAMERA_SHELL_APP_ID, "rusty-quest-makepad.other-shell");
        assert_eq!(
            CameraShellReplayConfig::from_effective_settings_json(&wrong_app).unwrap_err(),
            CameraShellConfigError::UnexpectedApp(Some(
                "rusty-quest-makepad.other-shell".to_string()
            ))
        );
    }

    #[test]
    fn rejects_missing_replay_setting() {
        let missing_setting =
            EFFECTIVE_SETTINGS_FIXTURE.replace(SETTING_MESH_REPLAY_OPACITY, "makepad.unused");
        assert_eq!(
            CameraShellReplayConfig::from_effective_settings_json(&missing_setting).unwrap_err(),
            CameraShellConfigError::MissingSetting(SETTING_MESH_REPLAY_OPACITY)
        );
    }

    #[test]
    fn rejects_missing_non_replay_setting() {
        let missing_setting =
            EFFECTIVE_SETTINGS_FIXTURE.replace(SETTING_COLLISION_ENABLED, "makepad.unused");
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&missing_setting).unwrap_err(),
            CameraShellConfigError::MissingSetting(SETTING_COLLISION_ENABLED)
        );
    }

    #[test]
    fn parses_remote_camera_session_handoff_settings() {
        let remote = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_REMOTE_CAMERA_ENABLED,
            serde_json::json!(true),
        );
        let remote = effective_settings_with_value(
            &remote,
            SETTING_REMOTE_CAMERA_SESSION_ID,
            serde_json::json!("session.remote_camera.q2q_two_way_lan_smoke"),
        );
        let remote = effective_settings_with_value(
            &remote,
            SETTING_REMOTE_CAMERA_TOPOLOGY_ID,
            serde_json::json!("quest_to_quest_two_way"),
        );
        let remote = effective_settings_with_value(
            &remote,
            SETTING_REMOTE_CAMERA_ENDPOINT_DEVICE_ID,
            serde_json::json!("quest-a"),
        );
        let remote = effective_settings_with_value(
            &remote,
            SETTING_REMOTE_CAMERA_ENDPOINT_DEVICE_KIND,
            serde_json::json!("quest"),
        );
        let remote = effective_settings_with_value(
            &remote,
            SETTING_REMOTE_CAMERA_ENDPOINT_ROLE,
            serde_json::json!("sender_receiver"),
        );
        let remote = effective_settings_with_value(
            &remote,
            SETTING_REMOTE_CAMERA_LANE_COUNT,
            serde_json::json!(4),
        );
        let remote = effective_settings_with_value(
            &remote,
            SETTING_REMOTE_CAMERA_INCOMING_LANE_COUNT,
            serde_json::json!(2),
        );
        let remote = effective_settings_with_value(
            &remote,
            SETTING_REMOTE_CAMERA_OUTGOING_LANE_COUNT,
            serde_json::json!(2),
        );
        let remote = effective_settings_with_value(
            &remote,
            SETTING_REMOTE_CAMERA_TRANSPORT_KIND,
            serde_json::json!("lan_tcp"),
        );
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&remote).unwrap();

        assert!(config.remote_camera.enabled);
        assert_eq!(
            config.remote_camera.session_id,
            "session.remote_camera.q2q_two_way_lan_smoke"
        );
        assert_eq!(config.remote_camera.endpoint_role, "sender_receiver");
        assert_eq!(config.remote_camera.lane_count, 4);
        assert_eq!(config.remote_camera.incoming_lane_count, 2);
        assert_eq!(config.remote_camera.outgoing_lane_count, 2);
        assert_eq!(config.remote_camera.transport_kind, "lan_tcp");
    }

    #[test]
    fn parses_stimulus_profile_handoff_settings() {
        let profile_sha = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let tuning_sha = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let stimulus = effective_settings_with_appended_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_STIMULUS_ENABLED,
            serde_json::json!(true),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_PATH,
            serde_json::json!("stimulus/stimulus-profile.json"),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_SHA256,
            serde_json::json!(profile_sha),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_SCHEMA,
            serde_json::json!(DEFAULT_STIMULUS_PROFILE_SCHEMA_ID),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_TUNING_PATH,
            serde_json::json!("stimulus/stimulus-tuning.json"),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_TUNING_SHA256,
            serde_json::json!(tuning_sha),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PRESENTATION_MODE,
            serde_json::json!("StereoEyeField"),
        );
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&stimulus).unwrap();

        assert!(config.stimulus.enabled);
        assert_eq!(
            config.stimulus.profile_path,
            "stimulus/stimulus-profile.json"
        );
        assert_eq!(config.stimulus.profile_sha256, profile_sha);
        assert_eq!(
            config.stimulus.profile_schema,
            DEFAULT_STIMULUS_PROFILE_SCHEMA_ID
        );
        assert_eq!(config.stimulus.tuning_path, "stimulus/stimulus-tuning.json");
        assert_eq!(config.stimulus.tuning_sha256, tuning_sha);
        assert_eq!(
            config.stimulus.presentation_mode,
            StimulusPresentationMode::StereoEyeField
        );
    }

    #[test]
    fn rejects_enabled_stimulus_without_valid_profile_digest() {
        let stimulus = effective_settings_with_appended_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_STIMULUS_ENABLED,
            serde_json::json!(true),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_PATH,
            serde_json::json!("stimulus/stimulus-profile.json"),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_SHA256,
            serde_json::json!("not-a-sha"),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&stimulus).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_STIMULUS_PROFILE_SHA256)
        );
    }

    #[test]
    fn rejects_unsafe_stimulus_profile_path() {
        let stimulus = effective_settings_with_appended_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_STIMULUS_ENABLED,
            serde_json::json!(true),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_PATH,
            serde_json::json!("../stimulus-profile.json"),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_SHA256,
            serde_json::json!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&stimulus).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_STIMULUS_PROFILE_PATH)
        );
    }

    #[test]
    fn loads_staged_stimulus_profile_payload() {
        let profile_json = r#"{"profile_id":"stimulus.profile.test","schema_id":"rusty.optics.stimulus.profile.v1","presentation":{"mode":"StereoEyeField","coverage":"FullViewport","eye_count":2}}"#;
        let profile_sha = sha256_hex(profile_json.as_bytes());
        let root = unique_temp_dir("stimulus-profile-payload");
        let stimulus_dir = root.join("stimulus");
        std::fs::create_dir_all(&stimulus_dir).expect("create stimulus dir");
        std::fs::write(stimulus_dir.join("profile.json"), profile_json).expect("write profile");
        let stimulus = effective_settings_with_appended_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_STIMULUS_ENABLED,
            serde_json::json!(true),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_PATH,
            serde_json::json!("stimulus/profile.json"),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_SHA256,
            serde_json::json!(profile_sha),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_SCHEMA,
            serde_json::json!(DEFAULT_STIMULUS_PROFILE_SCHEMA_ID),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PRESENTATION_MODE,
            serde_json::json!("StereoEyeField"),
        );
        let payload =
            stimulus_profile_payload_from_effective_settings_json_with_root(&stimulus, &root)
                .unwrap()
                .expect("stimulus payload");

        assert_eq!(payload.profile_id, "stimulus.profile.test");
        assert_eq!(payload.profile_schema, DEFAULT_STIMULUS_PROFILE_SCHEMA_ID);
        assert_eq!(payload.profile_json, profile_json);
        assert_eq!(payload.config.profile_path, "stimulus/profile.json");

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn loads_staged_stimulus_volume_profile_summary() {
        let profile_json = r#"{"profile_id":"stimulus.profile.volume.test","schema_id":"rusty.optics.stimulus.profile.v1","presentation":{"mode":"StereoEyeField","coverage":"FullViewport","eye_count":2},"volume":{"schema_id":"rusty.optics.stimulus.volume.v1","volume_id":"stimulus.volume.test","field_kind":"ProceduralLayerStack3d","storage_hint":"StorageBuffer","grid_dimensions":[32,32,32],"step_count":32},"kernel_abi":{"abi_id":"stimulus.kernel.volume_compute_v1","compute_passes":[{"kind":"VolumeDensityCache","output_layers":1},{"kind":"VolumeReadbackProbe","bounded_readback_samples":512},{"kind":"VolumeRaymarchStereoField","output_layers":2}]}}"#;
        let profile_sha = sha256_hex(profile_json.as_bytes());
        let root = unique_temp_dir("stimulus-volume-profile-payload");
        let stimulus_dir = root.join("stimulus");
        std::fs::create_dir_all(&stimulus_dir).expect("create stimulus dir");
        std::fs::write(stimulus_dir.join("volume-profile.json"), profile_json)
            .expect("write profile");
        let stimulus = effective_settings_with_appended_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_STIMULUS_ENABLED,
            serde_json::json!(true),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_PATH,
            serde_json::json!("stimulus/volume-profile.json"),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_SHA256,
            serde_json::json!(profile_sha),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PROFILE_SCHEMA,
            serde_json::json!(DEFAULT_STIMULUS_PROFILE_SCHEMA_ID),
        );
        let stimulus = effective_settings_with_appended_value(
            &stimulus,
            SETTING_STIMULUS_PRESENTATION_MODE,
            serde_json::json!("StereoEyeField"),
        );
        let payload =
            stimulus_profile_payload_from_effective_settings_json_with_root(&stimulus, &root)
                .unwrap()
                .expect("stimulus payload");

        assert_eq!(payload.profile_id, "stimulus.profile.volume.test");
        assert!(payload.volume_summary.volume_present);
        assert_eq!(
            payload.volume_summary.volume_schema.as_deref(),
            Some(STIMULUS_VOLUME_SCHEMA_ID)
        );
        assert_eq!(
            payload.volume_summary.field_kind.as_deref(),
            Some("ProceduralLayerStack3d")
        );
        assert_eq!(
            payload.volume_summary.storage_hint.as_deref(),
            Some("StorageBuffer")
        );
        assert_eq!(payload.volume_summary.grid_dimensions, Some([32, 32, 32]));
        assert_eq!(payload.volume_summary.step_count, Some(32));
        assert_eq!(
            payload.volume_summary.kernel_abi_id.as_deref(),
            Some("stimulus.kernel.volume_compute_v1")
        );
        assert_eq!(payload.volume_summary.compute_pass_count, 3);
        assert_eq!(
            payload.volume_summary.volume_readback_probe_samples,
            Some(512)
        );
        assert_eq!(payload.volume_summary.stereo_field_output_layers, Some(2));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn parses_non_default_sdf_adf_overlay_mode() {
        let combined =
            EFFECTIVE_SETTINGS_FIXTURE.replace("\"value\": \"sdf\"", "\"value\": \"combined\"");
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&combined).unwrap();
        assert_eq!(config.sdf_adf_overlay_mode, SdfAdfOverlayMode::Combined);
        let runtime_mode = SdfAdfRuntimeMode::from_overlay_mode(config.sdf_adf_overlay_mode);
        assert_eq!(runtime_mode, SdfAdfRuntimeMode::UnsupportedCombined);
        assert!(runtime_mode.is_unsupported_adf_placeholder());
        assert!(!config.matter_surface.sdf_slice_enabled);
        assert!(!config.matter_surface.adf_debug_enabled);
    }

    #[test]
    fn sdf_mode_enables_matter_surface_sdf_without_adf() {
        let config =
            CameraShellEffectiveConfig::from_effective_settings_json(EFFECTIVE_SETTINGS_FIXTURE)
                .unwrap();

        assert_eq!(config.sdf_adf_overlay_mode, SdfAdfOverlayMode::Sdf);
        assert_eq!(
            SdfAdfRuntimeMode::from_overlay_mode(config.sdf_adf_overlay_mode),
            SdfAdfRuntimeMode::Sdf
        );
        assert!(config.matter_surface.enabled);
        assert!(config.matter_surface.sdf_slice_enabled);
        assert!(!config.matter_surface.adf_debug_enabled);
        assert!(config.matter_surface.particles_enabled);
    }

    #[test]
    fn adf_mode_enables_matter_surface_adf_debug_without_sdf() {
        let adf = EFFECTIVE_SETTINGS_FIXTURE.replace("\"value\": \"sdf\"", "\"value\": \"adf\"");
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&adf).unwrap();

        assert_eq!(config.sdf_adf_overlay_mode, SdfAdfOverlayMode::Adf);
        assert_eq!(
            SdfAdfRuntimeMode::from_overlay_mode(config.sdf_adf_overlay_mode),
            SdfAdfRuntimeMode::Adf
        );
        assert!(config.matter_surface.enabled);
        assert!(!config.matter_surface.sdf_slice_enabled);
        assert!(config.matter_surface.adf_debug_enabled);
        assert!(config.matter_surface.particles_enabled);
    }

    #[test]
    fn parses_adf_debug_settings() {
        let custom = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_ADF_DEBUG_MAX_DEPTH,
            serde_json::json!(2),
        );
        let custom = effective_settings_with_value(
            &custom,
            SETTING_MATTER_ADF_DEBUG_MAX_CELLS,
            serde_json::json!(128),
        );
        let custom = effective_settings_with_value(
            &custom,
            SETTING_MATTER_ADF_DEBUG_ERROR_TOLERANCE,
            serde_json::json!(0.05),
        );

        let config = CameraShellEffectiveConfig::from_effective_settings_json(&custom).unwrap();

        assert_eq!(config.matter_surface.adf_debug_config.max_depth, 2);
        assert_eq!(config.matter_surface.adf_debug_config.max_cells, 128);
        assert!((config.matter_surface.adf_debug_config.error_tolerance - 0.05).abs() < 0.000_001);
    }

    #[test]
    fn parses_sdf_adf_debug_update_interval_setting() {
        let custom = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_SDF_ADF_DEBUG_UPDATE_INTERVAL_FRAMES,
            serde_json::json!(3),
        );

        let config = CameraShellEffectiveConfig::from_effective_settings_json(&custom).unwrap();

        assert_eq!(
            config
                .matter_surface
                .sdf_adf_debug_update_interval_frames
                .get(),
            3
        );
    }

    #[test]
    fn rejects_invalid_sdf_adf_overlay_mode() {
        let invalid =
            EFFECTIVE_SETTINGS_FIXTURE.replace("\"value\": \"sdf\"", "\"value\": \"private\"");
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_SDF_ADF_OVERLAY_MODE)
        );
    }

    #[test]
    fn parses_particle_execution_settings() {
        let custom = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_EXECUTION_BATCH_SIZE,
            serde_json::json!(128),
        );
        let custom = effective_settings_with_value(
            &custom,
            SETTING_MATTER_PARTICLE_EXECUTION_MAX_THREADS,
            serde_json::json!(2),
        );
        let custom = effective_settings_with_value(
            &custom,
            SETTING_MATTER_PARTICLE_MAX_FRAME_DELTA_SECONDS,
            serde_json::json!(0.033333335),
        );
        let custom = effective_settings_with_value(
            &custom,
            SETTING_MATTER_PARTICLE_DISTANCE_REFRESH_POLICY,
            serde_json::json!("disabled"),
        );
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&custom).unwrap();

        assert_eq!(
            config.matter_surface.particle_distance_refresh_policy,
            MatterSurfaceParticleDistanceRefreshPolicy::Disabled
        );
        assert_eq!(
            config.matter_surface.particle_execution_backend,
            ParticleExecutionBackend::Serial
        );
        assert_eq!(
            config.matter_surface.particle_execution_batch_size.get(),
            128
        );
        assert_eq!(
            config.matter_surface.particle_execution_max_threads,
            Some(2)
        );
        assert_eq!(
            config.matter_surface.particle_max_frame_delta_seconds,
            Some(0.033333335)
        );
    }

    #[test]
    fn parses_particle_force_settings() {
        let custom = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_FORCE_SOURCE,
            serde_json::json!("none"),
        );
        let custom = effective_settings_with_value(
            &custom,
            SETTING_MATTER_PARTICLE_FORCE_UPDATE_INTERVAL_FRAMES,
            serde_json::json!(3),
        );
        let custom = effective_settings_with_value(
            &custom,
            SETTING_MATTER_PARTICLE_FORCE_COMPARE_PROBE_COUNT,
            serde_json::json!(5),
        );
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&custom).unwrap();

        assert_eq!(
            config.matter_surface.particle_force_source,
            MatterSurfaceParticleForceSource::None
        );
        assert_eq!(
            config
                .matter_surface
                .particle_force_update_interval_frames
                .get(),
            3
        );
        assert_eq!(config.matter_surface.particle_force_compare_probe_count, 5);

        let adf = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_FORCE_SOURCE,
            serde_json::json!("adf-field"),
        );
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&adf).unwrap();
        assert_eq!(
            config.matter_surface.particle_force_source,
            MatterSurfaceParticleForceSource::AdfField
        );
    }

    #[test]
    fn parses_gpu_force_authority_without_changing_matter_force_source() {
        let custom = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_PARTICLE_FORCE_AUTHORITY,
            serde_json::json!("gpu-dense-sdf-field-particle-force"),
        );
        let custom = effective_settings_with_value(
            &custom,
            SETTING_MATTER_PARTICLE_FORCE_SOURCE,
            serde_json::json!("sdf-field"),
        );

        let config = CameraShellEffectiveConfig::from_effective_settings_json(&custom).unwrap();

        assert_eq!(
            config.particle_force_authority,
            QuestMakepadForceAuthorityMode::GpuDenseSdfFieldParticleForce
        );
        assert_eq!(
            config.matter_surface.particle_force_source,
            MatterSurfaceParticleForceSource::SdfField
        );
    }

    #[test]
    fn parses_gpu_force_provider_ab_receipt_without_payload_data() {
        let custom = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_GPU_FORCE_LIVE_RECORDED_PROVIDER_AB_RECEIPT,
            serde_json::json!("live-recorded-provider-ab-check-v1"),
        );

        let config = CameraShellEffectiveConfig::from_effective_settings_json(&custom).unwrap();

        assert_eq!(
            config.gpu_force_provider_ab_receipt,
            QuestMakepadGpuForceProviderAbReceipt::LiveRecordedProviderAbCheckV1
        );
        assert!(config
            .gpu_force_provider_ab_receipt
            .live_recorded_provider_ab_ready());
        assert_eq!(
            config.gpu_force_provider_ab_receipt.as_str(),
            "live-recorded-provider-ab-check-v1"
        );
    }

    #[test]
    fn rejects_invalid_gpu_force_provider_ab_receipt() {
        let invalid = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_GPU_FORCE_LIVE_RECORDED_PROVIDER_AB_RECEIPT,
            serde_json::json!("topology-inferred"),
        );

        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(
                SETTING_GPU_FORCE_LIVE_RECORDED_PROVIDER_AB_RECEIPT
            )
        );
    }

    #[test]
    fn rejects_invalid_gpu_force_authority() {
        let invalid = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_PARTICLE_FORCE_AUTHORITY,
            serde_json::json!("both"),
        );

        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_PARTICLE_FORCE_AUTHORITY)
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parses_parallel_particle_execution_backend_when_feature_enabled() {
        let custom = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_EXECUTION_BACKEND,
            serde_json::json!("rayon"),
        );
        let custom = effective_settings_with_value(
            &custom,
            SETTING_MATTER_PARTICLE_EXECUTION_MAX_THREADS,
            serde_json::json!(2),
        );
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&custom).unwrap();

        assert_eq!(
            config.matter_surface.particle_execution_backend,
            ParticleExecutionBackend::Parallel
        );
        assert_eq!(
            config.matter_surface.particle_execution_max_threads,
            Some(2)
        );
    }

    #[test]
    fn rejects_invalid_particle_execution_settings() {
        let invalid_batch = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_EXECUTION_BATCH_SIZE,
            serde_json::json!(0),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid_batch).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(
                SETTING_MATTER_PARTICLE_EXECUTION_BATCH_SIZE
            )
        );

        let invalid_backend = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_EXECUTION_BACKEND,
            serde_json::json!("private"),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid_backend).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_MATTER_PARTICLE_EXECUTION_BACKEND)
        );

        let invalid_delta = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_MAX_FRAME_DELTA_SECONDS,
            serde_json::json!(-0.1),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid_delta).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(
                SETTING_MATTER_PARTICLE_MAX_FRAME_DELTA_SECONDS
            )
        );

        let invalid_policy = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_DISTANCE_REFRESH_POLICY,
            serde_json::json!("private"),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid_policy).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(
                SETTING_MATTER_PARTICLE_DISTANCE_REFRESH_POLICY
            )
        );

        let invalid_force_source = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_FORCE_SOURCE,
            serde_json::json!("private"),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid_force_source)
                .unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_MATTER_PARTICLE_FORCE_SOURCE)
        );

        let invalid_force_interval = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_PARTICLE_FORCE_UPDATE_INTERVAL_FRAMES,
            serde_json::json!(0),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid_force_interval)
                .unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(
                SETTING_MATTER_PARTICLE_FORCE_UPDATE_INTERVAL_FRAMES
            )
        );

        let invalid_adf_tolerance = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_ADF_DEBUG_ERROR_TOLERANCE,
            serde_json::json!(0.0),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid_adf_tolerance)
                .unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_MATTER_ADF_DEBUG_ERROR_TOLERANCE)
        );

        let invalid_sdf_adf_debug_interval = effective_settings_with_value(
            EFFECTIVE_SETTINGS_FIXTURE,
            SETTING_MATTER_SDF_ADF_DEBUG_UPDATE_INTERVAL_FRAMES,
            serde_json::json!(0),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(
                &invalid_sdf_adf_debug_interval
            )
            .unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(
                SETTING_MATTER_SDF_ADF_DEBUG_UPDATE_INTERVAL_FRAMES
            )
        );
    }

    #[test]
    fn normalizes_values_at_runtime_boundary() {
        let high_values = EFFECTIVE_SETTINGS_FIXTURE
            .replace("\"value\": 1.5", "\"value\": 12.0")
            .replace("\"value\": 0.75", "\"value\": 2.0");
        let config = mesh_replay_config_from_effective_settings_json(&high_values).unwrap();
        assert_eq!(config.speed, 8.0);
        assert_eq!(config.opacity, 1.0);
    }

    #[test]
    fn lattice_view_set_builds_optics_projection_reports() {
        let reports =
            projection_reports_from_lattice_view_set_json(LATTICE_VIEW_SET_FIXTURE).unwrap();
        assert_eq!(
            reports.view_set_id,
            "view_set.quest_makepad.synthetic_stereo"
        );
        assert_eq!(
            reports.left.schema,
            "rusty.optics.video_projection_geometry.v1"
        );
        assert_eq!(reports.left.view_id, "left");
        assert_eq!(reports.right.view_id, "right");
        assert_eq!(
            reports
                .left
                .source_valid_screen_uv_footprint
                .active_fraction,
            1.0
        );
    }

    #[test]
    fn damaged_lattice_view_set_is_rejected() {
        let damaged = LATTICE_VIEW_SET_FIXTURE.replace("\"eye\": \"left\"", "\"eye\": \"mono\"");
        let error = projection_reports_from_lattice_view_set_json(&damaged).unwrap_err();
        assert!(matches!(
            error,
            CameraShellConfigError::InvalidDisplayViewSet(_)
        ));
    }

    fn effective_settings_with_value(json: &str, setting_id: &str, value: Value) -> String {
        let mut report: Value = serde_json::from_str(json).expect("effective settings JSON");
        let settings = report
            .get_mut("settings")
            .and_then(Value::as_array_mut)
            .expect("settings array");
        let setting = settings
            .iter_mut()
            .find(|candidate| {
                candidate
                    .get("setting_id")
                    .and_then(Value::as_str)
                    .is_some_and(|candidate| candidate == setting_id)
            })
            .expect("effective setting");
        setting["value"] = value;
        serde_json::to_string(&report).expect("effective settings JSON")
    }

    fn effective_settings_with_appended_value(
        json: &str,
        setting_id: &str,
        value: Value,
    ) -> String {
        let mut report: Value = serde_json::from_str(json).expect("effective settings JSON");
        let settings = report
            .get_mut("settings")
            .and_then(Value::as_array_mut)
            .expect("settings array");
        if let Some(setting) = settings.iter_mut().find(|candidate| {
            candidate
                .get("setting_id")
                .and_then(Value::as_str)
                .is_some_and(|candidate| candidate == setting_id)
        }) {
            setting["value"] = value;
            return serde_json::to_string(&report).expect("effective settings JSON");
        }
        settings.push(serde_json::json!({
            "setting_id": setting_id,
            "value": value,
            "winning_layer": "runtime_profile",
            "winning_source_id": "profile.quest_makepad.browser_stimulus_export",
            "rejected_layers": [],
            "hotload_policy": "scene_rebuild",
            "writer_policy": "profile_owned",
            "readback_field": "stimulus"
        }));
        serde_json::to_string(&report).expect("effective settings JSON")
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{nanos}"))
    }
}
