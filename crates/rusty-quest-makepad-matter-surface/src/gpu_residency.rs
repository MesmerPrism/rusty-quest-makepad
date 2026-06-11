//! Compact GPU residency proof descriptors for Quest Makepad render-plane rows.
//!
//! This module does not introduce GPU compute or renderer ownership into
//! Matter. It records when an existing bounded data-plane batch is ready to be
//! adopted by a Makepad instanced draw path, while preserving Matter's CPU
//! reference as the source of truth.

use crate::{
    sanitize_marker_value, QuestMakepadMatterSurfaceFrame, QuestMakepadWorldAdfDebugBatch,
    QuestMakepadWorldParticleBatch,
};

use rusty_matter_surface_runtime::{
    MatterSurfaceParticleForceSource, MatterSurfaceParticleForceSourceStatus,
};

/// Quest Makepad GPU residency proof schema.
pub const QUEST_MAKEPAD_GPU_RESIDENCY_PROOF_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_residency_proof.v1";
/// Quest Makepad GPU residency proof marker prefix.
pub const QUEST_MAKEPAD_GPU_RESIDENCY_MARKER_PREFIX: &str = "RUSTY_QUEST_MAKEPAD_GPU_RESIDENCY";
/// Current first proof backend: Makepad instanced draw buffers.
pub const QUEST_MAKEPAD_GPU_RESIDENCY_BACKEND_MAKEPAD_INSTANCED_DRAW: &str =
    "makepad-xr-instanced-draw-buffer";
/// Render-plane resource class for this first residency proof.
pub const QUEST_MAKEPAD_GPU_RESIDENCY_RESOURCE_PLANE: &str = "render-gpu-instance-buffer";
/// Cadence marker fields that measure the backend-side upload/render result.
pub const QUEST_MAKEPAD_GPU_RESIDENCY_MEASUREMENT_SOURCE: &str =
    "RUSTY_MAKEPAD_CADENCE.xrRepaintGeometryUploadBytes,xrRepaintInstances,xrRepaintGpuMs";
/// Packed adapter payload stride for one particle instance row.
pub const QUEST_MAKEPAD_PARTICLE_GPU_RESIDENCY_ROW_STRIDE_BYTES: usize = 64;
/// Packed adapter payload stride for one ADF debug-cell row.
pub const QUEST_MAKEPAD_ADF_DEBUG_GPU_RESIDENCY_ROW_STRIDE_BYTES: usize = 48;
/// Quest Makepad GPU compute preflight schema.
pub const QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_compute_preflight.v1";
/// Quest Makepad GPU compute preflight marker prefix.
pub const QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT";
/// Resource plane that the next compute slice must provide.
pub const QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_RESOURCE_PLANE: &str = "future-storage-buffer";
/// Backend status for the current app boundary.
pub const QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_BACKEND_STATUS: &str =
    "makepad-command-encoder-pending";
/// Readback policy for future GPU-vs-CPU oracle validation.
pub const QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_READBACK_POLICY: &str = "bounded-cpu-oracle-probes";
/// Evidence fields that should be paired with a compute preflight marker.
pub const QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_MEASUREMENT_SOURCE: &str =
    "RUSTY_QUEST_MAKEPAD_MATTER_SURFACE_RUNTIME.particleStepMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs";
/// Conservative default number of oracle probes for future GPU readback checks.
pub const QUEST_MAKEPAD_GPU_COMPUTE_DEFAULT_READBACK_PROBE_COUNT: usize = 16;
/// Quest Makepad GPU storage-buffer command/readback probe schema.
pub const QUEST_MAKEPAD_GPU_STORAGE_PROBE_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_storage_probe.v1";
/// Quest Makepad GPU storage-buffer command/readback probe marker prefix.
pub const QUEST_MAKEPAD_GPU_STORAGE_PROBE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_STORAGE_PROBE";
/// Resource plane proven by the storage-buffer command/readback probe.
pub const QUEST_MAKEPAD_GPU_STORAGE_PROBE_RESOURCE_PLANE: &str =
    "vulkan-storage-buffer-command-readback";
