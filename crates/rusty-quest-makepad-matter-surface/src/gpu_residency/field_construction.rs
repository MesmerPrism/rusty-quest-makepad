use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token},
    QuestMakepadGpuMeshSdfProbe, QuestMakepadGpuMeshSdfProbeGrid,
    QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_BACKEND, QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SCHEMA_ID,
};

/// Quest Makepad GPU dense-field construction receipt schema.
pub const QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_field_construction_receipt.v1";
/// Quest Makepad GPU dense-field construction receipt marker prefix.
pub const QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION";
/// Resource plane proven by the dense-SDF field construction receipt.
pub const QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE: &str =
    "vulkan-compute-dense-sdf-buffer";
/// Renderer-neutral field kind described by the current receipt.
pub const QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND: &str = "dense-sdf";
/// Input source shape for the current receipt.
pub const QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SOURCE_KIND: &str =
    "recorded-hand-skinned-mesh";
/// Matter CPU oracle retained for bounded validation.
pub const QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_CPU_ORACLE: &str = "matter-mesh-to-sdf";
/// Compact input contract shared by recorded replay and future live OpenXR hands.
pub const QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE: &str =
    "bind-mesh-plus-compact-joint-frame";

/// Low-rate receipt proving a dense-SDF field buffer boundary without making it force authority.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuFieldConstructionReceipt {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable field resource identity for this renderer-lifetime dense SDF.
    pub field_resource_id: String,
    /// Stable recorded/live-equivalent source identity.
    pub source_id: String,
    /// Recorded/live-equivalent source frame index.
    pub source_frame_index: usize,
    /// Full topology vertex count consumed by the GPU command.
    pub topology_vertex_count: usize,
    /// Full topology triangle count consumed by the GPU command.
    pub topology_triangle_count: usize,
    /// Full flattened triangle index count consumed by the GPU command.
    pub topology_index_count: usize,
    /// Dense SDF grid shape used for bounded Matter CPU-oracle comparison.
    pub grid: QuestMakepadGpuMeshSdfProbeGrid,
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
    /// True when bounded dense-SDF samples matched the Matter CPU oracle.
    pub readback_matched: bool,
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
    /// Renderer-lifetime derived skinned/SDF buffer generation used by this proof.
    pub derived_buffer_generation: u64,
    /// True when the derived skinned/SDF buffers were renderer-lifetime resident.
    pub derived_buffers_resident: bool,
    /// True when existing resident derived skinned/SDF buffers were reused.
    pub derived_buffers_reused: bool,
    /// Skinned-position storage-buffer byte size.
    pub skinned_position_buffer_bytes: u64,
    /// Dense-SDF distance storage-buffer byte size.
    pub sdf_distance_buffer_bytes: u64,
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

impl QuestMakepadGpuFieldConstructionReceipt {
    /// Builds a dense-SDF field receipt from the bounded mesh-to-SDF proof.
    #[must_use]
    pub fn from_mesh_sdf_probe(probe: &QuestMakepadGpuMeshSdfProbe) -> Self {
        let readback = probe.readback;
        let field_resource_id = format!(
            "{}.frame{}.dense_sdf.g{}",
            probe.input.source_id,
            probe.input.source_frame_index,
            readback.derived_buffer_generation
        );

        Self {
            schema_id: QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID.to_owned(),
            field_resource_id,
            source_id: probe.input.source_id.clone(),
            source_frame_index: probe.input.source_frame_index,
            topology_vertex_count: probe.input.topology_vertex_count,
            topology_triangle_count: probe.input.topology_triangle_count,
            topology_index_count: probe.input.topology_index_count,
            grid: probe.input.grid,
            vertex_count: readback.vertex_count,
            triangle_count: readback.triangle_count,
            index_count: readback.index_count,
            voxel_count: readback.voxel_count,
            sample_count: readback.sample_count,
            checked_sample_count: readback.checked_sample_count,
            mismatched_samples: readback.mismatched_samples,
            max_abs_error: readback.max_abs_error,
            tolerance: readback.tolerance,
            readback_matched: readback.readback_matched(),
            queue_submit_serial: readback.queue_submit_serial,
            fence_serial: readback.fence_serial,
            resource_generation: readback.resource_generation,
            program_generation: readback.program_generation,
            program_reused: readback.program_reused,
            shader_compiled_this_submit: readback.shader_compiled_this_submit,
            pipeline_created_this_submit: readback.pipeline_created_this_submit,
            source_mesh_buffer_generation: readback.source_mesh_buffer_generation,
            source_mesh_buffers_resident: readback.source_mesh_buffers_resident,
            source_mesh_buffers_reused: readback.source_mesh_buffers_reused,
            source_vertex_buffer_bytes: readback.source_vertex_buffer_bytes,
            source_triangle_buffer_bytes: readback.source_triangle_buffer_bytes,
            derived_buffer_generation: readback.derived_buffer_generation,
            derived_buffers_resident: readback.derived_buffers_resident,
            derived_buffers_reused: readback.derived_buffers_reused,
            skinned_position_buffer_bytes: readback.skinned_position_buffer_bytes,
            sdf_distance_buffer_bytes: readback.sdf_distance_buffer_bytes,
            pending_retire_count: readback.pending_retire_count,
            retained_resource_count: readback.retained_resource_count,
            retired_after_fence_count: readback.retired_after_fence_count,
            queue_wait_idle_performed: readback.queue_wait_idle_performed,
            elapsed_ms: readback.elapsed_ms,
        }
    }

