use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token},
    QuestMakepadGpuForceAuthorityGate, QuestMakepadRuntimeForceAuthoritySelection,
    QUEST_MAKEPAD_FORCE_AUTHORITY_ROLLBACK_POLICY,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
    QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_CPU_ORACLE,
    QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_KIND,
    QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_POLICY,
};

/// Quest Makepad GPU force-authority residency-health schema.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_force_authority_residency_health.v1";
/// Quest Makepad GPU force-authority residency-health marker prefix.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY";
/// Health kind for the steady-state GPU force-authority promotion boundary.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_KIND: &str =
    "steady-state-gpu-force-authority-residency-health";
/// Minimum reused resident proofs before a GPU force path can be considered steady-state.
pub const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_REQUIRED_PROOFS: usize = 4;

const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_REQUESTED: &str =
    "profile-prefers-matter-cpu";
const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_STEADY_STATE: &str =
    "gpu-residency-health-not-steady-state";
const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_FRESH: &str =
    "gpu-freshness-not-proven";
const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_CADENCE: &str =
    "gpu-cadence-not-proven";
const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_EXPANDED_ORACLE: &str =
    "gpu-expanded-oracle-comparison-not-proven";
const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_PROVIDER_AB: &str =
    "gpu-live-recorded-provider-ab-not-proven";
const QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_RUNTIME_GUARD: &str =
    "gpu-runtime-selection-guarded";

/// Promotion evidence that must be present before a GPU force-authority equivalent may be selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuestMakepadGpuForceAuthorityPromotionEvidence {
    /// Number of resident GPU field/force proofs observed by the runtime.
    pub observed_resident_proofs: usize,
    /// True when GPU data freshness is tracked against current frame adoption.
    pub freshness_ready: bool,
    /// True when GPU force cadence is tracked and inside the profile budget.
    pub cadence_ready: bool,
    /// True when CPU-oracle comparison covers the steady-state sample set.
    pub expanded_oracle_comparison_ready: bool,
    /// True when live OpenXR hands and recorded replay pass through the same provider boundary.
    pub live_recorded_provider_ab_ready: bool,
}

impl QuestMakepadGpuForceAuthorityPromotionEvidence {
    /// Conservative bounded-proof evidence. This never permits runtime GPU authority.
    #[must_use]
    pub const fn bounded(observed_resident_proofs: usize) -> Self {
        Self {
            observed_resident_proofs,
            freshness_ready: false,
            cadence_ready: false,
            expanded_oracle_comparison_ready: false,
            live_recorded_provider_ab_ready: false,
        }
    }
}

/// Conservative steady-state health receipt for a GPU force-authority gate.
///
/// This is a low-rate adapter receipt. It records why a selected GPU-backed
/// equivalent remains non-authoritative until residency, freshness, cadence,
/// expanded CPU-oracle comparisons, and live-vs-recorded provider evidence are
/// all available.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuForceAuthorityResidencyHealth {
    /// Schema identifier.
    pub schema_id: String,
    /// Source force-authority gate.
    pub gate: QuestMakepadGpuForceAuthorityGate,
    /// Number of bounded resident-field force proofs represented by this health receipt.
    pub observed_resident_proofs: usize,
    /// Minimum resident-field force proof count required before steady-state promotion.
    pub required_resident_proofs: usize,
    /// True when GPU data freshness is tracked against current frame adoption.
    pub freshness_ready: bool,
    /// True when GPU force cadence is tracked and inside the profile budget.
    pub cadence_ready: bool,
    /// True when CPU-oracle comparison covers the steady-state sample set.
    pub expanded_oracle_comparison_ready: bool,
    /// True when live OpenXR hands and recorded replay pass through the same provider boundary.
    pub live_recorded_provider_ab_ready: bool,
}

impl QuestMakepadGpuForceAuthorityResidencyHealth {
    /// Builds conservative health from a profile gate.
    #[must_use]
    pub fn from_gate(gate: &QuestMakepadGpuForceAuthorityGate) -> Self {
        Self::from_gate_with_observed_proofs(gate, 1)
    }

    /// Builds health with an explicit observed proof count for future steady-state trackers.
    #[must_use]
    pub fn from_gate_with_observed_proofs(
        gate: &QuestMakepadGpuForceAuthorityGate,
        observed_resident_proofs: usize,
    ) -> Self {
        Self::from_gate_with_promotion_evidence(
            gate,
            QuestMakepadGpuForceAuthorityPromotionEvidence::bounded(observed_resident_proofs),
        )
    }

