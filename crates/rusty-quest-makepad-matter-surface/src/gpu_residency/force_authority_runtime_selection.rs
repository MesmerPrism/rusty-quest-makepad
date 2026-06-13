use rusty_matter_surface_runtime::MatterSurfaceParticleForceSource;

use super::{
    QuestMakepadForceAuthorityMode, QuestMakepadGpuForceAuthorityResidencyHealth,
    QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_GPU_DENSE_SDF_FIELD_PARTICLE_FORCE,
    QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_MATTER_CPU,
};

/// Rollback policy used by the GPU-backed force-authority gate.
pub const QUEST_MAKEPAD_FORCE_AUTHORITY_ROLLBACK_POLICY: &str =
    "matter-cpu-oracle-on-gpu-freshness-or-cadence-failure";

/// Runtime force authority selected by the Quest-Makepad adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestMakepadRuntimeForceAuthorityKind {
    /// The active runtime force authority is the selected Matter CPU mode.
    MatterCpu,
    /// The active runtime force authority is the GPU dense-SDF particle-force equivalent.
    GpuDenseSdfFieldParticleForce,
}

impl QuestMakepadRuntimeForceAuthorityKind {
    /// Stable marker token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MatterCpu => QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_MATTER_CPU,
            Self::GpuDenseSdfFieldParticleForce => {
                QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_GPU_DENSE_SDF_FIELD_PARTICLE_FORCE
            }
        }
    }

    /// True when the active runtime authority is GPU-backed.
    #[must_use]
    pub const fn is_gpu_backed(self) -> bool {
        matches!(self, Self::GpuDenseSdfFieldParticleForce)
    }
}

/// Exclusive runtime force-authority decision for a GPU residency health receipt.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadRuntimeForceAuthoritySelection {
    /// Requested low-rate profile gate.
    pub requested_authority: QuestMakepadForceAuthorityMode,
    /// Exactly one active runtime force authority.
    pub active_authority: QuestMakepadRuntimeForceAuthorityKind,
    /// Active authority count; kept explicit for evidence validators.
    pub active_authority_count: usize,
    /// Matter CPU oracle/fallback source preserved for rollback and validation.
    pub matter_cpu_fallback_authority: MatterSurfaceParticleForceSource,
    /// True when the GPU-backed equivalent may be used as runtime authority.
    pub runtime_selection_permitted: bool,
    /// Reason Matter CPU remains active, or why the GPU authority was selected.
    pub decision_reason: &'static str,
    /// Whether Matter CPU fallback remains available.
    pub matter_cpu_fallback_ready: bool,
}

impl QuestMakepadRuntimeForceAuthoritySelection {
    /// Selects exactly one runtime authority from residency health.
    #[must_use]
    pub fn from_residency_health(health: &QuestMakepadGpuForceAuthorityResidencyHealth) -> Self {
        let runtime_selection_permitted = health.runtime_selection_permitted();
        let active_authority = if runtime_selection_permitted {
            QuestMakepadRuntimeForceAuthorityKind::GpuDenseSdfFieldParticleForce
        } else {
            QuestMakepadRuntimeForceAuthorityKind::MatterCpu
        };
        let decision_reason = if runtime_selection_permitted {
            "gpu-force-authority-selected"
        } else {
            health.fallback_reason()
        };
        Self {
            requested_authority: health.gate.requested_authority,
            active_authority,
            active_authority_count: 1,
            matter_cpu_fallback_authority: health.gate.active_force_source,
            runtime_selection_permitted,
            decision_reason,
            matter_cpu_fallback_ready: true,
        }
    }

    /// True when the GPU candidate becomes the only active force authority.
    #[must_use]
    pub const fn gpu_authority_selected(&self) -> bool {
        self.active_authority.is_gpu_backed()
    }

    /// Marker value for the Matter CPU field when it is active, or oracle-only when GPU is active.
    #[must_use]
    pub fn active_matter_force_authority_marker(&self) -> &'static str {
        if self.gpu_authority_selected() {
            "oracle-only"
        } else {
            self.matter_cpu_fallback_authority.marker_value()
        }
    }
}
