//! Profile-driven Quest Makepad camera shell adapter.

mod mesh_replay_source;

use std::{num::NonZeroUsize, path::Path};

use rusty_lattice_model::{validate_display_view_set, DisplayViewSet};
use rusty_optics_model::{
    ProjectionGeometryReport, Rect2, VideoProjectionMapping, IDENTITY_HOMOGRAPHY,
};
pub use rusty_quest_makepad_matter_surface::{
    world_particle_batch_from_upload, MatterSurfaceContactProbe, ParticleExecutionBackend,
    QuestMakepadMatterSurfaceConfig, QuestMakepadMatterSurfaceFrame,
    QuestMakepadMatterSurfaceRuntime, QuestMakepadMatterSurfaceStageTimings,
    QuestMakepadMatterSurfaceWorker, QuestMakepadMatterSurfaceWorkerFrame,
    QuestMakepadMatterSurfaceWorkerOutput, QuestMakepadParticleRow, QuestMakepadParticleUpload,
    QuestMakepadWorldParticleBatch, QuestMakepadWorldParticleInstance,
    QuestMakepadWorldParticlePlacement, DEFAULT_PARTICLE_EXECUTION_BATCH_SIZE,
    DEFAULT_WORLD_CONTENT_CENTER, DEFAULT_WORLD_CONTENT_TARGET_RADIUS,
    QUEST_MAKEPAD_CENTER_PROJECTED_BILLBOARD_MODE, QUEST_MAKEPAD_CONTENT_LOCAL_SPACE,
    QUEST_MAKEPAD_MATTER_SURFACE_MARKER_PREFIX, QUEST_MAKEPAD_MATTER_SURFACE_SCHEMA_ID,
    QUEST_MAKEPAD_MATTER_SURFACE_WORKER_MARKER_PREFIX,
    QUEST_MAKEPAD_MATTER_SURFACE_WORKER_SCHEMA_ID, QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE,
    QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID,
    QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_MODE,
    QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_SOURCE,
    QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_REFERENCE,
    QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_RENDERER_ID,
    QUEST_MAKEPAD_WORLD_PARTICLE_EVEN_SELECTION_POLICY, QUEST_MAKEPAD_WORLD_PARTICLE_MARKER_PREFIX,
};
use rusty_quest_makepad_mesh_replay::MeshReplayConfig;
pub use rusty_quest_makepad_mesh_replay::{
    MeshReplayRuntime, MeshReplayUniforms, REPLAY_MARKER_PREFIX, REPLAY_SCHEMA_ID,
    SELECTED_SEGMENT_COUNT,
};
use serde_json::Value;

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
/// Native Matter SDF slice voxel-size setting id.
pub const SETTING_MATTER_SDF_SLICE_VOXEL_SIZE: &str = "makepad.sdf.slice.voxel_size";
/// Native Matter SDF slice max-cell setting id.
pub const SETTING_MATTER_SDF_SLICE_MAX_CELLS: &str = "makepad.sdf.slice.max_cells";
/// Default world-particle draw cap for current Quest Makepad billboard smoke.
pub const DEFAULT_PARTICLE_RENDER_DRAW_LIMIT: usize = 96;
/// Default particle renderer animation mode.
pub const DEFAULT_PARTICLE_RENDER_ANIMATION_MODE: ParticleRenderAnimationMode =
    ParticleRenderAnimationMode::ProceduralMorphRing;
/// Default particle renderer size scale.
pub const DEFAULT_PARTICLE_RENDER_SIZE_SCALE: f32 = 1.0;
/// Default projection footprint sample grid for app-shell contract smoke tests.
pub const DEFAULT_PROJECTION_FOOTPRINT_GRID: usize = 8;

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

/// Complete camera-shell subset of the canonical effective-settings report.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraShellEffectiveConfig {
    /// Replay settings.
    pub replay: CameraShellReplayConfig,
    /// Render scale for the Makepad/XR runtime.
    pub render_scale: f32,
    /// Whether the app shell should acquire direct or broker camera frames.
    pub camera_streaming_enabled: bool,
    /// Whether collision behavior is enabled.
    pub collision_enabled: bool,
    /// SDF/ADF overlay mode.
    pub sdf_adf_overlay_mode: SdfAdfOverlayMode,
    /// Whether particle behavior is enabled.
    pub particles_enabled: bool,
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
        let collision_enabled = parse_bool_setting(settings, SETTING_COLLISION_ENABLED)?;
        let sdf_adf_overlay_mode = parse_sdf_adf_overlay_mode(settings)?;
        let particles_enabled = parse_bool_setting(settings, SETTING_PARTICLES_ENABLED)?;
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
        )?;

        Ok(Self {
            replay,
            render_scale,
            camera_streaming_enabled,
            collision_enabled,
            sdf_adf_overlay_mode,
            particles_enabled,
            particle_render_draw_limit,
            particle_render_animation_mode,
            particle_render_size_scale,
            matter_surface,
        })
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
    /// ADF was requested but no Matter ADF contract exists yet.
    UnsupportedAdf,
    /// Combined SDF/ADF was requested but ADF has no Matter contract yet.
    UnsupportedCombined,
}

