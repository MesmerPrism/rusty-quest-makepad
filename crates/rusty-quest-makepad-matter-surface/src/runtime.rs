use rusty_matter_surface_runtime::{MatterSurfaceContactProbe, MatterSurfaceRuntime};
use rusty_optics_mesh::SdfSliceVisual;
use rusty_optics_particles::{
    resolve_animated_particle_visual_frame, ParticleVisualAnimationProfile,
};
use rusty_quest_makepad_mesh_replay::MeshReplayRuntime;

use crate::{
    adf::{adf_debug_frame_from_report, build_adf_report},
    markers::{elapsed_ms, optional_usize_marker_token, sanitize_marker_value},
    uploads::{
        collision_upload_from_batch, distance_slice_upload_from_visual,
        particle_render_payload_for_visual_limit, particle_upload_from_visual_frame,
    },
    QuestMakepadAdfDebugFrame, QuestMakepadDistanceSliceUpload, QuestMakepadMatterSurfaceConfig,
    QuestMakepadMatterSurfaceError, QuestMakepadMatterSurfaceFrame,
    QuestMakepadMatterSurfaceSourceFrame, QuestMakepadMatterSurfaceStageTimings,
    DEFAULT_MIN_PARTICLE_RADIUS, DEFAULT_PARTICLE_CLOUD_RADIUS_SCALE,
    DEFAULT_PARTICLE_RADIUS_SCALE, QUEST_MAKEPAD_MATTER_SURFACE_MARKER_PREFIX,
    QUEST_MAKEPAD_MATTER_SURFACE_SCHEMA_ID,
};

/// Quest Makepad Matter surface adapter runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadMatterSurfaceRuntime {
    config: QuestMakepadMatterSurfaceConfig,
    matter: MatterSurfaceRuntime,
    particle_profile: ParticleVisualAnimationProfile,
    particles_initialized: bool,
    sdf_adf_debug_frame_counter: usize,
    sdf_adf_debug_cache: Option<QuestMakepadSdfAdfDebugCache>,
}

impl QuestMakepadMatterSurfaceRuntime {
    /// Creates an adapter runtime.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when the Matter runtime
    /// config is invalid.
    pub fn new(
        config: QuestMakepadMatterSurfaceConfig,
    ) -> Result<Self, QuestMakepadMatterSurfaceError> {
        let matter = MatterSurfaceRuntime::new(config.to_matter_config())?;
        Ok(Self {
            config,
            matter,
            particle_profile: ParticleVisualAnimationProfile::new(
                "quest.makepad.particles.browser_parity",
            ),
            particles_initialized: false,
            sdf_adf_debug_frame_counter: 0,
            sdf_adf_debug_cache: None,
        })
    }

    /// Returns the adapter config.
    #[must_use]
    pub fn config(&self) -> &QuestMakepadMatterSurfaceConfig {
        &self.config
    }

    /// Returns the underlying Matter runtime.
    #[must_use]
    pub fn matter_runtime(&self) -> &MatterSurfaceRuntime {
        &self.matter
    }

    /// Steps from the current mesh replay frame.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when replay, Matter, or
    /// Optics payload construction fails.
    pub fn step_from_replay(
        &mut self,
        replay: &MeshReplayRuntime,
        delta_seconds: f32,
        probes: &[MatterSurfaceContactProbe],
    ) -> Result<QuestMakepadMatterSurfaceFrame, QuestMakepadMatterSurfaceError> {
        self.step_from_source_frame(
            QuestMakepadMatterSurfaceSourceFrame::from_replay(replay)?,
            delta_seconds,
            probes,
        )
    }

