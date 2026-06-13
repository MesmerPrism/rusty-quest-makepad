//! Bounded stereo sampled-image preview contract for stimulus-volume profiles.
//!
//! This module owns the Quest-Makepad marker shape for the Vulkan image-view and
//! sampler adoption proof. Makepad owns the generic storage-image/sample/readback
//! API; the CPU oracle stays shared with the stereo raymarch preview contract.

use crate::{
    stimulus_volume_raymarch_preview::{
        expected_stimulus_volume_raymarch_preview_output,
        QuestMakepadStimulusVolumeRaymarchPreviewInput,
        QuestMakepadStimulusVolumeRaymarchPreviewPixel,
        QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_DEFAULT_TOLERANCE,
        QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_EYE_COUNT,
        QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_HEIGHT,
        QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH,
    },
    StimulusVolumeProfileSummary,
};

/// Marker emitted after bounded sampled-image readback evidence.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW";
/// Schema for the bounded stereo storage-image marker.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SCHEMA_ID: &str =
    "rusty.quest.makepad.stimulus_volume_image_preview.v1";
/// Generic Makepad backend used by the sampled-image preview proof.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_BACKEND: &str =
    "makepad-vulkan-volume-storage-image-sample-compute-copy-readback";
/// Storage/adoption plane used by the sampled-image preview proof.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_RESOURCE_PLANE: &str =
    "vulkan-storage-image-stereo-atlas-sampled-image-readback";
/// Compact payload identifier for the sampled-image oracle.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PAYLOAD: &str =
    "bounded-optics-stimulus-volume-image-sample-preview-v1";
/// Measurement source for the marker.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_MEASUREMENT_SOURCE: &str =
    "quest-makepad-stimulus-volume-image-preview";
/// Default generated eye tile width for the scalable stereo atlas.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_TILE_WIDTH: usize = 64;
/// Default generated eye tile height for the scalable stereo atlas.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_TILE_HEIGHT: usize = 64;
/// Current bounded stereo eye count.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_COUNT: usize =
    QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_EYE_COUNT;
/// Stereo atlas image width.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_IMAGE_WIDTH: usize =
    QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_TILE_WIDTH
        * QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_COUNT;
/// Stereo atlas image height.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_IMAGE_HEIGHT: usize =
    QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_TILE_HEIGHT;
/// Stereo atlas image layers.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_IMAGE_LAYERS: usize = 1;
/// Bounded sample grid width used for GPU-vs-CPU readback checks.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_WIDTH: usize =
    QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_WIDTH;
/// Bounded sample grid height used for GPU-vs-CPU readback checks.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_HEIGHT: usize =
    QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_HEIGHT;
/// Current bounded stereo readback sample count.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS: usize =
    QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_WIDTH
        * QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_HEIGHT
        * QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_COUNT;
/// Conservative f32 tolerance for CPU-oracle comparison.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_DEFAULT_TOLERANCE: f32 =
    QUEST_MAKEPAD_STIMULUS_VOLUME_RAYMARCH_PREVIEW_DEFAULT_TOLERANCE;
/// Initial image format used by the generic Makepad Vulkan proof.
pub const QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_FORMAT: &str = "R32G32B32A32_SFLOAT";

/// Vec4-aligned pixel input submitted to the generic Makepad image preview.
pub type QuestMakepadStimulusVolumeImagePreviewPixel =
    QuestMakepadStimulusVolumeRaymarchPreviewPixel;

/// Vec4-aligned image output read back from the generic Makepad sampled image.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuestMakepadStimulusVolumeImagePreviewOutput {
    /// Raymarched preview RGBA stored in the stereo atlas image.
    pub rgba: [f32; 4],
}

/// Full adapter input for the bounded stereo sampled-image preview proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadStimulusVolumeImagePreviewInput {
    /// Shared profile and compact raymarch pixel input.
    pub raymarch_input: QuestMakepadStimulusVolumeRaymarchPreviewInput,
    /// Stereo atlas image width.
    pub image_width: usize,
    /// Stereo atlas image height.
    pub image_height: usize,
    /// Stereo atlas image layers.
    pub image_layers: usize,
    /// Low-resolution eye tile width.
    pub eye_tile_width: usize,
    /// Low-resolution eye tile height.
    pub eye_tile_height: usize,
    /// Output eye count.
    pub eye_count: usize,
    /// Number of populated preview pixels.
    pub pixel_count: usize,
    /// Bounded sample pixels used for readback parity against the generated atlas.
    pub pixels: [QuestMakepadStimulusVolumeImagePreviewPixel;
        QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS],
}

