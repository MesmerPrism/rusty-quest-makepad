use rusty_matter_mesh::{
    HandJointFrame, HandJointPose, HandRigCapture, HandValidationMeshFrame, Handedness,
    TriangleMeshSurface, HAND_JOINT_FRAME_SCHEMA_ID,
};
use rusty_matter_model::Vec3;
use serde_json::Value;

use crate::MeshReplayError;

/// Recorder schema for Quest hand bind mesh captures.
pub const RECORDED_HAND_RIG_SCHEMA_ID: &str = "rusty.matter.hand_mesh_rig.v1";
/// Recorder schema for compact Quest hand joint frames.
pub const RECORDED_HAND_JOINT_FRAME_SCHEMA_ID: &str = "rusty.matter.hand_joint_frame.v1";
/// Recorder schema for baked hand validation mesh frames.
pub const RECORDED_HAND_VALIDATION_FRAME_SCHEMA_ID: &str = "rusty.matter.hand_validation_frame.v1";
/// Provider label emitted by the current Makepad/OpenXR compact hand path.
pub const RECORDED_HAND_COMPACT_21_PROVIDER: &str = "makepad-xr-hand-compact-21";

/// Recorded hand rig with bind mesh, bind-joint poses, and compact replay map.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedHandRig {
    /// Captured hand side.
    pub handedness: Handedness,
    /// Provider reference-space label.
    pub reference_space: String,
    /// Recorder topology key label.
    pub topology_key_label: String,
    /// Provider bind version.
    pub bind_version: u32,
    /// Compact runtime provider label.
    pub runtime_provider: String,
    /// Bind-pose triangle mesh.
    pub bind_surface: TriangleMeshSurface,
    /// Optional bind-pose normals.
    pub bind_normals: Vec<Vec3>,
    /// Bind joints in bind-joint index order.
    pub joints: Vec<RecordedHandBindJoint>,
    /// Skinning joint indices per vertex.
    pub vertex_blend_indices: Vec<[u16; 4]>,
    /// Skinning weights per vertex.
    pub vertex_blend_weights: Vec<[f32; 4]>,
    /// Source map from bind joints to compact runtime poses or tip lengths.
    pub bind_joint_sources: Vec<RecordedBindJointSource>,
}

impl RecordedHandRig {
    /// Parses one recorded `left.rig.json` or `right.rig.json` payload.
    ///
    /// # Errors
    ///
    /// Returns [`MeshReplayError`] when required recorder fields are missing or
    /// malformed.
    pub fn from_json_str(json: &str) -> Result<Self, MeshReplayError> {
        let value: Value =
            serde_json::from_str(json).map_err(|_| MeshReplayError::MalformedJson)?;
        if required_text(&value, "schema")? != RECORDED_HAND_RIG_SCHEMA_ID {
            return Err(MeshReplayError::UnexpectedSchema);
        }

        let handedness = parse_handedness(&required_text(&value, "handedness")?)?;
        let reference_space = required_text(&value, "reference_space")?;
        let topology_key_label = required_text(&value, "topology_key")?;
        let bind_version = required_u32(&value, "bind_version")?;
        let runtime_joint_set = value
            .get("runtime_joint_set")
            .ok_or(MeshReplayError::MissingField("runtime_joint_set"))?;
        let runtime_provider = required_text(runtime_joint_set, "provider")?;
        let joints = parse_bind_joints(array_field(&value, "joints")?)?;
        let bind_vertices = parse_vec3_array(array_field(&value, "bind_vertices")?)?;
        let bind_normals = parse_vec3_array(array_field(&value, "bind_normals")?)?;
        let triangles = parse_triangles(array_field(&value, "triangle_indices")?)?;
        let vertex_blend_indices =
            parse_blend_indices(array_field(&value, "vertex_blend_indices")?)?;
        let vertex_blend_weights =
            parse_blend_weights(array_field(&value, "vertex_blend_weights")?)?;
        let bind_joint_sources =
            parse_bind_joint_sources(array_field(runtime_joint_set, "bind_joint_sources")?)?;

        let bind_surface = TriangleMeshSurface::new(
            format!(
                "recorded.hand.bind.{}.v{}",
                handedness_label(handedness),
                bind_version
            ),
            bind_vertices,
            triangles,
        );

        let rig = Self {
            handedness,
            reference_space,
            topology_key_label,
            bind_version,
            runtime_provider,
            bind_surface,
            bind_normals,
            joints,
            vertex_blend_indices,
            vertex_blend_weights,
            bind_joint_sources,
        };
        rig.validate()?;
        Ok(rig)
    }

