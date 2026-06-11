use rusty_matter_model::Vec3;
use rusty_matter_surface_runtime::MatterSurfaceFrameInput;
use rusty_quest_makepad_mesh_replay::MeshReplayRuntime;

use crate::{
    bounds_max_half_extent, QuestMakepadGpuMeshSdfProbeInput,
    QuestMakepadGpuSkinningMeshProbeInput, QuestMakepadGpuSkinningProbeInput,
    QuestMakepadMatterSurfaceError,
};

/// One animated hand/surface source frame ready for the native Matter runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadMatterSurfaceSourceFrame {
    /// Stable source identity, for example a recorded replay or realtime hand pair.
    pub source_id: String,
    /// Matter-owned surface frame input.
    pub frame: MatterSurfaceFrameInput,
    /// Source-space bounds minimum for reset/scaling policy.
    pub bounds_min: [f32; 3],
    /// Source-space bounds maximum for reset/scaling policy.
    pub bounds_max: [f32; 3],
    /// Source-space radius used for particle cloud sizing.
    pub bounds_radius: f32,
    /// Optional bounded recorded-hand GPU skinning probe input.
    pub gpu_skinning_probe: Option<QuestMakepadGpuSkinningProbeInput>,
    /// Optional full recorded-hand GPU skinning mesh residency probe input.
    pub gpu_skinning_mesh_probe: Option<QuestMakepadGpuSkinningMeshProbeInput>,
    /// Optional tiny GPU mesh-to-dense-SDF probe input.
    pub gpu_mesh_sdf_probe: Option<QuestMakepadGpuMeshSdfProbeInput>,
}

impl QuestMakepadMatterSurfaceSourceFrame {
    /// Creates a source frame from a Matter frame input and source bounds.
    #[must_use]
    pub fn new(
        source_id: impl Into<String>,
        frame: MatterSurfaceFrameInput,
        bounds_min: [f32; 3],
        bounds_max: [f32; 3],
    ) -> Self {
        Self {
            source_id: source_id.into(),
            frame,
            bounds_min,
            bounds_max,
            bounds_radius: bounds_max_half_extent(bounds_min, bounds_max),
            gpu_skinning_probe: None,
            gpu_skinning_mesh_probe: None,
            gpu_mesh_sdf_probe: None,
        }
    }

    /// Creates a source frame from the current replay frame.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when replay frame conversion
    /// fails.
    pub fn from_replay(replay: &MeshReplayRuntime) -> Result<Self, QuestMakepadMatterSurfaceError> {
        let sequence = replay.sequence();
        let source_id = if replay.config().source.trim().is_empty() {
            sequence.sequence_id().to_owned()
        } else {
            replay.config().source.clone()
        };
        Ok(Self {
            source_id,
            frame: MatterSurfaceFrameInput::new(
                replay.current_frame_index(),
                replay.playback_seconds().max(0.0),
                replay.current_surface()?,
            ),
            bounds_min: sequence.bounds_min(),
            bounds_max: sequence.bounds_max(),
            bounds_radius: sequence.bounds_radius(),
            gpu_skinning_probe: None,
            gpu_skinning_mesh_probe: None,
            gpu_mesh_sdf_probe: None,
        })
    }

    /// Attaches bounded diagnostic GPU skinning probe input to this source frame.
    #[must_use]
    pub fn with_gpu_skinning_probe(
        mut self,
        probe: Option<QuestMakepadGpuSkinningProbeInput>,
    ) -> Self {
        self.gpu_skinning_probe = probe;
        self
    }

    /// Attaches full-mesh diagnostic GPU skinning residency input to this source frame.
    #[must_use]
    pub fn with_gpu_skinning_mesh_probe(
        mut self,
        probe: Option<QuestMakepadGpuSkinningMeshProbeInput>,
    ) -> Self {
        self.gpu_skinning_mesh_probe = probe;
        self
    }

    /// Attaches tiny diagnostic GPU mesh-to-dense-SDF input to this source frame.
    #[must_use]
    pub fn with_gpu_mesh_sdf_probe(
        mut self,
        probe: Option<QuestMakepadGpuMeshSdfProbeInput>,
    ) -> Self {
        self.gpu_mesh_sdf_probe = probe;
        self
    }

    pub(crate) fn bounds_center(&self) -> Vec3 {
        Vec3::new(
            (self.bounds_min[0] + self.bounds_max[0]) * 0.5,
            (self.bounds_min[1] + self.bounds_max[1]) * 0.5,
            (self.bounds_min[2] + self.bounds_max[2]) * 0.5,
        )
    }

    pub(crate) fn surface_radius(&self) -> f32 {
        self.bounds_radius.max(0.001)
    }
}
