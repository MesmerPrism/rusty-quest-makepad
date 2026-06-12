use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token},
    QuestMakepadGpuFieldConstructionReceipt, QuestMakepadGpuMeshSdfProbeInput,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
};

/// Number of bounded dense-SDF force samples echoed in the resident-field proof marker.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_SAMPLES: usize = 4;
/// Conservative f32 tolerance for bounded resident-field force readback comparison.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_DEFAULT_TOLERANCE: f32 = 0.001;
/// Quest Makepad GPU resident dense-SDF field force sampling schema.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_field_force_sampling_probe.v1";
/// Quest Makepad GPU resident dense-SDF field force sampling marker prefix.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE";
/// Resource plane proven by the resident dense-SDF field force sampling probe.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_RESOURCE_PLANE: &str =
    "vulkan-compute-resident-dense-sdf-force-sampling";
/// Matter CPU oracle retained for bounded force validation.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_CPU_ORACLE: &str =
    "matter-dense-sdf-field-force-sampler";
/// Measurement companion for the resident field force sampling probe.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_MEASUREMENT_SOURCE: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE.elapsedMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs";

/// Generic Makepad GPU resident dense-SDF force sampling readback result.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuFieldForceSamplingProbeReadback {
    /// Number of bounded force samples submitted to the GPU.
    pub sample_count: usize,
    /// Number of checked acceleration components.
    pub component_count: usize,
    /// Number of acceleration components outside tolerance.
    pub mismatched_components: usize,
    /// Maximum absolute GPU-vs-CPU acceleration error.
    pub max_abs_error: f32,
    /// Absolute tolerance used by the bounded readback comparison.
    pub tolerance: f32,
    /// Makepad XR/Vulkan submit serial for the proof command.
    pub queue_submit_serial: u64,
    /// Fence serial observed for the proof command.
    pub fence_serial: u64,
    /// Monotonic proof-resource generation for the current renderer lifetime.
    pub resource_generation: u64,
    /// Renderer-lifetime GPU program generation used by this proof.
    pub program_generation: u64,
    /// True when shader modules/layouts/pipelines were already resident.
    pub program_reused: bool,
    /// True when WGSL-to-SPIR-V and shader-module setup happened during this submit.
    pub shader_compiled_this_submit: bool,
    /// True when compute pipeline creation happened during this submit.
    pub pipeline_created_this_submit: bool,
    /// Renderer-lifetime source field generation used by this proof.
    pub source_field_generation: u64,
    /// True when the dense-SDF field buffer was resident in the backend.
    pub source_field_buffer_resident: bool,
    /// Dense-SDF distance storage-buffer byte size.
    pub source_field_buffer_bytes: u64,
    /// Input force-sample storage-buffer byte size.
    pub sample_input_buffer_bytes: u64,
    /// Output acceleration storage-buffer byte size.
    pub sample_output_buffer_bytes: u64,
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

impl QuestMakepadGpuFieldForceSamplingProbeReadback {
    /// True when the bounded GPU force samples matched the Matter CPU oracle.
    #[must_use]
    pub fn readback_matched(self) -> bool {
        self.sample_count > 0
            && self.component_count == self.sample_count * 3
            && self.mismatched_components == 0
            && self.max_abs_error.is_finite()
            && self.tolerance.is_finite()
            && self.max_abs_error <= self.tolerance.max(0.0)
    }
}

/// Bounded resident dense-SDF force sampling proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuFieldForceSamplingProbe {
    /// Schema identifier.
    pub schema_id: String,
    /// Source dense-SDF field construction receipt.
    pub receipt: QuestMakepadGpuFieldConstructionReceipt,
    /// Number of CPU-oracle force samples available on the source input.
    pub oracle_sample_count: usize,
    /// Makepad readback result.
    pub readback: QuestMakepadGpuFieldForceSamplingProbeReadback,
}

impl QuestMakepadGpuFieldForceSamplingProbe {
    /// Builds a resident field-force marker from a field construction receipt.
    #[must_use]
    pub fn from_receipt_and_input(
        receipt: &QuestMakepadGpuFieldConstructionReceipt,
        input: &QuestMakepadGpuMeshSdfProbeInput,
        readback: QuestMakepadGpuFieldForceSamplingProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_SCHEMA_ID.to_owned(),
            receipt: receipt.clone(),
            oracle_sample_count: input.force_sample_count,
            readback,
        }
    }

    /// Builds a compact marker without logging field, particle, or GPU-buffer data.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} proofKind=resident-dense-sdf-field-force-sampling computeStage=dense-sdf-field-force-sample-readback fieldKind={} sourceReceiptSchema={} sourceId={} sourceFrameIndex={} fieldResourceId={} topologyVertexCount={} topologyTriangleCount={} topologyIndexCount={} validationInputShape={} resourcePlane={} sourceResourcePlane={} computeProbeBackend=makepad-vulkan-compute-resident-dense-sdf-force-sampling gridOriginX={} gridOriginY={} gridOriginZ={} gridVoxelSize={} gridDimX={} gridDimY={} gridDimZ={} voxelCount={} oracleSampleCount={} sampleCount={} componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} runtimeFieldBoundaryReady={} runtimeForceSamplingBoundaryReady=true sourceFieldGeneration={} expectedSourceFieldGeneration={} sourceFieldGenerationMatched={} sourceFieldBufferResident={} sourceFieldBufferBytes={} expectedSourceFieldBufferBytes={} sampleInputBufferBytes={} sampleOutputBufferBytes={} cpuOracle={} cpuOraclePreserved=true recordedInputEquivalent=true residentFieldBufferSampled=true denseSdfConstructedOnGpu=true meshToSdfKernel=false fieldSamplingKernel=true fieldForceSamplingKernel=true fieldParticleKernel=false runtimeParticleIntegration=false computeKernel=true gpuComputeReady=false forceAuthorityReady=false runtimeForceAuthority=false highRateJsonPayload=false commandEncoderSubmitted=true computeDispatchSubmitted=true queueSubmitSerial={} fenceSerial={} resourceGeneration={} programGeneration={} programReused={} shaderCompiledThisSubmit={} pipelineCreatedThisSubmit={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            if self.readback.readback_matched() {
                "ready"
            } else {
                "mismatch"
            },
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
            sanitize_marker_value(&self.receipt.source_id),
            self.receipt.source_frame_index,
            sanitize_marker_value(&self.receipt.field_resource_id),
            self.receipt.topology_vertex_count,
            self.receipt.topology_triangle_count,
            self.receipt.topology_index_count,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
            QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
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
            self.readback.component_count,
            self.readback.mismatched_components,
            finite_f32_marker_token(self.readback.max_abs_error),
            finite_f32_marker_token(self.readback.tolerance),
            self.readback.readback_matched(),
            self.receipt.runtime_field_boundary_ready(),
            self.readback.source_field_generation,
            self.receipt.derived_buffer_generation,
            self.readback.source_field_generation == self.receipt.derived_buffer_generation,
            self.readback.source_field_buffer_resident,
            self.readback.source_field_buffer_bytes,
            self.receipt.sdf_distance_buffer_bytes,
            self.readback.sample_input_buffer_bytes,
            self.readback.sample_output_buffer_bytes,
            QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_CPU_ORACLE,
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
            QUEST_MAKEPAD_GPU_FIELD_FORCE_SAMPLING_PROBE_MEASUREMENT_SOURCE,
        )
    }
}