    /// Validates that the recorded rig has the shape needed for replay.
    ///
    /// # Errors
    ///
    /// Returns [`MeshReplayError`] when the rig is incomplete.
    pub fn validate(&self) -> Result<(), MeshReplayError> {
        if self.runtime_provider != RECORDED_HAND_COMPACT_21_PROVIDER {
            return Err(MeshReplayError::InvalidValue("runtime_joint_set.provider"));
        }
        if self.joints.is_empty() {
            return Err(MeshReplayError::InvalidValue("joints"));
        }
        if self.bind_normals.len() != self.bind_surface.vertex_count() {
            return Err(MeshReplayError::InvalidValue("bind_normals"));
        }
        if self.vertex_blend_indices.len() != self.bind_surface.vertex_count()
            || self.vertex_blend_weights.len() != self.bind_surface.vertex_count()
        {
            return Err(MeshReplayError::InvalidValue("vertex_blend_metadata"));
        }
        if self.bind_joint_sources.len() != self.joints.len() {
            return Err(MeshReplayError::InvalidValue("bind_joint_sources"));
        }
        Ok(())
    }

    /// Converts the recorded bind data into Matter's neutral rig capture.
    ///
    /// # Errors
    ///
    /// Returns [`MeshReplayError`] when the rig cannot be represented by
    /// Matter's CPU oracle contract.
    pub fn to_matter_rig_capture(
        &self,
        rig_capture_id: impl Into<String>,
    ) -> Result<HandRigCapture, MeshReplayError> {
        self.validate()?;
        let mut rig = HandRigCapture::from_bind_surface(
            rig_capture_id,
            self.handedness,
            self.reference_space.clone(),
            self.runtime_provider.clone(),
            self.bind_surface.clone(),
        );
        rig.bind_normals = self.bind_normals.clone();
        rig.joint_parent_indices = self
            .joints
            .iter()
            .map(|joint| match joint.parent_index {
                Some(parent) => i16::try_from(parent).unwrap_or(-1),
                None => -1,
            })
            .collect();
        rig.joint_radii_m = self.joints.iter().map(|joint| joint.radius_m).collect();
        rig.joint_bind_poses = self
            .joints
            .iter()
            .map(|joint| joint.bind_pose.to_matter_pose(joint.radius_m))
            .collect();
        rig.vertex_joint_indices = self.vertex_blend_indices.clone();
        rig.vertex_joint_weights = self.vertex_blend_weights.clone();
        rig.validate()
            .map_err(|_| MeshReplayError::InvalidValue("matter_rig_capture"))?;
        Ok(rig)
    }
}

/// One bind joint from a recorded hand rig.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedHandBindJoint {
    /// Bind-joint index.
    pub index: usize,
    /// Provider joint name.
    pub name: String,
    /// Optional parent bind-joint index.
    pub parent_index: Option<usize>,
    /// Joint radius in meters.
    pub radius_m: f32,
    /// Bind-pose transform.
    pub bind_pose: RecordedHandPose,
}

/// Compact recorded hand joint frame.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedCompactHandJointFrame {
    /// Captured hand side.
    pub handedness: Handedness,
    /// Provider frame index.
    pub frame_index: usize,
    /// Provider timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Runtime-joint poses in compact Makepad order.
    pub joints: Vec<RecordedRuntimeJointPose>,
    /// Five fingertip lengths in meters.
    pub tip_lengths_m: Vec<f32>,
    /// Four pinch strengths.
    pub pinch_strengths: Vec<f32>,
}

