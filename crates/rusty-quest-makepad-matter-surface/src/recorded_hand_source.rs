use rusty_matter_model::Vec3;
use rusty_matter_surface_runtime::MatterSurfaceFrameInput;
use rusty_quest_makepad_mesh_replay::{
    MeshReplayError, RecordedCompactHandJointFrame, RecordedHandRig,
};

use crate::{
    sanitize_marker_value, QuestMakepadGpuSkinningProbeInput, QuestMakepadMatterSurfaceError,
    QuestMakepadMatterSurfaceSourceFrame,
};

impl QuestMakepadMatterSurfaceSourceFrame {
    /// Creates a source frame from a recorded live-equivalent hand capture.
    ///
    /// This path keeps the high-rate recorded hand data out of settings JSON:
    /// the adapter expands the compact recorded Makepad/OpenXR joint frame,
    /// asks Matter's CPU skinning oracle for the current surface, then feeds
    /// the same native source-frame boundary used by realtime providers.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when compact hand expansion,
    /// Matter CPU skinning, or surface bounds extraction fails.
    pub fn from_recorded_hand_capture(
        source_id: impl Into<String>,
        rig: &RecordedHandRig,
        compact_frame: &RecordedCompactHandJointFrame,
    ) -> Result<Self, QuestMakepadMatterSurfaceError> {
        let source_id = source_id.into();
        let source_id_token = sanitize_marker_value(&source_id);
        let matter_rig = rig.to_matter_rig_capture(format!("{source_id_token}.rig"))?;
        let joint_frame = compact_frame.expand_to_matter_joint_frame(
            rig,
            format!(
                "{source_id_token}.joint_frame.{}",
                compact_frame.frame_index
            ),
        )?;
        let validation_frame = matter_rig
            .skin_to_validation_frame(
                &joint_frame,
                format!(
                    "{source_id_token}.validation_frame.{}",
                    compact_frame.frame_index
                ),
            )
            .map_err(|_| MeshReplayError::InvalidValue("recorded_hand_skinning"))?;
        let gpu_skinning_probe = QuestMakepadGpuSkinningProbeInput::from_positions(
            &source_id,
            compact_frame.frame_index,
            validation_frame.surface.triangle_count(),
            &rig.bind_surface.positions,
            &validation_frame.surface.positions,
        );
        let (bounds_min, bounds_max) = bounds_from_positions(&validation_frame.surface.positions)?;
        Ok(Self::new(
            source_id,
            MatterSurfaceFrameInput::new(
                compact_frame.frame_index,
                compact_frame.timestamp_ns as f32 * 1.0e-9,
                validation_frame.surface,
            ),
            bounds_min,
            bounds_max,
        )
        .with_gpu_skinning_probe(gpu_skinning_probe))
    }
}

