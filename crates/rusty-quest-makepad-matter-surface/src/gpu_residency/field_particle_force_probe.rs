use rusty_matter_model::Vec3;
use rusty_matter_surface_runtime::MatterSurfaceParticleSnapshot;

use crate::{config::QuestMakepadMatterParticleForceOracleConfig, sanitize_marker_value};

use super::{
    marker::{finite_f32_marker_token, finite_f64_marker_token},
    mesh_sdf_probe::{build_cpu_grid_from_mesh_sdf_input, normalize_or, selected_index},
    QuestMakepadGpuFieldConstructionReceipt, QuestMakepadGpuFieldForceSamplingProbeReadback,
    QuestMakepadGpuMeshSdfProbeInput, QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
    QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
};

/// Number of bounded Matter particle rows sampled by the resident field-force proof.
pub const QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_SAMPLES: usize = 4;
/// Conservative f32 tolerance for bounded resident-field particle-force comparison.
pub const QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_DEFAULT_TOLERANCE: f32 = 0.001;
/// Quest Makepad GPU resident dense-SDF particle-force schema.
pub const QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_SCHEMA_ID: &str =
    "rusty.quest.makepad.gpu_field_particle_force_probe.v1";
/// Quest Makepad GPU resident dense-SDF particle-force marker prefix.
pub const QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE";
/// Resource plane proven by the resident dense-SDF particle-force probe.
pub const QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_RESOURCE_PLANE: &str =
    "vulkan-compute-resident-dense-sdf-particle-force-sampling";
/// Matter CPU oracle retained for bounded particle-force validation.
pub const QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_CPU_ORACLE: &str =
    "matter-particle-snapshot-dense-sdf-force-sampler";
/// Measurement companion for the resident field particle-force probe.
pub const QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_MEASUREMENT_SOURCE: &str =
    "RUSTY_QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE.elapsedMs,RUSTY_MAKEPAD_CADENCE.xrRepaintGpuMs";

/// One bounded Matter particle-force CPU oracle row over the dense-SDF proof grid.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadGpuFieldParticleForceProbeSample {
    /// Source particle index in the Matter particle snapshot.
    pub particle_index: usize,
    /// Probe particle position packed as `[x, y, z, 1]`.
    pub position: [f32; 4],
    /// Probe particle radius.
    pub radius: f32,
    /// Matter CPU dense-SDF value sampled at `position`.
    pub distance: f32,
    /// Matter CPU dense-SDF outward gradient packed as `[x, y, z, 0]`.
    pub outward: [f32; 4],
    /// Target signed-distance band from Matter's particle force equation.
    pub target_distance: f32,
    /// Attraction strength from Matter's particle force equation.
    pub attraction_strength: f32,
    /// Matter CPU expected acceleration packed as `[x, y, z, 0]`.
    pub expected_acceleration: [f32; 4],
}

/// Bounded Matter particle-force input for resident dense-SDF GPU comparison.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuFieldParticleForceProbeInput {
    /// Source Matter particle-set identifier.
    pub source_set_id: String,
    /// Number of particle rows available in the Matter snapshot.
    pub particle_rows: usize,
    /// Requested bounded particle sample count.
    pub requested_sample_count: usize,
    /// Number of populated particle-force samples.
    pub sample_count: usize,
    /// Number of selected particle rows rejected by the dense-SDF oracle.
    pub rejected_count: usize,
    /// Matter particle-force equation coefficients used by this CPU oracle.
    pub force_config: QuestMakepadMatterParticleForceOracleConfig,
    /// Bounded particle-force samples.
    pub samples: [QuestMakepadGpuFieldParticleForceProbeSample;
        QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_SAMPLES],
}

