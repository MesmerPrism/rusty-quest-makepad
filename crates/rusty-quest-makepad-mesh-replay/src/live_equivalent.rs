//! Primitive constructors for live-equivalent Makepad/OpenXR hand replay data.
//!
//! This module stays Makepad-free. Hostess or another platform adapter converts
//! native runtime types into primitive arrays before handing them to this crate.

use rusty_matter_mesh::{Handedness, TriangleMeshSurface};
use rusty_matter_model::Vec3;

use crate::recorded_hand::{
    RecordedBindJointSource, RecordedBindJointSourceKind, RecordedCompactHandJointFrame,
    RecordedHandBindJoint, RecordedHandPose, RecordedHandRig, RecordedRuntimeJointPose,
    RECORDED_HAND_COMPACT_21_PROVIDER,
};
use crate::MeshReplayError;

const MAKEPAD_OPENXR_BIND_JOINT_COUNT: usize = 26;
const MAKEPAD_OPENXR_COMPACT_JOINT_COUNT: usize = 21;
const MAKEPAD_OPENXR_TIP_LENGTH_COUNT: usize = 5;

impl RecordedHandRig {
    /// Builds a recorded-hand rig from live Makepad/OpenXR bind mesh values.
    ///
    /// The input shape mirrors `XrHandMeshBindData`, but uses primitive arrays
    /// so this crate does not depend on Makepad. The resulting rig is the same
    /// bind-mesh-plus-compact-joint-frame shape consumed by recorded replay.
    ///
    /// # Errors
    ///
    /// Returns [`MeshReplayError`] when the bind mesh cannot satisfy the
    /// current 26-joint Meta/OpenXR hand mesh mapping.
    pub fn from_makepad_openxr_bind_data(
        is_left: bool,
        bind_version: u32,
        joint_bind_poses: &[([f32; 3], [f32; 4])],
        joint_radii: &[f32],
        joint_parent_indices: &[i32],
        vertex_positions: &[[f32; 3]],
        vertex_normals: &[[f32; 3]],
        vertex_blend_indices: &[[i16; 4]],
        vertex_blend_weights: &[[f32; 4]],
        indices: &[i16],
    ) -> Result<Self, MeshReplayError> {
        if joint_bind_poses.len() != MAKEPAD_OPENXR_BIND_JOINT_COUNT {
            return Err(MeshReplayError::InvalidValue("joint_bind_poses"));
        }
        if joint_radii.len() != joint_bind_poses.len()
            || joint_parent_indices.len() != joint_bind_poses.len()
        {
            return Err(MeshReplayError::InvalidValue("joint_metadata"));
        }
        if vertex_positions.is_empty()
            || vertex_normals.len() != vertex_positions.len()
            || vertex_blend_indices.len() != vertex_positions.len()
            || vertex_blend_weights.len() != vertex_positions.len()
        {
            return Err(MeshReplayError::InvalidValue("vertex_metadata"));
        }
        if indices.is_empty() || indices.len() % 3 != 0 {
            return Err(MeshReplayError::InvalidValue("indices"));
        }

        let mut joints = Vec::with_capacity(joint_bind_poses.len());
        for (index, (translation, rotation_xyzw)) in joint_bind_poses.iter().copied().enumerate() {
            let parent_index = match joint_parent_indices[index] {
                value if value < 0 => None,
                value => Some(
                    usize::try_from(value)
                        .map_err(|_| MeshReplayError::InvalidValue("joint_parent_indices"))?,
                ),
            };
            joints.push(RecordedHandBindJoint {
                index,
                name: bind_joint_name(index).to_owned(),
                parent_index,
                radius_m: joint_radii[index],
                bind_pose: pose_from_arrays(translation, rotation_xyzw)?,
            });
        }

        let bind_vertices = vertex_positions
            .iter()
            .copied()
            .map(vec3_from_array)
            .collect::<Result<Vec<_>, _>>()?;
        let bind_normals = vertex_normals
            .iter()
            .copied()
            .map(vec3_from_array)
            .collect::<Result<Vec<_>, _>>()?;
        let triangles = triangle_indices(indices, vertex_positions.len())?;
        let bind_surface = TriangleMeshSurface::new(
            format!(
                "recorded.hand.bind.{}.v{}",
                handedness_label(is_left),
                bind_version
            ),
            bind_vertices,
            triangles,
        );

        let rig = Self {
            handedness: handedness(is_left),
            reference_space: "makepad-openxr-local-space".to_owned(),
            topology_key_label: topology_key(
                bind_version,
                joint_bind_poses.len(),
                vertex_positions.len(),
                indices.len(),
            ),
            bind_version,
            runtime_provider: RECORDED_HAND_COMPACT_21_PROVIDER.to_owned(),
            bind_surface,
            bind_normals,
            joints,
            vertex_blend_indices: blend_indices(vertex_blend_indices)?,
            vertex_blend_weights: vertex_blend_weights.to_vec(),
            bind_joint_sources: bind_joint_sources(),
        };
        rig.validate()?;
        Ok(rig)
    }
}