/// Backend used by the current Makepad storage-buffer probe.
pub const QUEST_MAKEPAD_GPU_STORAGE_PROBE_BACKEND: &str =
    "makepad-vulkan-queue-submit-fill-copy-readback";
/// Measurement companion for the storage-buffer probe.
pub const QUEST_MAKEPAD_GPU_STORAGE_PROBE_MEASUREMENT_SOURCE: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_STORAGE_PROBE.elapsedMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs";
/// Conservative byte size for the current command/readback probe.
pub const QUEST_MAKEPAD_GPU_STORAGE_PROBE_DEFAULT_BYTES: usize = 64;
/// Deterministic pattern for the current command/readback probe.
pub const QUEST_MAKEPAD_GPU_STORAGE_PROBE_DEFAULT_PATTERN: u32 = 0x5DF0_ADF1;
/// Quest Makepad GPU oracle compute probe schema.
pub const QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_oracle_compute_probe.v1";
/// Quest Makepad GPU oracle compute probe marker prefix.
pub const QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE";
/// Resource plane proven by the prototype compute probe.
pub const QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_RESOURCE_PLANE: &str =
    "vulkan-compute-storage-buffer-readback";
/// Backend used by the current Makepad prototype compute probe.
pub const QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_BACKEND: &str =
    "makepad-vulkan-compute-u32-oracle-probe";
/// Measurement companion for the prototype compute probe.
pub const QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_MEASUREMENT_SOURCE: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE.elapsedMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs";
/// Current bounded oracle probe word count.
pub const QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS: usize = 4;
/// Marker payload type for the current bounded oracle probe.
pub const QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_PAYLOAD: &str = "bounded-matter-frame-u32-probes";

/// Payload family adopted by a GPU residency proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestMakepadGpuResidencyPayloadKind {
    /// Bounded world-particle instance rows.
    WorldParticles,
    /// Bounded world ADF debug-cell rows.
    WorldAdfDebugCells,
}

impl QuestMakepadGpuResidencyPayloadKind {
    /// Stable marker value.
    #[must_use]
    pub const fn marker_value(self) -> &'static str {
        match self {
            Self::WorldParticles => "world-particles",
            Self::WorldAdfDebugCells => "world-adf-debug-cells",
        }
    }

    /// Stable render/data-plane resource id.
    #[must_use]
    pub const fn resource_id(self) -> &'static str {
        match self {
            Self::WorldParticles => "quest.makepad.world_particles.instances",
            Self::WorldAdfDebugCells => "quest.makepad.world_adf_debug.cells",
        }
    }

    /// Adapter-row stride used by the bounded data-plane payload.
    #[must_use]
    pub const fn adapter_row_stride_bytes(self) -> usize {
        match self {
            Self::WorldParticles => QUEST_MAKEPAD_PARTICLE_GPU_RESIDENCY_ROW_STRIDE_BYTES,
            Self::WorldAdfDebugCells => QUEST_MAKEPAD_ADF_DEBUG_GPU_RESIDENCY_ROW_STRIDE_BYTES,
        }
    }
}

/// Compact proof that a bounded Quest-Makepad payload is ready for GPU adoption.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuResidencyProof {
    /// Schema identifier.
    pub schema_id: String,
    /// Payload family.
    pub payload_kind: QuestMakepadGpuResidencyPayloadKind,
    /// Source adapter schema identifier.
    pub source_schema_id: String,
    /// Renderer-facing primitive mode.
    pub render_mode: String,
    /// Renderer id that will submit the Makepad draw calls.
    pub renderer_id: String,
    /// Full source rows before any visual/draw cap.
    pub source_rows: usize,
    /// Rows submitted to the Makepad instanced draw path.
    pub resident_rows: usize,
    /// Rows dropped by the selected draw cap.
    pub dropped_rows: usize,
    /// Adapter payload bytes per resident row.
    pub adapter_row_stride_bytes: usize,
    /// Adapter payload bytes represented by the resident rows.
    pub adapter_payload_bytes: usize,
}

