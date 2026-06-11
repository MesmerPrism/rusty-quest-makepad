use rusty_matter_surface_runtime::MatterSurfaceParticleForceSource;

use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token, optional_usize_marker_token},
    preflight::{QuestMakepadGpuComputePreflight, QuestMakepadGpuComputeResourceKind},
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_READBACK_POLICY,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_SCHEMA_ID, QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_BACKEND,
    QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_PAYLOAD, QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE,
};

/// Generic Makepad GPU f32 force arithmetic readback result consumed by the adapter marker.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuFieldForceProbeReadback {
    /// Number of bounded particle-force samples submitted to the GPU.
    pub sample_count: usize,
    /// Number of checked f32 acceleration components.
    pub component_count: usize,
    /// Number of output acceleration components outside tolerance.
    pub mismatched_components: usize,
    /// Maximum absolute output-vs-CPU-oracle acceleration error.
    pub max_abs_error: f32,
    /// Absolute tolerance used by the bounded readback comparison.
    pub tolerance: f32,
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

impl QuestMakepadGpuFieldForceProbeReadback {
    /// True when the bounded GPU output matched the Matter CPU-oracle force arithmetic.
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

/// Prototype f32 force arithmetic dispatch tied to a Matter field-force oracle.
///
/// This proves the GPU can recompute Matter's bounded particle-force arithmetic
/// for sampled SDF/ADF force probes. It still does not move field sampling,
/// particle integration, or simulation truth out of Matter.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuFieldForceProbe {
    /// Schema identifier.
    pub schema_id: String,
    /// Future GPU resource family.
    pub resource_kind: QuestMakepadGpuComputeResourceKind,
    /// Active Matter force source.
    pub force_source: MatterSurfaceParticleForceSource,
    /// Full Matter particle count in the oracle frame.
    pub particle_rows: usize,
    /// Renderer-facing particle rows in the oracle frame.
    pub visual_rows: usize,
    /// Source mesh vertex count.
    pub topology_vertex_count: usize,
    /// Source mesh triangle count.
    pub topology_triangle_count: usize,
    /// Source frame index.
    pub source_frame_index: Option<usize>,
    /// Bounded readback probe count requested by the Matter runtime profile.
    pub readback_probe_count: usize,
    /// Makepad f32 force arithmetic readback result.
    pub readback: QuestMakepadGpuFieldForceProbeReadback,
}

impl QuestMakepadGpuFieldForceProbe {
    /// Builds a f32 force arithmetic marker from the current compute preflight.
    #[must_use]
    pub fn from_preflight(
        preflight: &QuestMakepadGpuComputePreflight,
        readback: QuestMakepadGpuFieldForceProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_SCHEMA_ID.to_owned(),
            resource_kind: preflight.resource_kind,
            force_source: preflight.force_source,
            particle_rows: preflight.particle_rows,
            visual_rows: preflight.visual_rows,
            topology_vertex_count: preflight.topology_vertex_count,
            topology_triangle_count: preflight.topology_triangle_count,
            source_frame_index: preflight.source_frame_index,
            readback_probe_count: preflight.readback_probe_count,
            readback,
        }
    }

    /// Builds a compact marker without logging high-rate field or particle data.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} proofKind=f32-field-force-arithmetic computeStage=field-particle-force-prototype resourceKind={} resourceId={} fieldResourceId={} resourcePlane={} computeProbeBackend={} particleForceSource={} particleSamplingAuthority={} particleFieldSource={} particleRows={} visualRows={} topologyVertexCount={} topologyTriangleCount={} sourceFrameIndex={} cpuOracle={} cpuOraclePreserved=true preflightSchema={} readbackPolicy={} readbackProbeCount={} oraclePayload={} sampleCount={} componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} commandEncoderSubmitted=true storageBufferResident=true computeDispatchSubmitted=true prototypeComputeKernel=true forceArithmeticKernel=true fieldSamplingKernel=false fieldParticleKernel=false computeKernel=true gpuComputeReady=false highRateJsonPayload=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            if self.readback.readback_matched() {
                "ready"
            } else {
                "mismatch"
            },
            self.resource_kind.marker_value(),
            self.resource_kind.resource_id(),
            self.resource_kind.field_resource_id(),
            QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_BACKEND,
            self.force_source.marker_value(),
            self.force_source.sampling_authority_marker(),
            self.force_source.field_source_marker(),
            self.particle_rows,
            self.visual_rows,
            self.topology_vertex_count,
            self.topology_triangle_count,
            optional_usize_marker_token(self.source_frame_index),
            self.force_source.sampling_authority_marker(),
            QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_SCHEMA_ID,
            QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_READBACK_POLICY,
            self.readback_probe_count,
            QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_PAYLOAD,
            self.readback.sample_count,
            self.readback.component_count,
            self.readback.mismatched_components,
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
            QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_MEASUREMENT_SOURCE,
        )
    }
}