impl RecordedCompactHandJointFrame {
    /// Parses one recorded `*.clip.jsonl` row.
    ///
    /// # Errors
    ///
    /// Returns [`MeshReplayError`] when the row is malformed.
    pub fn from_json_line(line: &str) -> Result<Self, MeshReplayError> {
        let value: Value =
            serde_json::from_str(line).map_err(|_| MeshReplayError::MalformedJson)?;
        if required_text(&value, "schema")? != RECORDED_HAND_JOINT_FRAME_SCHEMA_ID {
            return Err(MeshReplayError::UnexpectedSchema);
        }
        let handedness = parse_handedness(&required_text(&value, "handedness")?)?;
        let frame_index = required_usize(&value, "frame_index")?;
        let timestamp_ns = required_u64(&value, "timestamp_ns")?;
        let joints = parse_runtime_joints(array_field(&value, "joints")?)?;
        let tip_lengths_m = parse_f32_array(array_field(&value, "tip_lengths_m")?)?;
        let pinch_strengths = parse_f32_array(array_field(&value, "pinch_strengths")?)?;
        Ok(Self {
            handedness,
            frame_index,
            timestamp_ns,
            joints,
            tip_lengths_m,
            pinch_strengths,
        })
    }

    /// Expands compact runtime poses into Matter's full bind-joint frame.
    ///
    /// Tip joints are reconstructed the same way as Makepad `XrHand`: the tip
    /// position is the parent distal joint transformed by `(0, 0, -length)`,
    /// and the tip orientation is the parent distal orientation.
    ///
    /// # Errors
    ///
    /// Returns [`MeshReplayError`] when the compact frame does not satisfy the
    /// rig's bind-joint source map.
    pub fn expand_to_matter_joint_frame(
        &self,
        rig: &RecordedHandRig,
        frame_id: impl Into<String>,
    ) -> Result<HandJointFrame, MeshReplayError> {
        if self.handedness != rig.handedness {
            return Err(MeshReplayError::InvalidValue("handedness"));
        }
        let mut poses = vec![None; rig.joints.len()];
        for source in &rig.bind_joint_sources {
            let pose = match &source.source {
                RecordedBindJointSourceKind::RuntimePose {
                    runtime_joint_index,
                } => self
                    .runtime_joint(*runtime_joint_index)
                    .ok_or(MeshReplayError::InvalidValue("runtime_joint_index"))?
                    .pose
                    .to_matter_pose(
                        rig.joints
                            .get(source.bind_joint_index)
                            .map_or(0.0, |joint| joint.radius_m),
                    ),
                RecordedBindJointSourceKind::TipLengthFromParentPose {
                    tip_length_index,
                    parent_runtime_joint_index,
                } => {
                    let parent = self
                        .runtime_joint(*parent_runtime_joint_index)
                        .ok_or(MeshReplayError::InvalidValue("parent_runtime_joint_index"))?;
                    let length = self
                        .tip_lengths_m
                        .get(*tip_length_index)
                        .copied()
                        .filter(|value| value.is_finite() && *value >= 0.0)
                        .ok_or(MeshReplayError::InvalidValue("tip_lengths_m"))?;
                    let offset =
                        rotate_by_quat(parent.pose.rotation_xyzw, Vec3::new(0.0, 0.0, -length))?;
                    HandJointPose {
                        position: parent.pose.translation + offset,
                        orientation_xyzw: parent.pose.rotation_xyzw,
                        radius_m: rig
                            .joints
                            .get(source.bind_joint_index)
                            .map_or(parent.radius_m, |joint| joint.radius_m),
                    }
                }
            };
            let slot = poses
                .get_mut(source.bind_joint_index)
                .ok_or(MeshReplayError::InvalidValue("bind_joint_index"))?;
            *slot = Some(pose);
        }
        let poses = poses
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(MeshReplayError::InvalidValue("bind_joint_sources"))?;

        let frame = HandJointFrame {
            schema_id: HAND_JOINT_FRAME_SCHEMA_ID.to_owned(),
            frame_id: frame_id.into(),
            handedness: self.handedness,
            reference_space: rig.reference_space.clone(),
            source: rig.runtime_provider.clone(),
            time_seconds: self.timestamp_ns as f32 * 1.0e-9,
            poses,
            confidence: vec![1.0; rig.joints.len()],
        };
        frame
            .validate()
            .map_err(|_| MeshReplayError::InvalidValue("matter_joint_frame"))?;
        Ok(frame)
    }