impl RecordedCompactHandJointFrame {
    /// Builds one compact recorded-frame equivalent from live Makepad/OpenXR hand values.
    ///
    /// # Errors
    ///
    /// Returns [`MeshReplayError`] when the frame is not a 21-joint compact
    /// Makepad hand packet with five tip lengths.
    pub fn from_makepad_openxr_compact_frame(
        is_left: bool,
        frame_index: usize,
        timestamp_ns: u64,
        joint_poses: &[([f32; 3], [f32; 4])],
        tip_lengths_m: [f32; MAKEPAD_OPENXR_TIP_LENGTH_COUNT],
        pinch_strengths: [f32; 4],
    ) -> Result<Self, MeshReplayError> {
        if joint_poses.len() != MAKEPAD_OPENXR_COMPACT_JOINT_COUNT {
            return Err(MeshReplayError::InvalidValue("joint_poses"));
        }
        let joints = joint_poses
            .iter()
            .copied()
            .enumerate()
            .map(|(joint_index, (translation, rotation_xyzw))| {
                Ok(RecordedRuntimeJointPose {
                    joint_index,
                    pose: pose_from_arrays(translation, rotation_xyzw)?,
                    radius_m: 0.0,
                })
            })
            .collect::<Result<Vec<_>, MeshReplayError>>()?;
        if !tip_lengths_m
            .iter()
            .chain(pinch_strengths.iter())
            .all(|value| value.is_finite())
        {
            return Err(MeshReplayError::InvalidValue("compact_frame_values"));
        }
        Ok(Self {
            handedness: handedness(is_left),
            frame_index,
            timestamp_ns,
            joints,
            tip_lengths_m: tip_lengths_m.to_vec(),
            pinch_strengths: pinch_strengths.to_vec(),
        })
    }
}

fn handedness(is_left: bool) -> Handedness {
    if is_left {
        Handedness::Left
    } else {
        Handedness::Right
    }
}

fn handedness_label(is_left: bool) -> &'static str {
    if is_left {
        "left"
    } else {
        "right"
    }
}

fn pose_from_arrays(
    translation: [f32; 3],
    rotation_xyzw: [f32; 4],
) -> Result<RecordedHandPose, MeshReplayError> {
    Ok(RecordedHandPose {
        translation: vec3_from_array(translation)?,
        rotation_xyzw: finite_quat(rotation_xyzw)?,
    })
}

fn vec3_from_array(value: [f32; 3]) -> Result<Vec3, MeshReplayError> {
    if value.iter().all(|component| component.is_finite()) {
        Ok(Vec3::new(value[0], value[1], value[2]))
    } else {
        Err(MeshReplayError::InvalidValue("vec3"))
    }
}

