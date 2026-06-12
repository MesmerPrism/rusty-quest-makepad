//! Quest Makepad profile bundle descriptors.

use serde::{Deserialize, Serialize};

/// Quest Makepad runtime profile bundle schema id.
pub const QUEST_MAKEPAD_PROFILE_SCHEMA: &str = "rusty.quest.makepad.runtime_profile.v1";

/// Bundle tying a Makepad settings profile to a Quest runtime profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestMakepadProfileBundle {
    /// Schema id.
    pub schema: String,
    /// Stable bundle id.
    pub bundle_id: String,
    /// Target app id.
    pub app_id: String,
    /// Path to the Makepad settings surface.
    pub settings_surface: String,
    /// Path to the Makepad settings profile.
    pub settings_profile: String,
    /// Path to the Quest runtime profile.
    pub quest_runtime_profile: String,
    /// Marker emitted by the app after resolving settings.
    pub effective_settings_marker: String,
    /// Optional notes.
    #[serde(default)]
    pub notes: Option<String>,
}

/// Validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Human-readable message.
    pub message: String,
}

impl ValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Validate a Quest Makepad profile bundle.
pub fn validate_bundle(bundle: &QuestMakepadProfileBundle) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    if bundle.schema != QUEST_MAKEPAD_PROFILE_SCHEMA {
        errors.push(ValidationError::new(format!(
            "unsupported Quest Makepad profile schema {}",
            bundle.schema
        )));
    }
    if bundle.bundle_id.trim().is_empty() {
        errors.push(ValidationError::new("bundle_id must not be empty"));
    }
    if bundle.app_id.trim().is_empty() {
        errors.push(ValidationError::new("app_id must not be empty"));
    }
    for (label, path) in [
        ("settings_surface", &bundle.settings_surface),
        ("settings_profile", &bundle.settings_profile),
        ("quest_runtime_profile", &bundle.quest_runtime_profile),
    ] {
        if path.trim().is_empty() {
            errors.push(ValidationError::new(format!("{label} must not be empty")));
        }
    }
    if !bundle
        .effective_settings_marker
        .starts_with("RUSTY_QUEST_MAKEPAD_")
    {
        errors.push(ValidationError::new(
            "effective_settings_marker must use RUSTY_QUEST_MAKEPAD_*",
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_bundle, QuestMakepadProfileBundle};

    #[test]
    fn valid_bundle_passes() {
        let bundle: QuestMakepadProfileBundle = serde_json::from_str(include_str!(
            "../../../fixtures/profiles/mesh-replay.bundle.json"
        ))
        .expect("valid bundle JSON");
        validate_bundle(&bundle).expect("bundle validates");
    }

    #[test]
    fn remote_camera_bundle_passes() {
        let bundle: QuestMakepadProfileBundle = serde_json::from_str(include_str!(
            "../../../fixtures/profiles/remote-camera-q2q.bundle.json"
        ))
        .expect("valid bundle JSON");
        validate_bundle(&bundle).expect("bundle validates");
    }

    #[test]
    fn stimulus_bundle_passes() {
        let bundle: QuestMakepadProfileBundle = serde_json::from_str(include_str!(
            "../../../fixtures/profiles/stimulus-interference.bundle.json"
        ))
        .expect("valid bundle JSON");
        validate_bundle(&bundle).expect("bundle validates");
    }

    #[test]
    fn missing_profile_is_rejected() {
        let bundle: QuestMakepadProfileBundle = serde_json::from_str(include_str!(
            "../../../fixtures/damaged/missing-profile.bundle.json"
        ))
        .expect("damaged bundle JSON");
        let errors = validate_bundle(&bundle).expect_err("must reject missing profile");
        assert!(errors
            .iter()
            .any(|error| error.message.contains("settings_profile")));
    }
}
