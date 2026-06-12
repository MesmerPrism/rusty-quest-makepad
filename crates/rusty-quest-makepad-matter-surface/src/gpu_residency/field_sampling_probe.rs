use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token, optional_usize_marker_token},
    QuestMakepadGpuFieldConstructionReceipt, QuestMakepadGpuMeshSdfProbeInput,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
    QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_BACKEND,
};

/// Number of bounded dense-SDF samples echoed in the field-sampling marker.
pub const QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_SAMPLES: usize = 8;
/// Conservative f32 tolerance for resident dense-SDF field sampling.
pub const QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_DEFAULT_TOLERANCE: f32 = 0.001;

/// Makepad GPU resident dense-SDF sampling readback result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadGpuFieldSamplingProbeReadback {
    /// Number of bounded dense-SDF samples submitted to the sampler kernel.
    pub sample_count: usize,
    /// Number of sample distances checked.
    pub checked_sample_count: usize,
    /// Sampled dense-grid linear indices.
    pub sample_linear_indices: [usize; QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_SAMPLES],
    /// GPU output sample distances.
    pub output_distances: [f32; QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_SAMPLES],
    /// Matter CPU expected sample distances.
    pub expected_distances: [f32; QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_SAMPLES],
    /// Number of sample distances outside tolerance.
    pub mismatched_samples: usize,
    /// Maximum absolute GPU-vs-CPU sample distance error.
    pub max_abs_error: f32,
    /// Absolute tolerance used by the readback comparison.
    pub tolerance: f32,
    /// Makepad XR/Vulkan submit serial for the proof command.
    pub queue_submit_serial: u64,
    /// Fence serial observed for the proof command.
    pub fence_serial: u64,
    /// Monotonic proof-resource generation for the current renderer lifetime.
    pub resource_generation: u64,
    /// Renderer-lifetime GPU sampler program generation used by this proof.
    pub program_generation: u64,
    /// True when the sampler shader module/layout/pipeline was already resident.
    pub program_reused: bool,
    /// True when WGSL-to-SPIR-V and shader-module setup happened during this submit.
    pub shader_compiled_this_submit: bool,
    /// True when compute pipeline creation happened during this submit.
    pub pipeline_created_this_submit: bool,
    /// Dense-SDF derived buffer generation sampled by this proof.
    pub source_field_generation: u64,
    /// True when a resident dense-SDF buffer was sampled by the GPU command.
    pub source_field_buffer_resident: bool,
    /// Dense-SDF distance storage-buffer byte size.
    pub source_field_buffer_bytes: u64,
    /// Sample-index storage-buffer byte size.
    pub sample_index_buffer_bytes: u64,
    /// Sample-output storage-buffer byte size.
    pub sample_output_buffer_bytes: u64,
    /// Proof resources still pending retirement.
    pub pending_retire_count: usize,
    /// Proof resources retained by the current Makepad backend.
    pub retained_resource_count: usize,
    /// Proof resources destroyed after fence evidence in this call.
    pub retired_after_fence_count: usize,
    /// True when the Makepad backend waited for queue idle after the proof.
    pub queue_wait_idle_performed: bool,
    /// CPU-side elapsed time for command submission, fence wait, and readback.
    pub elapsed_ms: f64,
}

impl QuestMakepadGpuFieldSamplingProbeReadback {
    /// True when bounded resident-field samples matched the Matter CPU oracle.
    #[must_use]
    pub fn readback_matched(self) -> bool {
        self.sample_count > 0
            && self.checked_sample_count == self.sample_count
            && self.mismatched_samples == 0
            && self.max_abs_error.is_finite()
            && self.tolerance.is_finite()
            && self.max_abs_error <= self.tolerance.max(0.0)
    }
}

/// Low-rate proof that a resident dense-SDF field buffer can be sampled on the GPU.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuFieldSamplingProbe {
    /// Schema identifier.
    pub schema_id: String,
    /// Dense-SDF field construction receipt this sampler consumes.
    pub receipt: QuestMakepadGpuFieldConstructionReceipt,
    /// Bounded Matter CPU oracle sample count requested by the adapter.
    pub oracle_sample_count: usize,
    /// First populated sample linear index from the Matter CPU oracle.
    pub first_sample_linear_index: Option<usize>,
    /// Last populated sample linear index from the Matter CPU oracle.
    pub last_sample_linear_index: Option<usize>,
    /// Makepad resident dense-SDF sampling readback.
    pub readback: QuestMakepadGpuFieldSamplingProbeReadback,
}