    /// Steps from a source frame that already carries a Matter surface.
    ///
    /// Recorded replay and future realtime Quest hand-mesh providers should
    /// converge here so Matter remains the only SDF/collider/particle authority.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when Matter or Optics payload
    /// construction fails.
    pub fn step_from_source_frame(
        &mut self,
        source_frame: QuestMakepadMatterSurfaceSourceFrame,
        delta_seconds: f32,
        probes: &[MatterSurfaceContactProbe],
    ) -> Result<QuestMakepadMatterSurfaceFrame, QuestMakepadMatterSurfaceError> {
        let total_started_at = std::time::Instant::now();
        let mut stage_timings = QuestMakepadMatterSurfaceStageTimings::default();
        let center = source_frame.bounds_center();
        let surface_radius = source_frame.surface_radius();
        let cloud_radius = surface_radius * DEFAULT_PARTICLE_CLOUD_RADIUS_SCALE;
        let particle_radius =
            (surface_radius * DEFAULT_PARTICLE_RADIUS_SCALE).max(DEFAULT_MIN_PARTICLE_RADIUS);
        let source_id = source_frame.source_id.clone();

        let started_at = std::time::Instant::now();
        let matter_update = self.matter.update_frame(source_frame.frame)?;
        stage_timings.matter_update_ms = elapsed_ms(started_at);

        let mut particle_step = None;
        if self.config.enabled && self.config.particles_enabled {
            if !self.particles_initialized
                || self.matter.stats().particle_count != self.config.particle_count
            {
                let started_at = std::time::Instant::now();
                self.matter.reset_particles(
                    center,
                    self.config.particle_count,
                    cloud_radius,
                    particle_radius,
                    surface_radius,
                    self.config.particle_seed,
                )?;
                stage_timings.particle_reset_ms += elapsed_ms(started_at);
                self.particles_initialized = true;
            }
            let started_at = std::time::Instant::now();
            particle_step = Some(self.matter.step_particles(
                surface_radius,
                center,
                cloud_radius,
                delta_seconds.max(0.0),
            )?);
            stage_timings.particle_step_ms = elapsed_ms(started_at);
        }

        let started_at = std::time::Instant::now();
        let collision = if self.config.enabled && self.config.collision_enabled {
            self.matter.probe_contacts(probes)
        } else {
            self.matter.probe_contacts(&[])
        };
        stage_timings.collision_probe_ms = elapsed_ms(started_at);
        let started_at = std::time::Instant::now();
        let collision_upload = collision_upload_from_batch(&collision);
        stage_timings.collision_upload_ms = elapsed_ms(started_at);

        let needs_sdf_adf_debug =
            self.config.enabled && (self.config.sdf_slice_enabled || self.config.adf_debug_enabled);
        let (sdf_slice, sdf_slice_upload, adf_debug, sdf_adf_debug_reused, sdf_adf_debug_frame) =
            if needs_sdf_adf_debug {
                let interval = self.config.sdf_adf_debug_update_interval_frames.get();
                let should_rebuild = self.sdf_adf_debug_cache.is_none()
                    || self.sdf_adf_debug_frame_counter % interval == 0;
                self.sdf_adf_debug_frame_counter =
                    self.sdf_adf_debug_frame_counter.saturating_add(1);
                if should_rebuild {
                    let debug_frame = self
                        .build_sdf_adf_debug_frame(&mut stage_timings, matter_update.frame_index)?;
                    self.sdf_adf_debug_cache = Some(debug_frame.clone());
                    (
                        debug_frame.sdf_slice,
                        debug_frame.sdf_slice_upload,
                        debug_frame.adf_debug,
                        false,
                        debug_frame.source_frame_index,
                    )
                } else {
                    let debug_frame = self
                        .sdf_adf_debug_cache
                        .as_ref()
                        .expect("SDF/ADF debug cache exists when rebuild is skipped")
                        .clone();
                    (
                        debug_frame.sdf_slice,
                        debug_frame.sdf_slice_upload,
                        debug_frame.adf_debug,
                        true,
                        debug_frame.source_frame_index,
                    )
                }
            } else {
                (None, None, None, false, None)
            };

        let started_at = std::time::Instant::now();
        let particle_snapshot = self.matter.particle_snapshot();
        stage_timings.particle_snapshot_ms = elapsed_ms(started_at);
        let particle_source_rows = self.matter.stats().particle_count;
        let (particle_visual_frame, particle_upload) =
            if self.config.enabled && self.config.particles_enabled {
                let started_at = std::time::Instant::now();
                let payload = particle_render_payload_for_visual_limit(
                    &self.matter,
                    "quest.makepad.particles.current",
                    self.config.particle_visual_row_limit,
                )?;
                stage_timings.particle_payload_ms = elapsed_ms(started_at);
                let started_at = std::time::Instant::now();
                let frame = resolve_animated_particle_visual_frame(
                    "quest.makepad.particles.visual.current",
                    &payload,
                    &self.particle_profile,
                )?;
                stage_timings.particle_visual_ms = elapsed_ms(started_at);
                let started_at = std::time::Instant::now();
                let upload = particle_upload_from_visual_frame(&frame, particle_source_rows);
                stage_timings.particle_upload_ms = elapsed_ms(started_at);
                (Some(frame), Some(upload))
            } else {
                (None, None)
            };
        stage_timings.total_ms = elapsed_ms(total_started_at);

        Ok(QuestMakepadMatterSurfaceFrame {
            source_id,
            matter_update,
            stats: self.matter.stats(),
            collision,
            collision_upload,
            sdf_slice,
            sdf_slice_upload,
            adf_debug,
            sdf_adf_debug_reused,
            sdf_adf_debug_source_frame_index: sdf_adf_debug_frame,
            sdf_adf_debug_update_interval_frames: self
                .config
                .sdf_adf_debug_update_interval_frames
                .get(),
            particle_snapshot,
            particle_step,
            particle_visual_frame,
            particle_upload,
            stage_timings,
        })
    }

