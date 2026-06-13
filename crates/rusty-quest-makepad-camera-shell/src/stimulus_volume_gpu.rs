//! Bounded stimulus-volume GPU proof contract.
//!
//! Optics owns the profile and shader ABI intent. This module only defines the
//! Quest-Makepad adapter proof shape and a deterministic CPU oracle for a tiny
//! storage-buffer volume probe.

use crate::{StimulusVolumeProfileSummary, STIMULUS_VOLUME_SCHEMA_ID};

/// Marker emitted after bounded stimulus volume compute/readback evidence.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE";
/// Schema for the bounded stimulus volume compute/readback marker.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SCHEMA_ID: &str =
    "rusty.quest.makepad.stimulus_volume_gpu_probe.v1";
/// Generic Makepad backend used by the first bounded volume proof.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_BACKEND: &str =
    "makepad-vulkan-volume-probe-compute-readback";
/// Storage plane used by the first bounded volume proof.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_RESOURCE_PLANE: &str =
    "vulkan-storage-buffer-volume-probe-readback";
/// Compact payload identifier for the volume probe oracle.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_PAYLOAD: &str =
    "bounded-optics-stimulus-volume-probe-v1";
/// Measurement source for the marker.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_MEASUREMENT_SOURCE: &str =
    "quest-makepad-stimulus-volume-proof";
/// Current bounded Quest Vulkan sample count for the proof command.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES: usize = 8;
/// Conservative f32 tolerance for CPU-oracle comparison.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_DEFAULT_TOLERANCE: f32 = 0.001;

/// Vec4-aligned CPU-oracle sample submitted to the generic Makepad volume probe.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadStimulusVolumeProbeSample {
    /// `[u, v, eye_index, time_seconds]`.
    pub uv_eye_time: [f32; 4],
    /// `[origin_x, origin_y, origin_z, probe_depth]`.
    pub ray_origin_depth: [f32; 4],
    /// `[dir_x, dir_y, dir_z, step_count]`.
    pub ray_direction_step: [f32; 4],
    /// `[frequency, phase, opacity, reserved]`.
    pub volume_params: [f32; 4],
    /// Expected RGBA from the CPU oracle.
    pub expected_rgba: [f32; 4],
    /// Expected `[density, depth, status, reserved]` from the CPU oracle.
    pub expected_density_depth_status: [f32; 4],
}

/// Vec4-aligned GPU output read back from the generic Makepad volume probe.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadStimulusVolumeProbeOutput {
    /// RGBA volume probe output.
    pub rgba: [f32; 4],
    /// `[density, depth, status, reserved]`.
    pub density_depth_status: [f32; 4],
}

/// Full adapter input for the bounded stimulus volume proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadStimulusVolumeProbeInput {
    /// Staged Optics profile id.
    pub profile_id: String,
    /// Verified staged Optics profile SHA-256.
    pub profile_sha256: String,
    /// Volume descriptor schema.
    pub volume_schema: String,
    /// Volume id from the staged Optics profile.
    pub volume_id: String,
    /// Optics volume field kind.
    pub field_kind: String,
    /// Optics storage hint.
    pub storage_hint: String,
    /// Volume grid dimensions declared by Optics.
    pub grid_dimensions: [u64; 3],
    /// Volume step count declared by Optics.
    pub step_count: u64,
    /// Kernel ABI id declared by Optics.
    pub kernel_abi_id: String,
    /// Browser/Optics declared bounded readback sample count.
    pub declared_readback_samples: u64,
    /// Number of populated Quest proof samples.
    pub sample_count: usize,
    /// Deterministic compact probe samples.
    pub samples:
        [QuestMakepadStimulusVolumeProbeSample; QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES],
}

