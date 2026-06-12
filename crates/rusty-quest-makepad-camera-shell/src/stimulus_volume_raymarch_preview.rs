//! Bounded stereo raymarch preview contract for stimulus-volume profiles.
//!
//! This module owns the Quest-Makepad marker shape and deterministic CPU oracle
//! for a tiny stereo output buffer. Makepad owns only the generic Vulkan
//! storage-buffer compute/readback API used to produce matching outputs.

use crate::{
    stimulus_volume_gpu::QuestMakepadStimulusVolumeProbeInput, StimulusVolumeProfileSummary,
    STIMULUS_VOLUME_SCHEMA_ID,
};

/// Marker emitted after bounded stereo raymarch output readback evidence.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW";
/// Schema for the bounded stereo raymarch output marker.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_SCHEMA_ID: &str =
    "rusty.quest.makepad.stimulus_volume_raymarch_preview.v1";
/// Generic Makepad backend used by the stereo raymarch preview proof.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_BACKEND: &str =
    "makepad-vulkan-volume-raymarch-preview-compute-readback";
/// Storage plane used by the stereo raymarch preview proof.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_RESOURCE_PLANE: &str =
    "vulkan-storage-buffer-stereo-raymarch-preview-readback";
/// Compact payload identifier for the stereo raymarch oracle.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PAYLOAD: &str =
    "bounded-optics-stimulus-volume-raymarch-preview-v1";
/// Measurement source for the marker.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_MEASUREMENT_SOURCE: &str =
    "quest-makepad-stimulus-volume-raymarch-preview";
/// Low-resolution preview width per eye.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH: usize = 4;
/// Low-resolution preview height per eye.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_HEIGHT: usize = 4;
/// Current bounded stereo output layer count.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_EYE_COUNT: usize = 2;
/// Current bounded stereo pixel count.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS: usize =
    QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH
        * QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_HEIGHT
        * QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_EYE_COUNT;
/// Conservative f32 tolerance for CPU-oracle comparison.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_DEFAULT_TOLERANCE: f32 = 0.002;

/// Vec4-aligned pixel input submitted to the generic Makepad raymarch preview.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadStimulusVolumeRaymarchPreviewPixel {
    /// `[u, v, eye_index, time_seconds]`.
    pub uv_eye_time: [f32; 4],
    /// `[origin_x, origin_y, origin_z, reserved]`.
    pub ray_origin: [f32; 4],
    /// `[dir_x, dir_y, dir_z, step_count]`.
    pub ray_direction_step: [f32; 4],
    /// `[frequency, phase, opacity, step_alpha_scale]`.
    pub volume_params: [f32; 4],
    /// Expected RGBA from the CPU oracle.
    pub expected_rgba: [f32; 4],
    /// Expected `[alpha, first_hit_depth, hit_status, step_count]`.
    pub expected_density_depth_status: [f32; 4],
}

/// Vec4-aligned GPU output read back from the generic Makepad raymarch preview.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadStimulusVolumeRaymarchPreviewOutput {
    /// Raymarched preview RGBA.
    pub rgba: [f32; 4],
    /// `[alpha, first_hit_depth, hit_status, step_count]`.
    pub density_depth_status: [f32; 4],
}

/// Full adapter input for the bounded stereo raymarch preview proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadStimulusVolumeRaymarchPreviewInput {
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
    /// Optics declared stereo output layer count.
    pub stereo_field_output_layers: u64,
    /// Low-resolution preview width per eye.
    pub preview_width: usize,
    /// Low-resolution preview height per eye.
    pub preview_height: usize,
    /// Output eye/layer count.
    pub eye_count: usize,
    /// Number of populated preview pixels.
    pub pixel_count: usize,
    /// Deterministic compact preview pixels.
    pub pixels: [QuestMakepadStimulusVolumeRaymarchPreviewPixel;
        QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS],
}

impl QuestMakepadStimulusVolumeRaymarchPreviewInput {
    /// Builds the bounded stereo preview input from the existing point-probe input.
    #[must_use]
    pub fn from_volume_probe_input(input: &QuestMakepadStimulusVolumeProbeInput) -> Self {
        let mut pixels = [QuestMakepadStimulusVolumeRaymarchPreviewPixel::default();
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS];
        for (index, target) in pixels.iter_mut().enumerate() {
            *target = deterministic_volume_raymarch_preview_pixel(
                index,
                input.grid_dimensions,
                input.step_count,
            );
        }
        Self {
            profile_id: input.profile_id.clone(),
            profile_sha256: input.profile_sha256.clone(),
            volume_schema: input.volume_schema.clone(),
            volume_id: input.volume_id.clone(),
            field_kind: input.field_kind.clone(),
            storage_hint: input.storage_hint.clone(),
            grid_dimensions: input.grid_dimensions,
            step_count: input.step_count,
            kernel_abi_id: input.kernel_abi_id.clone(),
            declared_readback_samples: input.declared_readback_samples,
            stereo_field_output_layers: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_EYE_COUNT
                as u64,
            preview_width: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH,
            preview_height: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_HEIGHT,
            eye_count: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_EYE_COUNT,
            pixel_count: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS,
            pixels,
        }
    }

