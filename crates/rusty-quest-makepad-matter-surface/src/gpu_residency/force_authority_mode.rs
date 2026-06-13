/// Profile token for keeping Matter's selected CPU force source authoritative.
pub const QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_MATTER_CPU: &str = "matter-cpu";
/// Profile token for the future GPU dense-SDF field particle-force authority.
pub const QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_GPU_DENSE_SDF_FIELD_PARTICLE_FORCE: &str =
    "gpu-dense-sdf-field-particle-force";

/// Adapter-level force-authority selector.
///
/// This is intentionally separate from Matter's `MatterSurfaceParticleForceSource`.
/// Matter keeps CPU force semantics; Quest-Makepad uses this low-rate profile
/// selector to decide whether a ready GPU equivalent may ever be considered for
/// runtime authority.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum QuestMakepadForceAuthorityMode {
    /// Use the selected Matter CPU force source as the runtime authority.
    #[default]
    MatterCpu,
    /// Request the GPU dense-SDF field particle-force equivalent.
    GpuDenseSdfFieldParticleForce,
}

impl QuestMakepadForceAuthorityMode {
    /// Parse a stable settings/profile token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_MATTER_CPU => Some(Self::MatterCpu),
            QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_GPU_DENSE_SDF_FIELD_PARTICLE_FORCE => {
                Some(Self::GpuDenseSdfFieldParticleForce)
            }
            _ => None,
        }
    }

    /// Stable marker/settings token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MatterCpu => QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_MATTER_CPU,
            Self::GpuDenseSdfFieldParticleForce => {
                QUEST_MAKEPAD_FORCE_AUTHORITY_MODE_GPU_DENSE_SDF_FIELD_PARTICLE_FORCE
            }
        }
    }

    /// Whether the explicit GPU force-authority profile gate was requested.
    #[must_use]
    pub const fn gpu_profile_enabled(self) -> bool {
        matches!(self, Self::GpuDenseSdfFieldParticleForce)
    }
}
