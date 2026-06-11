use rusty_matter_mesh::{
    HandSkinningMatrixSample, HandSkinningMeshBufferOracle, HAND_SKINNING_MATRIX_INFLUENCE_COUNT,
};

use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token},
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_BACKEND,
    QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_PAYLOAD, QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_SCHEMA_ID,
};

/// Number of compact sample positions echoed in the full mesh residency marker.
pub const QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_SAMPLES: usize = 4;
/// Conservative f32 tolerance for full mesh skinning readback comparison.
pub const QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_DEFAULT_TOLERANCE: f32 = 0.0001;

/// One full-buffer GPU skinning vertex row.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuSkinningMeshVertex {
    /// Source bind-mesh vertex index.
    pub vertex_index: usize,
    /// Bind-pose vertex position, packed as `[x, y, z, 1]`.
    pub bind_position: [f32; 4],
    /// Influencing joint indices in the source rig.
    pub joint_indices: [u16; HAND_SKINNING_MATRIX_INFLUENCE_COUNT],
    /// Skinning weights for each influence slot.
    pub joint_weights: [f32; HAND_SKINNING_MATRIX_INFLUENCE_COUNT],
    /// Row-major bind-pose-to-current-joint matrices.
    pub joint_matrices: [[[f32; 4]; 4]; HAND_SKINNING_MATRIX_INFLUENCE_COUNT],
    /// Matter CPU-skinned oracle position, packed as `[x, y, z, 1]`.
    pub expected_position: [f32; 4],
}

/// Full recorded-hand mesh-buffer GPU skinning probe input.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuSkinningMeshProbeInput {
    /// Stable source identity.
    pub source_id: String,
    /// Recorded/live-equivalent source frame index.
    pub source_frame_index: usize,
    /// Full topology vertex count.
    pub topology_vertex_count: usize,
    /// Full topology triangle count.
    pub topology_triangle_count: usize,
    /// Full flattened triangle index count.
    pub topology_index_count: usize,
    /// One row per bind vertex.
    pub vertices: Vec<QuestMakepadGpuSkinningMeshVertex>,
    /// Full triangle index buffer.
    pub triangles: Vec<[u32; 3]>,
    /// Compact sample vertices echoed in markers and readback summaries.
    pub sample_vertex_indices: [usize; QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_SAMPLES],
    /// Number of populated sample indices.
    pub sample_count: usize,
}

impl QuestMakepadGpuSkinningMeshProbeInput {
    /// Builds full mesh skinning probe input from the Matter-owned CPU oracle.
    #[must_use]
    pub fn from_matter_oracle(
        source_id: impl Into<String>,
        source_frame_index: usize,
        oracle: &HandSkinningMeshBufferOracle,
    ) -> Option<Self> {
        if oracle.vertex_count() == 0 || oracle.triangle_count() == 0 {
            return None;
        }
        if !oracle
            .vertices
            .iter()
            .copied()
            .enumerate()
            .all(|(index, sample)| matter_sample_is_valid(index, sample, oracle.vertex_count()))
        {
            return None;
        }
        if !triangles_are_valid(&oracle.triangles, oracle.vertex_count()) {
            return None;
        }

        let vertices = oracle
            .vertices
            .iter()
            .copied()
            .map(QuestMakepadGpuSkinningMeshVertex::from)
            .collect::<Vec<_>>();
        let sample_count = oracle
            .vertex_count()
            .min(QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_SAMPLES);
        let mut sample_vertex_indices = [0; QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_SAMPLES];
        for (sample_index, target) in sample_vertex_indices
            .iter_mut()
            .take(sample_count)
            .enumerate()
        {
            *target = selected_vertex_index(oracle.vertex_count(), sample_count, sample_index);
        }

        Some(Self {
            source_id: source_id.into(),
            source_frame_index,
            topology_vertex_count: oracle.vertex_count(),
            topology_triangle_count: oracle.triangle_count(),
            topology_index_count: oracle.index_count(),
            vertices,
            triangles: oracle.triangles.clone(),
            sample_vertex_indices,
            sample_count,
        })
    }