    /// Builds the bounded stereo preview input from a staged profile summary.
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
        let stereo_field_output_layers = summary.stereo_field_output_layers?;
        if declared_readback_samples == 0
            || step_count == 0
            || stereo_field_output_layers
                < QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_EYE_COUNT as u64
        {
            return None;
        }

        let mut pixels = [QuestMakepadStimulusVolumeRaymarchPreviewPixel::default();
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS];
        for (index, target) in pixels.iter_mut().enumerate() {
            *target =
                deterministic_volume_raymarch_preview_pixel(index, grid_dimensions, step_count);
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
            stereo_field_output_layers,
            preview_width: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH,
            preview_height: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_HEIGHT,
            eye_count: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_EYE_COUNT,
            pixel_count: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS,
            pixels,
        })
    }

    /// First populated pixel index.
    #[must_use]
    pub const fn first_pixel_index(&self) -> Option<usize> {
        if self.pixel_count > 0 {
            Some(0)
        } else {
            None
        }
    }

    /// Last populated pixel index.
    #[must_use]
    pub fn last_pixel_index(&self) -> Option<usize> {
        self.pixel_count.checked_sub(1)
    }
}

/// Generic Makepad raymarch preview readback summary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadStimulusVolumeRaymarchPreviewReadback {
    /// Low-resolution preview width per eye.
    pub preview_width: usize,
    /// Low-resolution preview height per eye.
    pub preview_height: usize,
    /// Output eye/layer count.
    pub eye_count: usize,
    /// Number of bounded pixels submitted to the GPU.
    pub pixel_count: usize,
    /// Number of checked f32 components.
    pub component_count: usize,
    /// Number of f32 components outside tolerance.
    pub mismatched_components: usize,
    /// Maximum absolute GPU-vs-CPU-oracle error.
    pub max_abs_error: f32,
    /// Absolute tolerance used by the comparison.
    pub tolerance: f32,
    /// GPU outputs read back from Makepad.
    pub outputs: [QuestMakepadStimulusVolumeRaymarchPreviewOutput;
        QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS],
    /// CPU expected outputs used for comparison.
    pub expected_outputs: [QuestMakepadStimulusVolumeRaymarchPreviewOutput;
        QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS],
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

impl Default for QuestMakepadStimulusVolumeRaymarchPreviewReadback {
    fn default() -> Self {
        Self {
            preview_width: 0,
            preview_height: 0,
            eye_count: 0,
            pixel_count: 0,
            component_count: 0,
            mismatched_components: 0,
            max_abs_error: 0.0,
            tolerance: 0.0,
            outputs: [QuestMakepadStimulusVolumeRaymarchPreviewOutput::default();
                QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS],
            expected_outputs: [QuestMakepadStimulusVolumeRaymarchPreviewOutput::default();
                QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS],
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

impl QuestMakepadStimulusVolumeRaymarchPreviewReadback {
    /// True when the bounded GPU output matched the CPU-oracle preview output.
    #[must_use]
    pub fn readback_matched(self) -> bool {
        self.pixel_count > 0
            && self.component_count == self.pixel_count * 8
            && self.mismatched_components == 0
            && self.max_abs_error.is_finite()
            && self.tolerance.is_finite()
            && self.max_abs_error <= self.tolerance.max(0.0)
    }
}

/// Bounded stimulus-volume stereo raymarch preview proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadStimulusVolumeRaymarchPreview {
    /// Schema identifier.
    pub schema_id: String,
    /// Staged Optics profile and compact preview input.
    pub input: QuestMakepadStimulusVolumeRaymarchPreviewInput,
    /// Makepad raymarch preview readback result.
    pub readback: QuestMakepadStimulusVolumeRaymarchPreviewReadback,
}

impl QuestMakepadStimulusVolumeRaymarchPreview {
    /// Builds a marker object from the compact preview input and Makepad readback.
    #[must_use]
    pub fn from_input(
        input: &QuestMakepadStimulusVolumeRaymarchPreviewInput,
        readback: QuestMakepadStimulusVolumeRaymarchPreviewReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_SCHEMA_ID.to_owned(),
            input: input.clone(),
            readback,
        }
    }

    /// Builds a compact marker without logging the full profile or GPU buffers.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} proofKind=stimulus-volume-raymarch-preview-v1 computeStage=optics-stimulus-volume-raymarch-preview profileId={} profileSha256={} volumeSchema={} volumeId={} volumeFieldKind={} volumeStorageHint={} volumeGridDimensions={} volumeStepCount={} kernelAbiId={} declaredReadbackSamples={} stereoFieldOutputLayers={} previewWidth={} previewHeight={} eyeCount={} pixelCount={} firstPixelIndex={} lastPixelIndex={} outputTextureShape=stereo-rgba-lowres-buffer resourcePlane={} computeProbeBackend={} oraclePayload={} storageLayout=std430-vec4 componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} lowResolutionStereoOutput=true runtimeTextureBound=false commandEncoderSubmitted=true storageBufferResident=true computeDispatchSubmitted=true volumeFieldKernel=true volumeRaymarchKernel=true fieldParticleKernel=false computeKernel=true cpuOracle=quest-makepad-deterministic-volume-raymarch-preview cpuOraclePreserved=true opticsProfilePreserved=true highRateJsonPayload=false gpuComputeReady=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_MARKER_PREFIX,
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
            self.input.stereo_field_output_layers,
            self.readback.preview_width,
            self.readback.preview_height,
            self.readback.eye_count,
            self.readback.pixel_count,
            optional_usize_marker_token(self.input.first_pixel_index()),
            optional_usize_marker_token(self.input.last_pixel_index()),
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_RESOURCE_PLANE,
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_BACKEND,
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PAYLOAD,
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
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_MEASUREMENT_SOURCE,
        )
    }
}

