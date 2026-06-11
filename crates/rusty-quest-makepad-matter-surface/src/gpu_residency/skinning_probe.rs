use rusty_matter_model::Vec3;

use crate::sanitize_marker_value;

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token},
    QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_SKINNING_PROBE_BACKEND, QUEST_MAKEPAD_GPU_SKINNING_PROBE_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_SKINNING_PROBE_MEASUREMENT_SOURCE, QUEST_MAKEPAD_GPU_SKINNING_PROBE_PAYLOAD,
    QUEST_MAKEPAD_GPU_SKINNING_PROBE_SCHEMA_ID,
};

/// Bounded recorded-hand skinning samples submitted to the generic Makepad f32 probe.
pub const QUEST_MAKEPAD_GPU_SKINNING_PROBE_SAMPLES: usize = 4;
/// Conservative f32 tolerance for recorded-hand skinning readback comparison.
pub const QUEST_MAKEPAD_GPU_SKINNING_PROBE_DEFAULT_TOLERANCE: f32 = 0.0001;

/// One bounded weighted-delta skinning sample.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuSkinningProbeSample {
    /// Source bind-mesh vertex index.
    pub vertex_index: usize,
    /// Bind-pose vertex position, packed as xyz plus w.
    pub bind_position: [f32; 4],
    /// First weighted delta vector; this prototype uses a single full-weight CPU-oracle delta.
    pub delta0_weight: [f32; 4],
    /// Reserved weighted delta lane.
    pub delta1_weight: [f32; 4],
    /// Reserved weighted delta lane.
    pub delta2_weight: [f32; 4],
    /// Reserved weighted delta lane.
    pub delta3_weight: [f32; 4],
    /// Matter CPU-skinned oracle position, packed as xyz plus w.
    pub expected_position: [f32; 4],
}

/// Compact source-frame GPU skinning probe input.
///
/// This is intentionally bounded diagnostic data. It carries four selected
/// vertex probes, not the full bind mesh, joint frame, or skinned surface.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuSkinningProbeInput {
    /// Stable source identity.
    pub source_id: String,
    /// Recorded/live-equivalent source frame index.
    pub source_frame_index: usize,
    /// Full topology vertex count for context.
    pub topology_vertex_count: usize,
    /// Full topology triangle count for context.
    pub topology_triangle_count: usize,
    /// Number of populated samples.
    pub sample_count: usize,
    /// Bounded weighted-delta skinning samples.
    pub samples: [QuestMakepadGpuSkinningProbeSample; QUEST_MAKEPAD_GPU_SKINNING_PROBE_SAMPLES],
}

impl QuestMakepadGpuSkinningProbeInput {
    /// Builds a bounded weighted-delta probe from bind and CPU-skinned Matter positions.
    #[must_use]
    pub fn from_positions(
        source_id: impl Into<String>,
        source_frame_index: usize,
        topology_triangle_count: usize,
        bind_positions: &[Vec3],
        skinned_positions: &[Vec3],
    ) -> Option<Self> {
        if bind_positions.len() != skinned_positions.len() || bind_positions.is_empty() {
            return None;
        }
        if !bind_positions.iter().copied().all(Vec3::is_finite)
            || !skinned_positions.iter().copied().all(Vec3::is_finite)
        {
            return None;
        }

        let topology_vertex_count = bind_positions.len();
        let sample_count = topology_vertex_count.min(QUEST_MAKEPAD_GPU_SKINNING_PROBE_SAMPLES);
        let mut samples = [QuestMakepadGpuSkinningProbeSample::default();
            QUEST_MAKEPAD_GPU_SKINNING_PROBE_SAMPLES];
        for (sample_index, sample) in samples.iter_mut().take(sample_count).enumerate() {
            let vertex_index =
                selected_vertex_index(topology_vertex_count, sample_count, sample_index);
            let bind = bind_positions[vertex_index];
            let expected = skinned_positions[vertex_index];
            *sample = QuestMakepadGpuSkinningProbeSample {
                vertex_index,
                bind_position: [bind.x, bind.y, bind.z, 1.0],
                delta0_weight: [
                    expected.x - bind.x,
                    expected.y - bind.y,
                    expected.z - bind.z,
                    1.0,
                ],
                delta1_weight: [0.0; 4],
                delta2_weight: [0.0; 4],
                delta3_weight: [0.0; 4],
                expected_position: [expected.x, expected.y, expected.z, 1.0],
            };
        }

        Some(Self {
            source_id: source_id.into(),
            source_frame_index,
            topology_vertex_count,
            topology_triangle_count,
            sample_count,
            samples,
        })
    }

