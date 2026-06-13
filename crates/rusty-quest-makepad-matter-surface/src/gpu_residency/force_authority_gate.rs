use rusty_matter_surface_runtime::MatterSurfaceParticleForceSource;

use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token},
    QuestMakepadForceAuthorityMode, QuestMakepadGpuForceAuthorityCandidate,
    QUEST_MAKEPAD_FORCE_AUTHORITY_ROLLBACK_POLICY,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
    QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_CPU_ORACLE,
    QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_ACTIVE_AUTHORITY_SOURCE,
    QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_KIND,
    QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_SCHEMA_ID,
};

/// Quest Makepad GPU force-authority gate schema.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_force_authority_gate.v1";
/// Quest Makepad GPU force-authority gate marker prefix.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE";
/// Gate kind for the adapter-side force-authority promotion boundary.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_KIND: &str = "single-authority-profile-gate";
/// Profile gate required before the GPU candidate may become runtime authority.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_POLICY: &str = "explicit-profile-required";
const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_FALLBACK_NOT_REQUESTED: &str =
    "profile-prefers-matter-cpu";
const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_FALLBACK_NOT_READY: &str =
    "gpu-steady-state-residency-not-ready";

/// Adapter-side gate receipt for a GPU force-authority candidate.
///
/// This remains a low-rate evidence contract. It records the selected
/// adapter-level force-authority profile and keeps exactly one active runtime
/// authority. Until steady-state GPU residency, freshness, cadence, and
/// rollback evidence are implemented, Matter's CPU oracle remains active even
/// when the GPU profile gate is explicitly requested.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuForceAuthorityGate {
    /// Schema identifier.
    pub schema_id: String,
    /// Source non-authoritative GPU candidate.
    pub candidate: QuestMakepadGpuForceAuthorityCandidate,
    /// Requested adapter-level force-authority mode.
    pub requested_authority: QuestMakepadForceAuthorityMode,
    /// Active Matter CPU force source preserved for this frame.
    pub active_force_source: MatterSurfaceParticleForceSource,
}

impl QuestMakepadGpuForceAuthorityGate {
    /// Builds a gate only after the GPU candidate is ready.
    #[must_use]
    pub fn from_candidate(
        candidate: &QuestMakepadGpuForceAuthorityCandidate,
        active_force_source: MatterSurfaceParticleForceSource,
        requested_authority: QuestMakepadForceAuthorityMode,
    ) -> Option<Self> {
        if !candidate.force_authority_candidate_ready() {
            return None;
        }
        Some(Self {
            schema_id: QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_SCHEMA_ID.to_owned(),
            candidate: candidate.clone(),
            requested_authority,
            active_force_source,
        })
    }

    /// True when the candidate is valid but runtime promotion is still gated.
    #[must_use]
    pub fn profile_gate_ready(&self) -> bool {
        self.candidate.force_authority_candidate_ready()
    }

    /// True when the low-rate profile explicitly asks for the GPU equivalent.
    #[must_use]
    pub const fn profile_gate_satisfied(&self) -> bool {
        self.requested_authority.gpu_profile_enabled()
    }

    /// Reason that the active runtime authority remains the Matter CPU oracle.
    #[must_use]
    pub const fn fallback_reason(&self) -> &'static str {
        if self.profile_gate_satisfied() {
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_FALLBACK_NOT_READY
        } else {
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_FALLBACK_NOT_REQUESTED
        }
    }

    /// Builds a compact marker without logging particle rows, fields, or GPU buffers.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        let source_probe = &self.candidate.source_probe;
        let receipt = &source_probe.receipt;
        let input = &source_probe.input;
        let readback = source_probe.readback;
        let gate_ready = self.profile_gate_ready();
        let profile_gate_satisfied = self.profile_gate_satisfied();
        let active_force_source = self.active_force_source.marker_value();
        format!(
            "{} schema={} phase={} status={} gateKind={} requestedForceAuthority={} candidateForceAuthority={} candidateSchema={} activeForceAuthorityKind=matter-cpu activeForceAuthoritySource={} activeMatterForceAuthority={} activeForceAuthorityChanged=false activeForceAuthorityPreserved=matter-cpu-runtime singleActiveForceAuthorityPreserved=true forceAuthoritySlotCount=1 activeForceAuthorityCount=1 profileGate={} profileGateSatisfied={} runtimeSelectionPermitted=false gpuForceAuthorityProfileKnown=true gpuForceAuthorityProfileEnabled={} candidateEligible={} candidateSelected=false candidatePromoted=false fallbackForceAuthority={} fallbackReason={} matterCpuFallbackReady=true rollbackPolicy={} sourceReceiptSchema={} sourceId={} sourceFrameIndex={} fieldResourceId={} fieldKind={} validationInputShape={} candidateResourcePlane={} sourceResourcePlane={} particleSampleSource=matter-particle-snapshot sourceParticleSetId={} particleRows={} requestedParticleSampleCount={} sampledParticleCount={} rejectedParticleCount={} sampleCount={} componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} runtimeFieldBoundaryReady={} runtimeParticleForceComparisonReady={} sourceFieldGeneration={} expectedSourceFieldGeneration={} sourceFieldGenerationMatched={} sourceFieldBufferResident={} sourceFieldBufferBytes={} expectedSourceFieldBufferBytes={} sampleInputBufferBytes={} sampleOutputBufferBytes={} cpuOracle={} cpuOraclePreserved=true recordedInputEquivalent=true residentFieldBufferSampled=true denseSdfConstructedOnGpu=true matterCpuParticleIntegration=true matterParticleForceEquation=true fieldSamplingKernel=true fieldForceSamplingKernel=true fieldParticleKernel=true computeKernel=true commandEncoderSubmitted=true computeDispatchSubmitted=true gpuComputeCandidateReady={} forceAuthorityCandidateReady={} forceAuthorityReady=false runtimeForceAuthority=false runtimeParticleIntegration=false gpuComputeReady=false highRateJsonPayload=false settingsControlPayload=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} programGeneration={} programReused={} shaderCompiledThisSubmit={} pipelineCreatedThisSubmit={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy=RUSTY_QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE.elapsedMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs",
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            if gate_ready { "profile-gated" } else { "not-ready" },
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_KIND,
            self.requested_authority.as_str(),
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_KIND,
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_SCHEMA_ID,
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_ACTIVE_AUTHORITY_SOURCE,
            active_force_source,
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_POLICY,
            profile_gate_satisfied,
            self.requested_authority.gpu_profile_enabled(),
            gate_ready,
            active_force_source,
            self.fallback_reason(),
            QUEST_MAKEPAD_FORCE_AUTHORITY_ROLLBACK_POLICY,
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
            source_probe.runtime_particle_force_comparison_ready(),
            readback.source_field_generation,
            receipt.derived_buffer_generation,
            readback.source_field_generation == receipt.derived_buffer_generation,
            readback.source_field_buffer_resident,
            readback.source_field_buffer_bytes,
            receipt.sdf_distance_buffer_bytes,
            readback.sample_input_buffer_bytes,
            readback.sample_output_buffer_bytes,
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_CPU_ORACLE,
            gate_ready,
            gate_ready,
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
        )
    }
}
