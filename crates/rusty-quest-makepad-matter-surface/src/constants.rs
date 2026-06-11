/// Quest Makepad Matter surface marker schema.
pub const QUEST_MAKEPAD_MATTER_SURFACE_SCHEMA_ID: &str =
    "rusty.quest.makepad.matter_surface_runtime.v1";
/// Quest Makepad Matter surface marker prefix.
pub const QUEST_MAKEPAD_MATTER_SURFACE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_MATTER_SURFACE_RUNTIME";
/// Quest Makepad Matter distance slice upload schema.
pub const QUEST_MAKEPAD_DISTANCE_SLICE_UPLOAD_SCHEMA_ID: &str =
    "rusty.quest.makepad.matter_distance_slice_upload.v1";
/// Quest Makepad Matter collision upload schema.
pub const QUEST_MAKEPAD_COLLISION_UPLOAD_SCHEMA_ID: &str =
    "rusty.quest.makepad.matter_collision_upload.v1";
/// Quest Makepad Matter particle upload schema.
pub const QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID: &str =
    "rusty.quest.makepad.matter_particle_upload.v1";
/// Quest Makepad world-particle batch schema.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID: &str =
    "rusty.quest.makepad.world_particle_batch.v1";
/// Quest Makepad world-particle marker prefix.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_MARKER_PREFIX: &str = "RUSTY_QUEST_MAKEPAD_WORLD_PARTICLES";
/// Start-head-local coordinate space for first-visibility headset smoke tests.
pub const QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE: &str = "makepad-xr-start-head-local";
/// Makepad XR content-local coordinate space.
///
/// Host shells that render inside `XrRoot` should use this space when the root
/// already applies the initial headset-relative content pose.
pub const QUEST_MAKEPAD_CONTENT_LOCAL_SPACE: &str = "makepad-xr-content-local";
/// Initial world-particle render mode.
pub const QUEST_MAKEPAD_CENTER_PROJECTED_BILLBOARD_MODE: &str = "center-projected-billboard";
/// Current Quest Makepad world-particle renderer identity.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_RENDERER_ID: &str =
    "makepad-xr-procedural-ring-billboard";
/// Current Quest Makepad world-particle animation mode.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_MODE: &str = "procedural-morph-ring";
/// Renderer-neutral Optics frame source used by the billboard animation.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_SOURCE: &str =
    "rusty-optics-particle-visual-frame";
/// Reference visual direction borrowed for the current smoke renderer.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_REFERENCE: &str =
    "rusty-viscereality-billboard-ring";
/// Selection policy used when the source particle upload is larger than the
/// current world-object proof renderer can draw.
pub const QUEST_MAKEPAD_WORLD_PARTICLE_EVEN_SELECTION_POLICY: &str = "evenly-spaced-source-rows";

/// Browser-preview cloud-radius multiplier.
pub const DEFAULT_PARTICLE_CLOUD_RADIUS_SCALE: f32 = 2.45;
/// Browser-preview particle-radius multiplier.
pub const DEFAULT_PARTICLE_RADIUS_SCALE: f32 = 0.009;
/// Browser-preview minimum particle radius.
pub const DEFAULT_MIN_PARTICLE_RADIUS: f32 = 0.0012;
/// Default simulated-content center: about 0.5m in front of the initial camera pose.
pub const DEFAULT_WORLD_CONTENT_CENTER: [f32; 3] = [0.0, 0.0, -0.5];
/// Default displayed content radius in Makepad world units.
pub const DEFAULT_WORLD_CONTENT_TARGET_RADIUS: f32 = 0.16;
/// Default Matter particle execution batch size used by Quest Makepad profiles.
pub const DEFAULT_PARTICLE_EXECUTION_BATCH_SIZE: usize = 256;
/// Default SDF/ADF debug-field rebuild interval used by Quest Makepad profiles.
pub const DEFAULT_SDF_ADF_DEBUG_UPDATE_INTERVAL_FRAMES: usize = 1;