impl SdfAdfRuntimeMode {
    /// Builds the runtime-gated mode from the user-facing setting.
    #[must_use]
    pub const fn from_overlay_mode(mode: SdfAdfOverlayMode) -> Self {
        match mode {
            SdfAdfOverlayMode::Off => Self::Off,
            SdfAdfOverlayMode::Sdf => Self::Sdf,
            SdfAdfOverlayMode::Adf => Self::UnsupportedAdf,
            SdfAdfOverlayMode::Combined => Self::UnsupportedCombined,
        }
    }

    /// Returns whether Matter-backed SDF slice output is allowed.
    #[must_use]
    pub const fn matter_sdf_enabled(self) -> bool {
        matches!(self, Self::Sdf)
    }

    /// Returns whether this mode is an unsupported future ADF placeholder.
    #[must_use]
    pub const fn is_unsupported_adf_placeholder(self) -> bool {
        matches!(self, Self::UnsupportedAdf | Self::UnsupportedCombined)
    }

    /// Stable marker status label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Sdf => "sdf",
            Self::UnsupportedAdf => "unsupported_adf",
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
) -> Result<QuestMakepadMatterSurfaceConfig, CameraShellConfigError> {
    let runtime_mode = SdfAdfRuntimeMode::from_overlay_mode(overlay_mode);
    let mut config = QuestMakepadMatterSurfaceConfig::default();
    config.collision_enabled = collision_enabled;
    config.sdf_slice_enabled = runtime_mode.matter_sdf_enabled();
    config.particles_enabled = particles_enabled;
    config.enabled = replay_enabled
        && (config.collision_enabled || config.sdf_slice_enabled || particles_enabled);
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
        _ => None,
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
        assert!(config.collision_enabled);
        assert_eq!(config.sdf_adf_overlay_mode, SdfAdfOverlayMode::Sdf);
        assert_eq!(config.sdf_adf_overlay_mode.as_str(), "sdf");
        assert!(config.particles_enabled);
        assert_eq!(config.particle_render_draw_limit, 192);
        assert_eq!(
            config.particle_render_animation_mode,
            ParticleRenderAnimationMode::ProceduralMorphRing
        );
        assert_eq!(config.particle_render_size_scale, 1.0);
        assert!(config.matter_surface.enabled);
        assert!(config.matter_surface.sdf_slice_enabled);
        assert!(config.matter_surface.particles_enabled);
        assert_eq!(config.matter_surface.leaf_triangle_count, 8);
        assert_eq!(config.matter_surface.particle_count, 1_000);
        assert_eq!(
            config.matter_surface.particle_execution_backend,
            ParticleExecutionBackend::Serial
        );
        assert_eq!(
            config.matter_surface.particle_execution_batch_size.get(),
            DEFAULT_PARTICLE_EXECUTION_BATCH_SIZE
        );
        assert_eq!(config.matter_surface.particle_execution_max_threads, None);
    }

    #[test]
    fn effective_settings_builds_full_runtime_bundle() {
        let mut bundle =
            camera_shell_runtime_bundle_from_effective_settings_json(EFFECTIVE_SETTINGS_FIXTURE)
                .unwrap();

        assert!(bundle.effective_config.replay.enabled);
        assert_eq!(bundle.effective_config.render_scale, 0.9);
        assert!(!bundle.effective_config.camera_streaming_enabled);
        assert!(bundle.effective_config.collision_enabled);
        assert_eq!(
            bundle.effective_config.sdf_adf_overlay_mode,
            SdfAdfOverlayMode::Sdf
        );
        assert!(bundle.effective_config.particles_enabled);
        assert_eq!(bundle.effective_config.particle_render_draw_limit, 192);
        assert_eq!(
            bundle.effective_config.particle_render_animation_mode,
            ParticleRenderAnimationMode::ProceduralMorphRing
        );
        assert_eq!(bundle.effective_config.particle_render_size_scale, 1.0);
        assert!(bundle.effective_config.matter_surface.enabled);
        assert_eq!(bundle.matter_surface_runtime.config().particle_count, 1_000);
        assert_eq!(
            bundle
                .matter_surface_runtime
                .config()
                .particle_execution_batch_size
                .get(),
            DEFAULT_PARTICLE_EXECUTION_BATCH_SIZE
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
    fn parses_non_default_sdf_adf_overlay_mode() {
        let combined =
            EFFECTIVE_SETTINGS_FIXTURE.replace("\"value\": \"sdf\"", "\"value\": \"combined\"");
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&combined).unwrap();
        assert_eq!(config.sdf_adf_overlay_mode, SdfAdfOverlayMode::Combined);
        let runtime_mode = SdfAdfRuntimeMode::from_overlay_mode(config.sdf_adf_overlay_mode);
        assert_eq!(runtime_mode, SdfAdfRuntimeMode::UnsupportedCombined);
        assert!(runtime_mode.is_unsupported_adf_placeholder());
        assert!(!config.matter_surface.sdf_slice_enabled);
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
        assert!(config.matter_surface.particles_enabled);
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
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&custom).unwrap();

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
            serde_json::json!("rayon"),
        );
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid_backend).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_MATTER_PARTICLE_EXECUTION_BACKEND)
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
}