impl QuestMakepadStimulusVolumeProbeInput {
    /// Builds the first bounded Quest Vulkan proof input from a staged profile summary.
    #[must_use]
    pub fn from_profile_summary(
        profile_id: impl Into<String>,
        profile_sha256: impl Into<String>,
        summary: &StimulusVolumeProfileSummary,
    ) -> Option<Self> {
        if !summary.volume_present {
            return None;
        }
        let volume_schema = summary.volume_schema.as_ref()?;
        if volume_schema != STIMULUS_VOLUME_SCHEMA_ID {
            return None;
        }
        let volume_id = summary.volume_id.as_ref()?;
        let field_kind = summary.field_kind.as_ref()?;
        let storage_hint = summary.storage_hint.as_ref()?;
        let grid_dimensions = summary.grid_dimensions?;
        let step_count = summary.step_count?;
        let kernel_abi_id = summary.kernel_abi_id.as_ref()?;
        let declared_readback_samples = summary.volume_readback_probe_samples?;
        if declared_readback_samples == 0 || step_count == 0 {
            return None;
        }

        let sample_count = (declared_readback_samples as usize)
            .min(QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES);
        let mut samples = [QuestMakepadStimulusVolumeProbeSample::default();
            QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES];
        for (index, target) in samples.iter_mut().take(sample_count).enumerate() {
            *target = deterministic_volume_sample(index, grid_dimensions, step_count);
        }

        Some(Self {
            profile_id: profile_id.into(),
            profile_sha256: profile_sha256.into(),
            volume_schema: volume_schema.clone(),
            volume_id: volume_id.clone(),
            field_kind: field_kind.clone(),
            storage_hint: storage_hint.clone(),
            grid_dimensions,
            step_count,
            kernel_abi_id: kernel_abi_id.clone(),
            declared_readback_samples,
            sample_count,
            samples,
        })
    }

    /// First populated sample index.
    #[must_use]
    pub const fn first_sample_index(&self) -> Option<usize> {
        if self.sample_count > 0 {
            Some(0)
        } else {
            None
        }
    }

    /// Last populated sample index.
    #[must_use]
    pub fn last_sample_index(&self) -> Option<usize> {
        self.sample_count.checked_sub(1)
    }
}

/// Generic Makepad volume probe readback summary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadStimulusVolumeProbeReadback {
    /// Number of bounded samples submitted to the GPU.
    pub sample_count: usize,
    /// Number of checked f32 components.
    pub component_count: usize,
    /// Number of f32 components outside tolerance.
    pub mismatched_components: usize,
    /// Maximum absolute GPU-vs-CPU-oracle error.
    pub max_abs_error: f32,
    /// Absolute tolerance used by the comparison.
    pub tolerance: f32,
    /// GPU outputs read back from Makepad.
    pub outputs:
        [QuestMakepadStimulusVolumeProbeOutput; QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES],
    /// CPU expected outputs used for comparison.
    pub expected_outputs:
        [QuestMakepadStimulusVolumeProbeOutput; QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES],
    /// Makepad XR/Vulkan submit serial for the proof command.
    pub queue_submit_serial: u64,
    /// Fence serial observed for the proof command.
    pub fence_serial: u64,
    /// Monotonic proof-resource generation for the current renderer lifetime.
    pub resource_generation: u64,
    /// Proof resources still pending retirement.
    pub pending_retire_count: usize,
    /// Proof resources retained by the current Makepad backend.
    pub retained_resource_count: usize,
    /// Proof resources destroyed after fence evidence in this call.
    pub retired_after_fence_count: usize,
    /// True when the Makepad backend waited for queue idle after the proof.
    pub queue_wait_idle_performed: bool,
    /// CPU-side elapsed time for shader compilation, command submission, wait, and readback.
    pub elapsed_ms: f64,
}

