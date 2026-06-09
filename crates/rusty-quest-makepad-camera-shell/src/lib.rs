//! Profile-driven Quest Makepad camera shell adapter.

use rusty_lattice_model::{validate_display_view_set, DisplayViewSet};
use rusty_optics_model::{
    ProjectionGeometryReport, Rect2, VideoProjectionMapping, IDENTITY_HOMOGRAPHY,
};
use rusty_quest_makepad_mesh_replay::{MeshReplayConfig, MeshReplayRuntime};
use serde_json::Value;

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
/// Collision enable setting id.
pub const SETTING_COLLISION_ENABLED: &str = "makepad.collision.enabled";
/// SDF/ADF overlay mode setting id.
pub const SETTING_SDF_ADF_OVERLAY_MODE: &str = "makepad.sdf_adf.overlay_mode";
/// Particle overlay enable setting id.
pub const SETTING_PARTICLES_ENABLED: &str = "makepad.particles.enabled";
/// Default projection footprint sample grid for app-shell contract smoke tests.
pub const DEFAULT_PROJECTION_FOOTPRINT_GRID: usize = 8;

/// Complete camera-shell subset of the canonical effective-settings report.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraShellEffectiveConfig {
    /// Replay settings.
    pub replay: CameraShellReplayConfig,
    /// Render scale for the Makepad/XR runtime.
    pub render_scale: f32,
    /// Whether collision behavior is enabled.
    pub collision_enabled: bool,
    /// SDF/ADF overlay mode.
    pub sdf_adf_overlay_mode: SdfAdfOverlayMode,
    /// Whether particle behavior is enabled.
    pub particles_enabled: bool,
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
        let collision_enabled = parse_bool_setting(settings, SETTING_COLLISION_ENABLED)?;
        let sdf_adf_overlay_mode = parse_sdf_adf_overlay_mode(settings)?;
        let particles_enabled = parse_bool_setting(settings, SETTING_PARTICLES_ENABLED)?;

        Ok(Self {
            replay,
            render_scale,
            collision_enabled,
            sdf_adf_overlay_mode,
            particles_enabled,
        })
    }
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
    let config = mesh_replay_config_from_effective_settings_json(json)?;
    let mut runtime = MeshReplayRuntime::default();
    runtime.configure(config);
    Ok(runtime)
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
        assert!(!config.collision_enabled);
        assert_eq!(config.sdf_adf_overlay_mode, SdfAdfOverlayMode::Off);
        assert_eq!(config.sdf_adf_overlay_mode.as_str(), "off");
        assert!(!config.particles_enabled);
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
            EFFECTIVE_SETTINGS_FIXTURE.replace("\"value\": \"off\"", "\"value\": \"combined\"");
        let config = CameraShellEffectiveConfig::from_effective_settings_json(&combined).unwrap();
        assert_eq!(config.sdf_adf_overlay_mode, SdfAdfOverlayMode::Combined);
    }

    #[test]
    fn rejects_invalid_sdf_adf_overlay_mode() {
        let invalid =
            EFFECTIVE_SETTINGS_FIXTURE.replace("\"value\": \"off\"", "\"value\": \"private\"");
        assert_eq!(
            CameraShellEffectiveConfig::from_effective_settings_json(&invalid).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_SDF_ADF_OVERLAY_MODE)
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
}