fn finite_quat(value: [f32; 4]) -> Result<[f32; 4], MeshReplayError> {
    if value.iter().all(|component| component.is_finite()) {
        Ok(value)
    } else {
        Err(MeshReplayError::InvalidValue("rotation"))
    }
}

fn triangle_indices(
    indices: &[i16],
    vertex_count: usize,
) -> Result<Vec<[u32; 3]>, MeshReplayError> {
    let mut triangles = Vec::with_capacity(indices.len() / 3);
    for chunk in indices.chunks_exact(3) {
        let triangle = [
            non_negative_index(chunk[0], vertex_count)?,
            non_negative_index(chunk[1], vertex_count)?,
            non_negative_index(chunk[2], vertex_count)?,
        ];
        triangles.push(triangle);
    }
    Ok(triangles)
}

fn non_negative_index(value: i16, vertex_count: usize) -> Result<u32, MeshReplayError> {
    let index = usize::try_from(value).map_err(|_| MeshReplayError::InvalidValue("indices"))?;
    if index >= vertex_count {
        return Err(MeshReplayError::InvalidValue("indices"));
    }
    u32::try_from(index).map_err(|_| MeshReplayError::InvalidValue("indices"))
}

fn blend_indices(values: &[[i16; 4]]) -> Result<Vec<[u16; 4]>, MeshReplayError> {
    values
        .iter()
        .copied()
        .map(|indices| {
            Ok([
                non_negative_blend_index(indices[0])?,
                non_negative_blend_index(indices[1])?,
                non_negative_blend_index(indices[2])?,
                non_negative_blend_index(indices[3])?,
            ])
        })
        .collect()
}

fn non_negative_blend_index(value: i16) -> Result<u16, MeshReplayError> {
    u16::try_from(value.max(0)).map_err(|_| MeshReplayError::InvalidValue("vertex_blend_indices"))
}

fn bind_joint_sources() -> Vec<RecordedBindJointSource> {
    (0..MAKEPAD_OPENXR_BIND_JOINT_COUNT)
        .filter_map(|bind_joint_index| {
            if let Some(runtime_joint_index) = bind_joint_to_runtime_joint(bind_joint_index) {
                return Some(RecordedBindJointSource {
                    bind_joint_index,
                    source: RecordedBindJointSourceKind::RuntimePose {
                        runtime_joint_index,
                    },
                });
            }
            let (tip_length_index, parent_runtime_joint_index) =
                bind_tip_to_runtime_parent(bind_joint_index)?;
            Some(RecordedBindJointSource {
                bind_joint_index,
                source: RecordedBindJointSourceKind::TipLengthFromParentPose {
                    tip_length_index,
                    parent_runtime_joint_index,
                },
            })
        })
        .collect()
}

fn bind_joint_to_runtime_joint(index: usize) -> Option<usize> {
    match index {
        0 => Some(0),
        1 => Some(1),
        2 => Some(2),
        3 => Some(3),
        4 => Some(4),
        6 => Some(5),
        7 => Some(6),
        8 => Some(7),
        9 => Some(8),
        11 => Some(9),
        12 => Some(10),
        13 => Some(11),
        14 => Some(12),
        16 => Some(13),
        17 => Some(14),
        18 => Some(15),
        19 => Some(16),
        21 => Some(17),
        22 => Some(18),
        23 => Some(19),
        24 => Some(20),
        _ => None,
    }
}

fn bind_tip_to_runtime_parent(index: usize) -> Option<(usize, usize)> {
    match index {
        5 => Some((0, 4)),
        10 => Some((1, 8)),
        15 => Some((2, 12)),
        20 => Some((3, 16)),
        25 => Some((4, 20)),
        _ => None,
    }
}