fn bounds_from_positions(positions: &[Vec3]) -> Result<([f32; 3], [f32; 3]), MeshReplayError> {
    let mut minimum = positions
        .first()
        .copied()
        .filter(|position| position.is_finite())
        .ok_or(MeshReplayError::InvalidValue("recorded_hand_bounds"))?;
    let mut maximum = minimum;
    for position in positions.iter().skip(1).copied() {
        if !position.is_finite() {
            return Err(MeshReplayError::InvalidValue("recorded_hand_bounds"));
        }
        minimum = minimum.min(position);
        maximum = maximum.max(position);
    }
    Ok((
        [minimum.x, minimum.y, minimum.z],
        [maximum.x, maximum.y, maximum.z],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MatterSurfaceContactProbe, QuestMakepadMatterSurfaceConfig};
    use crate::{QuestMakepadMatterSurfaceRuntime, QuestMakepadMatterSurfaceSourceFrame};

    fn synthetic_recorded_source_frame() -> QuestMakepadMatterSurfaceSourceFrame {
        let rig = RecordedHandRig::from_json_str(SYNTHETIC_RIG).expect("rig parses");
        let compact =
            RecordedCompactHandJointFrame::from_json_line(SYNTHETIC_FRAME).expect("frame parses");
        QuestMakepadMatterSurfaceSourceFrame::from_recorded_hand_capture(
            "recorded-hand-synthetic",
            &rig,
            &compact,
        )
        .expect("recorded hand source frame builds")
    }

    #[test]
    fn recorded_hand_capture_builds_native_source_frame() {
        let source_frame = synthetic_recorded_source_frame();

        assert_eq!(source_frame.source_id, "recorded-hand-synthetic");
        assert_eq!(source_frame.frame.frame_index, 7);
        assert_eq!(source_frame.frame.time_seconds, 2.0);
        assert_eq!(source_frame.frame.surface.vertex_count(), 3);
        assert_eq!(source_frame.frame.surface.triangle_count(), 1);
        assert_eq!(source_frame.bounds_min, [0.0, 0.0, -0.25]);
        assert_eq!(source_frame.bounds_max, [1.0, 1.0, 0.0]);
        assert_eq!(source_frame.bounds_radius, 0.5);
        assert_eq!(
            source_frame.frame.surface.positions[2],
            Vec3::new(1.0, 1.0, -0.25)
        );
        let probe = source_frame
            .gpu_skinning_probe
            .as_ref()
            .expect("recorded hand source carries GPU skinning probe");
        assert_eq!(probe.source_id, "recorded-hand-synthetic");
        assert_eq!(probe.source_frame_index, 7);
        assert_eq!(probe.topology_vertex_count, 3);
        assert_eq!(probe.topology_triangle_count, 1);
        assert_eq!(probe.sample_count, 3);
        assert_eq!(probe.samples[2].vertex_index, 2);
        assert_eq!(probe.samples[2].bind_position, [1.0, 1.0, -0.5, 1.0]);
        assert_eq!(probe.samples[2].expected_position, [1.0, 1.0, -0.25, 1.0]);
        assert_eq!(probe.samples[2].delta0_weight, [0.0, 0.0, 0.25, 1.0]);
    }

    #[test]
    fn recorded_hand_capture_steps_through_matter_source_frame() {
        let source_frame = synthetic_recorded_source_frame();
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
                source_frame,
                1.0 / 60.0,
                &[MatterSurfaceContactProbe::sphere(
                    "probe.recorded_hand",
                    Vec3::new(0.5, 0.5, -0.125),
                    0.75,
                )],
            )
            .expect("recorded hand source frame steps");

        assert_eq!(frame.source_id, "recorded-hand-synthetic");
        assert_eq!(frame.matter_update.frame_index, Some(7));
        assert_eq!(frame.matter_update.vertex_count, 3);
        assert_eq!(frame.matter_update.triangle_count, 1);
        assert_eq!(frame.collision_upload.rows.len(), 1);
        assert!(frame.gpu_skinning_probe.is_some());

        let marker = runtime.marker_line("unit-test", &frame);
        assert!(marker.contains("sourceId=recorded-hand-synthetic"));
        assert!(marker.contains("nativeMatterRuntime=true"));
        assert!(!marker.contains("rusty.xr"));
        assert!(!marker.contains("RUSTY_XR"));
    }

    #[test]
    #[ignore = "requires RUSTY_QUEST_MAKEPAD_RECORDED_HAND_CAPTURE_DIR"]
    fn external_recorded_hand_capture_steps_through_source_frame_when_configured() {
        let root = std::env::var("RUSTY_QUEST_MAKEPAD_RECORDED_HAND_CAPTURE_DIR")
            .expect("set recorded hand capture directory");
        let root = std::path::PathBuf::from(root);
        let rig_json = std::fs::read_to_string(root.join("left.rig.json")).expect("read rig");
        let clip_text = std::fs::read_to_string(root.join("left.clip.jsonl")).expect("read clip");
        let rig = RecordedHandRig::from_json_str(&rig_json).expect("rig parses");
        let compact = clip_text
            .lines()
            .filter_map(|line| RecordedCompactHandJointFrame::from_json_line(line).ok())
            .next()
            .expect("compact frame exists");
        let source_frame = QuestMakepadMatterSurfaceSourceFrame::from_recorded_hand_capture(
            "recorded-meta-quest-hand-left-capture",
            &rig,
            &compact,
        )
        .expect("recorded hand source frame builds");

        assert!(source_frame.frame.surface.vertex_count() > 8);
        assert!(source_frame.frame.surface.triangle_count() > 6);
        let probe_center = source_frame.bounds_center();
        let probe_radius = source_frame.surface_radius();
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
                source_frame,
                1.0 / 60.0,
                &[MatterSurfaceContactProbe::sphere(
                    "probe.recorded_hand_capture",
                    probe_center,
                    probe_radius.max(0.01),
                )],
            )
            .expect("recorded hand capture source frame steps");

        assert_eq!(frame.source_id, "recorded-meta-quest-hand-left-capture");
        assert!(frame.matter_update.vertex_count > 8);
        assert!(frame.matter_update.triangle_count > 6);
        assert_eq!(frame.collision_upload.rows.len(), 1);
        let marker = runtime.marker_line("unit-test", &frame);
        assert!(marker.contains("sourceId=recorded-meta-quest-hand-left-capture"));
        assert!(marker.contains("nativeMatterRuntime=true"));
    }

    const SYNTHETIC_RIG: &str = r#"{
      "schema": "rusty.matter.hand_mesh_rig.v1",
      "handedness": "left",
      "reference_space": "makepad-openxr-local-space",
      "topology_key": "synthetic-openxr-handmesh-j3-v3-i3",
      "bind_version": 1,
      "runtime_joint_set": {
        "schema": "rusty.matter.hand_runtime_joint_set.v1",
        "provider": "makepad-xr-hand-compact-21",
        "joint_count": 2,
        "tip_length_count": 1,
        "tip_length_units": "meters",
        "runtime_joints": [
          {"runtime_index": 0, "runtime_name": "palm", "bind_joint_index": 0, "bind_joint_name": "palm_ext"},
          {"runtime_index": 1, "runtime_name": "index_distal", "bind_joint_index": 1, "bind_joint_name": "index_distal_ext"}
        ],
        "bind_joint_sources": [
          {"bind_joint_index": 0, "bind_joint_name": "palm_ext", "source_kind": "runtime_pose", "runtime_joint_index": 0, "runtime_joint_name": "palm", "tip_length_index": null, "parent_runtime_joint_index": null, "parent_runtime_joint_name": null},
          {"bind_joint_index": 1, "bind_joint_name": "index_distal_ext", "source_kind": "runtime_pose", "runtime_joint_index": 1, "runtime_joint_name": "index_distal", "tip_length_index": null, "parent_runtime_joint_index": null, "parent_runtime_joint_name": null},
          {"bind_joint_index": 2, "bind_joint_name": "index_tip_ext", "source_kind": "tip_length_from_parent_pose", "runtime_joint_index": null, "runtime_joint_name": null, "tip_length_index": 0, "parent_runtime_joint_index": 1, "parent_runtime_joint_name": "index_distal"}
        ]
      },
      "joints": [
        {"index": 0, "name": "palm_ext", "parent_index": 65535, "radius_m": 0.01, "bind_pose": {"translation": [0.0, 0.0, 0.0], "rotation": [0.0, 0.0, 0.0, 1.0]}},
        {"index": 1, "name": "index_distal_ext", "parent_index": 0, "radius_m": 0.008, "bind_pose": {"translation": [1.0, 0.0, -0.5], "rotation": [0.0, 0.0, 0.0, 1.0]}},
        {"index": 2, "name": "index_tip_ext", "parent_index": 1, "radius_m": 0.006, "bind_pose": {"translation": [1.0, 0.0, -0.5], "rotation": [0.0, 0.0, 0.0, 1.0]}}
      ],
      "bind_vertices": [[0.0, 0.0, 0.0], [1.0, 0.0, -0.5], [1.0, 1.0, -0.5]],
      "bind_normals": [[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
      "triangle_indices": [[0, 1, 2]],
      "vertex_uvs": [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
      "vertex_blend_indices": [[0, 0, 0, 0], [1, 0, 0, 0], [2, 0, 0, 0]],
      "vertex_blend_weights": [[1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]]
    }"#;

    const SYNTHETIC_FRAME: &str = r#"{
      "schema": "rusty.matter.hand_joint_frame.v1",
      "handedness": "left",
      "frame_index": 7,
      "timestamp_ns": 2000000000,
      "joints": [
        {"joint_index": 0, "pose": {"translation": [0.0, 0.0, 0.0], "rotation": [0.0, 0.0, 0.0, 1.0]}, "radius_m": 0.01, "location_flags": ["position_valid"], "confidence": "high"},
        {"joint_index": 1, "pose": {"translation": [1.0, 0.0, 0.0], "rotation": [0.0, 0.0, 0.0, 1.0]}, "radius_m": 0.008, "location_flags": ["position_valid"], "confidence": "high"}
      ],
      "tip_lengths_m": [0.25],
      "pinch_strengths": [0.0, 0.0, 0.0, 0.0]
    }"#;
}