impl QuestMakepadGpuResidencyProof {
    /// Builds a proof for a world-particle batch.
    #[must_use]
    pub fn from_world_particle_batch(
        batch: &QuestMakepadWorldParticleBatch,
        drawn_instances: usize,
        renderer_id: impl Into<String>,
    ) -> Self {
        Self::new(
            QuestMakepadGpuResidencyPayloadKind::WorldParticles,
            batch.source_schema_id.clone(),
            batch.render_mode.clone(),
            renderer_id,
            batch.source_rows,
            drawn_instances.min(batch.instances.len()),
            batch.dropped_rows,
        )
    }

    /// Builds a proof for a world ADF debug-cell batch.
    #[must_use]
    pub fn from_world_adf_debug_batch(
        batch: &QuestMakepadWorldAdfDebugBatch,
        drawn_cells: usize,
        renderer_id: impl Into<String>,
    ) -> Self {
        Self::new(
            QuestMakepadGpuResidencyPayloadKind::WorldAdfDebugCells,
            batch.source_schema_id.clone(),
            batch.render_mode.clone(),
            renderer_id,
            batch.source_cells,
            drawn_cells.min(batch.cells.len()),
            batch.dropped_cells,
        )
    }

    fn new(
        payload_kind: QuestMakepadGpuResidencyPayloadKind,
        source_schema_id: String,
        render_mode: String,
        renderer_id: impl Into<String>,
        source_rows: usize,
        resident_rows: usize,
        dropped_rows: usize,
    ) -> Self {
        let adapter_row_stride_bytes = payload_kind.adapter_row_stride_bytes();
        Self {
            schema_id: QUEST_MAKEPAD_GPU_RESIDENCY_PROOF_SCHEMA_ID.to_owned(),
            payload_kind,
            source_schema_id,
            render_mode,
            renderer_id: renderer_id.into(),
            source_rows,
            resident_rows,
            dropped_rows,
            adapter_row_stride_bytes,
            adapter_payload_bytes: resident_rows.saturating_mul(adapter_row_stride_bytes),
        }
    }

    /// Builds a compact marker without logging high-rate payload rows.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} payloadKind={} resourceId={} resourcePlane={} residencyBackend={} renderer={} renderMode={} sourceSchema={} sourceRows={} residentRows={} droppedRows={} adapterRowStrideBytes={} adapterPayloadBytes={} gpuResident={} makepadInstancedDraw=true computeKernel=false matterCpuReferencePreserved=true highRateJsonPayload=false readbackPolicy=low-rate-cadence-markers measuredBy={}",
            QUEST_MAKEPAD_GPU_RESIDENCY_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            if self.resident_rows == 0 { "empty" } else { "ready" },
            self.payload_kind.marker_value(),
            self.payload_kind.resource_id(),
            QUEST_MAKEPAD_GPU_RESIDENCY_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_RESIDENCY_BACKEND_MAKEPAD_INSTANCED_DRAW,
            sanitize_marker_value(&self.renderer_id),
            sanitize_marker_value(&self.render_mode),
            sanitize_marker_value(&self.source_schema_id),
            self.source_rows,
            self.resident_rows,
            self.dropped_rows,
            self.adapter_row_stride_bytes,
            self.adapter_payload_bytes,
            self.resident_rows > 0,
            QUEST_MAKEPAD_GPU_RESIDENCY_MEASUREMENT_SOURCE,
        )
    }
}

/// Compute resource family covered by a GPU compute preflight marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestMakepadGpuComputeResourceKind {
    /// Particles driven by a Matter-owned dense SDF field.
    SdfParticleForces,
    /// Particles driven by a Matter-owned indexed ADF field.
    AdfParticleForces,
}