impl Default for QuestMakepadStimulusVolumeProbeReadback {
    fn default() -> Self {
        Self {
            sample_count: 0,
            component_count: 0,
            mismatched_components: 0,
            max_abs_error: 0.0,
            tolerance: 0.0,
            outputs: [QuestMakepadStimulusVolumeProbeOutput::default();
                QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES],
            expected_outputs: [QuestMakepadStimulusVolumeProbeOutput::default();
                QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES],
            queue_submit_serial: 0,
            fence_serial: 0,
            resource_generation: 0,
            pending_retire_count: 0,
            retained_resource_count: 0,
            retired_after_fence_count: 0,
            queue_wait_idle_performed: false,
            elapsed_ms: 0.0,
        }
    }
}

impl QuestMakepadStimulusVolumeProbeReadback {
    /// True when the bounded GPU output matched the CPU-oracle volume output.
    #[must_use]
    pub fn readback_matched(self) -> bool {
        self.sample_count > 0
            && self.component_count == self.sample_count * 8
            && self.mismatched_components == 0
            && self.max_abs_error.is_finite()
            && self.tolerance.is_finite()
            && self.max_abs_error <= self.tolerance.max(0.0)
    }
}

/// Bounded stimulus-volume GPU compute/readback proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadStimulusVolumeProbe {
    /// Schema identifier.
    pub schema_id: String,
    /// Staged Optics profile and compact probe input.
    pub input: QuestMakepadStimulusVolumeProbeInput,
    /// Makepad volume probe readback result.
    pub readback: QuestMakepadStimulusVolumeProbeReadback,
}

impl QuestMakepadStimulusVolumeProbe {
    /// Builds a marker object from the compact volume input and Makepad readback.
    #[must_use]
    pub fn from_input(
        input: &QuestMakepadStimulusVolumeProbeInput,
        readback: QuestMakepadStimulusVolumeProbeReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SCHEMA_ID.to_owned(),
            input: input.clone(),
            readback,
        }
    }

    /// Builds a compact marker without logging the full profile or GPU buffers.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} proofKind=stimulus-volume-compute-proof-v1 computeStage=optics-stimulus-volume-probe profileId={} profileSha256={} volumeSchema={} volumeId={} volumeFieldKind={} volumeStorageHint={} volumeGridDimensions={} volumeStepCount={} kernelAbiId={} declaredReadbackSamples={} questProofSamples={} firstSampleIndex={} lastSampleIndex={} resourcePlane={} computeProbeBackend={} oraclePayload={} storageLayout=std430-vec4 sampleCount={} componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} commandEncoderSubmitted=true storageBufferResident=true computeDispatchSubmitted=true volumeFieldKernel=true volumeRaymarchKernel=false fieldParticleKernel=false computeKernel=true cpuOracle=quest-makepad-deterministic-volume-probe cpuOraclePreserved=true opticsProfilePreserved=true highRateJsonPayload=false gpuComputeReady=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_MARKER_PREFIX,
            self.schema_id,
            marker_token(phase),
            if self.readback.readback_matched() {
                "ready"
            } else {
                "mismatch"
            },
            marker_token(&self.input.profile_id),
            marker_token(&self.input.profile_sha256),
            marker_token(&self.input.volume_schema),
            marker_token(&self.input.volume_id),
            marker_token(&self.input.field_kind),
            marker_token(&self.input.storage_hint),
            marker_grid(self.input.grid_dimensions),
            self.input.step_count,
            marker_token(&self.input.kernel_abi_id),
            self.input.declared_readback_samples,
            self.input.sample_count,
            optional_usize_marker_token(self.input.first_sample_index()),
            optional_usize_marker_token(self.input.last_sample_index()),
            QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_RESOURCE_PLANE,
            QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_BACKEND,
            QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_PAYLOAD,
            self.readback.sample_count,
            self.readback.component_count,
            self.readback.mismatched_components,
            finite_f32_marker_token(self.readback.max_abs_error),
            finite_f32_marker_token(self.readback.tolerance),
            self.readback.readback_matched(),
            self.readback.queue_submit_serial,
            self.readback.fence_serial,
            self.readback.resource_generation,
            self.readback.pending_retire_count,
            self.readback.retained_resource_count,
            self.readback.retired_after_fence_count,
            self.readback.queue_wait_idle_performed,
            finite_f64_marker_token(self.readback.elapsed_ms),
            QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_MEASUREMENT_SOURCE,
        )
    }
}

