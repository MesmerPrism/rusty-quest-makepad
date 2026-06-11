//! Compact GPU residency proof descriptors for Quest Makepad render-plane rows.
//!
//! This module does not introduce GPU compute or renderer ownership into
//! Matter. It records when an existing bounded data-plane batch is ready to be
//! adopted by a Makepad instanced draw path, while preserving Matter's CPU
//! reference as the source of truth.

mod field_force_probe;
mod marker;
mod oracle_probe;
mod preflight;
mod render;
mod skinning_probe;
mod storage_probe;

pub use field_force_probe::{
    QuestMakepadGpuFieldForceProbe, QuestMakepadGpuFieldForceProbeReadback,
};
pub use oracle_probe::{
    QuestMakepadGpuOracleComputeProbe, QuestMakepadGpuOracleComputeProbeReadback,
};
pub use preflight::{QuestMakepadGpuComputePreflight, QuestMakepadGpuComputeResourceKind};
pub use render::{QuestMakepadGpuResidencyPayloadKind, QuestMakepadGpuResidencyProof};
pub use skinning_probe::{
    QuestMakepadGpuSkinningProbe, QuestMakepadGpuSkinningProbeInput,
    QuestMakepadGpuSkinningProbeReadback, QuestMakepadGpuSkinningProbeSample,
    QUEST_MAKEPAD_GPU_SKINNING_PROBE_DEFAULT_TOLERANCE, QUEST_MAKEPAD_GPU_SKINNING_PROBE_SAMPLES,
};
pub use storage_probe::{QuestMakepadGpuStorageProbe, QuestMakepadGpuStorageProbeReadback};

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
/// Quest Makepad GPU field-force arithmetic probe schema.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_field_force_probe.v1";
/// Quest Makepad GPU field-force arithmetic probe marker prefix.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE";
/// Backend used by the current Makepad f32 force arithmetic probe.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_BACKEND: &str =
    "makepad-vulkan-compute-f32-force-probe";
/// Measurement companion for the f32 force arithmetic probe.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_MEASUREMENT_SOURCE: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE.elapsedMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs";
/// Marker payload type for the current bounded f32 force arithmetic probe.
pub const QUEST_MAKEPAD_GPU_FIELD_FORCE_PROBE_PAYLOAD: &str =
    "bounded-matter-particle-force-probes";
/// Quest Makepad GPU recorded-hand skinning arithmetic probe schema.
pub const QUEST_MAKEPAD_GPU_SKINNING_PROBE_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_skinning_probe.v1";
/// Quest Makepad GPU recorded-hand skinning arithmetic probe marker prefix.
pub const QUEST_MAKEPAD_GPU_SKINNING_PROBE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_SKINNING_PROBE";
/// Backend used by the current Makepad f32 joint-matrix skinning probe.
pub const QUEST_MAKEPAD_GPU_SKINNING_PROBE_BACKEND: &str =
    "makepad-vulkan-compute-f32-skinning-probe";
/// Measurement companion for the f32 skinning matrix probe.
pub const QUEST_MAKEPAD_GPU_SKINNING_PROBE_MEASUREMENT_SOURCE: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_SKINNING_PROBE.elapsedMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs";
/// Marker payload type for the current bounded recorded-hand skinning probe.
pub const QUEST_MAKEPAD_GPU_SKINNING_PROBE_PAYLOAD: &str = "bounded-recorded-hand-skinning-probes";