    fn runtime_joint(&self, runtime_joint_index: usize) -> Option<&RecordedRuntimeJointPose> {
        self.joints
            .iter()
            .find(|joint| joint.joint_index == runtime_joint_index)
    }
}

/// One compact runtime-joint pose from a recorded frame.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedRuntimeJointPose {
    /// Compact runtime joint index.
    pub joint_index: usize,
    /// Runtime pose.
    pub pose: RecordedHandPose,
    /// Runtime radius in meters.
    pub radius_m: f32,
}

/// One baked validation mesh frame from the recorder.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedHandValidationMeshFrame {
    /// Captured hand side.
    pub handedness: Handedness,
    /// Provider frame index.
    pub frame_index: usize,
    /// Provider timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Recorder topology key label.
    pub topology_key_label: String,
    /// Baked deformed vertices.
    pub vertices: Vec<Vec3>,
    /// Baked deformed normals.
    pub normals: Vec<Vec3>,
}

impl RecordedHandValidationMeshFrame {
    /// Parses one recorded `*.validation_mesh.jsonl` row.
    ///
    /// # Errors
    ///
    /// Returns [`MeshReplayError`] when the row is malformed.
    pub fn from_json_line(line: &str) -> Result<Self, MeshReplayError> {
        let value: Value =
            serde_json::from_str(line).map_err(|_| MeshReplayError::MalformedJson)?;
        if required_text(&value, "schema")? != RECORDED_HAND_VALIDATION_FRAME_SCHEMA_ID {
            return Err(MeshReplayError::UnexpectedSchema);
        }
        Ok(Self {
            handedness: parse_handedness(&required_text(&value, "handedness")?)?,
            frame_index: required_usize(&value, "frame_index")?,
            timestamp_ns: required_u64(&value, "timestamp_ns")?,
            topology_key_label: required_text(&value, "topology_key")?,
            vertices: parse_vec3_array(array_field(&value, "vertices")?)?,
            normals: parse_vec3_array(array_field(&value, "normals")?)?,
        })
    }

    /// Converts a recorded validation frame into Matter's neutral frame shape.
    ///
    /// # Errors
    ///
    /// Returns [`MeshReplayError`] when the validation mesh does not match the
    /// rig topology.
    pub fn to_matter_validation_frame(
        &self,
        rig: &RecordedHandRig,
        frame_id: impl Into<String>,
    ) -> Result<HandValidationMeshFrame, MeshReplayError> {
        if self.handedness != rig.handedness || self.topology_key_label != rig.topology_key_label {
            return Err(MeshReplayError::InvalidValue("validation topology"));
        }
        if self.vertices.len() != rig.bind_surface.vertex_count()
            || self.normals.len() != rig.bind_surface.vertex_count()
        {
            return Err(MeshReplayError::InvalidValue(
                "validation mesh vertex count",
            ));
        }
        let surface = TriangleMeshSurface::new(
            format!("recorded.hand.validation.frame.{}", self.frame_index),
            self.vertices.clone(),
            rig.bind_surface.triangles.clone(),
        );
        let mut frame = HandValidationMeshFrame::from_surface(
            frame_id,
            self.handedness,
            rig.reference_space.clone(),
            rig.runtime_provider.clone(),
            self.timestamp_ns as f32 * 1.0e-9,
            surface,
        );
        frame.normals = self.normals.clone();
        frame
            .validate()
            .map_err(|_| MeshReplayError::InvalidValue("matter_validation_frame"))?;
        Ok(frame)
    }
}

