//! Low-rate GPU force-authority promotion settings.
//!
//! These values are profile/evidence receipts only. They do not carry hand,
//! mesh, field, particle, or GPU-buffer payloads.

/// Provider A/B receipt setting id for GPU force-authority promotion.
pub const SETTING_GPU_FORCE_LIVE_RECORDED_PROVIDER_AB_RECEIPT: &str =
    "makepad.particles.force.live_recorded_provider_ab_receipt";
/// Default provider A/B receipt token.
pub const GPU_FORCE_PROVIDER_AB_RECEIPT_NONE: &str = "none";
/// Receipt token consumed after the live-vs-recorded provider A/B checker passes.
pub const GPU_FORCE_PROVIDER_AB_RECEIPT_LIVE_RECORDED_CHECK_V1: &str =
    "live-recorded-provider-ab-check-v1";

/// Low-rate receipt that says whether the live/recorded provider A/B gate was proven.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum QuestMakepadGpuForceProviderAbReceipt {
    /// No provider A/B receipt has been supplied.
    #[default]
    None,
    /// The live-vs-recorded provider A/B checker passed for the current proof profile.
    LiveRecordedProviderAbCheckV1,
}

impl QuestMakepadGpuForceProviderAbReceipt {
    /// Parse a stable settings token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            GPU_FORCE_PROVIDER_AB_RECEIPT_NONE => Some(Self::None),
            GPU_FORCE_PROVIDER_AB_RECEIPT_LIVE_RECORDED_CHECK_V1 => {
                Some(Self::LiveRecordedProviderAbCheckV1)
            }
            _ => None,
        }
    }

    /// Stable marker/settings token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => GPU_FORCE_PROVIDER_AB_RECEIPT_NONE,
            Self::LiveRecordedProviderAbCheckV1 => {
                GPU_FORCE_PROVIDER_AB_RECEIPT_LIVE_RECORDED_CHECK_V1
            }
        }
    }

    /// True when this receipt may satisfy the provider A/B promotion gate.
    #[must_use]
    pub const fn live_recorded_provider_ab_ready(self) -> bool {
        matches!(self, Self::LiveRecordedProviderAbCheckV1)
    }
}