impl QuestMakepadStimulusVolumeImagePreviewInput {
    /// Builds the image-preview input from the existing raymarch preview input.
    #[must_use]
    pub fn from_raymarch_preview_input(
        input: &QuestMakepadStimulusVolumeRaymarchPreviewInput,
    ) -> Self {
        let mut pixels = [QuestMakepadStimulusVolumeImagePreviewPixel::default();
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS];
        for (index, target) in pixels.iter_mut().enumerate() {
            *target = deterministic_volume_image_preview_pixel(
                index,
                input.grid_dimensions,
                input.step_count,
                QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_TILE_WIDTH,
                QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_TILE_HEIGHT,
            );
        }
        Self {
            raymarch_input: input.clone(),
            image_width: QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_IMAGE_WIDTH,
            image_height: QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_IMAGE_HEIGHT,
            image_layers: QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_IMAGE_LAYERS,
            eye_tile_width: QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_TILE_WIDTH,
            eye_tile_height: QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_TILE_HEIGHT,
            eye_count: QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_COUNT,
            pixel_count: QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS,
            pixels,
        }
    }

    /// Builds the bounded sampled-image preview input from a staged profile summary.
    #[must_use]
    pub fn from_profile_summary(
        profile_id: impl Into<String>,
        profile_sha256: impl Into<String>,
        summary: &StimulusVolumeProfileSummary,
    ) -> Option<Self> {
        QuestMakepadStimulusVolumeRaymarchPreviewInput::from_profile_summary(
            profile_id,
            profile_sha256,
            summary,
        )
        .map(|input| Self::from_raymarch_preview_input(&input))
    }

    /// Compact GPU pixels in the shared raymarch/oracle order.
    #[must_use]
    pub const fn pixels(
        &self,
    ) -> [QuestMakepadStimulusVolumeImagePreviewPixel;
           QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS] {
        self.pixels
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

/// Generic Makepad sampled-image preview readback summary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadStimulusVolumeImagePreviewReadback {
    /// Stereo atlas image width.
    pub image_width: usize,
    /// Stereo atlas image height.
    pub image_height: usize,
    /// Stereo atlas image layers.
    pub image_layers: usize,
    /// Low-resolution eye tile width.
    pub eye_tile_width: usize,
    /// Low-resolution eye tile height.
    pub eye_tile_height: usize,
    /// Output eye count.
    pub eye_count: usize,
    /// Number of bounded pixels submitted to the GPU.
    pub pixel_count: usize,
    /// Number of checked f32 RGBA components.
    pub component_count: usize,
    /// Number of f32 components outside tolerance.
    pub mismatched_components: usize,
    /// Maximum absolute GPU-vs-CPU-oracle error.
    pub max_abs_error: f32,
    /// Absolute tolerance used by the comparison.
    pub tolerance: f32,
    /// GPU RGBA outputs read back from Makepad.
    pub outputs: [QuestMakepadStimulusVolumeImagePreviewOutput;
        QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS],
    /// CPU expected RGBA outputs used for comparison.
    pub expected_outputs: [QuestMakepadStimulusVolumeImagePreviewOutput;
        QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS],
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
    /// True when the Makepad compute shader wrote the storage image.
    pub storage_image_written: bool,
    /// True when the image was copied back through a transfer readback.
    pub transfer_readback_performed: bool,
    /// True when the image was allocated with sampled-image usage and transitioned for shader read.
    pub sampled_image_usage: bool,
    /// True when the Makepad backend bound the generated image through a sampled-image view/sampler.
    pub sampled_texture_bound: bool,
    /// True when the Makepad backend waited for queue idle after the proof.
    pub queue_wait_idle_performed: bool,
    /// CPU-side elapsed time for shader compilation, command submission, wait, and readback.
    pub elapsed_ms: f64,
}

impl Default for QuestMakepadStimulusVolumeImagePreviewReadback {
    fn default() -> Self {
        Self {
            image_width: 0,
            image_height: 0,
            image_layers: 0,
            eye_tile_width: 0,
            eye_tile_height: 0,
            eye_count: 0,
            pixel_count: 0,
            component_count: 0,
            mismatched_components: 0,
            max_abs_error: 0.0,
            tolerance: 0.0,
            outputs: [QuestMakepadStimulusVolumeImagePreviewOutput::default();
                QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS],
            expected_outputs: [QuestMakepadStimulusVolumeImagePreviewOutput::default();
                QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS],
            queue_submit_serial: 0,
            fence_serial: 0,
            resource_generation: 0,
            pending_retire_count: 0,
            retained_resource_count: 0,
            retired_after_fence_count: 0,
            storage_image_written: false,
            transfer_readback_performed: false,
            sampled_image_usage: false,
            sampled_texture_bound: false,
            queue_wait_idle_performed: false,
            elapsed_ms: 0.0,
        }
    }
}

impl QuestMakepadStimulusVolumeImagePreviewReadback {
    /// True when the bounded GPU image matched the CPU-oracle preview RGBA.
    #[must_use]
    pub fn readback_matched(self) -> bool {
        self.pixel_count > 0
            && self.component_count == self.pixel_count * 4
            && self.mismatched_components == 0
            && self.max_abs_error.is_finite()
            && self.tolerance.is_finite()
            && self.max_abs_error <= self.tolerance.max(0.0)
            && self.storage_image_written
            && self.transfer_readback_performed
            && self.sampled_image_usage
            && self.sampled_texture_bound
    }
}

/// Bounded stimulus-volume stereo sampled-image preview proof.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadStimulusVolumeImagePreview {
    /// Schema identifier.
    pub schema_id: String,
    /// Staged Optics profile and compact preview input.
    pub input: QuestMakepadStimulusVolumeImagePreviewInput,
    /// Makepad sampled-image preview readback result.
    pub readback: QuestMakepadStimulusVolumeImagePreviewReadback,
}