/// One recorded pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecordedHandPose {
    /// Translation.
    pub translation: Vec3,
    /// Orientation as `[x, y, z, w]`.
    pub rotation_xyzw: [f32; 4],
}

impl RecordedHandPose {
    fn to_matter_pose(self, radius_m: f32) -> HandJointPose {
        HandJointPose {
            position: self.translation,
            orientation_xyzw: self.rotation_xyzw,
            radius_m,
        }
    }
}

/// How a bind joint is reconstructed from the compact runtime packet.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedBindJointSource {
    /// Bind-joint index populated by this source.
    pub bind_joint_index: usize,
    /// Reconstruction source.
    pub source: RecordedBindJointSourceKind,
}

/// Source kind for reconstructing a bind joint.
#[derive(Clone, Debug, PartialEq)]
pub enum RecordedBindJointSourceKind {
    /// Copy a compact runtime joint pose.
    RuntimePose {
        /// Compact runtime joint index.
        runtime_joint_index: usize,
    },
    /// Reconstruct a tip pose from a parent runtime pose and stored tip length.
    TipLengthFromParentPose {
        /// Tip length index in `tip_lengths_m`.
        tip_length_index: usize,
        /// Parent compact runtime joint index.
        parent_runtime_joint_index: usize,
    },
}

fn parse_bind_joints(values: &[Value]) -> Result<Vec<RecordedHandBindJoint>, MeshReplayError> {
    let mut joints = Vec::with_capacity(values.len());
    for value in values {
        let index = required_usize(value, "index")?;
        let parent_raw = required_u64(value, "parent_index")?;
        let parent_index = if parent_raw == u64::from(u16::MAX) {
            None
        } else {
            Some(
                usize::try_from(parent_raw)
                    .map_err(|_| MeshReplayError::InvalidValue("parent_index"))?,
            )
        };
        joints.push(RecordedHandBindJoint {
            index,
            name: required_text(value, "name")?,
            parent_index,
            radius_m: required_f32(value, "radius_m")?,
            bind_pose: parse_pose(
                value
                    .get("bind_pose")
                    .ok_or(MeshReplayError::MissingField("bind_pose"))?,
            )?,
        });
    }
    for (expected, joint) in joints.iter().enumerate() {
        if joint.index != expected {
            return Err(MeshReplayError::InvalidValue("joint.index"));
        }
    }
    Ok(joints)
}

fn parse_runtime_joints(
    values: &[Value],
) -> Result<Vec<RecordedRuntimeJointPose>, MeshReplayError> {
    let mut joints = Vec::with_capacity(values.len());
    for value in values {
        joints.push(RecordedRuntimeJointPose {
            joint_index: required_usize(value, "joint_index")?,
            pose: parse_pose(
                value
                    .get("pose")
                    .ok_or(MeshReplayError::MissingField("pose"))?,
            )?,
            radius_m: required_f32(value, "radius_m")?,
        });
    }
    Ok(joints)
}

fn parse_bind_joint_sources(
    values: &[Value],
) -> Result<Vec<RecordedBindJointSource>, MeshReplayError> {
    let mut sources = Vec::with_capacity(values.len());
    for value in values {
        let bind_joint_index = required_usize(value, "bind_joint_index")?;
        let source_kind = required_text(value, "source_kind")?;
        let source = match source_kind.as_str() {
            "runtime_pose" => RecordedBindJointSourceKind::RuntimePose {
                runtime_joint_index: required_usize(value, "runtime_joint_index")?,
            },
            "tip_length_from_parent_pose" => RecordedBindJointSourceKind::TipLengthFromParentPose {
                tip_length_index: required_usize(value, "tip_length_index")?,
                parent_runtime_joint_index: required_usize(value, "parent_runtime_joint_index")?,
            },
            _ => return Err(MeshReplayError::InvalidValue("source_kind")),
        };
        sources.push(RecordedBindJointSource {
            bind_joint_index,
            source,
        });
    }
    for (expected, source) in sources.iter().enumerate() {
        if source.bind_joint_index != expected {
            return Err(MeshReplayError::InvalidValue("bind_joint_index"));
        }
    }
    Ok(sources)
}