impl QuestMakepadGpuFieldParticleForceProbeInput {
    /// Builds bounded particle-force samples from a Matter snapshot and dense-SDF proof input.
    #[must_use]
    pub fn from_mesh_sdf_input_and_particle_snapshot(
        mesh_sdf_input: &QuestMakepadGpuMeshSdfProbeInput,
        particle_snapshot: &MatterSurfaceParticleSnapshot,
        force_config: QuestMakepadMatterParticleForceOracleConfig,
    ) -> Option<Self> {
        if !force_config.target_distance_radius_scale.is_finite()
            || !force_config.minimum_target_distance.is_finite()
            || !force_config.attraction_strength.is_finite()
        {
            return None;
        }
        let particle_rows = particle_snapshot.samples.len();
        let requested_sample_count =
            particle_rows.min(QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_SAMPLES);
        if requested_sample_count == 0 {
            return None;
        }

        let cpu_grid = build_cpu_grid_from_mesh_sdf_input(mesh_sdf_input)?;
        let mut samples = [QuestMakepadGpuFieldParticleForceProbeSample::default();
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_SAMPLES];
        let mut sample_count = 0;
        let mut rejected_count = 0;
        for sample_index in 0..requested_sample_count {
            let particle_index =
                selected_index(particle_rows, requested_sample_count, sample_index);
            let Some(particle) = particle_snapshot.samples.get(particle_index) else {
                rejected_count += 1;
                continue;
            };
            if !particle.position.is_finite() || !particle.radius.is_finite() {
                rejected_count += 1;
                continue;
            }

            let Some(distance_sample) = cpu_grid.sample_nearest_clamped(particle.position) else {
                rejected_count += 1;
                continue;
            };
            let Some(gradient) = cpu_grid.gradient_nearest(particle.position) else {
                rejected_count += 1;
                continue;
            };
            let outward = normalize_or(gradient, Vec3::new(0.0, 1.0, 0.0));
            let target_distance = force_config.target_distance_for_radius(particle.radius);
            let error = distance_sample.distance - target_distance;
            let expected_acceleration = outward * (-error * force_config.attraction_strength);
            samples[sample_count] = QuestMakepadGpuFieldParticleForceProbeSample {
                particle_index,
                position: [
                    particle.position.x,
                    particle.position.y,
                    particle.position.z,
                    1.0,
                ],
                radius: particle.radius,
                distance: distance_sample.distance,
                outward: [outward.x, outward.y, outward.z, 0.0],
                target_distance,
                attraction_strength: force_config.attraction_strength,
                expected_acceleration: [
                    expected_acceleration.x,
                    expected_acceleration.y,
                    expected_acceleration.z,
                    0.0,
                ],
            };
            sample_count += 1;
        }
        if sample_count == 0 {
            return None;
        }

        Some(Self {
            source_set_id: particle_snapshot.source_set_id.clone(),
            particle_rows,
            requested_sample_count,
            sample_count,
            rejected_count,
            force_config,
            samples,
        })
    }
}

/// Bounded resident dense-SDF particle-force comparison proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadGpuFieldParticleForceProbe {
    /// Schema identifier.
    pub schema_id: String,
    /// Source dense-SDF field construction receipt.
    pub receipt: QuestMakepadGpuFieldConstructionReceipt,
    /// Bounded Matter particle-force CPU oracle input.
    pub input: QuestMakepadGpuFieldParticleForceProbeInput,
    /// Makepad resident dense-SDF force-sampling readback.
    pub readback: QuestMakepadGpuFieldForceSamplingProbeReadback,
}