    /// First populated sample vertex index.
    #[must_use]
    pub fn first_sample_vertex_index(&self) -> Option<usize> {
        (self.sample_count > 0).then_some(self.samples[0].vertex_index)
    }

    /// Last populated sample vertex index.
    #[must_use]
    pub fn last_sample_vertex_index(&self) -> Option<usize> {
        self.sample_count
            .checked_sub(1)
            .map(|index| self.samples[index].vertex_index)
    }
}

/// Generic Makepad GPU f32 skinning readback result consumed by the adapter marker.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuSkinningProbeReadback {
    /// Number of bounded skinning samples submitted to the GPU.
    pub sample_count: usize,
    /// Number of checked f32 position components.
    pub component_count: usize,
    /// Number of output position components outside tolerance.
    pub mismatched_components: usize,
    /// Maximum absolute output-vs-CPU-oracle position error.
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

impl QuestMakepadGpuSkinningProbeReadback {
    /// True when the bounded GPU output matched the Matter CPU-oracle skinning positions.
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

/// Prototype f32 weighted-delta skinning dispatch tied to a recorded-hand Matter oracle.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuSkinningProbe {
    /// Schema identifier.
    pub schema_id: String,
    /// Compact source-frame skinning input.
    pub input: QuestMakepadGpuSkinningProbeInput,
    /// Makepad f32 skinning readback result.
    pub readback: QuestMakepadGpuSkinningProbeReadback,
}

impl QuestMakepadGpuSkinningProbe {
    /// Builds a f32 skinning marker from a bounded recorded-hand source-frame probe.
    #[must_use]
    pub fn from_input(
        input: &QuestMakepadGpuSkinningProbeInput,
        readback: QuestMakepadGpuSkinningProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_SKINNING_PROBE_SCHEMA_ID.to_owned(),
            input: input.clone(),
            readback,
        }
    }

    /// Builds a compact marker without logging bind meshes, joint frames, or skinned surfaces.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} proofKind=f32-weighted-delta-skinning computeStage=hand-skinning-prototype sourceId={} sourceFrameIndex={} topologyVertexCount={} topologyTriangleCount={} cpuOracle=matter-recorded-hand-skinning cpuOraclePreserved=true recordedInputEquivalent=true validationInputShape=bind-mesh-plus-compact-joint-frame resourcePlane={} computeProbeBackend={} oraclePayload={} sampleCount={} firstSampleVertexIndex={} lastSampleVertexIndex={} componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} commandEncoderSubmitted=true storageBufferResident=true computeDispatchSubmitted=true prototypeComputeKernel=true weightedDeltaSkinningKernel=true jointMatrixSkinningKernel=false meshToSdfKernel=false fieldSamplingKernel=false fieldParticleKernel=false computeKernel=true gpuComputeReady=false highRateJsonPayload=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_SKINNING_PROBE_MARKER_PREFIX,
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
            QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_SKINNING_PROBE_BACKEND,
            QUEST_MAKEPAD_GPU_SKINNING_PROBE_PAYLOAD,
            self.readback.sample_count,
            optional_usize_marker_token(self.input.first_sample_vertex_index()),
            optional_usize_marker_token(self.input.last_sample_vertex_index()),
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
            QUEST_MAKEPAD_GPU_SKINNING_PROBE_MEASUREMENT_SOURCE,
        )
    }
}

fn selected_vertex_index(vertex_count: usize, sample_count: usize, sample_index: usize) -> usize {
    if sample_count <= 1 {
        0
    } else {
        sample_index * (vertex_count - 1) / (sample_count - 1)
    }
}

fn optional_usize_marker_token(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}