fn parse_pose(value: &Value) -> Result<RecordedHandPose, MeshReplayError> {
    let translation = parse_vec3_value(
        value
            .get("translation")
            .ok_or(MeshReplayError::MissingField("translation"))?,
    )?;
    let rotation_values = array_field(value, "rotation")?;
    if rotation_values.len() != 4 {
        return Err(MeshReplayError::InvalidValue("rotation"));
    }
    let rotation_xyzw = [
        parse_f32_value(&rotation_values[0], "rotation.x")?,
        parse_f32_value(&rotation_values[1], "rotation.y")?,
        parse_f32_value(&rotation_values[2], "rotation.z")?,
        parse_f32_value(&rotation_values[3], "rotation.w")?,
    ];
    Ok(RecordedHandPose {
        translation,
        rotation_xyzw,
    })
}

fn parse_triangles(values: &[Value]) -> Result<Vec<[u32; 3]>, MeshReplayError> {
    values
        .iter()
        .map(|value| {
            let indices = value
                .as_array()
                .ok_or(MeshReplayError::InvalidValue("triangle_indices"))?;
            if indices.len() != 3 {
                return Err(MeshReplayError::InvalidValue("triangle_indices"));
            }
            Ok([
                parse_u32_value(&indices[0], "triangle_indices")?,
                parse_u32_value(&indices[1], "triangle_indices")?,
                parse_u32_value(&indices[2], "triangle_indices")?,
            ])
        })
        .collect()
}

fn parse_vec3_array(values: &[Value]) -> Result<Vec<Vec3>, MeshReplayError> {
    values.iter().map(parse_vec3_value).collect()
}

fn parse_vec3_value(value: &Value) -> Result<Vec3, MeshReplayError> {
    let values = value
        .as_array()
        .ok_or(MeshReplayError::InvalidValue("vec3"))?;
    if values.len() != 3 {
        return Err(MeshReplayError::InvalidValue("vec3"));
    }
    Ok(Vec3::new(
        parse_f32_value(&values[0], "vec3.x")?,
        parse_f32_value(&values[1], "vec3.y")?,
        parse_f32_value(&values[2], "vec3.z")?,
    ))
}

fn parse_blend_indices(values: &[Value]) -> Result<Vec<[u16; 4]>, MeshReplayError> {
    values
        .iter()
        .map(|value| {
            let indices = value
                .as_array()
                .ok_or(MeshReplayError::InvalidValue("vertex_blend_indices"))?;
            if indices.len() != 4 {
                return Err(MeshReplayError::InvalidValue("vertex_blend_indices"));
            }
            Ok([
                parse_u16_value(&indices[0], "vertex_blend_indices")?,
                parse_u16_value(&indices[1], "vertex_blend_indices")?,
                parse_u16_value(&indices[2], "vertex_blend_indices")?,
                parse_u16_value(&indices[3], "vertex_blend_indices")?,
            ])
        })
        .collect()
}

fn parse_blend_weights(values: &[Value]) -> Result<Vec<[f32; 4]>, MeshReplayError> {
    values
        .iter()
        .map(|value| {
            let weights = value
                .as_array()
                .ok_or(MeshReplayError::InvalidValue("vertex_blend_weights"))?;
            if weights.len() != 4 {
                return Err(MeshReplayError::InvalidValue("vertex_blend_weights"));
            }
            Ok([
                parse_f32_value(&weights[0], "vertex_blend_weights")?,
                parse_f32_value(&weights[1], "vertex_blend_weights")?,
                parse_f32_value(&weights[2], "vertex_blend_weights")?,
                parse_f32_value(&weights[3], "vertex_blend_weights")?,
            ])
        })
        .collect()
}