fn bind_joint_name(index: usize) -> &'static str {
    match index {
        0 => "palm_ext",
        1 => "wrist_ext",
        2 => "thumb_metacarpal_ext",
        3 => "thumb_proximal_ext",
        4 => "thumb_distal_ext",
        5 => "thumb_tip_ext",
        6 => "index_metacarpal_ext",
        7 => "index_proximal_ext",
        8 => "index_intermediate_ext",
        9 => "index_distal_ext",
        10 => "index_tip_ext",
        11 => "middle_metacarpal_ext",
        12 => "middle_proximal_ext",
        13 => "middle_intermediate_ext",
        14 => "middle_distal_ext",
        15 => "middle_tip_ext",
        16 => "ring_metacarpal_ext",
        17 => "ring_proximal_ext",
        18 => "ring_intermediate_ext",
        19 => "ring_distal_ext",
        20 => "ring_tip_ext",
        21 => "little_metacarpal_ext",
        22 => "little_proximal_ext",
        23 => "little_intermediate_ext",
        24 => "little_distal_ext",
        25 => "little_tip_ext",
        _ => "unknown_ext",
    }
}

fn topology_key(
    bind_version: u32,
    joint_count: usize,
    vertex_count: usize,
    index_count: usize,
) -> String {
    format!("openxr-fb-handmesh-v{bind_version}-j{joint_count}-v{vertex_count}-i{index_count}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makepad_openxr_primitives_expand_to_live_equivalent_matter_frame() {
        let rig = synthetic_rig().expect("rig builds");
        let compact = synthetic_compact_frame().expect("compact frame builds");
        let matter_rig = rig
            .to_matter_rig_capture("hand.rig_capture.live_equivalent.synthetic")
            .expect("matter rig builds");
        let joint_frame = compact
            .expand_to_matter_joint_frame(&rig, "hand.joint_frame.live_equivalent.synthetic")
            .expect("joint frame expands");
        let validation = matter_rig
            .skin_to_validation_frame(
                &joint_frame,
                "hand.validation_mesh.live_equivalent.synthetic",
            )
            .expect("matter skins live-equivalent frame");

        assert_eq!(rig.runtime_provider, RECORDED_HAND_COMPACT_21_PROVIDER);
        assert_eq!(rig.joints.len(), MAKEPAD_OPENXR_BIND_JOINT_COUNT);
        assert_eq!(joint_frame.poses.len(), MAKEPAD_OPENXR_BIND_JOINT_COUNT);
        assert_eq!(joint_frame.poses[10].position, Vec3::new(1.0, 0.0, -0.25));
        assert_eq!(validation.surface.positions[2], Vec3::new(1.0, 1.0, -0.25));
    }

    fn synthetic_rig() -> Result<RecordedHandRig, MeshReplayError> {
        let mut joint_bind_poses = vec![([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]); 26];
        joint_bind_poses[9] = ([1.0, 0.0, -0.5], [0.0, 0.0, 0.0, 1.0]);
        joint_bind_poses[10] = ([1.0, 0.0, -0.5], [0.0, 0.0, 0.0, 1.0]);
        let joint_radii = vec![0.01; 26];
        let mut parents = vec![-1; 26];
        parents[10] = 9;
        RecordedHandRig::from_makepad_openxr_bind_data(
            true,
            1,
            &joint_bind_poses,
            &joint_radii,
            &parents,
            &[[0.0, 0.0, 0.0], [1.0, 0.0, -0.5], [1.0, 1.0, -0.5]],
            &[[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 0, 0, 0], [9, 0, 0, 0], [10, 0, 0, 0]],
            &[
                [1.0, 0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0, 0.0],
            ],
            &[0, 1, 2],
        )
    }

    fn synthetic_compact_frame() -> Result<RecordedCompactHandJointFrame, MeshReplayError> {
        let mut joints = vec![([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]); 21];
        joints[8] = ([1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]);
        RecordedCompactHandJointFrame::from_makepad_openxr_compact_frame(
            true,
            7,
            2_000_000_000,
            &joints,
            [0.0, 0.25, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
        )
    }
}