/// CPU oracle matching the first generic Makepad volume probe shader.
#[must_use]
pub fn expected_stimulus_volume_probe_output(
    sample: QuestMakepadStimulusVolumeProbeSample,
) -> QuestMakepadStimulusVolumeProbeOutput {
    let uv = sample.uv_eye_time;
    let origin = [
        sample.ray_origin_depth[0],
        sample.ray_origin_depth[1],
        sample.ray_origin_depth[2],
    ];
    let depth = sample.ray_origin_depth[3];
    let direction = [
        sample.ray_direction_step[0],
        sample.ray_direction_step[1],
        sample.ray_direction_step[2],
    ];
    let p = [
        origin[0] + direction[0] * depth,
        origin[1] + direction[1] * depth,
        origin[2] + direction[2] * depth,
    ];
    let frequency = sample.volume_params[0].max(0.001);
    let phase = sample.volume_params[1];
    let opacity = sample.volume_params[2].clamp(0.0, 4.0);
    let wave_a =
        triangle_wave((p[0] + uv[0] * 0.25 + p[2] * 0.5) * frequency + uv[3] * 0.07 + phase);
    let wave_b = triangle_wave(
        (p[1] - p[2] * 0.35 + uv[1] * 0.25) * frequency * 0.75 - uv[3] * 0.11 + phase * 0.5,
    );
    let interference = (1.0 - (wave_a - wave_b).abs()).clamp(0.0, 1.0);
    let density = (interference * opacity).clamp(0.0, 1.0);
    QuestMakepadStimulusVolumeProbeOutput {
        rgba: [density, density, density, density],
        density_depth_status: [density, depth, 1.0, 0.0],
    }
}

fn deterministic_volume_sample(
    index: usize,
    grid_dimensions: [u64; 3],
    step_count: u64,
) -> QuestMakepadStimulusVolumeProbeSample {
    let eye = (index % 2) as f32;
    let column = (index / 2) as f32;
    let u = (0.18 + column * 0.19).clamp(0.0, 1.0);
    let v = (0.22 + (index as f32 % 4.0) * 0.15).clamp(0.0, 1.0);
    let time = 0.125 * index as f32;
    let depth = 0.12 + 0.055 * index as f32;
    let max_grid_axis = grid_dimensions.iter().copied().max().unwrap_or(32).max(1) as f32;
    let frequency = (max_grid_axis / 8.0).clamp(1.0, 32.0);
    let phase = 0.37 + step_count as f32 * 0.003;
    let opacity = 0.72;
    let mut sample = QuestMakepadStimulusVolumeProbeSample {
        uv_eye_time: [u, v, eye, time],
        ray_origin_depth: [
            -0.42 + index as f32 * 0.07,
            -0.24 + (index as f32 % 3.0) * 0.12,
            -0.68,
            depth,
        ],
        ray_direction_step: [(u - 0.5) * 0.42, (v - 0.5) * 0.32, 1.0, step_count as f32],
        volume_params: [frequency, phase, opacity, 0.0],
        expected_rgba: [0.0; 4],
        expected_density_depth_status: [0.0; 4],
    };
    let expected = expected_stimulus_volume_probe_output(sample);
    sample.expected_rgba = expected.rgba;
    sample.expected_density_depth_status = expected.density_depth_status;
    sample
}

fn triangle_wave(value: f32) -> f32 {
    ((value - value.floor()) * 2.0 - 1.0).abs()
}

fn marker_token(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "none".to_owned();
    }
    trimmed
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':' | '/') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn marker_grid(value: [u64; 3]) -> String {
    format!("{},{},{}", value[0], value[1], value[2])
}