fn parse_f32_array(values: &[Value]) -> Result<Vec<f32>, MeshReplayError> {
    values
        .iter()
        .map(|value| parse_f32_value(value, "f32_array"))
        .collect()
}

fn parse_handedness(value: &str) -> Result<Handedness, MeshReplayError> {
    match value {
        "left" => Ok(Handedness::Left),
        "right" => Ok(Handedness::Right),
        "unknown" => Ok(Handedness::Unknown),
        _ => Err(MeshReplayError::InvalidValue("handedness")),
    }
}

fn handedness_label(handedness: Handedness) -> &'static str {
    match handedness {
        Handedness::Unknown => "unknown",
        Handedness::Left => "left",
        Handedness::Right => "right",
    }
}

fn array_field<'a>(value: &'a Value, field: &'static str) -> Result<&'a [Value], MeshReplayError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or(MeshReplayError::MissingField(field))
}

fn required_text(value: &Value, field: &'static str) -> Result<String, MeshReplayError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or(MeshReplayError::MissingField(field))
}

fn required_f32(value: &Value, field: &'static str) -> Result<f32, MeshReplayError> {
    parse_f32_value(
        value
            .get(field)
            .ok_or(MeshReplayError::MissingField(field))?,
        field,
    )
}

fn required_u32(value: &Value, field: &'static str) -> Result<u32, MeshReplayError> {
    parse_u32_value(
        value
            .get(field)
            .ok_or(MeshReplayError::MissingField(field))?,
        field,
    )
}

fn required_usize(value: &Value, field: &'static str) -> Result<usize, MeshReplayError> {
    let raw = required_u64(value, field)?;
    usize::try_from(raw).map_err(|_| MeshReplayError::InvalidValue(field))
}

fn required_u64(value: &Value, field: &'static str) -> Result<u64, MeshReplayError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or(MeshReplayError::InvalidValue(field))
}

fn parse_f32_value(value: &Value, field: &'static str) -> Result<f32, MeshReplayError> {
    value
        .as_f64()
        .filter(|number| number.is_finite())
        .map(|number| number as f32)
        .ok_or(MeshReplayError::InvalidValue(field))
}

fn parse_u16_value(value: &Value, field: &'static str) -> Result<u16, MeshReplayError> {
    let raw = value.as_u64().ok_or(MeshReplayError::InvalidValue(field))?;
    u16::try_from(raw).map_err(|_| MeshReplayError::InvalidValue(field))
}

fn parse_u32_value(value: &Value, field: &'static str) -> Result<u32, MeshReplayError> {
    let raw = value.as_u64().ok_or(MeshReplayError::InvalidValue(field))?;
    u32::try_from(raw).map_err(|_| MeshReplayError::InvalidValue(field))
}

fn rotate_by_quat(quat: [f32; 4], vector: Vec3) -> Result<Vec3, MeshReplayError> {
    let [x, y, z, w] = normalize_quat(quat)?;
    let q_vec = Vec3::new(x, y, z);
    let uv = q_vec.cross(vector);
    let uuv = q_vec.cross(uv);
    Ok(vector + uv * (2.0 * w) + uuv * 2.0)
}

