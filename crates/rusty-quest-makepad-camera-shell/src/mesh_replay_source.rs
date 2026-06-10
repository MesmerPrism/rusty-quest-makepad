use std::path::{Path, PathBuf};

use rusty_quest_makepad_mesh_replay::{MeshReplayConfig, MeshReplayRuntime, MeshReplaySequence};

use crate::{CameraShellConfigError, CameraShellReplayConfig, SETTING_MESH_REPLAY_SOURCE};

/// Bundled smoke replay source id.
pub const MESH_REPLAY_SOURCE_PUBLIC_SYNTHETIC_HAND_SEQUENCE: &str =
    "public-synthetic-hand-sequence";
/// Recorded Quest hand replay left-hand source id.
pub const MESH_REPLAY_SOURCE_RECORDED_META_QUEST_HAND_LEFT: &str = "recorded-meta-quest-hand-left";
/// Recorded Quest hand replay right-hand source id.
pub const MESH_REPLAY_SOURCE_RECORDED_META_QUEST_HAND_RIGHT: &str =
    "recorded-meta-quest-hand-right";
/// Relative file name for the recorded left-hand replay sequence.
pub const RECORDED_META_QUEST_HAND_LEFT_SEQUENCE_FILE: &str =
    "recorded-meta-quest-hand-sequence-mesh0.json";
/// Relative file name for the recorded right-hand replay sequence.
pub const RECORDED_META_QUEST_HAND_RIGHT_SEQUENCE_FILE: &str =
    "recorded-meta-quest-hand-sequence-mesh1.json";

pub(crate) fn mesh_replay_runtime_from_config(
    replay: &CameraShellReplayConfig,
    replay_asset_dir: Option<&Path>,
) -> Result<MeshReplayRuntime, CameraShellConfigError> {
    let config = replay.clone().into_mesh_replay_config();
    match replay.source.as_str() {
        MESH_REPLAY_SOURCE_PUBLIC_SYNTHETIC_HAND_SEQUENCE => {
            let mut runtime = MeshReplayRuntime::default();
            runtime.configure(config);
            Ok(runtime)
        }
        MESH_REPLAY_SOURCE_RECORDED_META_QUEST_HAND_LEFT => mesh_replay_runtime_from_asset(
            config,
            replay_asset_dir,
            RECORDED_META_QUEST_HAND_LEFT_SEQUENCE_FILE,
        ),
        MESH_REPLAY_SOURCE_RECORDED_META_QUEST_HAND_RIGHT => mesh_replay_runtime_from_asset(
            config,
            replay_asset_dir,
            RECORDED_META_QUEST_HAND_RIGHT_SEQUENCE_FILE,
        ),
        _ => Err(CameraShellConfigError::InvalidSettingValue(
            SETTING_MESH_REPLAY_SOURCE,
        )),
    }
}

fn mesh_replay_runtime_from_asset(
    config: MeshReplayConfig,
    replay_asset_dir: Option<&Path>,
    sequence_file_name: &str,
) -> Result<MeshReplayRuntime, CameraShellConfigError> {
    let Some(replay_asset_dir) = replay_asset_dir else {
        return Err(CameraShellConfigError::MeshReplayAsset(format!(
            "source {} requires a replay asset directory",
            config.source
        )));
    };
    let sequence_path = resolve_replay_sequence_path(replay_asset_dir, sequence_file_name)?;
    let sequence_json = std::fs::read_to_string(&sequence_path).map_err(|error| {
        CameraShellConfigError::MeshReplayAsset(format!(
            "read {} failed: {error}",
            sequence_path.display()
        ))
    })?;
    let sequence = MeshReplaySequence::from_json_str(&sequence_json).map_err(|error| {
        CameraShellConfigError::MeshReplayAsset(format!(
            "parse {} failed: {error}",
            sequence_path.display()
        ))
    })?;
    Ok(MeshReplayRuntime::from_sequence(sequence, config))
}

