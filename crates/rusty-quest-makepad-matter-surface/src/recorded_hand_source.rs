use rusty_matter_mesh::HandRigCapture;
use rusty_matter_model::Vec3;
use rusty_matter_surface_runtime::MatterSurfaceFrameInput;
use rusty_quest_makepad_mesh_replay::{
    MeshReplayError, RecordedCompactHandJointFrame, RecordedHandRig,
};

use crate::{
    sanitize_marker_value, QuestMakepadGpuMeshSdfProbeInput, QuestMakepadGpuSkinningMeshProbeInput,
    QuestMakepadGpuSkinningProbeInput, QuestMakepadMatterSurfaceError,
    QuestMakepadMatterSurfaceProviderShape, QuestMakepadMatterSurfaceSourceFrame,
    QUEST_MAKEPAD_GPU_SKINNING_PROBE_SAMPLES,
};

/// Optional CPU-oracle GPU probe payloads to attach while building a recorded hand source frame.
///
/// Matter remains the CPU truth for every option. The flags only decide
/// whether the adapter spends the extra per-frame work to package bounded GPU
/// validation payloads beside the Matter source surface.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QuestMakepadRecordedHandSourceFrameOptions {
    /// Include bounded matrix-sample input for the GPU skinning probe.
    pub include_gpu_skinning_probe: bool,
    /// Include full skinned vertex/index oracle input for mesh residency.
    pub include_gpu_skinning_mesh_probe: bool,
    /// Include bounded dense-SDF oracle samples derived from the skinned mesh.
    pub include_gpu_mesh_sdf_probe: bool,
}

impl QuestMakepadRecordedHandSourceFrameOptions {
    /// Builds the Matter source frame only, without optional GPU diagnostic payloads.
    #[must_use]
    pub const fn matter_only() -> Self {
        Self {
            include_gpu_skinning_probe: false,
            include_gpu_skinning_mesh_probe: false,
            include_gpu_mesh_sdf_probe: false,
        }
    }

    /// Builds the Matter source frame plus all current bounded GPU oracle payloads.
    #[must_use]
    pub const fn gpu_oracle_probes() -> Self {
        Self {
            include_gpu_skinning_probe: true,
            include_gpu_skinning_mesh_probe: true,
            include_gpu_mesh_sdf_probe: true,
        }
    }

    /// Returns true if any optional GPU oracle payload is requested.
    #[must_use]
    pub const fn includes_gpu_oracle_payload(self) -> bool {
        self.include_gpu_skinning_probe
            || self.include_gpu_skinning_mesh_probe
            || self.include_gpu_mesh_sdf_probe
    }
}

/// Cached adapter for replay/live-equivalent hand source frames.
///
/// The recorded rig is converted to Matter's neutral CPU oracle rig once, then
/// compact joint frames can be expanded into per-frame source payloads without
/// rebuilding bind-mesh authority on every submit.
#[derive(Clone, Debug)]
pub struct QuestMakepadRecordedHandSourceFrameBuilder {
    source_id: String,
    source_id_token: String,
    rig: RecordedHandRig,
    matter_rig: HandRigCapture,
}

impl QuestMakepadRecordedHandSourceFrameBuilder {
    /// Builds a cached source-frame adapter for one recorded hand rig.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when the recorded rig cannot
    /// be represented by Matter's CPU oracle hand rig.
    pub fn new(
        source_id: impl Into<String>,
        rig: RecordedHandRig,
    ) -> Result<Self, QuestMakepadMatterSurfaceError> {
        let source_id = source_id.into();
        let source_id_token = sanitize_marker_value(&source_id);
        let matter_rig = rig.to_matter_rig_capture(format!("{source_id_token}.rig"))?;
        Ok(Self {
            source_id,
            source_id_token,
            rig,
            matter_rig,
        })
    }

    /// Expands one compact joint frame into a Matter source frame and GPU proof payloads.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when compact frame expansion,
    /// Matter CPU skinning, or source-frame bounds extraction fails.
    pub fn source_frame(
        &self,
        compact_frame: &RecordedCompactHandJointFrame,
    ) -> Result<QuestMakepadMatterSurfaceSourceFrame, QuestMakepadMatterSurfaceError> {
        self.source_frame_with_options(
            compact_frame,
            QuestMakepadRecordedHandSourceFrameOptions::gpu_oracle_probes(),
        )
    }