/// CPU oracle matching the generic Makepad stereo raymarch preview shader.
#[must_use]
pub fn expected_stimulus_volume_raymarch_preview_output(
    pixel: QuestMakepadStimulusVolumeRaymarchPreviewPixel,
) -> QuestMakepadStimulusVolumeRaymarchPreviewOutput {
    let uv = pixel.uv_eye_time;
    let origin = [
        pixel.ray_origin[0],
        pixel.ray_origin[1],
        pixel.ray_origin[2],
    ];
    let direction = [
        pixel.ray_direction_step[0],
        pixel.ray_direction_step[1],
        pixel.ray_direction_step[2],
    ];
    let step_count = pixel.ray_direction_step[3].clamp(1.0, 32.0);
    let step_alpha_scale = pixel.volume_params[3].clamp(0.001, 4.0);
    let eye_gain = 0.65 + 0.35 * uv[2].clamp(0.0, 1.0);
    let mut accum_rgb = [0.0_f32; 3];
    let mut accum_alpha = 0.0_f32;
    let mut first_depth = 0.0_f32;
    let mut hit = 0.0_f32;

    for step in 0..32 {
        let step_f = step as f32;
        if step_f < step_count {
            let unit_depth = (step_f + 0.5) / step_count;
            let p = [
                origin[0] + direction[0] * unit_depth,
                origin[1] + direction[1] * unit_depth,
                origin[2] + direction[2] * unit_depth,
            ];
            let density = volume_density(p, uv, pixel.volume_params);
            let sample_alpha = (density * step_alpha_scale / step_count).clamp(0.0, 1.0);
            let sample_rgb = [density, density * eye_gain, 1.0 - density];
            let contribution = (1.0 - accum_alpha) * sample_alpha;
            accum_rgb[0] += sample_rgb[0] * contribution;
            accum_rgb[1] += sample_rgb[1] * contribution;
            accum_rgb[2] += sample_rgb[2] * contribution;
            if hit < 0.5 && density > 0.05 {
                first_depth = unit_depth;
                hit = 1.0;
            }
            accum_alpha = (accum_alpha + contribution).clamp(0.0, 1.0);
        }
    }

    QuestMakepadStimulusVolumeRaymarchPreviewOutput {
        rgba: [accum_rgb[0], accum_rgb[1], accum_rgb[2], accum_alpha],
        density_depth_status: [accum_alpha, first_depth, hit, step_count],
    }
}