    /// True when the dense-SDF field buffer can be handed to the next GPU adapter boundary.
    #[must_use]
    pub fn runtime_field_boundary_ready(&self) -> bool {
        self.readback_matched
            && self.source_mesh_buffers_resident
            && self.derived_buffers_resident
            && self.voxel_count == self.grid.voxel_count
            && self.voxel_count > 0
            && self.sdf_distance_buffer_bytes > 0
            && self.sample_count > 0
            && self.checked_sample_count == self.sample_count
    }

    /// True when the GPU path produced a dense-SDF buffer that matched bounded Matter samples.
    #[must_use]
    pub fn gpu_field_constructed(&self) -> bool {
        self.readback_matched && self.derived_buffers_resident && self.sdf_distance_buffer_bytes > 0
    }

    /// Builds a compact marker without logging mesh vertices, fields, particles, or GPU buffers.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} receiptKind=gpu-dense-sdf-field-construction computeStage=hand-mesh-to-dense-sdf-field-residency fieldKind={} fieldConstructionSource={} sourceProbeSchema={} sourceId={} sourceFrameIndex={} fieldResourceId={} topologyVertexCount={} topologyTriangleCount={} topologyIndexCount={} cpuOracle={} cpuOraclePreserved=true recordedInputEquivalent=true validationInputShape={} resourcePlane={} computeProbeBackend={} gridOriginX={} gridOriginY={} gridOriginZ={} gridVoxelSize={} gridDimX={} gridDimY={} gridDimZ={} voxelCount={} vertexCount={} triangleCount={} indexCount={} sampleCount={} checkedSampleCount={} mismatchedSamples={} maxAbsError={} tolerance={} readbackMatched={} runtimeFieldBoundaryReady={} forceAuthorityReady=false runtimeForceAuthority=false commandEncoderSubmitted=true sourceMeshBuffersResident={} sourceMeshBuffersReused={} derivedBuffersResident={} derivedBuffersReused={} denseSdfConstructedOnGpu={} meshToSdfKernel=true fieldSamplingKernel=false fieldParticleKernel=false computeKernel=true gpuComputeReady=false highRateJsonPayload=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} programGeneration={} programReused={} shaderCompiledThisSubmit={} pipelineCreatedThisSubmit={} sourceMeshBufferGeneration={} sourceVertexBufferBytes={} sourceTriangleBufferBytes={} derivedBufferGeneration={} skinnedPositionBufferBytes={} sdfDistanceBufferBytes={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            self.status_marker(),
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SOURCE_KIND,
            QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_SCHEMA_ID,
            sanitize_marker_value(&self.source_id),
            self.source_frame_index,
            sanitize_marker_value(&self.field_resource_id),
            self.topology_vertex_count,
            self.topology_triangle_count,
            self.topology_index_count,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_CPU_ORACLE,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_BACKEND,
            finite_f32_marker_token(self.grid.origin[0]),
            finite_f32_marker_token(self.grid.origin[1]),
            finite_f32_marker_token(self.grid.origin[2]),
            finite_f32_marker_token(self.grid.voxel_size),
            self.grid.dimensions[0],
            self.grid.dimensions[1],
            self.grid.dimensions[2],
            self.voxel_count,
            self.vertex_count,
            self.triangle_count,
            self.index_count,
            self.sample_count,
            self.checked_sample_count,
            self.mismatched_samples,
            finite_f32_marker_token(self.max_abs_error),
            finite_f32_marker_token(self.tolerance),
            self.readback_matched,
            self.runtime_field_boundary_ready(),
            self.source_mesh_buffers_resident,
            self.source_mesh_buffers_reused,
            self.derived_buffers_resident,
            self.derived_buffers_reused,
            self.gpu_field_constructed(),
            self.queue_submit_serial,
            self.fence_serial,
            self.resource_generation,
            self.program_generation,
            self.program_reused,
            self.shader_compiled_this_submit,
            self.pipeline_created_this_submit,
            self.source_mesh_buffer_generation,
            self.source_vertex_buffer_bytes,
            self.source_triangle_buffer_bytes,
            self.derived_buffer_generation,
            self.skinned_position_buffer_bytes,
            self.sdf_distance_buffer_bytes,
            self.pending_retire_count,
            self.retained_resource_count,
            self.retired_after_fence_count,
            self.queue_wait_idle_performed,
            finite_f64_marker_token(self.elapsed_ms),
            QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_MEASUREMENT_SOURCE,
        )
    }

    fn status_marker(&self) -> &'static str {
        if !self.readback_matched {
            "mismatch"
        } else if self.runtime_field_boundary_ready() {
            "ready"
        } else {
            "not-ready"
        }
    }
}
