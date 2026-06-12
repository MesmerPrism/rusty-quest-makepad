use rusty_matter_model::{TriangleMeshSnapshot, Vec3};
use rusty_matter_sdf::{build_sdf_from_mesh, MeshSdfSignMode, MeshToSdfConfig};

use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token},
    QuestMakepadGpuSkinningMeshProbeInput, QuestMakepadGpuSkinningMeshVertex,
    QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_BACKEND, QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MEASUREMENT_SOURCE, QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_PAYLOAD,
    QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE,
};

/// Number of bounded dense-SDF samples echoed in the mesh-to-SDF proof marker.
pub const QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SAMPLES: usize = 8;
/// Conservative f32 tolerance for bounded dense-SDF readback comparison.
pub const QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_DEFAULT_TOLERANCE: f32 = 0.001;
/// Maximum voxel count for the current bounded dense-SDF construction probe.
pub const QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MAX_VOXELS: usize = 2_048;
const QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_TARGET_AXIS_CELLS: f32 = 10.0;
const QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MIN_VOXEL_SIZE: f32 = 0.001;

/// Bounded Matter-oracle dense SDF grid shape for GPU mesh-to-SDF validation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuMeshSdfProbeGrid {
    /// Matter CPU grid origin.
    pub origin: [f32; 3],
    /// Matter CPU grid voxel size.
    pub voxel_size: f32,
    /// Matter CPU grid dimensions.
    pub dimensions: [u32; 3],
    /// Dense voxel count.
    pub voxel_count: usize,
}

/// One bounded CPU-oracle sample from the dense SDF grid.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuMeshSdfProbeSample {
    /// Packed x-fastest dense-grid index.
    pub linear_index: usize,
    /// Grid cell coordinate.
    pub cell: [u32; 3],
    /// World-space cell center, packed as `[x, y, z, 1]`.
    pub center: [f32; 4],
    /// Matter CPU dense-SDF value for the cell.
    pub expected_distance: f32,
}

/// Full recorded-hand mesh-to-dense-SDF GPU probe input.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuMeshSdfProbeInput {
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
    /// One skinning row per bind vertex.
    pub vertices: Vec<QuestMakepadGpuSkinningMeshVertex>,
    /// Full triangle index buffer.
    pub triangles: Vec<[u32; 3]>,
    /// Bounded Matter CPU dense-SDF oracle grid shape.
    pub grid: QuestMakepadGpuMeshSdfProbeGrid,
    /// Compact Matter CPU oracle samples for readback comparison.
    pub samples: [QuestMakepadGpuMeshSdfProbeSample; QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SAMPLES],
    /// Number of populated samples.
    pub sample_count: usize,
}

impl QuestMakepadGpuMeshSdfProbeInput {
    /// Builds a bounded dense-SDF GPU probe input from the full skinning mesh oracle.
    #[must_use]
    pub fn from_skinning_mesh_input(input: &QuestMakepadGpuSkinningMeshProbeInput) -> Option<Self> {
        if input.topology_vertex_count == 0
            || input.topology_triangle_count == 0
            || input.vertices.len() != input.topology_vertex_count
            || input.triangles.len() != input.topology_triangle_count
            || input.topology_index_count != input.topology_triangle_count * 3
        {
            return None;
        }

        let positions = input
            .vertices
            .iter()
            .map(|vertex| {
                Vec3::new(
                    vertex.expected_position[0],
                    vertex.expected_position[1],
                    vertex.expected_position[2],
                )
            })
            .collect::<Vec<_>>();
        if !positions.iter().copied().all(Vec3::is_finite) {
            return None;
        }

        let mesh = TriangleMeshSnapshot::new(
            format!("{}.gpu_mesh_sdf_oracle", input.source_id),
            positions,
            input.triangles.clone(),
        );
        mesh.validate().ok()?;
        let bounds = mesh.bounds().ok()?;
        let size = bounds.size();
        let max_extent = size.x.max(size.y).max(size.z);
        if !max_extent.is_finite() || max_extent <= 0.0 {
            return None;
        }
        let voxel_size = (max_extent / QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_TARGET_AXIS_CELLS)
            .max(QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MIN_VOXEL_SIZE);
        let config = MeshToSdfConfig {
            voxel_size,
            padding_voxels: 1,
            max_voxels: QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MAX_VOXELS,
            sign_mode: MeshSdfSignMode::TriangleNormal,
        };
        let cpu_grid = build_sdf_from_mesh(&mesh, config).ok()?;
        let voxel_count = cpu_grid.sample_count();
        if voxel_count == 0 || voxel_count > QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MAX_VOXELS {
            return None;
        }

        let sample_count = voxel_count.min(QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SAMPLES);
        let mut samples = [QuestMakepadGpuMeshSdfProbeSample::default();
            QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SAMPLES];
        for (sample_index, target) in samples.iter_mut().take(sample_count).enumerate() {
            let linear_index = selected_index(voxel_count, sample_count, sample_index);
            let cell = cpu_grid.linear_to_cell(linear_index)?;
            let center = cpu_grid.linear_cell_center(linear_index)?;
            let expected_distance = *cpu_grid.distances.get(linear_index)?;
            *target = QuestMakepadGpuMeshSdfProbeSample {
                linear_index,
                cell,
                center: [center.x, center.y, center.z, 1.0],
                expected_distance,
            };
        }

        Some(Self {
            source_id: input.source_id.clone(),
            source_frame_index: input.source_frame_index,
            topology_vertex_count: input.topology_vertex_count,
            topology_triangle_count: input.topology_triangle_count,
            topology_index_count: input.topology_index_count,
            vertices: input.vertices.clone(),
            triangles: input.triangles.clone(),
            grid: QuestMakepadGpuMeshSdfProbeGrid {
                origin: [cpu_grid.origin.x, cpu_grid.origin.y, cpu_grid.origin.z],
                voxel_size: cpu_grid.voxel_size,
                dimensions: cpu_grid.dimensions,
                voxel_count,
            },
            samples,
            sample_count,
        })
    }

