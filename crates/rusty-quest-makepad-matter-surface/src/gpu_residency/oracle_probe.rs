use rusty_matter_surface_runtime::MatterSurfaceParticleForceSource;

use crate::sanitize_marker_value;

use super::{
    marker::{finite_f64_marker_token, optional_usize_marker_token, u32_words_marker_token},
    preflight::{QuestMakepadGpuComputePreflight, QuestMakepadGpuComputeResourceKind},
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_READBACK_POLICY,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_SCHEMA_ID, QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_BACKEND,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_PAYLOAD,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_SCHEMA_ID, QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS,
};

/// Generic Makepad GPU u32 compute readback result consumed by the adapter marker.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuOracleComputeProbeReadback {
    /// Bounded input words derived from the Matter CPU oracle frame.
    pub input_words: [u32; QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS],
    /// GPU output words read back after the prototype compute dispatch.
    pub output_words: [u32; QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS],
    /// CPU-expected words for the same bounded probe transform.
    pub expected_words: [u32; QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS],
    /// Number of checked u32 words.
    pub word_count: usize,
    /// Number of output words that did not match the CPU-expected value.
    pub mismatched_words: usize,
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

impl QuestMakepadGpuOracleComputeProbeReadback {
    /// True when the bounded GPU output matched the CPU-expected probe transform.
    #[must_use]
    pub fn readback_matched(self) -> bool {
        self.word_count == QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS
            && self.mismatched_words == 0
            && self.output_words == self.expected_words
    }
}

/// Prototype compute dispatch tied to a Matter field-force oracle.
///
/// This proves shader dispatch and bounded GPU-vs-CPU readback over compact
/// oracle-derived words. It still does not move SDF/ADF/particle force
/// semantics out of Matter.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuOracleComputeProbe {
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
    /// Bounded readback probe count reserved for future GPU-vs-CPU oracle checks.
    pub readback_probe_count: usize,
    /// Makepad prototype compute readback result.
    pub readback: QuestMakepadGpuOracleComputeProbeReadback,
}

impl QuestMakepadGpuOracleComputeProbe {
    /// Builds a prototype compute probe marker from the current compute preflight.
    #[must_use]
    pub fn from_preflight(
        preflight: &QuestMakepadGpuComputePreflight,
        readback: QuestMakepadGpuOracleComputeProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_SCHEMA_ID.to_owned(),
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
            "{} schema={} phase={} status={} proofKind=u32-oracle-compute computeStage=field-particle-force-prototype resourceKind={} resourceId={} fieldResourceId={} resourcePlane={} computeProbeBackend={} particleForceSource={} particleSamplingAuthority={} particleFieldSource={} particleRows={} visualRows={} topologyVertexCount={} topologyTriangleCount={} sourceFrameIndex={} cpuOracle={} cpuOraclePreserved=true preflightSchema={} readbackPolicy={} readbackProbeCount={} oraclePayload={} oracleWordCount={} oracleInputWords={} gpuOutputWords={} cpuExpectedWords={} mismatchedWords={} readbackMatched={} commandEncoderSubmitted=true storageBufferResident=true computeDispatchSubmitted=true prototypeComputeKernel=true fieldParticleKernel=false computeKernel=true gpuComputeReady=false highRateJsonPayload=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_MARKER_PREFIX,
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
            QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_BACKEND,
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
            QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_PAYLOAD,
            self.readback.word_count,
            u32_words_marker_token(&self.readback.input_words),
            u32_words_marker_token(&self.readback.output_words),
            u32_words_marker_token(&self.readback.expected_words),
            self.readback.mismatched_words,
            self.readback.readback_matched(),
            self.readback.queue_submit_serial,
            self.readback.fence_serial,
            self.readback.resource_generation,
            self.readback.pending_retire_count,
            self.readback.retained_resource_count,
            self.readback.retired_after_fence_count,
            self.readback.queue_wait_idle_performed,
            finite_f64_marker_token(self.readback.elapsed_ms),
            QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_MEASUREMENT_SOURCE,
        )
    }
}