impl QuestMakepadStimulusVolumeImagePreview {
    /// Builds a marker object from the compact preview input and Makepad readback.
    #[must_use]
    pub fn from_input(
        input: &QuestMakepadStimulusVolumeImagePreviewInput,
        readback: QuestMakepadStimulusVolumeImagePreviewReadback,
    ) -> Self {
        Self {
            schema_id: QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SCHEMA_ID.to_owned(),
            input: input.clone(),
            readback,
        }
    }

    /// Builds a compact marker without logging the full profile or image payload.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        let profile = &self.input.raymarch_input;
        format!(
            "{} schema={} phase={} status={} proofKind=stimulus-volume-scalable-image-atlas-v1 computeStage=optics-stimulus-volume-scalable-image-atlas profileId={} profileSha256={} volumeSchema={} volumeId={} volumeFieldKind={} volumeStorageHint={} volumeGridDimensions={} volumeStepCount={} kernelAbiId={} declaredReadbackSamples={} stereoFieldOutputLayers={} imageWidth={} imageHeight={} imageLayers={} eyeTileWidth={} eyeTileHeight={} eyeCount={} pixelCount={} sampleGrid={} firstPixelIndex={} lastPixelIndex={} imageFormat={} outputTextureShape=stereo-rgba-scalable-atlas resourcePlane={} computeProbeBackend={} oraclePayload={} storageLayout=rgba32float-atlas componentCount={} mismatchedComponents={} maxAbsError={} tolerance={} readbackMatched={} lowResolutionStereoOutput=false scalableStereoAtlas=true runtimeTextureBound=false storageImageResident=true storageImageWritten={} transferReadbackPerformed={} sampledImageUsage={} sampledTextureBound={} sampledTextureResident={} commandEncoderSubmitted=true computeDispatchSubmitted=true volumeFieldKernel=true volumeRaymarchKernel=true volumeImageKernel=true fieldParticleKernel=false computeKernel=true cpuOracle=quest-makepad-deterministic-volume-scalable-image-samples cpuOraclePreserved=true opticsProfilePreserved=true highRateJsonPayload=false gpuComputeReady=true frameCriticalComputeReady=false queueSubmitSerial={} fenceSerial={} resourceGeneration={} pendingRetireCount={} retainedResourceCount={} retiredAfterFenceCount={} queueWaitIdlePerformed={} retirementPolicy=retained-until-vulkan-drop hwbAcquiredCount=0 hwbReleasedAfterFenceCount=0 kgslFaultsBeforeMarker=unavailable kgslFaultsAfterMarker=unavailable elapsedMs={} measuredBy={}",
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_MARKER_PREFIX,
            self.schema_id,
            marker_token(phase),
            if self.readback.readback_matched() {
                "ready"
            } else {
                "mismatch"
            },
            marker_token(&profile.profile_id),
            marker_token(&profile.profile_sha256),
            marker_token(&profile.volume_schema),
            marker_token(&profile.volume_id),
            marker_token(&profile.field_kind),
            marker_token(&profile.storage_hint),
            marker_grid(profile.grid_dimensions),
            profile.step_count,
            marker_token(&profile.kernel_abi_id),
            profile.declared_readback_samples,
            profile.stereo_field_output_layers,
            self.readback.image_width,
            self.readback.image_height,
            self.readback.image_layers,
            self.readback.eye_tile_width,
            self.readback.eye_tile_height,
            self.readback.eye_count,
            self.readback.pixel_count,
            marker_sample_grid(),
            optional_usize_marker_token(self.input.first_pixel_index()),
            optional_usize_marker_token(self.input.last_pixel_index()),
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_FORMAT,
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_RESOURCE_PLANE,
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_BACKEND,
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PAYLOAD,
            self.readback.component_count,
            self.readback.mismatched_components,
            finite_f32_marker_token(self.readback.max_abs_error),
            finite_f32_marker_token(self.readback.tolerance),
            self.readback.readback_matched(),
            self.readback.storage_image_written,
            self.readback.transfer_readback_performed,
            self.readback.sampled_image_usage,
            self.readback.sampled_texture_bound,
            self.readback.sampled_texture_bound,
            self.readback.queue_submit_serial,
            self.readback.fence_serial,
            self.readback.resource_generation,
            self.readback.pending_retire_count,
            self.readback.retained_resource_count,
            self.readback.retired_after_fence_count,
            self.readback.queue_wait_idle_performed,
            finite_f64_marker_token(self.readback.elapsed_ms),
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_MEASUREMENT_SOURCE,
        )
    }
}