    /// First populated sample linear index.
    #[must_use]
    pub fn first_sample_linear_index(&self) -> Option<usize> {
        (self.sample_count > 0).then_some(self.samples[0].linear_index)
    }

    /// Last populated sample linear index.
    #[must_use]
    pub fn last_sample_linear_index(&self) -> Option<usize> {
        self.sample_count
            .checked_sub(1)
            .map(|index| self.samples[index].linear_index)
    }
}

/// Makepad GPU bounded dense-SDF readback summary.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuMeshSdfProbeReadback {
    /// Number of vertices skinned by the GPU command.
    pub vertex_count: usize,
    /// Number of triangles read by the GPU command.
    pub triangle_count: usize,
    /// Number of flattened triangle indices read by the GPU command.
    pub index_count: usize,
    /// Number of dense SDF voxels written by the GPU command.
    pub voxel_count: usize,
    /// Number of compact samples read back and compared.
    pub sample_count: usize,
    /// Number of sample distances checked.
    pub checked_sample_count: usize,
    /// Number of sample distances outside tolerance.
    pub mismatched_samples: usize,
    /// Maximum absolute GPU-vs-CPU sample distance error.
    pub max_abs_error: f32,
    /// Absolute tolerance used by the readback comparison.
    pub tolerance: f32,
    /// Sample linear indices included in the compact summary.
    pub sample_linear_indices: [usize; QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SAMPLES],
    /// GPU output sample distances.
    pub output_distances: [f32; QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SAMPLES],
    /// Matter CPU expected sample distances.
    pub expected_distances: [f32; QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SAMPLES],
    /// Makepad XR/Vulkan submit serial for the proof command.
    pub queue_submit_serial: u64,
    /// Fence serial observed for the proof command.
    pub fence_serial: u64,
    /// Monotonic proof-resource generation for the current renderer lifetime.
    pub resource_generation: u64,
    /// Renderer-lifetime GPU program generation used by this proof.
    pub program_generation: u64,
    /// True when the shader modules/layouts/pipelines were already resident before this submit.
    pub program_reused: bool,
    /// True when WGSL-to-SPIR-V and shader-module setup happened during this submit.
    pub shader_compiled_this_submit: bool,
    /// True when compute pipeline creation happened during this submit.
    pub pipeline_created_this_submit: bool,
    /// Renderer-lifetime source mesh buffer generation used by this proof.
    pub source_mesh_buffer_generation: u64,
    /// True when the source vertex/index buffers were resident in the backend.
    pub source_mesh_buffers_resident: bool,
    /// True when existing resident source vertex/index buffers were reused.
    pub source_mesh_buffers_reused: bool,
    /// Source vertex storage-buffer byte size.
    pub source_vertex_buffer_bytes: u64,
    /// Source triangle storage-buffer byte size.
    pub source_triangle_buffer_bytes: u64,
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

impl QuestMakepadGpuMeshSdfProbeReadback {
    /// True when bounded dense-SDF samples matched the Matter CPU oracle.
    #[must_use]
    pub fn readback_matched(self) -> bool {
        self.vertex_count > 0
            && self.triangle_count > 0
            && self.index_count == self.triangle_count * 3
            && self.voxel_count > 0
            && self.sample_count > 0
            && self.checked_sample_count == self.sample_count
            && self.mismatched_samples == 0
            && self.max_abs_error.is_finite()
            && self.tolerance.is_finite()
            && self.max_abs_error <= self.tolerance.max(0.0)
    }
}