    /// Builds health from explicit promotion evidence.
    #[must_use]
    pub fn from_gate_with_promotion_evidence(
        gate: &QuestMakepadGpuForceAuthorityGate,
        evidence: QuestMakepadGpuForceAuthorityPromotionEvidence,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_SCHEMA_ID.to_owned(),
            gate: gate.clone(),
            observed_resident_proofs: evidence.observed_resident_proofs,
            required_resident_proofs: QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_REQUIRED_PROOFS,
            freshness_ready: evidence.freshness_ready,
            cadence_ready: evidence.cadence_ready,
            expanded_oracle_comparison_ready: evidence.expanded_oracle_comparison_ready,
            live_recorded_provider_ab_ready: evidence.live_recorded_provider_ab_ready,
        }
    }

    /// True when the bounded candidate evidence exists.
    #[must_use]
    pub fn evidence_ready(&self) -> bool {
        self.gate.profile_gate_ready()
    }

    /// True once repeated resident proofs establish a steady-state path.
    #[must_use]
    pub fn steady_state_residency_ready(&self) -> bool {
        self.evidence_ready()
            && self.gate.profile_gate_satisfied()
            && self.observed_resident_proofs >= self.required_resident_proofs
            && self.source_probe_reused()
            && self.derived_probe_reused()
            && self.gate.candidate.source_probe.readback.program_reused
    }

    /// True once GPU freshness is tracked against runtime frame/adoption cadence.
    #[must_use]
    pub const fn freshness_ready(&self) -> bool {
        self.freshness_ready
    }

    /// True once GPU force cadence is tracked and within the profile budget.
    #[must_use]
    pub const fn cadence_ready(&self) -> bool {
        self.cadence_ready
    }

    /// True once validation exceeds the current bounded proof samples.
    #[must_use]
    pub const fn expanded_oracle_comparison_ready(&self) -> bool {
        self.expanded_oracle_comparison_ready
    }

    /// True once live Quest hands and recorded replay prove the same provider boundary.
    #[must_use]
    pub const fn live_recorded_provider_ab_ready(&self) -> bool {
        self.live_recorded_provider_ab_ready
    }

    /// Exclusive runtime selection derived from this health receipt.
    #[must_use]
    pub fn runtime_selection(&self) -> QuestMakepadRuntimeForceAuthoritySelection {
        QuestMakepadRuntimeForceAuthoritySelection::from_residency_health(self)
    }

    /// Runtime selection stays blocked until every promotion prerequisite is met.
    #[must_use]
    pub fn runtime_selection_permitted(&self) -> bool {
        self.steady_state_residency_ready()
            && self.freshness_ready()
            && self.cadence_ready()
            && self.expanded_oracle_comparison_ready()
            && self.live_recorded_provider_ab_ready()
    }

    /// Reason that the active runtime authority remains the Matter CPU oracle.
    #[must_use]
    pub fn fallback_reason(&self) -> &'static str {
        if !self.gate.profile_gate_satisfied() {
            return QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_REQUESTED;
        }
        if !self.steady_state_residency_ready() {
            return QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_STEADY_STATE;
        }
        if !self.freshness_ready() {
            return QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_FRESH;
        }
        if !self.cadence_ready() {
            return QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_CADENCE;
        }
        if !self.expanded_oracle_comparison_ready() {
            return QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_EXPANDED_ORACLE;
        }
        if !self.live_recorded_provider_ab_ready() {
            return QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_NOT_PROVIDER_AB;
        }
        QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_FALLBACK_RUNTIME_GUARD
    }

    /// Builds a compact marker without logging particle rows, fields, or GPU buffers.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        let source_probe = &self.gate.candidate.source_probe;
        let receipt = &source_probe.receipt;
        let input = &source_probe.input;
        let readback = source_probe.readback;
        let evidence_ready = self.evidence_ready();
        let steady_state_ready = self.steady_state_residency_ready();
        let freshness_ready = self.freshness_ready();
        let cadence_ready = self.cadence_ready();
        let expanded_oracle_ready = self.expanded_oracle_comparison_ready();
        let provider_ab_ready = self.live_recorded_provider_ab_ready();
        let runtime_selection_permitted = self.runtime_selection_permitted();
        let selection = self.runtime_selection();
        let candidate_selected = selection.gpu_authority_selected();
        let active_force_source = self.gate.active_force_source.marker_value();
        let active_force_authority_source = if candidate_selected {
            "quest-makepad-gpu-runtime-selector"
        } else {
            "matter-runtime-profile"
        };
        let active_matter_force_authority = selection.active_matter_force_authority_marker();
        let force_authority_ready = candidate_selected;
        let bounded_proof_only = !runtime_selection_permitted;
        format!(
            "{} schema={} phase={} status={} healthKind={} requestedForceAuthority={} candidateForceAuthority={} candidateSchema={} activeForceAuthorityKind={} activeForceAuthoritySource={} activeMatterForceAuthority={} matterCpuOracleForceAuthority={} activeForceAuthorityChanged=false activeForceAuthorityPreserved={} singleActiveForceAuthorityPreserved=true forceAuthoritySlotCount=1 activeForceAuthorityCount={} profileGate={} profileGateSatisfied={} gpuForceAuthorityProfileKnown=true gpuForceAuthorityProfileEnabled={} candidateEligible={} candidateSelected={} candidatePromoted={} observedResidentProofs={} requiredResidentProofs={} boundedProofOnly={} steadyStateResidencyReady={} freshnessReady={} cadenceReady={} expandedOracleComparisonReady={} liveRecordedProviderAbReady={} runtimeSelectionPermitted={} fallbackForceAuthority={} fallbackReason={} matterCpuFallbackReady={} rollbackPolicy={} sourceReceiptSchema={} sourceId={} sourceFrameIndex={} fieldResourceId={} fieldKind={} validationInputShape={} candidateResourcePlane={} sourceResourcePlane={} particleSampleSource=matter-particle-snapshot sourceParticleSetId={} particleRows={} requestedParticleSampleCount={} sampledParticleCount={} rejectedParticleCount={} sampleCount={} componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} runtimeFieldBoundaryReady={} runtimeParticleForceComparisonReady={} sourceFieldGeneration={} expectedSourceFieldGeneration={} sourceFieldGenerationMatched={} sourceFieldBufferResident={} sourceFieldBufferBytes={} expectedSourceFieldBufferBytes={} sampleInputBufferBytes={} sampleOutputBufferBytes={} cpuOracle={} cpuOraclePreserved=true recordedInputEquivalent=true residentFieldBufferSampled=true denseSdfConstructedOnGpu=true matterCpuParticleIntegration=true matterParticleForceEquation=true fieldSamplingKernel=true fieldForceSamplingKernel=true fieldParticleKernel=true computeKernel=true commandEncoderSubmitted=true computeDispatchSubmitted=true sourceMeshBuffersResident={} sourceMeshBuffersReused={} derivedBuffersResident={} derivedBuffersReused={} gpuComputeCandidateReady={} forceAuthorityCandidateReady={} forceAuthorityReady={} runtimeForceAuthority={} runtimeParticleIntegration={} gpuComputeReady={} highRateJsonPayload=false settingsControlPayload=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} programGeneration={} programReused={} shaderCompiledThisSubmit={} pipelineCreatedThisSubmit={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy=RUSTY_QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE.elapsedMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs",
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            if runtime_selection_permitted {
                "runtime-selectable"
            } else if evidence_ready {
                "fallback-matter-cpu"
            } else {
                "not-ready"
            },
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_RESIDENCY_KIND,
            self.gate.requested_authority.as_str(),
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_KIND,
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_CANDIDATE_SCHEMA_ID,
            selection.active_authority.as_str(),
            active_force_authority_source,
            active_matter_force_authority,
            active_force_source,
            if candidate_selected {
                "gpu-backed-runtime"
            } else {
                "matter-cpu-runtime"
            },
            selection.active_authority_count,
            QUEST_MAKEPAD_GPU_FORCE_AUTHORITY_GATE_POLICY,
            self.gate.profile_gate_satisfied(),
            self.gate.requested_authority.gpu_profile_enabled(),
            evidence_ready,
            candidate_selected,
            candidate_selected,
            self.observed_resident_proofs,
            self.required_resident_proofs,
            bounded_proof_only,
            steady_state_ready,
            freshness_ready,
            cadence_ready,
            expanded_oracle_ready,
            provider_ab_ready,
            runtime_selection_permitted,
            active_force_source,
            selection.decision_reason,
            selection.matter_cpu_fallback_ready,
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
            receipt.source_mesh_buffers_resident,
            receipt.source_mesh_buffers_reused,
            receipt.derived_buffers_resident,
            receipt.derived_buffers_reused,
            evidence_ready,
            evidence_ready,
            force_authority_ready,
            runtime_selection_permitted,
            runtime_selection_permitted,
            runtime_selection_permitted,
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

    fn source_probe_reused(&self) -> bool {
        self.gate
            .candidate
            .source_probe
            .receipt
            .source_mesh_buffers_reused
    }

    fn derived_probe_reused(&self) -> bool {
        self.gate
            .candidate
            .source_probe
            .receipt
            .derived_buffers_reused
    }
}