    /// Builds an evidence marker for a frame.
    #[must_use]
    pub fn marker_line(&self, phase: &str, frame: &QuestMakepadMatterSurfaceFrame) -> String {
        let particle_step = frame.particle_step.as_ref();
        let particle_surface_node_tests =
            particle_step.map_or(0, |diagnostics| diagnostics.particles.surface_node_tests);
        let particle_surface_leaf_tests =
            particle_step.map_or(0, |diagnostics| diagnostics.particles.surface_leaf_tests);
        let particle_surface_triangle_tests = particle_step.map_or(0, |diagnostics| {
            diagnostics.particles.surface_triangle_tests
        });
        let particle_refresh_node_tests = particle_step.map_or(0, |diagnostics| {
            diagnostics.refreshed_distance_diagnostics.node_tests
        });
        let particle_refresh_leaf_tests = particle_step.map_or(0, |diagnostics| {
            diagnostics.refreshed_distance_diagnostics.leaf_tests
        });
        let particle_refresh_triangle_tests = particle_step.map_or(0, |diagnostics| {
            diagnostics.refreshed_distance_diagnostics.triangle_tests
        });
        let adf_debug = frame.adf_debug.as_ref();
        let adf_status = match (self.config.adf_debug_enabled, adf_debug.is_some()) {
            (false, _) => "disabled",
            (true, true) => "ready",
            (true, false) => "empty",
        };
        let sdf_adf_debug_source = match (
            self.config.sdf_slice_enabled || self.config.adf_debug_enabled,
            frame.sdf_adf_debug_reused,
        ) {
            (false, _) => "disabled",
            (true, false) => "fresh",
            (true, true) => "reused",
        };
        let particle_force_source = frame.stats.particle_force_source;
        let particle_force_source_status = particle_step.map_or(
            if self.config.particles_enabled {
                "not-stepped"
            } else {
                "disabled"
            },
            |diagnostics| diagnostics.particle_force_source_status.marker_value(),
        );
        let particle_force_refresh = particle_step.map_or(
            if self.config.particles_enabled {
                "not-stepped"
            } else {
                "disabled"
            },
            |diagnostics| diagnostics.particle_force_refresh.marker_value(),
        );
        let sdf_adf_debug_particle_authority =
            particle_step.is_some_and(|diagnostics| diagnostics.sdf_adf_debug_particle_authority);
        format!(
            "{} schema={} phase={} status={} nativeMatterRuntime=true wasmRuntimeUsed=false shaderScaffoldUsed=false proceduralParticleOverlayUsed=false proceduralSdfOverlayUsed=false proceduralCollisionOverlayUsed=false dataPlane=makepad-compact-uniform-rows sourceId={} sourceSchema={} frameIndex={} vertexCount={} triangleCount={} sdfAdfDebugSource={} sdfAdfDebugFrameInterval={} sdfAdfDebugSourceFrameIndex={} particleCount={} particleForceSource={} particleForceSourceStatus={} particleForceRefresh={} particleForceUpdateIntervalFrames={} particleForceCompareProbeCount={} particleSamplingAuthority={} particleFieldSource={} sdfAdfDebugParticleAuthority={} particleDistanceRefreshPolicy={} particleDistanceSamples={} particleInputDeltaSeconds={:.6} particleSimulatedDeltaSeconds={:.6} particleDroppedDeltaSeconds={:.6} particleSubsteps={} particleClosestSamples={} particleSurfaceNodeTests={} particleSurfaceLeafTests={} particleSurfaceTriangleTests={} particleRefreshSamples={} particleRefreshNodeTests={} particleRefreshLeafTests={} particleRefreshTriangleTests={} particleExecutionBackend={} particleExecutionBatchSize={} particleExecutionChunks={} particleExecutionWorkers={} particleExecutionElapsedMicros={} collisionRows={} particleSourceRows={} particleRows={} particleVisualRowLimit={} sdfRows={} adfDebugEnabled={} adfStatus={} adfSchema={} adfVisualSchema={} adfCells={} adfSourceSamples={} adfSplitCount={} adfMaxLevel={} adfMaxDepth={} adfMaxCells={} adfErrorTolerance={:.6} leafTriangleCount={} distanceSamplerRefit={} adapterTotalMs={:.3} matterUpdateMs={:.3} particleResetMs={:.3} particleStepMs={:.3} collisionProbeMs={:.3} collisionUploadMs={:.3} sdfBuildMs={:.3} sdfUploadMs={:.3} adfBuildMs={:.3} adfVisualMs={:.3} particleSnapshotMs={:.3} particlePayloadMs={:.3} particleVisualMs={:.3} particleUploadMs={:.3}",
            QUEST_MAKEPAD_MATTER_SURFACE_MARKER_PREFIX,
            QUEST_MAKEPAD_MATTER_SURFACE_SCHEMA_ID,
            sanitize_marker_value(phase),
            if self.config.enabled { "ready" } else { "disabled" },
            sanitize_marker_value(&frame.source_id),
            frame.matter_update.topology_key.schema_id,
            frame.matter_update.frame_index.unwrap_or(0),
            frame.matter_update.vertex_count,
            frame.matter_update.triangle_count,
            sdf_adf_debug_source,
            frame.sdf_adf_debug_update_interval_frames,
            optional_usize_marker_token(frame.sdf_adf_debug_source_frame_index),
            frame.stats.particle_count,
            particle_force_source.marker_value(),
            particle_force_source_status,
            particle_force_refresh,
            frame.stats.particle_force_update_interval_frames,
            frame.stats.particle_force_compare_probe_count,
            particle_force_source.sampling_authority_marker(),
            particle_force_source.field_source_marker(),
            sdf_adf_debug_particle_authority,
            frame.stats.particle_distance_refresh_policy.marker_value(),
            frame.stats.particle_distance_samples,
            frame
                .particle_step
                .as_ref()
                .map_or(0.0, |diagnostics| diagnostics.particles.input_delta_seconds),
            frame
                .particle_step
                .as_ref()
                .map_or(0.0, |diagnostics| diagnostics.particles.simulated_delta_seconds),
            frame
                .particle_step
                .as_ref()
                .map_or(0.0, |diagnostics| diagnostics.particles.dropped_delta_seconds),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.substeps),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.closest_samples),
            particle_surface_node_tests,
            particle_surface_leaf_tests,
            particle_surface_triangle_tests,
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.refreshed_distance_samples),
            particle_refresh_node_tests,
            particle_refresh_leaf_tests,
            particle_refresh_triangle_tests,
            frame.particle_step.as_ref().map_or("none", |diagnostics| {
                diagnostics.particles.execution.backend.marker_value()
            }),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.execution.batch_size),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.execution.chunk_count),
            frame
                .particle_step
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.particles.execution.worker_count),
            frame.particle_step.as_ref().map_or(0, |diagnostics| {
                diagnostics.particles.execution.elapsed_micros
            }),
            frame.collision_upload.rows.len(),
            frame
                .particle_upload
                .as_ref()
                .map_or(0, |upload| upload.source_rows),
            frame
                .particle_upload
                .as_ref()
                .map_or(0, |upload| upload.rows.len()),
            optional_usize_marker_token(self.config.particle_visual_row_limit),
            frame
                .sdf_slice_upload
                .as_ref()
                .map_or(0, |upload| upload.rows.len()),
            self.config.adf_debug_enabled,
            adf_status,
            adf_debug.map_or("none", |frame| frame.schema_id.as_str()),
            adf_debug.map_or("none", |frame| frame.visual_schema_id.as_str()),
            adf_debug.map_or(0, |frame| frame.visual.cell_count),
            adf_debug.map_or(0, |frame| frame.diagnostics.source_sample_count),
            adf_debug.map_or(0, |frame| frame.diagnostics.split_count),
            adf_debug.map_or(0, |frame| frame.diagnostics.max_level),
            self.config.adf_debug_config.max_depth,
            self.config.adf_debug_config.max_cells,
            self.config.adf_debug_config.error_tolerance,
            frame
                .stats
                .distance_sampler
                .as_ref()
                .map_or(0, |stats| stats.leaf_triangle_count),
            frame.matter_update.distance_sampler_refit,
            frame.stage_timings.total_ms,
            frame.stage_timings.matter_update_ms,
            frame.stage_timings.particle_reset_ms,
            frame.stage_timings.particle_step_ms,
            frame.stage_timings.collision_probe_ms,
            frame.stage_timings.collision_upload_ms,
            frame.stage_timings.sdf_build_ms,
            frame.stage_timings.sdf_upload_ms,
            frame.stage_timings.adf_build_ms,
            frame.stage_timings.adf_visual_ms,
            frame.stage_timings.particle_snapshot_ms,
            frame.stage_timings.particle_payload_ms,
            frame.stage_timings.particle_visual_ms,
            frame.stage_timings.particle_upload_ms,
        )
    }

    fn build_sdf_adf_debug_frame(
        &self,
        stage_timings: &mut QuestMakepadMatterSurfaceStageTimings,
        source_frame_index: Option<usize>,
    ) -> Result<QuestMakepadSdfAdfDebugCache, QuestMakepadMatterSurfaceError> {
        let started_at = std::time::Instant::now();
        let grid = self.matter.build_sdf_grid(self.config.sdf_config())?;
        stage_timings.sdf_build_ms = elapsed_ms(started_at);
        let (sdf_slice, sdf_slice_upload) = if self.config.sdf_slice_enabled {
            let slice = SdfSliceVisual::middle_z("quest.makepad.sdf_slice.middle_z", &grid)?;
            let started_at = std::time::Instant::now();
            let upload = distance_slice_upload_from_visual(&slice);
            stage_timings.sdf_upload_ms = elapsed_ms(started_at);
            (Some(slice), Some(upload))
        } else {
            (None, None)
        };
        let adf_debug = if self.config.adf_debug_enabled {
            let started_at = std::time::Instant::now();
            let report = build_adf_report(&grid, self.config.adf_debug_config)?;
            stage_timings.adf_build_ms = elapsed_ms(started_at);
            let started_at = std::time::Instant::now();
            let frame = adf_debug_frame_from_report(report)?;
            stage_timings.adf_visual_ms = elapsed_ms(started_at);
            Some(frame)
        } else {
            None
        };
        Ok(QuestMakepadSdfAdfDebugCache {
            source_frame_index,
            sdf_slice,
            sdf_slice_upload,
            adf_debug,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct QuestMakepadSdfAdfDebugCache {
    source_frame_index: Option<usize>,
    sdf_slice: Option<SdfSliceVisual>,
    sdf_slice_upload: Option<QuestMakepadDistanceSliceUpload>,
    adf_debug: Option<QuestMakepadAdfDebugFrame>,
}

impl Default for QuestMakepadMatterSurfaceRuntime {
    fn default() -> Self {
        Self::new(QuestMakepadMatterSurfaceConfig::default()).expect("default config is valid")
    }
}
