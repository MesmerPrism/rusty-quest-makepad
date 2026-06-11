use crate::{
    sanitize_marker_value, QuestMakepadWorldAdfDebugBatch, QuestMakepadWorldParticleBatch,
};

use super::{
    QUEST_MAKEPAD_ADF_DEBUG_GPU_RESIDENCY_ROW_STRIDE_BYTES,
    QUEST_MAKEPAD_GPU_RESIDENCY_BACKEND_MAKEPAD_INSTANCED_DRAW,
    QUEST_MAKEPAD_GPU_RESIDENCY_MARKER_PREFIX, QUEST_MAKEPAD_GPU_RESIDENCY_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_RESIDENCY_PROOF_SCHEMA_ID, QUEST_MAKEPAD_GPU_RESIDENCY_RESOURCE_PLANE,
    QUEST_MAKEPAD_PARTICLE_GPU_RESIDENCY_ROW_STRIDE_BYTES,
};

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