fn optional_usize_marker_token(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn finite_f32_marker_token(value: f32) -> String {
    if value.is_finite() {
        format!("{value:.6}")
    } else {
        "nonfinite".to_owned()
    }
}

fn finite_f64_marker_token(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.3}")
    } else {
        "nonfinite".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_volume_probe_input_from_summary() {
        let summary = StimulusVolumeProfileSummary {
            volume_present: true,
            volume_schema: Some(STIMULUS_VOLUME_SCHEMA_ID.to_owned()),
            volume_id: Some("stimulus.volume.test".to_owned()),
            field_kind: Some("ProceduralLayerStack3d".to_owned()),
            storage_hint: Some("StorageBuffer".to_owned()),
            grid_dimensions: Some([32, 32, 32]),
            step_count: Some(32),
            kernel_abi_id: Some("stimulus.kernel.volume_compute_v1".to_owned()),
            compute_pass_count: 3,
            volume_readback_probe_samples: Some(512),
            stereo_field_output_layers: Some(2),
        };

        let input = QuestMakepadStimulusVolumeProbeInput::from_profile_summary(
            "stimulus.profile.test",
            "0123456789abcdef",
            &summary,
        )
        .expect("volume input");

        assert_eq!(
            input.sample_count,
            QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES
        );
        assert_eq!(input.declared_readback_samples, 512);
        assert_eq!(input.samples[0].expected_density_depth_status[2], 1.0);
    }

    #[test]
    fn marker_reports_compute_kernel_without_runtime_ready_claim() {
        let summary = StimulusVolumeProfileSummary {
            volume_present: true,
            volume_schema: Some(STIMULUS_VOLUME_SCHEMA_ID.to_owned()),
            volume_id: Some("stimulus.volume.test".to_owned()),
            field_kind: Some("ProceduralLayerStack3d".to_owned()),
            storage_hint: Some("StorageBuffer".to_owned()),
            grid_dimensions: Some([32, 32, 32]),
            step_count: Some(32),
            kernel_abi_id: Some("stimulus.kernel.volume_compute_v1".to_owned()),
            compute_pass_count: 3,
            volume_readback_probe_samples: Some(512),
            stereo_field_output_layers: Some(2),
        };
        let input = QuestMakepadStimulusVolumeProbeInput::from_profile_summary(
            "stimulus.profile.test",
            "0123456789abcdef",
            &summary,
        )
        .expect("volume input");
        let mut expected_outputs = [QuestMakepadStimulusVolumeProbeOutput::default();
            QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_SAMPLES];
        for (target, sample) in expected_outputs
            .iter_mut()
            .zip(input.samples.iter().copied())
            .take(input.sample_count)
        {
            *target = expected_stimulus_volume_probe_output(sample);
        }
        let readback = QuestMakepadStimulusVolumeProbeReadback {
            sample_count: input.sample_count,
            component_count: input.sample_count * 8,
            mismatched_components: 0,
            max_abs_error: 0.0,
            tolerance: QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_DEFAULT_TOLERANCE,
            outputs: expected_outputs,
            expected_outputs,
            queue_submit_serial: 7,
            fence_serial: 7,
            resource_generation: 1,
            pending_retire_count: 0,
            retained_resource_count: 1,
            retired_after_fence_count: 0,
            queue_wait_idle_performed: false,
            elapsed_ms: 0.25,
        };

        let marker =
            QuestMakepadStimulusVolumeProbe::from_input(&input, readback).marker_line("unit-test");

        assert!(marker.contains(QUEST_MAKEPAD_STIMULUS_VOLUME_GPU_PROBE_MARKER_PREFIX));
        assert!(marker.contains("computeKernel=true"));
        assert!(marker.contains("volumeFieldKernel=true"));
        assert!(marker.contains("gpuComputeReady=false"));
        assert!(marker.contains("highRateJsonPayload=false"));
        assert!(marker.contains("readbackMatched=true"));
    }
}