/// Bounded recorded-hand GPU mesh-to-dense-SDF construction proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuMeshSdfProbe {
    /// Schema identifier.
    pub schema_id: String,
    /// Full source-frame skinning and bounded SDF oracle input.
    pub input: QuestMakepadGpuMeshSdfProbeInput,
    /// Makepad readback result.
    pub readback: QuestMakepadGpuMeshSdfProbeReadback,
}

impl QuestMakepadGpuMeshSdfProbe {
    /// Builds a bounded dense-SDF marker from recorded-hand source-frame input.
    #[must_use]
    pub fn from_input(
        input: &QuestMakepadGpuMeshSdfProbeInput,
        readback: QuestMakepadGpuMeshSdfProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SCHEMA_ID.to_owned(),
            input: input.clone(),
            readback,
        }
    }

    /// Builds a compact marker without logging mesh, field, particle, or GPU buffers.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} proofKind=bounded-recorded-hand-mesh-to-dense-sdf computeStage=hand-skinning-to-dense-sdf sourceId={} sourceFrameIndex={} topologyVertexCount={} topologyTriangleCount={} topologyIndexCount={} cpuOracle=matter-mesh-to-sdf cpuOraclePreserved=true recordedInputEquivalent=true validationInputShape=bind-mesh-plus-compact-joint-frame resourcePlane={} computeProbeBackend={} oraclePayload={} gridOriginX={} gridOriginY={} gridOriginZ={} gridVoxelSize={} gridDimX={} gridDimY={} gridDimZ={} voxelCount={} vertexCount={} triangleCount={} indexCount={} sampleCount={} checkedSampleCount={} firstSampleLinearIndex={} lastSampleLinearIndex={} mismatchedSamples={} maxAbsError={} tolerance={} readbackMatched={} commandEncoderSubmitted=true skinnedVertexBufferResident=true denseSdfVoxelBufferResident=true denseSdfConstructedOnGpu=true indexBufferConsumedByGpu=true fullSourceMeshConsumedByGpu=true computeDispatchSubmitted=true boundedSampleOnly=false prototypeComputeKernel=false weightedDeltaSkinningKernel=false jointMatrixSkinningKernel=true meshToSdfKernel=true fieldSamplingKernel=false fieldParticleKernel=false computeKernel=true gpuComputeReady=false highRateJsonPayload=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} programGeneration={} programReused={} shaderCompiledThisSubmit={} pipelineCreatedThisSubmit={} sourceMeshBufferGeneration={} sourceMeshBuffersResident={} sourceMeshBuffersReused={} sourceVertexBufferBytes={} sourceTriangleBufferBytes={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MARKER_PREFIX,
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
            QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_BACKEND,
            QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_PAYLOAD,
            finite_f32_marker_token(self.input.grid.origin[0]),
            finite_f32_marker_token(self.input.grid.origin[1]),
            finite_f32_marker_token(self.input.grid.origin[2]),
            finite_f32_marker_token(self.input.grid.voxel_size),
            self.input.grid.dimensions[0],
            self.input.grid.dimensions[1],
            self.input.grid.dimensions[2],
            self.readback.voxel_count,
            self.readback.vertex_count,
            self.readback.triangle_count,
            self.readback.index_count,
            self.readback.sample_count,
            self.readback.checked_sample_count,
            optional_usize_marker_token(self.input.first_sample_linear_index()),
            optional_usize_marker_token(self.input.last_sample_linear_index()),
            self.readback.mismatched_samples,
            finite_f32_marker_token(self.readback.max_abs_error),
            finite_f32_marker_token(self.readback.tolerance),
            self.readback.readback_matched(),
            self.readback.queue_submit_serial,
            self.readback.fence_serial,
            self.readback.resource_generation,
            self.readback.program_generation,
            self.readback.program_reused,
            self.readback.shader_compiled_this_submit,
            self.readback.pipeline_created_this_submit,
            self.readback.source_mesh_buffer_generation,
            self.readback.source_mesh_buffers_resident,
            self.readback.source_mesh_buffers_reused,
            self.readback.source_vertex_buffer_bytes,
            self.readback.source_triangle_buffer_bytes,
            self.readback.pending_retire_count,
            self.readback.retained_resource_count,
            self.readback.retired_after_fence_count,
            self.readback.queue_wait_idle_performed,
            finite_f64_marker_token(self.readback.elapsed_ms),
            QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MEASUREMENT_SOURCE,
        )
    }
}

fn selected_index(count: usize, sample_count: usize, sample_index: usize) -> usize {
    if sample_count <= 1 {
        0
    } else {
        sample_index * (count - 1) / (sample_count - 1)
    }
}

fn optional_usize_marker_token(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}