fn normalize_quat(quat: [f32; 4]) -> Result<[f32; 4], MeshReplayError> {
    let length_squared: f32 = quat.iter().map(|value| *value * *value).sum();
    if !length_squared.is_finite() || length_squared <= 1.0e-12 {
        return Err(MeshReplayError::InvalidValue("rotation"));
    }
    let scale = 1.0 / length_squared.sqrt();
    Ok([
        quat[0] * scale,
        quat[1] * scale,
        quat[2] * scale,
        quat[3] * scale,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_matter_mesh::HandValidationMeshTolerance;

    #[test]
    fn recorded_hand_capture_expands_compact_frame_for_matter_oracle() {
        let rig = RecordedHandRig::from_json_str(SYNTHETIC_RIG).expect("rig parses");
        let compact =
            RecordedCompactHandJointFrame::from_json_line(SYNTHETIC_FRAME).expect("frame parses");
        let validation = RecordedHandValidationMeshFrame::from_json_line(SYNTHETIC_VALIDATION)
            .expect("validation parses");

        let matter_rig = rig
            .to_matter_rig_capture("hand.rig_capture.recorded.synthetic")
            .expect("matter rig builds");
        let joint_frame = compact
            .expand_to_matter_joint_frame(&rig, "hand.joint_frame.recorded.synthetic.0001")
            .expect("compact frame expands");
        let actual = matter_rig
            .skin_to_validation_frame(&joint_frame, "hand.validation_mesh.recorded.actual.0001")
            .expect("matter skins");
        let expected = validation
            .to_matter_validation_frame(&rig, "hand.validation_mesh.recorded.expected.0001")
            .expect("validation converts");
        let comparison = expected
            .compare_with(&actual, HandValidationMeshTolerance::default())
            .expect("comparison builds");

        assert_eq!(rig.bind_surface.vertex_count(), 3);
        assert_eq!(joint_frame.poses.len(), 3);
        assert_eq!(joint_frame.poses[2].position, Vec3::new(1.0, 0.0, -0.25));
        assert!(comparison.passed, "{comparison:?}");
    }

    #[test]
    #[ignore = "requires RUSTY_QUEST_MAKEPAD_RECORDED_HAND_CAPTURE_DIR"]
    fn external_recorded_hand_capture_matches_validation_frame_when_configured() {
        let root = std::env::var("RUSTY_QUEST_MAKEPAD_RECORDED_HAND_CAPTURE_DIR")
            .expect("set recorded hand capture directory");
        let root = std::path::PathBuf::from(root);
        let rig_json = std::fs::read_to_string(root.join("left.rig.json")).expect("read rig");
        let validation_line = std::fs::read_to_string(root.join("left.validation_mesh.jsonl"))
            .expect("read validation mesh")
            .lines()
            .next()
            .expect("validation line exists")
            .to_owned();
        let rig = RecordedHandRig::from_json_str(&rig_json).expect("rig parses");
        let validation =
            RecordedHandValidationMeshFrame::from_json_line(&validation_line).expect("validation");
        let clip_text = std::fs::read_to_string(root.join("left.clip.jsonl")).expect("read clip");
        let compact = clip_text
            .lines()
            .filter_map(|line| RecordedCompactHandJointFrame::from_json_line(line).ok())
            .find(|frame| frame.frame_index == validation.frame_index)
            .expect("matching compact frame exists");

        let matter_rig = rig
            .to_matter_rig_capture("hand.rig_capture.recorded.external.left")
            .expect("matter rig builds");
        let joint_frame = compact
            .expand_to_matter_joint_frame(&rig, "hand.joint_frame.recorded.external.left")
            .expect("compact frame expands");
        let actual = matter_rig
            .skin_to_validation_frame(
                &joint_frame,
                "hand.validation_mesh.recorded.external.actual",
            )
            .expect("matter skins external frame");
        let expected = validation
            .to_matter_validation_frame(&rig, "hand.validation_mesh.recorded.external.expected")
            .expect("validation converts");
        let comparison = expected
            .compare_with(&actual, HandValidationMeshTolerance::default())
            .expect("comparison builds");

        assert!(comparison.passed, "{comparison:?}");
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

    const SYNTHETIC_VALIDATION: &str = r#"{
      "schema": "rusty.matter.hand_validation_frame.v1",
      "handedness": "left",
      "frame_index": 7,
      "timestamp_ns": 2000000000,
      "topology_key": "synthetic-openxr-handmesh-j3-v3-i3",
      "vertices": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, -0.25]],
      "normals": [[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]]
    }"#;
}