impl QuestMakepadGpuComputeResourceKind {
    /// Builds a resource kind from the active Matter force source.
    #[must_use]
    pub const fn from_force_source(force_source: MatterSurfaceParticleForceSource) -> Option<Self> {
        match force_source {
            MatterSurfaceParticleForceSource::SdfField => Some(Self::SdfParticleForces),
            MatterSurfaceParticleForceSource::AdfField => Some(Self::AdfParticleForces),
            MatterSurfaceParticleForceSource::MeshDistance
            | MatterSurfaceParticleForceSource::None => None,
        }
    }

    /// Stable marker value.
    #[must_use]
    pub const fn marker_value(self) -> &'static str {
        match self {
            Self::SdfParticleForces => "sdf-particle-forces",
            Self::AdfParticleForces => "adf-particle-forces",
        }
    }

    /// Stable future resource id.
    #[must_use]
    pub const fn resource_id(self) -> &'static str {
        match self {
            Self::SdfParticleForces => "quest.makepad.gpu_compute.sdf_particle_forces",
            Self::AdfParticleForces => "quest.makepad.gpu_compute.adf_particle_forces",
        }
    }

    /// Stable future field-buffer id.
    #[must_use]
    pub const fn field_resource_id(self) -> &'static str {
        match self {
            Self::SdfParticleForces => "quest.makepad.gpu_compute.sdf_force_field",
            Self::AdfParticleForces => "quest.makepad.gpu_compute.adf_force_field",
        }
    }

    /// Bounded u32 tag used by the prototype GPU oracle probe.
    #[must_use]
    pub const fn oracle_probe_tag(self) -> u32 {
        match self {
            Self::SdfParticleForces => 0x5DF0_0001,
            Self::AdfParticleForces => 0xADF0_0001,
        }
    }
}

/// Compact preflight for the future field/particle GPU compute boundary.
///
/// This is intentionally not a GPU compute proof. It records that the current
/// Matter frame has a CPU oracle and single field-force authority that a future
/// Quest/Makepad command-encoder/storage-buffer path can validate against.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuComputePreflight {
    /// Schema identifier.
    pub schema_id: String,
    /// Future GPU resource family.
    pub resource_kind: QuestMakepadGpuComputeResourceKind,
    /// Active Matter force source.
    pub force_source: MatterSurfaceParticleForceSource,
    /// Force-source refresh marker from the CPU oracle frame.
    pub force_refresh: String,
    /// Configured force refresh interval.
    pub force_update_interval_frames: usize,
    /// Full Matter particle count.
    pub particle_rows: usize,
    /// Renderer-facing particle rows available this frame.
    pub visual_rows: usize,
    /// Source mesh vertex count.
    pub topology_vertex_count: usize,
    /// Source mesh triangle count.
    pub topology_triangle_count: usize,
    /// Source frame index.
    pub source_frame_index: Option<usize>,
    /// Requested bounded readback probes for the future GPU path.
    pub readback_probe_count: usize,
}

impl QuestMakepadGpuComputePreflight {
    /// Builds a compute-resource preflight from a Matter surface frame.
    #[must_use]
    pub fn from_frame(
        frame: &QuestMakepadMatterSurfaceFrame,
        readback_probe_count: usize,
    ) -> Option<Self> {
        let particle_step = frame.particle_step.as_ref()?;
        if particle_step.particle_force_source_status
            != MatterSurfaceParticleForceSourceStatus::Ready
        {
            return None;
        }
        let resource_kind = QuestMakepadGpuComputeResourceKind::from_force_source(
            particle_step.particle_force_source,
        )?;
        let particle_rows = frame.stats.particle_count;
        if particle_rows == 0 {
            return None;
        }

        Some(Self {
            schema_id: QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_SCHEMA_ID.to_owned(),
            resource_kind,
            force_source: particle_step.particle_force_source,
            force_refresh: particle_step
                .particle_force_refresh
                .marker_value()
                .to_owned(),
            force_update_interval_frames: particle_step.particle_force_update_interval_frames,
            particle_rows,
            visual_rows: frame
                .particle_upload
                .as_ref()
                .map_or(0, |upload| upload.rows.len()),
            topology_vertex_count: frame.matter_update.vertex_count,
            topology_triangle_count: frame.matter_update.triangle_count,
            source_frame_index: frame.matter_update.frame_index,
            readback_probe_count: readback_probe_count.min(particle_rows),
        })
    }