    /// First populated sample vertex index.
    #[must_use]
    pub fn first_sample_vertex_index(&self) -> Option<usize> {
        (self.sample_count > 0).then_some(self.sample_vertex_indices[0])
    }

    /// Last populated sample vertex index.
    #[must_use]
    pub fn last_sample_vertex_index(&self) -> Option<usize> {
        self.sample_count
            .checked_sub(1)
            .map(|index| self.sample_vertex_indices[index])
    }
}

impl From<HandSkinningMatrixSample> for QuestMakepadGpuSkinningMeshVertex {
    fn from(source: HandSkinningMatrixSample) -> Self {
        Self {
            vertex_index: source.vertex_index,
            bind_position: source.bind_position,
            joint_indices: source.joint_indices,
            joint_weights: source.joint_weights,
            joint_matrices: source.joint_matrices,
            expected_position: source.expected_position,
        }
    }
}

/// Makepad GPU full mesh skinning readback summary.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuSkinningMeshProbeReadback {
    /// Number of vertices skinned by the GPU command.
    pub vertex_count: usize,
    /// Number of triangles read by the GPU command.
    pub triangle_count: usize,
    /// Number of flattened triangle indices read by the GPU command.
    pub index_count: usize,
    /// Number of checked f32 position components.
    pub checked_position_components: usize,
    /// Number of output position components outside tolerance.
    pub mismatched_position_components: usize,
    /// Number of triangle-index observations that failed CPU comparison.
    pub mismatched_triangle_indices: usize,
    /// Maximum absolute output-vs-CPU-oracle position error.
    pub max_abs_error: f32,
    /// Absolute tolerance used by the full-buffer readback comparison.
    pub tolerance: f32,
    /// Number of compact sample positions copied into this summary.
    pub sample_count: usize,
    /// Sample vertex indices included in the compact summary.
    pub sample_vertex_indices: [usize; QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_SAMPLES],
    /// Makepad XR/Vulkan submit serial for the proof command.
    pub queue_submit_serial: u64,
    /// Fence serial observed for the proof command.
    pub fence_serial: u64,
    /// Monotonic proof-resource generation for the current renderer lifetime.
    pub resource_generation: u64,
    /// Proof resources still pending retirement.
    pub pending_retire_count: usize,
    /// Proof resources retained by the current Makepad backend.
    pub retained_resource_count: usize,
    /// Proof resources destroyed after fence evidence in this call.
    pub retired_after_fence_count: usize,
    /// True when the Makepad backend waited for queue idle after the proof.
    pub queue_wait_idle_performed: bool,
    /// CPU-side elapsed time for shader compilation, command submission, wait, and readback.
    pub elapsed_ms: f64,
}

impl QuestMakepadGpuSkinningMeshProbeReadback {
    /// True when the full GPU output matched the Matter CPU-oracle skinning positions and indices.
    #[must_use]
    pub fn readback_matched(self) -> bool {
        self.vertex_count > 0
            && self.triangle_count > 0
            && self.index_count == self.triangle_count * 3
            && self.checked_position_components == self.vertex_count * 3
            && self.mismatched_position_components == 0
            && self.mismatched_triangle_indices == 0
            && self.max_abs_error.is_finite()
            && self.tolerance.is_finite()
            && self.max_abs_error <= self.tolerance.max(0.0)
    }
}

/// Full recorded-hand skinned vertex/index buffer GPU residency proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuSkinningMeshProbe {
    /// Schema identifier.
    pub schema_id: String,
    /// Full source-frame skinning input.
    pub input: QuestMakepadGpuSkinningMeshProbeInput,
    /// Makepad full-buffer readback result.
    pub readback: QuestMakepadGpuSkinningMeshProbeReadback,
}

