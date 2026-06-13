use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token},
    QuestMakepadGpuFieldParticleForceProbe,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
    QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_CPU_ORACLE,
    QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_SCHEMA_ID,
};

/// Quest Makepad GPU force-authority candidate schema.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_force_authority_candidate.v1";
/// Quest Makepad GPU force-authority candidate marker prefix.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE";
/// Candidate force authority proven only as a bounded non-authoritative readback.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_KIND: &str =
    "gpu-dense-sdf-field-particle-force";
/// Source of the currently preserved active runtime authority.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_ACTIVE_AUTHORITY_SOURCE: &str =
    "matter-runtime-profile";

/// Non-authoritative promotion candidate derived from bounded resident-field particle-force proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuForceAuthorityCandidate {
    /// Schema identifier.
    pub schema_id: String,
    /// Source bounded GPU particle-force proof.
    pub source_probe: QuestMakepadGpuFieldParticleForceProbe,
}

impl QuestMakepadGpuForceAuthorityCandidate {
    /// Builds a candidate marker only after the bounded proof matches the Matter CPU oracle.
    #[must_use]
    pub fn from_particle_force_probe(
        source_probe: &QuestMakepadGpuFieldParticleForceProbe,
    ) -> Option<Self> {
        if !source_probe.runtime_particle_force_comparison_ready() {
            return None;
        }
        Some(Self {
            schema_id: QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_SCHEMA_ID.to_owned(),
            source_probe: source_probe.clone(),
        })
    }

    /// True when this GPU force path is ready only as a non-authoritative candidate.
    #[must_use]
    pub fn force_authority_candidate_ready(&self) -> bool {
        self.source_probe.runtime_particle_force_comparison_ready()
    }

    /// Builds a compact marker without logging particle rows, fields, or GPU buffers.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        let receipt = &self.source_probe.receipt;
        let input = &self.source_probe.input;
        let readback = self.source_probe.readback;
        let candidate_ready = self.force_authority_candidate_ready();
        format!(
            "{} schema={} phase={} status={} proofKind=non-authoritative-gpu-force-authority-candidate candidateForceAuthority={} activeForceAuthoritySource={} activeForceAuthorityChanged=false activeForceAuthorityPreserved=matter-cpu-runtime singleActiveForceAuthorityPreserved=true sourceProbeSchema={} sourceReceiptSchema={} sourceId={} sourceFrameIndex={} fieldResourceId={} fieldKind={} validationInputShape={} candidateResourcePlane={} sourceResourcePlane={} particleSampleSource=matter-particle-snapshot sourceParticleSetId={} particleRows={} requestedParticleSampleCount={} sampledParticleCount={} rejectedParticleCount={} sampleCount={} componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} runtimeFieldBoundaryReady={} runtimeParticleForceComparisonReady={} sourceFieldGeneration={} expectedSourceFieldGeneration={} sourceFieldGenerationMatched={} sourceFieldBufferResident={} sourceFieldBufferBytes={} expectedSourceFieldBufferBytes={} sampleInputBufferBytes={} sampleOutputBufferBytes={} cpuOracle={} cpuOraclePreserved=true recordedInputEquivalent=true residentFieldBufferSampled=true denseSdfConstructedOnGpu=true matterCpuParticleIntegration=true matterParticleForceEquation=true fieldSamplingKernel=true fieldForceSamplingKernel=true fieldParticleKernel=true computeKernel=true commandEncoderSubmitted=true computeDispatchSubmitted=true gpuComputeCandidateReady={} forceAuthorityCandidateReady={} candidateSelected=false candidatePromoted=false forceAuthorityReady=false runtimeForceAuthority=false runtimeParticleIntegration=false gpuComputeReady=false highRateJsonPayload=false settingsControlPayload=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} programGeneration={} programReused={} shaderCompiledThisSubmit={} pipelineCreatedThisSubmit={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            if candidate_ready { "candidate-ready" } else { "not-ready" },
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_KIND,
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_ACTIVE_AUTHORITY_SOURCE,
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_SCHEMA_ID,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
            sanitize_marker_value(&receipt.source_id),
            receipt.source_frame_index,
            sanitize_marker_value(&receipt.field_resource_id),
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
            sanitize_marker_value(&input.source_set_id),
            input.particle_rows,
            input.requested_sample_count,
            input.sample_count,
            input.rejected_count,
            readback.sample_count,
            readback.component_count,
            readback.mismatched_components,
            finite_f32_marker_token(readback.max_abs_error),
            finite_f32_marker_token(readback.tolerance),
            readback.readback_matched(),
            receipt.runtime_field_boundary_ready(),
            self.source_probe.runtime_particle_force_comparison_ready(),
            readback.source_field_generation,
            receipt.derived_buffer_generation,
            readback.source_field_generation == receipt.derived_buffer_generation,
            readback.source_field_buffer_resident,
            readback.source_field_buffer_bytes,
            receipt.sdf_distance_buffer_bytes,
            readback.sample_input_buffer_bytes,
            readback.sample_output_buffer_bytes,
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_CPU_ORACLE,
            candidate_ready,
            candidate_ready,
            readback.queue_submit_serial,
            readback.fence_serial,
            readback.resource_generation,
            readback.program_generation,
            readback.program_reused,
            readback.shader_compiled_this_submit,
            readback.pipeline_created_this_submit,
            readback.pending_retire_count,
            readback.retained_resource_count,
            readback.retired_after_fence_count,
            readback.queue_wait_idle_performed,
            finite_f64_marker_token(readback.elapsed_ms),
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_MEASUREMENT_SOURCE,
        )
    }
}