    /// Builds a compact marker without logging high-rate field or particle data.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status=eligible computeStage=field-particle-force resourceKind={} resourceId={} fieldResourceId={} resourcePlane={} particleForceSource={} particleSamplingAuthority={} particleFieldSource={} particleForceRefresh={} particleForceUpdateIntervalFrames={} particleRows={} visualRows={} topologyVertexCount={} topologyTriangleCount={} sourceFrameIndex={} cpuOracle={} cpuOraclePreserved=true readbackPolicy={} readbackProbeCount={} commandEncoderRequired=true makepadComputeBackend={} gpuComputeReady=false computeKernel=false highRateJsonPayload=false measuredBy={}",
            QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            self.resource_kind.marker_value(),
            self.resource_kind.resource_id(),
            self.resource_kind.field_resource_id(),
            QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_RESOURCE_PLANE,
            self.force_source.marker_value(),
            self.force_source.sampling_authority_marker(),
            self.force_source.field_source_marker(),
            sanitize_marker_value(&self.force_refresh),
            self.force_update_interval_frames,
            self.particle_rows,
            self.visual_rows,
            self.topology_vertex_count,
            self.topology_triangle_count,
            optional_usize_marker_token(self.source_frame_index),
            self.force_source.sampling_authority_marker(),
            QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_READBACK_POLICY,
            self.readback_probe_count,
            QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_BACKEND_STATUS,
            QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_MEASUREMENT_SOURCE,
        )
    }

    /// Builds bounded words for the prototype GPU oracle compute probe.
    ///
    /// These are compact frame/classification words only. They do not serialize
    /// particle rows, SDF grids, ADF cells, mesh frames, or GPU buffers.
    #[must_use]
    pub fn oracle_compute_probe_words(
        &self,
    ) -> [u32; QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS] {
        [
            self.resource_kind.oracle_probe_tag(),
            saturating_u32(self.particle_rows),
            saturating_u32(self.topology_vertex_count),
            saturating_u32(self.topology_triangle_count),
        ]
    }
}

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
            "{} schema={} phase={} status={} computeStage=field-particle-force-prototype resourceKind={} resourceId={} fieldResourceId={} resourcePlane={} computeProbeBackend={} particleForceSource={} particleSamplingAuthority={} particleFieldSource={} particleRows={} visualRows={} topologyVertexCount={} topologyTriangleCount={} sourceFrameIndex={} cpuOracle={} cpuOraclePreserved=true preflightSchema={} readbackPolicy={} readbackProbeCount={} oraclePayload={} oracleWordCount={} oracleInputWords={} gpuOutputWords={} cpuExpectedWords={} mismatchedWords={} readbackMatched={} commandEncoderSubmitted=true storageBufferResident=true computeDispatchSubmitted=true prototypeComputeKernel=true fieldParticleKernel=false computeKernel=true gpuComputeReady=false highRateJsonPayload=false elapsedMs={} measuredBy={}",
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
            finite_f64_marker_token(self.readback.elapsed_ms),
            QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_MEASUREMENT_SOURCE,
        )
    }
}

fn optional_usize_marker_token(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn saturating_u32(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

fn hex_u32_marker_token(value: u32) -> String {
    format!("0x{value:08X}")
}

fn u32_words_marker_token(words: &[u32; QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS]) -> String {
    words
        .iter()
        .map(|word| hex_u32_marker_token(*word))
        .collect::<Vec<_>>()
        .join(",")
}

fn finite_f64_marker_token(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.3}")
    } else {
        "unavailable".to_owned()
    }
}