impl QuestMakepadGpuFieldParticleForceProbe {
    /// Builds a resident particle-force marker from a field construction receipt.
    #[must_use]
    pub fn from_receipt_and_input(
        receipt: &QuestMakepadGpuFieldConstructionReceipt,
        input: &QuestMakepadGpuFieldParticleForceProbeInput,
        readback: QuestMakepadGpuFieldForceSamplingProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_SCHEMA_ID.to_owned(),
            receipt: receipt.clone(),
            input: input.clone(),
            readback,
        }
    }

    /// True when the resident field produced matching particle-force samples.
    #[must_use]
    pub fn runtime_particle_force_comparison_ready(&self) -> bool {
        self.receipt.runtime_field_boundary_ready()
            && self.readback.readback_matched()
            && self.readback.source_field_buffer_resident
            && self.readback.source_field_generation == self.receipt.derived_buffer_generation
            && self.readback.source_field_buffer_bytes == self.receipt.sdf_distance_buffer_bytes
            && self.readback.sample_count == self.input.sample_count
    }

    /// Builds a compact marker without logging particle rows, field buffers, or GPU buffers.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} proofKind=resident-dense-sdf-field-particle-force-sampling computeStage=dense-sdf-field-particle-force-readback fieldKind={} sourceReceiptSchema={} sourceId={} sourceFrameIndex={} fieldResourceId={} topologyVertexCount={} topologyTriangleCount={} topologyIndexCount={} validationInputShape={} resourcePlane={} sourceResourcePlane={} computeProbeBackend=makepad-vulkan-compute-resident-dense-sdf-force-sampling gridOriginX={} gridOriginY={} gridOriginZ={} gridVoxelSize={} gridDimX={} gridDimY={} gridDimZ={} voxelCount={} particleSampleSource=matter-particle-snapshot sourceParticleSetId={} particleRows={} requestedParticleSampleCount={} sampledParticleCount={} rejectedParticleCount={} sampleCount={} componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} runtimeFieldBoundaryReady={} runtimeParticleForceComparisonReady={} sourceFieldGeneration={} expectedSourceFieldGeneration={} sourceFieldGenerationMatched={} sourceFieldBufferResident={} sourceFieldBufferBytes={} expectedSourceFieldBufferBytes={} sampleInputBufferBytes={} sampleOutputBufferBytes={} targetDistanceRadiusScale={} minimumTargetDistance={} attractionStrength={} cpuOracle={} cpuOraclePreserved=true recordedInputEquivalent=true residentFieldBufferSampled=true denseSdfConstructedOnGpu=true matterCpuParticleIntegration=true matterParticleForceEquation=true fieldSamplingKernel=true fieldForceSamplingKernel=true fieldParticleKernel=true runtimeParticleIntegration=false computeKernel=true gpuComputeReady=false forceAuthorityReady=false runtimeForceAuthority=false highRateJsonPayload=false commandEncoderSubmitted=true computeDispatchSubmitted=true queueSubmitSerial={} fenceSerial={} resourceGeneration={} programGeneration={} programReused={} shaderCompiledThisSubmit={} pipelineCreatedThisSubmit={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_MARKER_PREFIX,
            self.schema_id,
            sanitize_marker_value(phase),
            self.status_marker(),
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_FIELD_KIND,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_SCHEMA_ID,
            sanitize_marker_value(&self.receipt.source_id),
            self.receipt.source_frame_index,
            sanitize_marker_value(&self.receipt.field_resource_id),
            self.receipt.topology_vertex_count,
            self.receipt.topology_triangle_count,
            self.receipt.topology_index_count,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_VALIDATION_INPUT_SHAPE,
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_RESOURCE_PLANE,
            QUEST_MAKEPAD_GPU_FIELD_CONSTRUCTION_RECEIPT_RESOURCE_PLANE,
            finite_f32_marker_token(self.receipt.grid.origin[0]),
            finite_f32_marker_token(self.receipt.grid.origin[1]),
            finite_f32_marker_token(self.receipt.grid.origin[2]),
            finite_f32_marker_token(self.receipt.grid.voxel_size),
            self.receipt.grid.dimensions[0],
            self.receipt.grid.dimensions[1],
            self.receipt.grid.dimensions[2],
            self.receipt.voxel_count,
            sanitize_marker_value(&self.input.source_set_id),
            self.input.particle_rows,
            self.input.requested_sample_count,
            self.input.sample_count,
            self.input.rejected_count,
            self.readback.sample_count,
            self.readback.component_count,
            self.readback.mismatched_components,
            finite_f32_marker_token(self.readback.max_abs_error),
            finite_f32_marker_token(self.readback.tolerance),
            self.readback.readback_matched(),
            self.receipt.runtime_field_boundary_ready(),
            self.runtime_particle_force_comparison_ready(),
            self.readback.source_field_generation,
            self.receipt.derived_buffer_generation,
            self.readback.source_field_generation == self.receipt.derived_buffer_generation,
            self.readback.source_field_buffer_resident,
            self.readback.source_field_buffer_bytes,
            self.receipt.sdf_distance_buffer_bytes,
            self.readback.sample_input_buffer_bytes,
            self.readback.sample_output_buffer_bytes,
            finite_f32_marker_token(self.input.force_config.target_distance_radius_scale),
            finite_f32_marker_token(self.input.force_config.minimum_target_distance),
            finite_f32_marker_token(self.input.force_config.attraction_strength),
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_CPU_ORACLE,
            self.readback.queue_submit_serial,
            self.readback.fence_serial,
            self.readback.resource_generation,
            self.readback.program_generation,
            self.readback.program_reused,
            self.readback.shader_compiled_this_submit,
            self.readback.pipeline_created_this_submit,
            self.readback.pending_retire_count,
            self.readback.retained_resource_count,
            self.readback.retired_after_fence_count,
            self.readback.queue_wait_idle_performed,
            finite_f64_marker_token(self.readback.elapsed_ms),
            QUEST_MAKEPAD_GPU_FIELD_PARTICLE_FORCE_PROBE_MEASUREMENT_SOURCE,
        )
    }

    fn status_marker(&self) -> &'static str {
        if !self.readback.readback_matched() {
            "mismatch"
        } else if self.runtime_particle_force_comparison_ready() {
            "ready"
        } else {
            "not-ready"
        }
    }
}
