use rusty_matter_surface_runtime::{
    MatterSurfaceParticleForceSource, MatterSurfaceParticleForceSourceStatus,
};

use crate::{sanitize_marker_value, QuestMakepadMatterSurfaceFrame};

use super::{
    marker::{optional_usize_marker_token, saturating_u32},
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_BACKEND_STATUS,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_MARKER_PREFIX,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_MEASUREMENT_SOURCE,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_READBACK_POLICY,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_COMPUTE_PREFLIGHT_SCHEMA_ID, QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS,
};

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