impl QuestMakepadGpuSkinningMeshProbe {
    /// Builds a full mesh skinning marker from recorded-hand source-frame input.
    #[must_use]
    pub fn from_input(
        input: &QuestMakepadGpuSkinningMeshProbeInput,
        readback: QuestMakepadGpuSkinningMeshProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_SCHEMA_ID.to_owned(),
            input: input.clone(),
            readback,
        }
    }

    /// Builds a compact marker without logging bind meshes, joint frames, or skinned surfaces.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} proofKind=full-recorded-hand-skinning-mesh-residency computeStage=hand-skinning-full-vertex-buffer sourceId={} sourceFrameIndex={} topologyVertexCount={} topologyTriangleCount={} topologyIndexCount={} cpuOracle=matter-recorded-hand-skinning cpuOraclePreserved=true recordedInputEquivalent=true validationInputShape=bind-mesh-plus-compact-joint-frame resourcePlane={} computeProbeBackend={} oraclePayload={} vertexCount={} triangleCount={} indexCount={} sampleCount={} firstSampleVertexIndex={} lastSampleVertexIndex={} influenceSlotsPerVertex={} matrixRowsPerInfluence=4 checkedPositionComponents={} mismatchedPositionComponents={} mismatchedTriangleIndices={} maxAbsError={} tolerance={} readbackMatched={} commandEncoderSubmitted=true fullVertexBufferResident=true fullIndexBufferResident=true skinnedVertexBufferResident=true indexBufferConsumedByGpu=true fullBufferGpuResidency=true computeDispatchSubmitted=true boundedSampleOnly=false prototypeComputeKernel=false weightedDeltaSkinningKernel=false jointMatrixSkinningKernel=true meshToSdfKernel=false fieldSamplingKernel=false fieldParticleKernel=false computeKernel=true gpuComputeReady=false highRateJsonPayload=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            if self.readback.readback_matched() {
                "ready"
            } else {
                "mismatch"
            },
            sanitize_marker_value(&self.input.source_id),
            self.input.source_frame_index,
            self.input.topology_vertex_count,
            self.input.topology_triangle_count,
            self.input.topology_index_count,
            QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_BACKEND,
            QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_PAYLOAD,
            self.readback.vertex_count,
            self.readback.triangle_count,
            self.readback.index_count,
            self.readback.sample_count,
            optional_usize_marker_token(self.input.first_sample_vertex_index()),
            optional_usize_marker_token(self.input.last_sample_vertex_index()),
            HAND_SKINNING_MATRIX_INFLUENCE_COUNT,
            self.readback.checked_position_components,
            self.readback.mismatched_position_components,
            self.readback.mismatched_triangle_indices,
            finite_f32_marker_token(self.readback.max_abs_error),
            finite_f32_marker_token(self.readback.tolerance),
            self.readback.readback_matched(),
            self.readback.queue_submit_serial,
            self.readback.fence_serial,
            self.readback.resource_generation,
            self.readback.pending_retire_count,
            self.readback.retained_resource_count,
            self.readback.retired_after_fence_count,
            self.readback.queue_wait_idle_performed,
            finite_f64_marker_token(self.readback.elapsed_ms),
            QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_MEASUREMENT_SOURCE,
        )
    }
}

fn matter_sample_is_valid(
    expected_vertex_index: usize,
    sample: HandSkinningMatrixSample,
    topology_vertex_count: usize,
) -> bool {
    sample.vertex_index == expected_vertex_index
        && sample.vertex_index < topology_vertex_count
        && finite_vec4(sample.bind_position)
        && finite_vec4(sample.expected_position)
        && sample
            .joint_weights
            .iter()
            .all(|weight| weight.is_finite() && *weight >= 0.0)
        && sample
            .joint_matrices
            .iter()
            .all(|matrix| matrix.iter().copied().all(finite_vec4))
}

fn triangles_are_valid(triangles: &[[u32; 3]], vertex_count: usize) -> bool {
    !triangles.is_empty()
        && triangles.iter().copied().all(|triangle| {
            let [a, b, c] = triangle;
            a != b
                && b != c
                && a != c
                && [a, b, c]
                    .iter()
                    .all(|index| usize::try_from(*index).is_ok_and(|index| index < vertex_count))
        })
}

fn selected_vertex_index(vertex_count: usize, sample_count: usize, sample_index: usize) -> usize {
    if sample_count <= 1 {
        0
    } else {
        sample_index * (vertex_count - 1) / (sample_count - 1)
    }
}

fn finite_vec4(row: [f32; 4]) -> bool {
    row.iter().all(|value| value.is_finite())
}

fn optional_usize_marker_token(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}
