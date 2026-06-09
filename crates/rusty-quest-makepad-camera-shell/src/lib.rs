//! Profile-driven Quest Makepad camera shell adapter.

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
        let settings = value
            .get("settings")
            .and_then(Value::as_array)
            .ok_or(CameraShellConfigError::MissingField("settings"))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    const EFFECTIVE_SETTINGS_FIXTURE: &str =
        include_str!("../../../fixtures/effective-settings/mesh-replay.effective-settings.json");

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
    fn normalizes_values_at_runtime_boundary() {
        let high_values = EFFECTIVE_SETTINGS_FIXTURE
            .replace("\"value\": 1.5", "\"value\": 12.0")
            .replace("\"value\": 0.75", "\"value\": 2.0");
        let config = mesh_replay_config_from_effective_settings_json(&high_values).unwrap();
        assert_eq!(config.speed, 8.0);
        assert_eq!(config.opacity, 1.0);
    }
}