impl QuestMakepadGpuFieldSamplingProbe {
    /// Builds a field-sampling proof from the construction receipt and bounded oracle input.
    #[must_use]
    pub fn from_receipt_and_input(
        receipt: &QuestMakepadGpuFieldConstructionReceipt,
        input: &QuestMakepadGpuMeshSdfProbeInput,
        readback: QuestMakepadGpuFieldSamplingProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_SCHEMA_ID.to_owned(),
            receipt: receipt.clone(),
            oracle_sample_count: input.sample_count,
            first_sample_linear_index: input.first_sample_linear_index(),
            last_sample_linear_index: input.last_sample_linear_index(),
            readback,
        }
    }

    /// True when the sampled field matches the construction receipt and CPU oracle.
    #[must_use]
    pub fn runtime_sampling_boundary_ready(&self) -> bool {
        self.receipt.runtime_field_boundary_ready()
            && self.readback.readback_matched()
            && self.readback.source_field_buffer_resident
            && self.readback.source_field_generation == self.receipt.derived_buffer_generation
            && self.readback.source_field_buffer_bytes == self.receipt.sdf_distance_buffer_bytes
            && self.readback.sample_count == self.oracle_sample_count
    }

    /// Builds a compact marker without logging field buffers or high-rate samples.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} proofKind=resident-dense-sdf-field-sampling computeStage=dense-sdf-field-sample-readback fieldKind={} sourceReceiptSchema={} sourceId={} sourceFrameIndex={} fieldResourceId={} topologyVertexCount={} topologyTriangleCount={} topologyIndexCount={} validationInputShape={} resourcePlane={} sourceResourcePlane={} computeProbeBackend={} gridOriginX={} gridOriginY={} gridOriginZ={} gridVoxelSize={} gridDimX={} gridDimY={} gridDimZ={} voxelCount={} oracleSampleCount={} sampleCount={} checkedSampleCount={} firstSampleLinearIndex={} lastSampleLinearIndex={} mismatchedSamples={} maxAbsError={} tolerance={} readbackMatched={} runtimeFieldBoundaryReady={} runtimeSamplingBoundaryReady={} sourceFieldGeneration={} expectedSourceFieldGeneration={} sourceFieldGenerationMatched={} sourceFieldBufferResident={} sourceFieldBufferBytes={} expectedSourceFieldBufferBytes={} sampleIndexBufferBytes={} sampleOutputBufferBytes={} cpuOracle={} cpuOraclePreserved=true recordedInputEquivalent=true residentFieldBufferSampled=true denseSdfConstructedOnGpu={} meshToSdfKernel=false fieldSamplingKernel=true fieldParticleKernel=false computeKernel=true gpuComputeReady=false forceAuthorityReady=false runtimeForceAuthority=false highRateJsonPayload=false commandEncoderSubmitted=true computeDispatchSubmitted=true queueSubmitSerial={} fenceSerial={} resourceGeneration={} programGeneration={} programReused={} shaderCompiledThisSubmit={} pipelineCreatedThisSubmit={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            self.status_marker(),
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
            sanitize_marker_value(&self.receipt.source_id),
            self.receipt.source_frame_index,
            sanitize_marker_value(&self.receipt.field_resource_id),
            self.receipt.topology_vertex_count,
            self.receipt.topology_triangle_count,
            self.receipt.topology_index_count,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
            QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_MESH_SDF_PROBE_BACKEND,
            finite_f32_marker_token(self.receipt.grid.origin[0]),
            finite_f32_marker_token(self.receipt.grid.origin[1]),
            finite_f32_marker_token(self.receipt.grid.origin[2]),
            finite_f32_marker_token(self.receipt.grid.voxel_size),
            self.receipt.grid.dimensions[0],
            self.receipt.grid.dimensions[1],
            self.receipt.grid.dimensions[2],
            self.receipt.voxel_count,
            self.oracle_sample_count,
            self.readback.sample_count,
            self.readback.checked_sample_count,
            optional_usize_marker_token(self.first_sample_linear_index),
            optional_usize_marker_token(self.last_sample_linear_index),
            self.readback.mismatched_samples,
            finite_f32_marker_token(self.readback.max_abs_error),
            finite_f32_marker_token(self.readback.tolerance),
            self.readback.readback_matched(),
            self.receipt.runtime_field_boundary_ready(),
            self.runtime_sampling_boundary_ready(),
            self.readback.source_field_generation,
            self.receipt.derived_buffer_generation,
            self.readback.source_field_generation == self.receipt.derived_buffer_generation,
            self.readback.source_field_buffer_resident,
            self.readback.source_field_buffer_bytes,
            self.receipt.sdf_distance_buffer_bytes,
            self.readback.sample_index_buffer_bytes,
            self.readback.sample_output_buffer_bytes,
            QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_CPU_ORACLE,
            self.receipt.gpu_field_constructed(),
            self.readback.queue_submit_serial,
            self.readback.fence_serial,
            self.readback.resource_generation,
            self.readback.program_generation,
            self.readback.program_reused,
            self.readback.shader_compiled_this_submit,
            self.readback.pipeline_created_this_submit,
            self.readback.pending_retire_count,
            self.readback.retained_resource_count,
            self.readback.retired_after_fence_count,
            self.readback.queue_wait_idle_performed,
            finite_f64_marker_token(self.readback.elapsed_ms),
            QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_MEASUREMENT_SOURCE,
        )
    }

    fn status_marker(&self) -> &'static str {
        if !self.readback.readback_matched() {
            "mismatch"
        } else if self.runtime_sampling_boundary_ready() {
            "ready"
        } else {
            "not-ready"
        }
    }
}

/// Quest Makepad GPU resident dense-SDF field sampling schema.
pub const QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_field_sampling_probe.v1";
/// Quest Makepad GPU resident dense-SDF field sampling marker prefix.
pub const QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE";
/// Resource plane proven by the resident field sampler.
pub const QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_RESOURCE_PLANE: &str =
    "vulkan-compute-resident-dense-sdf-sampler";
/// Matter CPU oracle retained for bounded field-sampling validation.
pub const QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_CPU_ORACLE: &str =
    "matter-mesh-to-sdf-sample-indices";
/// Measurement companion for the field-sampling probe.
pub const QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE_MEASUREMENT_SOURCE: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FIELD_SAMPLING_PROBE.elapsedMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs";