/// CPU oracle for the RGBA storage-image preview.
#[must_use]
pub fn expected_stimulus_volume_image_preview_output(
    pixel: QuestMakepadStimulusVolumeImagePreviewPixel,
) -> QuestMakepadStimulusVolumeImagePreviewOutput {
    QuestMakepadStimulusVolumeImagePreviewOutput {
        rgba: expected_stimulus_volume_raymarch_preview_output(pixel).rgba,
    }
}

fn deterministic_volume_image_preview_pixel(
    index: usize,
    grid_dimensions: [u64; 3],
    step_count: u64,
    eye_tile_width: usize,
    eye_tile_height: usize,
) -> QuestMakepadStimulusVolumeImagePreviewPixel {
    let samples_per_eye = QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_WIDTH
        * QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_HEIGHT;
    let eye_index = (index / samples_per_eye)
        .min(QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_EYE_COUNT.saturating_sub(1));
    let local = index % samples_per_eye;
    let sample_x = local % QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_WIDTH;
    let sample_y = local / QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_WIDTH;
    let tile_width =
        eye_tile_width.max(QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_WIDTH);
    let tile_height =
        eye_tile_height.max(QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_HEIGHT);
    let pixel_x = scalable_sample_coordinate(
        sample_x,
        QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_WIDTH,
        tile_width,
    );
    let pixel_y = scalable_sample_coordinate(
        sample_y,
        QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_HEIGHT,
        tile_height,
    );
    let u = (pixel_x as f32 + 0.5) / tile_width as f32;
    let v = (pixel_y as f32 + 0.5) / tile_height as f32;
    let eye = eye_index as f32;
    let eye_offset = (eye - 0.5) * 0.08;
    let max_grid_axis = grid_dimensions.iter().copied().max().unwrap_or(32).max(1) as f32;
    let frequency = (max_grid_axis / 8.0).clamp(1.0, 32.0);
    let phase = 0.37 + step_count as f32 * 0.003;
    let preview_steps = (step_count as f32).clamp(4.0, 32.0);
    let mut pixel = QuestMakepadStimulusVolumeImagePreviewPixel {
        uv_eye_time: [u, v, eye, 0.125],
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

fn scalable_sample_coordinate(sample: usize, sample_count: usize, tile_size: usize) -> usize {
    let sample_count = sample_count.max(1);
    let tile_size = tile_size.max(1);
    let center = tile_size / (sample_count * 2);
    ((sample * tile_size) / sample_count + center).min(tile_size.saturating_sub(1))
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

fn marker_sample_grid() -> String {
    format!(
        "{}x{}",
        QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_WIDTH,
        QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_SAMPLE_GRID_HEIGHT
    )
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
    use crate::{StimulusVolumeProfileSummary, STIMULUS_VOLUME_SCHEMA_ID};

    #[test]
    fn builds_image_preview_input_from_summary() {
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

        let input = QuestMakepadStimulusVolumeImagePreviewInput::from_profile_summary(
            "stimulus.profile.test",
            "0123456789abcdef",
            &summary,
        )
        .expect("image input");

        assert_eq!(
            input.pixel_count,
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS
        );
        assert_eq!(
            input.image_width,
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_IMAGE_WIDTH
        );
        assert_eq!(input.image_width, 128);
        assert_eq!(input.image_height, 64);
        assert_eq!(input.image_layers, 1);
        assert_eq!(input.pixels()[0].uv_eye_time[0], 8.5 / 64.0);
    }

    #[test]
    fn marker_reports_sampled_image_without_runtime_ready_claim() {
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
        let input = QuestMakepadStimulusVolumeImagePreviewInput::from_profile_summary(
            "stimulus.profile.test",
            "0123456789abcdef",
            &summary,
        )
        .expect("image input");
        let mut expected_outputs = [QuestMakepadStimulusVolumeImagePreviewOutput::default();
            QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_PIXELS];
        for (target, pixel) in expected_outputs
            .iter_mut()
            .zip(input.pixels().iter().copied())
            .take(input.pixel_count)
        {
            *target = expected_stimulus_volume_image_preview_output(pixel);
        }
        let readback = QuestMakepadStimulusVolumeImagePreviewReadback {
            image_width: input.image_width,
            image_height: input.image_height,
            image_layers: input.image_layers,
            eye_tile_width: input.eye_tile_width,
            eye_tile_height: input.eye_tile_height,
            eye_count: input.eye_count,
            pixel_count: input.pixel_count,
            component_count: input.pixel_count * 4,
            mismatched_components: 0,
            max_abs_error: 0.0,
            tolerance: QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_DEFAULT_TOLERANCE,
            outputs: expected_outputs,
            expected_outputs,
            queue_submit_serial: 12,
            fence_serial: 12,
            resource_generation: 1,
            pending_retire_count: 0,
            retained_resource_count: 1,
            retired_after_fence_count: 0,
            storage_image_written: true,
            transfer_readback_performed: true,
            sampled_image_usage: true,
            sampled_texture_bound: true,
            queue_wait_idle_performed: false,
            elapsed_ms: 0.5,
        };

        let marker = QuestMakepadStimulusVolumeImagePreview::from_input(&input, readback)
            .marker_line("unit-test");

        assert!(marker.contains(QUEST_MAKEPAD_STIMULUS_VOLUME_IMAGE_PREVIEW_MARKER_PREFIX));
        assert!(marker.contains("storageImageWritten=true"));
        assert!(marker.contains("transferReadbackPerformed=true"));
        assert!(marker.contains("sampledImageUsage=true"));
        assert!(marker.contains("sampledTextureBound=true"));
        assert!(marker.contains("sampledTextureResident=true"));
        assert!(marker.contains("lowResolutionStereoOutput=false"));
        assert!(marker.contains("scalableStereoAtlas=true"));
        assert!(marker.contains("runtimeTextureBound=false"));
        assert!(marker.contains("gpuComputeReady=true"));
        assert!(marker.contains("frameCriticalComputeReady=false"));
        assert!(marker.contains("highRateJsonPayload=false"));
        assert!(marker.contains("readbackMatched=true"));
    }
}
