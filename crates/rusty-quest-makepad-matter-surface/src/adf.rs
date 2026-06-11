//! Quest Makepad adapter helpers for Matter-owned ADF debug visuals.
//!
//! This module keeps ADF construction and renderer-neutral debug conversion
//! out of the crate root while preserving the ownership split: Matter builds
//! ADF fields, Optics prepares debug visuals, and this crate adapts the result
//! into Quest Makepad frame/evidence surfaces.

use core::fmt;

use rusty_matter_adf::{
    build_adf_from_sdf_grid_report, AdfBuildConfig, AdfBuildDiagnostics, AdfBuildReport, AdfError,
};
use rusty_matter_sdf::PackedSdfGrid;
use rusty_optics_mesh::AdfDebugVisual;
use rusty_optics_model::{OpticsError, ADF_DEBUG_VISUAL_SCHEMA_ID};

/// Quest Makepad ADF debug frame schema.
pub const QUEST_MAKEPAD_ADF_DEBUG_SCHEMA_ID: &str = "rusty.quest.makepad.matter_adf_debug.v1";
/// Stable Optics visual id used for the current adapter frame.
pub const QUEST_MAKEPAD_ADF_DEBUG_VISUAL_ID: &str = "quest.makepad.adf_debug.current";

/// Adapter-level ADF debug configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadAdfDebugConfig {
    /// Maximum ADF subdivision depth.
    pub max_depth: u32,
    /// Maximum leaf-cell budget.
    pub max_cells: usize,
    /// Maximum accepted source-distance range within one leaf cell.
    pub error_tolerance: f32,
}

impl Default for QuestMakepadAdfDebugConfig {
    fn default() -> Self {
        let config = AdfBuildConfig::default();
        Self {
            max_depth: config.max_depth,
            max_cells: config.max_cells,
            error_tolerance: config.error_tolerance,
        }
    }
}

impl QuestMakepadAdfDebugConfig {
    /// Converts to the Matter-owned ADF builder config.
    #[must_use]
    pub const fn to_matter_config(self) -> AdfBuildConfig {
        AdfBuildConfig {
            max_depth: self.max_depth,
            max_cells: self.max_cells,
            error_tolerance: self.error_tolerance,
        }
    }
}

/// One Quest Makepad ADF debug frame.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadAdfDebugFrame {
    /// Adapter schema identifier.
    pub schema_id: String,
    /// Renderer-neutral Optics visual schema identifier.
    pub visual_schema_id: String,
    /// Source Matter ADF field id.
    pub source_field_id: String,
    /// Source Matter SDF grid id.
    pub source_grid_id: String,
    /// Matter ADF build diagnostics.
    pub diagnostics: AdfBuildDiagnostics,
    /// Renderer-neutral Optics ADF debug visual.
    pub visual: AdfDebugVisual,
}

pub(crate) fn build_adf_report(
    grid: &PackedSdfGrid,
    config: QuestMakepadAdfDebugConfig,
) -> Result<AdfBuildReport, QuestMakepadAdfDebugError> {
    build_adf_from_sdf_grid_report(grid, config.to_matter_config())
        .map_err(QuestMakepadAdfDebugError::Matter)
}

pub(crate) fn adf_debug_frame_from_report(
    report: AdfBuildReport,
) -> Result<QuestMakepadAdfDebugFrame, QuestMakepadAdfDebugError> {
    let visual = AdfDebugVisual::from_field(QUEST_MAKEPAD_ADF_DEBUG_VISUAL_ID, &report.field)
        .map_err(QuestMakepadAdfDebugError::Optics)?;
    Ok(QuestMakepadAdfDebugFrame {
        schema_id: QUEST_MAKEPAD_ADF_DEBUG_SCHEMA_ID.to_owned(),
        visual_schema_id: ADF_DEBUG_VISUAL_SCHEMA_ID.to_owned(),
        source_field_id: report.field.field_id,
        source_grid_id: visual.source_grid_id.clone(),
        diagnostics: report.diagnostics,
        visual,
    })
}

/// ADF debug adapter failure.
#[derive(Clone, Debug, PartialEq)]
pub enum QuestMakepadAdfDebugError {
    /// Matter ADF build failed.
    Matter(AdfError),
    /// Optics debug visual build failed.
    Optics(OpticsError),
}

impl fmt::Display for QuestMakepadAdfDebugError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Matter(error) => write!(formatter, "{error}"),
            Self::Optics(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for QuestMakepadAdfDebugError {}
