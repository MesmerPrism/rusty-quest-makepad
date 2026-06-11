use rusty_matter_surface_runtime::MatterSurfaceParticleForceSource;

use crate::sanitize_marker_value;

use super::{
    marker::{finite_f64_marker_token, hex_u32_marker_token, optional_usize_marker_token},
    preflight::{QuestMakepadGpuComputePreflight, QuestMakepadGpuComputeResourceKind},
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_READBACK_POLICY,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_SCHEMA_ID, QUEST_MAKEPAD_GPU_STORAGE_PROBE_BACKEND,
    QUEST_MAKEPAD_GPU_STORAGE_PROBE_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_STORAGE_PROBE_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_STORAGE_PROBE_RESOURCE_PLANE, QUEST_MAKEPAD_GPU_STORAGE_PROBE_SCHEMA_ID,
};

/// Generic Makepad GPU storage-buffer readback result consumed by the adapter marker.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuStorageProbeReadback {
    /// Requested probe byte count.
    pub requested_bytes: usize,
    /// Allocated storage-buffer byte count.
    pub storage_buffer_bytes: usize,
    /// Allocated readback byte count.
    pub readback_bytes: usize,
    /// Pattern written by the GPU command.
    pub pattern: u32,
    /// First u32 read back from the buffer.
    pub first_word: u32,
    /// Number of checked u32 words.
    pub word_count: usize,
    /// Number of words that did not match the pattern.
    pub mismatched_words: usize,
    /// CPU-side elapsed time for command submission, wait, and readback.
    pub elapsed_ms: f64,
}

impl QuestMakepadGpuStorageProbeReadback {
    /// True when the bounded readback matched the submitted pattern.
    #[must_use]
    pub fn readback_matched(self) -> bool {
        self.word_count > 0 && self.mismatched_words == 0 && self.first_word == self.pattern
    }
}

/// Storage-buffer command/readback probe tied to a Matter field-force oracle.
///
/// This proves the Quest/Makepad adapter can submit a small storage-capable GPU
/// buffer command and read it back. It still does not claim the field/particle
/// force kernel has moved to GPU.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuStorageProbe {
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
    /// Makepad storage-buffer readback result.
    pub readback: QuestMakepadGpuStorageProbeReadback,
}

impl QuestMakepadGpuStorageProbe {
    /// Builds a storage-buffer probe marker from the current compute preflight.
    #[must_use]
    pub fn from_preflight(
        preflight: &QuestMakepadGpuComputePreflight,
        readback: QuestMakepadGpuStorageProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_STORAGE_PROBE_SCHEMA_ID.to_owned(),
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
            "{} schema={} phase={} status={} computeStage=field-particle-force resourceKind={} resourceId={} fieldResourceId={} resourcePlane={} storageProbeBackend={} particleForceSource={} particleSamplingAuthority={} particleFieldSource={} particleRows={} visualRows={} topologyVertexCount={} topologyTriangleCount={} sourceFrameIndex={} cpuOracle={} cpuOraclePreserved=true preflightSchema={} readbackPolicy={} readbackProbeCount={} requestedBytes={} storageBufferBytes={} readbackBytes={} wordCount={} pattern={} firstWord={} mismatchedWords={} readbackMatched={} commandEncoderSubmitted=true storageBufferResident=true gpuCommandExecuted=true gpuComputeReady=false computeKernel=false highRateJsonPayload=false elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_STORAGE_PROBE_MARKER_PREFIX,
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
            QUEST_MAKEPAD_GPU_STORAGE_PROBE_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_STORAGE_PROBE_BACKEND,
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
            self.readback.requested_bytes,
            self.readback.storage_buffer_bytes,
            self.readback.readback_bytes,
            self.readback.word_count,
            hex_u32_marker_token(self.readback.pattern),
            hex_u32_marker_token(self.readback.first_word),
            self.readback.mismatched_words,
            self.readback.readback_matched(),
            finite_f64_marker_token(self.readback.elapsed_ms),
            QUEST_MAKEPAD_GPU_STORAGE_PROBE_MEASUREMENT_SOURCE,
        )
    }
}