fn resolve_replay_sequence_path(
    replay_asset_dir: &Path,
    sequence_file_name: &str,
) -> Result<PathBuf, CameraShellConfigError> {
    let candidates = [
        replay_asset_dir
            .join("mesh-replay")
            .join(sequence_file_name),
        replay_asset_dir.join(sequence_file_name),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            CameraShellConfigError::MeshReplayAsset(format!(
                "missing {sequence_file_name} under {} or {}",
                replay_asset_dir.join("mesh-replay").display(),
                replay_asset_dir.display()
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYNTHETIC_SEQUENCE_FIXTURE: &str =
        include_str!("../../../fixtures/mesh-replay/synthetic-hand-mesh-sequence.json");

    #[test]
    fn public_source_uses_bundled_smoke_replay() {
        let replay = replay_config(MESH_REPLAY_SOURCE_PUBLIC_SYNTHETIC_HAND_SEQUENCE);

        let mut runtime = mesh_replay_runtime_from_config(&replay, None).unwrap();

        let step = runtime.step(0.0);
        assert!(step.enabled);
        assert_eq!(step.frame_index, 0);
        assert!(runtime
            .config_marker_line("unit-test")
            .contains("source=public-synthetic-hand-sequence"));
    }

    #[test]
    fn recorded_left_source_loads_from_staged_asset_dir() {
        let replay = replay_config(MESH_REPLAY_SOURCE_RECORDED_META_QUEST_HAND_LEFT);
        let asset_dir = write_temp_replay_asset(
            "recorded-left-source",
            RECORDED_META_QUEST_HAND_LEFT_SEQUENCE_FILE,
            SYNTHETIC_SEQUENCE_FIXTURE,
        );

        let mut runtime = mesh_replay_runtime_from_config(&replay, Some(&asset_dir)).unwrap();

        let step = runtime.step(0.0);
        assert!(step.enabled);
        assert!(runtime
            .config_marker_line("unit-test")
            .contains("source=recorded-meta-quest-hand-left"));
    }

    #[test]
    fn recorded_right_source_loads_from_staged_asset_dir() {
        let replay = replay_config(MESH_REPLAY_SOURCE_RECORDED_META_QUEST_HAND_RIGHT);
        let asset_dir = write_temp_replay_asset(
            "recorded-right-source",
            RECORDED_META_QUEST_HAND_RIGHT_SEQUENCE_FILE,
            SYNTHETIC_SEQUENCE_FIXTURE,
        );

        let mut runtime = mesh_replay_runtime_from_config(&replay, Some(&asset_dir)).unwrap();

        let step = runtime.step(0.0);
        assert!(step.enabled);
        assert!(runtime
            .config_marker_line("unit-test")
            .contains("source=recorded-meta-quest-hand-right"));
    }

    #[test]
    fn recorded_source_requires_staged_asset_dir() {
        let replay = replay_config(MESH_REPLAY_SOURCE_RECORDED_META_QUEST_HAND_LEFT);

        let error = mesh_replay_runtime_from_config(&replay, None).unwrap_err();

        assert!(matches!(error, CameraShellConfigError::MeshReplayAsset(_)));
        assert!(error
            .to_string()
            .contains("requires a replay asset directory"));
    }

    #[test]
    fn unknown_source_is_rejected() {
        let replay = replay_config("unknown-hand-source");

        assert_eq!(
            mesh_replay_runtime_from_config(&replay, None).unwrap_err(),
            CameraShellConfigError::InvalidSettingValue(SETTING_MESH_REPLAY_SOURCE)
        );
    }

    fn replay_config(source: &str) -> CameraShellReplayConfig {
        CameraShellReplayConfig {
            enabled: true,
            source: source.to_string(),
            speed: 1.0,
            opacity: 0.75,
        }
    }

    fn write_temp_replay_asset(name: &str, file_name: &str, text: &str) -> PathBuf {
        let root = temp_root(name);
        let mesh_replay_dir = root.join("mesh-replay");
        std::fs::create_dir_all(&mesh_replay_dir).expect("create mesh replay temp dir");
        std::fs::write(mesh_replay_dir.join(file_name), text).expect("write mesh replay asset");
        root
    }

    fn temp_root(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{name}-{stamp}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }
}