    /// Expands one compact joint frame into a Matter source frame with selected GPU proof payloads.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when compact frame expansion,
    /// Matter CPU skinning, optional oracle payload packaging, or source-frame
    /// bounds extraction fails.
    pub fn source_frame_with_options(
        &self,
        compact_frame: &RecordedCompactHandJointFrame,
        options: QuestMakepadRecordedHandSourceFrameOptions,
    ) -> Result<QuestMakepadMatterSurfaceSourceFrame, QuestMakepadMatterSurfaceError> {
        let joint_frame = compact_frame.expand_to_matter_joint_frame(
            &self.rig,
            format!(
                "{}.joint_frame.{}",
                self.source_id_token, compact_frame.frame_index
            ),
        )?;
        let validation_frame = self
            .matter_rig
            .skin_to_validation_frame(
                &joint_frame,
                format!(
                    "{}.validation_frame.{}",
                    self.source_id_token, compact_frame.frame_index
                ),
            )
            .map_err(|_| MeshReplayError::InvalidValue("recorded_hand_skinning"))?;

        let gpu_skinning_probe = if options.include_gpu_skinning_probe {
            let skinning_matrix_samples = self
                .matter_rig
                .skinning_matrix_samples(&joint_frame, QUEST_MAKEPAD_GPU_SKINNING_PROBE_SAMPLES)
                .map_err(|_| {
                    MeshReplayError::InvalidValue("recorded_hand_skinning_matrix_samples")
                })?;
            QuestMakepadGpuSkinningProbeInput::from_matter_samples(
                &self.source_id,
                compact_frame.frame_index,
                validation_frame.surface.vertex_count(),
                validation_frame.surface.triangle_count(),
                &skinning_matrix_samples,
            )
        } else {
            None
        };
        let gpu_skinning_mesh_input = if options.include_gpu_skinning_mesh_probe
            || options.include_gpu_mesh_sdf_probe
        {
            let skinning_mesh_oracle = self
                .matter_rig
                .skinning_mesh_buffer_oracle(&joint_frame)
                .map_err(|_| MeshReplayError::InvalidValue("recorded_hand_skinning_mesh_oracle"))?;
            QuestMakepadGpuSkinningMeshProbeInput::from_matter_oracle(
                &self.source_id,
                compact_frame.frame_index,
                &skinning_mesh_oracle,
            )
        } else {
            None
        };
        let gpu_mesh_sdf_probe = if options.include_gpu_mesh_sdf_probe {
            gpu_skinning_mesh_input
                .as_ref()
                .and_then(QuestMakepadGpuMeshSdfProbeInput::from_skinning_mesh_input)
        } else {
            None
        };
        let gpu_skinning_mesh_probe = if options.include_gpu_skinning_mesh_probe {
            gpu_skinning_mesh_input
        } else {
            None
        };
        let (bounds_min, bounds_max) = bounds_from_positions(&validation_frame.surface.positions)?;
        Ok(QuestMakepadMatterSurfaceSourceFrame::new(
            self.source_id.clone(),
            MatterSurfaceFrameInput::new(
                compact_frame.frame_index,
                compact_frame.timestamp_ns as f32 * 1.0e-9,
                validation_frame.surface,
            ),
            bounds_min,
            bounds_max,
        )
        .with_provider_shape(QuestMakepadMatterSurfaceProviderShape::BindMeshPlusCompactJointFrame)
        .with_gpu_skinning_probe(gpu_skinning_probe)
        .with_gpu_skinning_mesh_probe(gpu_skinning_mesh_probe)
        .with_gpu_mesh_sdf_probe(gpu_mesh_sdf_probe))
    }
}

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
        QuestMakepadRecordedHandSourceFrameBuilder::new(source_id, rig.clone())?
            .source_frame(compact_frame)
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
        assert_eq!(
            source_frame.provider_shape,
            QuestMakepadMatterSurfaceProviderShape::BindMeshPlusCompactJointFrame
        );
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
        assert_eq!(probe.samples[2].joint_indices, [2, 0, 0, 0]);
        assert_eq!(probe.samples[2].joint_weights, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(probe.samples[2].joint_matrices[0][2][3], 0.25);
        let mesh_probe = source_frame
            .gpu_skinning_mesh_probe
            .as_ref()
            .expect("recorded hand source carries full GPU skinning mesh probe");
        assert_eq!(mesh_probe.source_id, "recorded-hand-synthetic");
        assert_eq!(mesh_probe.source_frame_index, 7);
        assert_eq!(mesh_probe.topology_vertex_count, 3);
        assert_eq!(mesh_probe.topology_triangle_count, 1);
        assert_eq!(mesh_probe.topology_index_count, 3);
        assert_eq!(mesh_probe.vertices.len(), 3);
        assert_eq!(mesh_probe.triangles, vec![[0, 1, 2]]);
        assert_eq!(mesh_probe.sample_count, 3);
        assert_eq!(
            mesh_probe.vertices[2].expected_position,
            [1.0, 1.0, -0.25, 1.0]
        );
        let mesh_sdf_probe = source_frame
            .gpu_mesh_sdf_probe
            .as_ref()
            .expect("recorded hand source carries bounded GPU mesh SDF probe");
        assert_eq!(mesh_sdf_probe.source_id, "recorded-hand-synthetic");
        assert_eq!(mesh_sdf_probe.source_frame_index, 7);
        assert_eq!(mesh_sdf_probe.topology_vertex_count, 3);
        assert_eq!(mesh_sdf_probe.topology_triangle_count, 1);
        assert!(mesh_sdf_probe.grid.voxel_count > 0);
        assert!(mesh_sdf_probe.grid.voxel_count > 64);
        assert!(
            mesh_sdf_probe.grid.voxel_count <= crate::QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MAX_VOXELS
        );
        assert!(mesh_sdf_probe.grid.voxel_size > 0.0);
        assert_eq!(
            mesh_sdf_probe.sample_count,
            crate::QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SAMPLES
        );
        assert!(mesh_sdf_probe.samples[..mesh_sdf_probe.sample_count]
            .iter()
            .all(|sample| sample.expected_distance.is_finite()));
    }

    #[test]
    fn recorded_hand_source_frame_options_can_skip_gpu_oracle_payloads() {
        let rig = RecordedHandRig::from_json_str(SYNTHETIC_RIG).expect("rig parses");
        let compact =
            RecordedCompactHandJointFrame::from_json_line(SYNTHETIC_FRAME).expect("frame parses");
        let builder =
            QuestMakepadRecordedHandSourceFrameBuilder::new("recorded-hand-synthetic", rig)
                .expect("builder builds");

        let source_frame = builder
            .source_frame_with_options(
                &compact,
                QuestMakepadRecordedHandSourceFrameOptions::matter_only(),
            )
            .expect("recorded hand source frame builds");

        assert_eq!(
            source_frame.provider_shape,
            QuestMakepadMatterSurfaceProviderShape::BindMeshPlusCompactJointFrame
        );
        assert_eq!(source_frame.frame.surface.vertex_count(), 3);
        assert_eq!(source_frame.frame.surface.triangle_count(), 1);
        assert!(source_frame.gpu_skinning_probe.is_none());
        assert!(source_frame.gpu_skinning_mesh_probe.is_none());
        assert!(source_frame.gpu_mesh_sdf_probe.is_none());
        assert!(!QuestMakepadRecordedHandSourceFrameOptions::matter_only()
            .includes_gpu_oracle_payload());
        assert!(
            QuestMakepadRecordedHandSourceFrameOptions::gpu_oracle_probes()
                .includes_gpu_oracle_payload()
        );
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
        assert!(frame.gpu_skinning_mesh_probe.is_some());
        assert!(frame.gpu_mesh_sdf_probe.is_some());

        let marker = runtime.marker_line("unit-test", &frame);
        assert!(marker.contains("sourceId=recorded-hand-synthetic"));
        assert!(marker.contains("sourceProviderShape=bind-mesh-plus-compact-joint-frame"));
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
