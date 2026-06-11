use core::fmt;

use rusty_matter_surface_runtime::MatterSurfaceRuntimeError;
use rusty_optics_model::OpticsError;
use rusty_quest_makepad_mesh_replay::MeshReplayError;

use crate::QuestMakepadAdfDebugError;

/// Adapter failure.
#[derive(Clone, Debug, PartialEq)]
pub enum QuestMakepadMatterSurfaceError {
    /// Replay frame conversion failed.
    MeshReplay(MeshReplayError),
    /// Matter runtime failed.
    Matter(MatterSurfaceRuntimeError),
    /// ADF debug payload failed.
    Adf(QuestMakepadAdfDebugError),
    /// Optics visual payload failed.
    Optics(OpticsError),
}

impl fmt::Display for QuestMakepadMatterSurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MeshReplay(error) => write!(formatter, "{error}"),
            Self::Matter(error) => write!(formatter, "{error}"),
            Self::Adf(error) => write!(formatter, "{error}"),
            Self::Optics(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for QuestMakepadMatterSurfaceError {}

impl From<MeshReplayError> for QuestMakepadMatterSurfaceError {
    fn from(value: MeshReplayError) -> Self {
        Self::MeshReplay(value)
    }
}

impl From<MatterSurfaceRuntimeError> for QuestMakepadMatterSurfaceError {
    fn from(value: MatterSurfaceRuntimeError) -> Self {
        Self::Matter(value)
    }
}

impl From<QuestMakepadAdfDebugError> for QuestMakepadMatterSurfaceError {
    fn from(value: QuestMakepadAdfDebugError) -> Self {
        Self::Adf(value)
    }
}

impl From<OpticsError> for QuestMakepadMatterSurfaceError {
    fn from(value: OpticsError) -> Self {
        Self::Optics(value)
    }
}