fn deterministic_volume_raymarch_preview_pixel(
    index: usize,
    grid_dimensions: [u64; 3],
    step_count: u64,
) -> QuestMakepadStimulusVolumeRaymarchPreviewPixel {
    let pixels_per_eye = QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH
        * QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_HEIGHT;
    let eye_index = (index / pixels_per_eye)
        .min(QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_EYE_COUNT.saturating_sub(1));
    let local = index % pixels_per_eye;
    let x = local % QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH;
    let y = local / QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH;
    let u = (x as f32 + 0.5) / QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH as f32;
    let v = (y as f32 + 0.5) / QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_HEIGHT as f32;
    let eye = eye_index as f32;
    let eye_offset = (eye - 0.5) * 0.08;
    let max_grid_axis = grid_dimensions.iter().copied().max().unwrap_or(32).max(1) as f32;
    let frequency = (max_grid_axis / 8.0).clamp(1.0, 32.0);
    let phase = 0.37 + step_count as f32 * 0.003;
    let preview_steps = (step_count as f32).clamp(4.0, 32.0);
    let mut pixel = QuestMakepadStimulusVolumeRaymarchPreviewPixel {
        uv_eye_time: [u, v, eye, 0.125 + index as f32 * 0.0125],
        ray_origin: [u - 0.5 + eye_offset, v - 0.5, -0.72, 0.0],
        ray_direction_step: [
            (u - 0.5) * 0.42 + eye_offset * 0.25,
            (v - 0.5) * 0.32,
            1.0,
            preview_steps,
        ],
        volume_params: [frequency, phase, 0.72, 1.25],
        expected_rgba: [0.0; 4],
        expected_density_depth_status: [0.0; 4],
    };
    let expected = expected_stimulus_volume_raymarch_preview_output(pixel);
    pixel.expected_rgba = expected.rgba;
    pixel.expected_density_depth_status = expected.density_depth_status;
    pixel
}

fn volume_density(p: [f32; 3], uv: [f32; 4], params: [f32; 4]) -> f32 {
    let frequency = params[0].max(0.001);
    let phase = params[1];
    let opacity = params[2].clamp(0.0, 4.0);
    let wave_a =
        triangle_wave((p[0] + uv[0] * 0.25 + p[2] * 0.5) * frequency + uv[3] * 0.07 + phase);
    let wave_b = triangle_wave(
        (p[1] - p[2] * 0.35 + uv[1] * 0.25) * frequency * 0.75 - uv[3] * 0.11 + phase * 0.5,
    );
    let interference = (1.0 - (wave_a - wave_b).abs()).clamp(0.0, 1.0);
    (interference * opacity).clamp(0.0, 1.0)
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
    fn builds_raymarch_preview_input_from_summary() {
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

        let input = QuestMakepadStimulusVolumeRaymarchPreviewInput::from_profile_summary(
            "stimulus.profile.test",
            "0123456789abcdef",
            &summary,
        )
        .expect("raymarch input");

        assert_eq!(
            input.pixel_count,
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS
        );
        assert_eq!(
            input.preview_width,
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH
        );
        assert_eq!(input.pixels[0].expected_density_depth_status[2], 1.0);
    }

    #[test]
    fn marker_reports_stereo_raymarch_without_runtime_ready_claim() {
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
        let input = QuestMakepadStimulusVolumeRaymarchPreviewInput::from_profile_summary(
            "stimulus.profile.test",
            "0123456789abcdef",
            &summary,
        )
        .expect("raymarch input");
        let mut expected_outputs = [QuestMakepadStimulusVolumeRaymarchPreviewOutput::default();
            QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_PIXELS];
        for (target, pixel) in expected_outputs
            .iter_mut()
            .zip(input.pixels.iter().copied())
            .take(input.pixel_count)
        {
            *target = expected_stimulus_volume_raymarch_preview_output(pixel);
        }
        let readback = QuestMakepadStimulusVolumeRaymarchPreviewReadback {
            preview_width: input.preview_width,
            preview_height: input.preview_height,
            eye_count: input.eye_count,
            pixel_count: input.pixel_count,
            component_count: input.pixel_count * 8,
            mismatched_components: 0,
            max_abs_error: 0.0,
            tolerance: QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_DEFAULT_TOLERANCE,
            outputs: expected_outputs,
            expected_outputs,
            queue_submit_serial: 11,
            fence_serial: 11,
            resource_generation: 1,
            pending_retire_count: 0,
            retained_resource_count: 1,
            retired_after_fence_count: 0,
            queue_wait_idle_performed: false,
            elapsed_ms: 0.5,
        };

        let marker = QuestMakepadStimulusVolumeRaymarchPreview::from_input(&input, readback)
            .marker_line("unit-test");

        assert!(marker.contains(QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_MARKER_PREFIX));
        assert!(marker.contains("volumeRaymarchKernel=true"));
        assert!(marker.contains("lowResolutionStereoOutput=true"));
        assert!(marker.contains("runtimeTextureBound=false"));
        assert!(marker.contains("gpuComputeReady=false"));
        assert!(marker.contains("highRateJsonPayload=false"));
        assert!(marker.contains("readbackMatched=true"));
    }
}
