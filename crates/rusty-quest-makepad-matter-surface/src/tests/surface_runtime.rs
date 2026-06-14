use super::*;

#[test]
fn adapter_steps_matter_surface_from_replay() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        collision_enabled: true,
        sdf_slice_enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_execution_batch_size: NonZeroUsize::new(4).unwrap(),
        sdf_voxel_size: 0.12,
        sdf_max_voxels: 4_096,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(
            &replay,
            1.0 / 30.0,
            &[MatterSurfaceContactProbe::sphere(
                "probe.center",
                Vec3::new(0.0, 0.0, 0.0),
                0.25,
            )],
        )
        .expect("adapter frame builds");

    assert_eq!(
        frame.matter_update.vertex_count,
        replay.sequence().vertex_count()
    );
    assert_eq!(
        frame.matter_update.triangle_count,
        replay.sequence().triangle_count()
    );
    assert_eq!(frame.particle_snapshot.samples.len(), 16);
    assert_eq!(
        frame
            .particle_step
            .as_ref()
            .unwrap()
            .refreshed_distance_samples,
        16
    );
    let upload = frame.particle_upload.as_ref().unwrap();
    assert_eq!(upload.source_rows, 16);
    assert_eq!(upload.rows.len(), 16);
    let world_batch = frame
        .world_particle_batch(
            replay.sequence().bounds_min(),
            replay.sequence().bounds_max(),
            QuestMakepadWorldParticlePlacement::default(),
            8,
        )
        .expect("world particle batch builds");
    assert_eq!(world_batch.source_rows, 16);
    assert_eq!(world_batch.instances.len(), 8);
    assert_eq!(world_batch.dropped_rows, 8);
    assert_eq!(world_batch.content_center, DEFAULT_WORLD_CONTENT_CENTER);
    assert_eq!(
        world_batch.coordinate_space,
        QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE
    );
    assert!(frame.adf_debug.is_none());
    assert!(!frame.collision_upload.rows.is_empty());
    assert!(frame.sdf_slice_upload.as_ref().unwrap().rows.len() > 1);

    let marker = runtime.marker_line("unit-test", &frame);
    assert!(marker.contains("nativeMatterRuntime=true"));
    assert!(marker.contains("sourceId=public-synthetic-hand-sequence"));
    assert!(marker.contains("wasmRuntimeUsed=false"));
    assert!(marker.contains("shaderScaffoldUsed=false"));
    assert!(marker.contains("proceduralParticleOverlayUsed=false"));
    assert!(marker.contains("dataPlane=makepad-compact-uniform-rows"));
    assert!(marker.contains("distanceSamplerRefit=false"));
    assert!(marker.contains("particleForceSource=mesh-distance"));
    assert!(marker.contains("particleForceSourceStatus=ready"));
    assert!(marker.contains("particleForceRefresh=fresh"));
    assert!(marker.contains("particleForceUpdateIntervalFrames=1"));
    assert!(marker.contains("particleForceCompareProbeCount=0"));
    assert!(marker.contains("particleSamplingAuthority=matter-mesh-distance-sampler"));
    assert!(marker.contains("particleFieldSource=current-mesh-distance"));
    assert!(marker.contains("sdfAdfDebugParticleAuthority=false"));
    assert!(marker.contains("particleDistanceRefreshPolicy=step-only"));
    assert!(marker.contains("particleDistanceSamples=16"));
    assert!(marker.contains("particleInputDeltaSeconds=0.033333"));
    assert!(marker.contains("particleSimulatedDeltaSeconds=0.033333"));
    assert!(marker.contains("particleDroppedDeltaSeconds=0.000000"));
    assert!(marker.contains("particleSubsteps="));
    assert!(marker.contains("particleClosestSamples="));
    assert!(marker.contains("particleSurfaceNodeTests="));
    assert!(marker.contains("particleSurfaceLeafTests="));
    assert!(marker.contains("particleSurfaceTriangleTests="));
    assert!(marker.contains("particleRefreshSamples=16"));
    assert!(marker.contains("particleRefreshNodeTests="));
    assert!(marker.contains("particleRefreshLeafTests="));
    assert!(marker.contains("particleRefreshTriangleTests="));
    assert!(marker.contains("particleExecutionBackend=serial"));
    assert!(marker.contains("particleExecutionBatchSize=4"));
    assert!(marker.contains("particleExecutionChunks="));
    assert!(marker.contains("particleExecutionWorkers=1"));
    assert!(marker.contains("particleExecutionElapsedMicros="));
    assert!(marker.contains("particleSourceRows=16"));
    assert!(marker.contains("particleRows=16"));
    assert!(marker.contains("particleVisualRowLimit=none"));
    assert!(marker.contains("adfDebugEnabled=false"));
    assert!(marker.contains("adfStatus=disabled"));
    assert!(marker.contains("adfCells=0"));
    assert!(marker.contains("adapterTotalMs="));
    assert!(marker.contains("matterUpdateMs="));
    assert!(marker.contains("particleStepMs="));
    assert!(marker.contains("particleVisualMs="));
    assert!(frame.stage_timings.total_ms >= frame.stage_timings.matter_update_ms);
    assert!(!marker.contains("rusty.xr"));
    assert!(!marker.contains("RUSTY_XR"));

    let world_marker = world_batch.marker_line("unit-test");
    assert!(world_marker.contains(QUEST_MAKEPAD_WORLD_PARTICLE_MARKER_PREFIX));
    assert!(world_marker.contains("renderMode=center-projected-billboard"));
    assert!(world_marker.contains("selectionPolicy=evenly-spaced-source-rows"));
    assert!(world_marker.contains("instanceSpread="));
    assert!(world_marker.contains("contentCenterDistanceMeters=0.500"));
    assert!(!world_marker.contains("rusty.xr"));
    assert!(!world_marker.contains("RUSTY_XR"));
}

#[test]
fn adapter_builds_matter_adf_debug_visual_when_enabled() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        adf_debug_enabled: true,
        particles_enabled: false,
        collision_enabled: false,
        sdf_slice_enabled: false,
        sdf_voxel_size: 0.12,
        sdf_max_voxels: 4_096,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 0.0, &[])
        .expect("adapter frame builds");

    assert!(frame.sdf_slice.is_none());
    assert!(frame.sdf_slice_upload.is_none());
    let adf_debug = frame.adf_debug.as_ref().expect("ADF debug frame");
    assert_eq!(adf_debug.schema_id, QUEST_MAKEPAD_ADF_DEBUG_SCHEMA_ID);
    assert_eq!(
        adf_debug.visual.visual_id,
        QUEST_MAKEPAD_ADF_DEBUG_VISUAL_ID
    );
    assert_eq!(adf_debug.visual.cell_count, adf_debug.visual.cells.len());
    assert!(adf_debug.visual.cell_count > 0);
    assert_eq!(
        adf_debug.diagnostics.cell_count,
        adf_debug.visual.cell_count
    );
    assert!(adf_debug.diagnostics.source_sample_count > 0);
    let world_adf = frame
        .world_adf_debug_batch(QuestMakepadWorldAdfDebugPlacement::default(), 8)
        .expect("ADF world debug batch builds");
    assert_eq!(
        world_adf.schema_id,
        QUEST_MAKEPAD_WORLD_ADF_DEBUG_BATCH_SCHEMA_ID
    );
    assert_eq!(
        world_adf.source_schema_id,
        QUEST_MAKEPAD_ADF_DEBUG_SCHEMA_ID
    );
    assert_eq!(
        world_adf.source_visual_schema_id,
        "rusty.optics.adf.debug.visual.v1"
    );
    assert_eq!(world_adf.source_cells, adf_debug.visual.cell_count);
    assert_eq!(world_adf.cells.len(), adf_debug.visual.cell_count.min(8));
    assert_eq!(
        world_adf.dropped_cells,
        adf_debug
            .visual
            .cell_count
            .saturating_sub(world_adf.cells.len())
    );
    assert_eq!(world_adf.content_center, DEFAULT_WORLD_CONTENT_CENTER);
    assert_eq!(
        world_adf.coordinate_space,
        QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE
    );
    assert!(world_adf
        .cells
        .iter()
        .all(|cell| cell.center_extent[3] > 0.0));
    assert!(world_adf
        .cells
        .iter()
        .all(|cell| (0.0..=1.0).contains(&cell.distance[3])));
    assert!(world_adf
        .cells
        .iter()
        .all(|cell| (0.0..=1.0).contains(&cell.meta[1])));

    let marker = runtime.marker_line("unit-test-adf", &frame);
    assert!(marker.contains("nativeMatterRuntime=true"));
    assert!(marker.contains("adfDebugEnabled=true"));
    assert!(marker.contains("adfStatus=ready"));
    assert!(marker.contains("adfSchema=rusty.quest.makepad.matter_adf_debug.v1"));
    assert!(marker.contains("adfVisualSchema=rusty.optics.adf.debug.visual.v1"));
    assert!(marker.contains("adfCells="));
    assert!(marker.contains("adfSourceSamples="));
    assert!(marker.contains("adfBuildMs="));
    assert!(marker.contains("adfVisualMs="));
    assert!(!marker.contains("rusty.xr"));
    assert!(!marker.contains("RUSTY_XR"));

    let world_marker = world_adf.marker_line("unit-test-adf");
    assert!(world_marker.contains(QUEST_MAKEPAD_WORLD_ADF_DEBUG_MARKER_PREFIX));
    assert!(world_marker.contains("schema=rusty.quest.makepad.world_adf_debug_batch.v1"));
    assert!(world_marker.contains("renderMode=adf-debug-cell-boxes"));
    assert!(world_marker.contains("sourceSchema=rusty.quest.makepad.matter_adf_debug.v1"));
    assert!(world_marker.contains("sourceVisualSchema=rusty.optics.adf.debug.visual.v1"));
    assert!(world_marker.contains("selectionPolicy=evenly-spaced-source-cells"));
    assert!(world_marker.contains("contentCenterDistanceMeters=0.500"));
    assert!(world_marker.contains("dataPlane=makepad-world-adf-debug-cells"));
    assert!(!world_marker.contains("rusty.xr"));
    assert!(!world_marker.contains("RUSTY_XR"));
}

#[test]
fn adapter_reuses_sdf_adf_debug_payloads_between_interval_frames() {
    let mut replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        adf_debug_enabled: true,
        particles_enabled: false,
        collision_enabled: false,
        sdf_slice_enabled: false,
        sdf_adf_debug_update_interval_frames: NonZeroUsize::new(2).unwrap(),
        sdf_voxel_size: 0.12,
        sdf_max_voxels: 4_096,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let first = runtime
        .step_from_replay(&replay, 0.0, &[])
        .expect("first ADF debug frame builds");
    assert!(!first.sdf_adf_debug_reused);
    assert_eq!(first.sdf_adf_debug_update_interval_frames, 2);
    assert_eq!(
        first.sdf_adf_debug_source_frame_index,
        first.matter_update.frame_index
    );
    assert!(first.stage_timings.sdf_build_ms > 0.0);
    assert!(first.stage_timings.adf_build_ms > 0.0);

    replay.step(1.0 / 60.0);
    let second = runtime
        .step_from_replay(&replay, 1.0 / 60.0, &[])
        .expect("second ADF debug frame reuses cache");
    assert!(second.sdf_adf_debug_reused);
    assert_eq!(
        second.sdf_adf_debug_source_frame_index,
        first.sdf_adf_debug_source_frame_index
    );
    assert_eq!(second.stage_timings.sdf_build_ms, 0.0);
    assert_eq!(second.stage_timings.adf_build_ms, 0.0);
    assert_eq!(
        second
            .adf_debug
            .as_ref()
            .expect("cached ADF frame")
            .visual
            .cell_count,
        first
            .adf_debug
            .as_ref()
            .expect("fresh ADF frame")
            .visual
            .cell_count
    );

    let marker = runtime.marker_line("unit-test-adf-cache", &second);
    assert!(marker.contains("sdfAdfDebugSource=reused"));
    assert!(marker.contains("sdfAdfDebugFrameInterval=2"));
    assert!(marker.contains("sdfAdfDebugSourceFrameIndex="));
    assert!(marker.contains("sdfAdfDebugParticleAuthority=false"));

    replay.step(1.0 / 60.0);
    let third = runtime
        .step_from_replay(&replay, 1.0 / 60.0, &[])
        .expect("third ADF debug frame rebuilds");
    assert!(!third.sdf_adf_debug_reused);
    assert!(third.stage_timings.sdf_build_ms > 0.0);
    assert!(third.stage_timings.adf_build_ms > 0.0);
    assert!(runtime
        .marker_line("unit-test-adf-cache", &third)
        .contains("sdfAdfDebugSource=fresh"));
}

#[test]
fn adapter_can_disable_particle_force_without_disabling_integration() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_source: MatterSurfaceParticleForceSource::None,
        particle_distance_refresh_policy:
            MatterSurfaceParticleDistanceRefreshPolicy::SurfaceUpdateAndStep,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 90.0, &[])
        .expect("adapter frame builds");
    let diagnostics = frame
        .particle_step
        .as_ref()
        .expect("particles step when enabled");

    assert_eq!(
        diagnostics.particle_force_source,
        MatterSurfaceParticleForceSource::None
    );
    assert_eq!(diagnostics.particles.closest_samples, 0);
    assert_eq!(diagnostics.refreshed_distance_samples, 0);
    assert_eq!(frame.stats.particle_distance_samples, 0);
    let marker = runtime.marker_line("unit-test-force-none", &frame);
    assert!(marker.contains("particleForceSource=none"));
    assert!(marker.contains("particleForceSourceStatus=disabled"));
    assert!(marker.contains("particleForceRefresh=disabled"));
    assert!(marker.contains("particleSamplingAuthority=none"));
    assert!(marker.contains("particleFieldSource=none"));
    assert!(marker.contains("sdfAdfDebugParticleAuthority=false"));
    assert!(marker.contains("particleClosestSamples=0"));
    assert!(marker.contains("particleRefreshSamples=0"));
}

#[test]
fn adapter_reports_particle_force_update_interval_reuse() {
    let mut replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_update_interval_frames: NonZeroUsize::new(2).unwrap(),
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::Disabled,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let first = runtime
        .step_from_replay(&replay, 1.0 / 90.0, &[])
        .expect("first frame builds");
    replay.step(1.0 / 90.0);
    let second = runtime
        .step_from_replay(&replay, 1.0 / 90.0, &[])
        .expect("second frame builds");

    assert_eq!(
        first
            .particle_step
            .as_ref()
            .expect("first particle step")
            .particle_force_refresh,
        MatterSurfaceParticleForceRefresh::Fresh
    );
    assert_eq!(
        second
            .particle_step
            .as_ref()
            .expect("second particle step")
            .particle_force_refresh,
        MatterSurfaceParticleForceRefresh::Reused
    );
    assert_eq!(
        second
            .particle_step
            .as_ref()
            .expect("second particle step")
            .particles
            .closest_samples,
        0
    );

    let marker = runtime.marker_line("unit-test-force-reuse", &second);
    assert!(marker.contains("particleForceSource=mesh-distance"));
    assert!(marker.contains("particleForceRefresh=reused"));
    assert!(marker.contains("particleForceUpdateIntervalFrames=2"));
    assert!(marker.contains("particleClosestSamples=0"));
}

#[test]
fn adapter_marks_sdf_particle_force_as_matter_field_without_mesh_fallback() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_source: MatterSurfaceParticleForceSource::SdfField,
        particle_force_compare_probe_count: 3,
        particle_distance_refresh_policy:
            MatterSurfaceParticleDistanceRefreshPolicy::SurfaceUpdateAndStep,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 90.0, &[])
        .expect("adapter frame builds");
    let diagnostics = frame
        .particle_step
        .as_ref()
        .expect("particles step when enabled");

    assert_eq!(
        diagnostics.particle_force_source,
        MatterSurfaceParticleForceSource::SdfField
    );
    assert_eq!(
        diagnostics.particle_force_source_status,
        MatterSurfaceParticleForceSourceStatus::Ready
    );
    assert_eq!(
        diagnostics.particle_force_refresh,
        MatterSurfaceParticleForceRefresh::Fresh
    );
    assert!(diagnostics.particles.closest_samples > 0);
    assert_eq!(diagnostics.particles.surface_triangle_tests, 0);
    assert_eq!(diagnostics.refreshed_distance_samples, 0);
    assert_eq!(frame.stats.particle_distance_samples, 0);

    let marker = runtime.marker_line("unit-test-force-sdf", &frame);
    assert!(marker.contains("particleForceSource=sdf-field"));
    assert!(marker.contains("particleForceSourceStatus=ready"));
    assert!(marker.contains("particleForceRefresh=fresh"));
    assert!(marker.contains("particleForceCompareProbeCount=3"));
    assert!(marker.contains("particleSamplingAuthority=matter-sdf-field-sampler"));
    assert!(marker.contains("particleFieldSource=current-sdf-field"));
    assert!(marker.contains("sdfAdfDebugParticleAuthority=false"));
    assert!(marker.contains("particleClosestSamples="));
    assert!(marker.contains("particleSurfaceTriangleTests=0"));
    assert!(marker.contains("particleRefreshSamples=0"));
}

#[test]
fn adapter_marks_adf_particle_force_as_matter_field_without_debug_payload_authority() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_source: MatterSurfaceParticleForceSource::AdfField,
        particle_force_compare_probe_count: 3,
        particle_distance_refresh_policy:
            MatterSurfaceParticleDistanceRefreshPolicy::SurfaceUpdateAndStep,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 90.0, &[])
        .expect("adapter frame builds");
    let diagnostics = frame
        .particle_step
        .as_ref()
        .expect("particles step when enabled");

    assert_eq!(
        diagnostics.particle_force_source,
        MatterSurfaceParticleForceSource::AdfField
    );
    assert_eq!(
        diagnostics.particle_force_source_status,
        MatterSurfaceParticleForceSourceStatus::Ready
    );
    assert_eq!(
        diagnostics.particle_force_refresh,
        MatterSurfaceParticleForceRefresh::Fresh
    );
    assert!(diagnostics.particles.closest_samples > 0);
    assert_eq!(diagnostics.particles.surface_triangle_tests, 0);
    assert!(!diagnostics.sdf_adf_debug_particle_authority);

    let marker = runtime.marker_line("unit-test-force-adf", &frame);
    assert!(marker.contains("particleForceSource=adf-field"));
    assert!(marker.contains("particleForceSourceStatus=ready"));
    assert!(marker.contains("particleForceRefresh=fresh"));
    assert!(marker.contains("particleSamplingAuthority=matter-adf-field-sampler"));
    assert!(marker.contains("particleFieldSource=current-adf-field"));
    assert!(marker.contains("sdfAdfDebugParticleAuthority=false"));
    assert!(marker.contains("particleClosestSamples="));
    assert!(marker.contains("particleSurfaceTriangleTests=0"));
}

#[test]
fn adapter_can_bound_particle_simulation_delta() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_max_frame_delta_seconds: Some(1.0 / 60.0),
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 0.25, &[])
        .expect("adapter frame builds");
    let diagnostics = frame
        .particle_step
        .as_ref()
        .expect("particles step when enabled");

    assert_eq!(diagnostics.particles.input_delta_seconds, 0.25);
    assert!((diagnostics.particles.simulated_delta_seconds - 1.0 / 60.0).abs() < 1.0e-6);
    assert!((diagnostics.particles.dropped_delta_seconds - (0.25 - 1.0 / 60.0)).abs() < 1.0e-6);
    let marker = runtime.marker_line("unit-test", &frame);
    assert!(marker.contains("particleInputDeltaSeconds=0.250000"));
    assert!(marker.contains("particleSimulatedDeltaSeconds=0.016667"));
    assert!(marker.contains("particleDroppedDeltaSeconds=0.233333"));
}

#[test]
fn adapter_caps_particle_visual_rows_without_changing_matter_count() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 32,
        particle_visual_row_limit: Some(8),
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 60.0, &[])
        .expect("adapter frame builds");

    assert_eq!(frame.stats.particle_count, 32);
    assert_eq!(frame.particle_snapshot.samples.len(), 32);
    assert_eq!(
        frame
            .particle_visual_frame
            .as_ref()
            .expect("visual frame")
            .samples
            .len(),
        8
    );
    let upload = frame.particle_upload.as_ref().expect("particle upload");
    assert_eq!(upload.source_rows, 32);
    assert_eq!(upload.rows.len(), 8);

    let world_batch = frame
        .world_particle_batch(
            replay.sequence().bounds_min(),
            replay.sequence().bounds_max(),
            QuestMakepadWorldParticlePlacement::default(),
            8,
        )
        .expect("world particle batch builds");
    assert_eq!(world_batch.source_rows, 32);
    assert_eq!(world_batch.instances.len(), 8);
    assert_eq!(world_batch.dropped_rows, 24);

    let marker = runtime.marker_line("unit-test", &frame);
    assert!(marker.contains("particleCount=32"));
    assert!(marker.contains("particleSourceRows=32"));
    assert!(marker.contains("particleRows=8"));
    assert!(marker.contains("particleVisualRowLimit=8"));
}

#[cfg(feature = "parallel")]
#[test]
fn adapter_reports_parallel_particle_execution_when_feature_enabled() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 64,
        particle_execution_backend: ParticleExecutionBackend::Parallel,
        particle_execution_batch_size: NonZeroUsize::new(8).unwrap(),
        particle_execution_max_threads: Some(2),
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("parallel runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 30.0, &[])
        .expect("adapter frame builds");
    let diagnostics = frame
        .particle_step
        .as_ref()
        .expect("particles step when enabled");

    assert_eq!(
        diagnostics.particles.execution.backend,
        ParticleExecutionBackend::Parallel
    );
    assert_eq!(diagnostics.particles.execution.batch_size, 8);
    assert_eq!(diagnostics.particles.execution.worker_count, 2);
    let marker = runtime.marker_line("unit-test", &frame);
    assert!(marker.contains("particleExecutionBackend=rayon"));
    assert!(marker.contains("particleExecutionWorkers=2"));
}

#[test]
fn adapter_steps_generic_source_frame_like_replay_frame() {
    let replay = enabled_replay();
    let source_frame =
        QuestMakepadMatterSurfaceSourceFrame::from_replay(&replay).expect("source frame builds");

    assert_eq!(source_frame.source_id, "public-synthetic-hand-sequence");
    assert_eq!(
        source_frame.provider_shape,
        QuestMakepadMatterSurfaceProviderShape::PositionsOnlySurface
    );
    assert_eq!(source_frame.frame.frame_index, replay.current_frame_index());
    assert_eq!(source_frame.bounds_min, replay.sequence().bounds_min());
    assert_eq!(source_frame.bounds_max, replay.sequence().bounds_max());
    assert_eq!(
        source_frame.bounds_radius,
        replay.sequence().bounds_radius()
    );

    let config = QuestMakepadMatterSurfaceConfig {
        enabled: true,
        collision_enabled: true,
        sdf_slice_enabled: false,
        particles_enabled: false,
        ..QuestMakepadMatterSurfaceConfig::default()
    };
    let mut source_runtime =
        QuestMakepadMatterSurfaceRuntime::new(config.clone()).expect("runtime builds");
    let mut replay_runtime = QuestMakepadMatterSurfaceRuntime::new(config).expect("runtime builds");

    let probes = [MatterSurfaceContactProbe::sphere(
        "probe.center",
        Vec3::new(0.0, 0.0, 0.0),
        0.25,
    )];
    let from_source = source_runtime
        .step_from_source_frame(source_frame, 1.0 / 60.0, &probes)
        .expect("source frame steps");
    let from_replay = replay_runtime
        .step_from_replay(&replay, 1.0 / 60.0, &probes)
        .expect("replay frame steps");

    assert_eq!(from_source.source_id, from_replay.source_id);
    assert_eq!(
        from_source.source_provider_shape,
        from_replay.source_provider_shape
    );
    assert_eq!(
        from_source.source_provider_shape,
        QuestMakepadMatterSurfaceProviderShape::PositionsOnlySurface
    );
    assert_eq!(
        from_source.source_bounds_min,
        replay.sequence().bounds_min()
    );
    assert_eq!(
        from_source.source_bounds_max,
        replay.sequence().bounds_max()
    );
    assert_eq!(
        from_source.source_bounds_radius,
        replay.sequence().bounds_radius()
    );
    assert_eq!(
        from_source.matter_update.frame_index,
        from_replay.matter_update.frame_index
    );
    assert_eq!(
        from_source.matter_update.vertex_count,
        from_replay.matter_update.vertex_count
    );
    assert_eq!(
        from_source.matter_update.triangle_count,
        from_replay.matter_update.triangle_count
    );
    assert_eq!(
        from_source.collision_upload.rows.len(),
        from_replay.collision_upload.rows.len()
    );

    let marker = source_runtime.marker_line("unit-test", &from_source);
    assert!(marker.contains("sourceId=public-synthetic-hand-sequence"));
    assert!(marker.contains("sourceProviderShape=positions-only-surface"));
    assert!(!marker.contains("rusty.xr"));
    assert!(!marker.contains("RUSTY_XR"));
}

#[test]
fn external_recorded_sequence_steps_through_source_frame_when_configured() {
    let Ok(sequence_path) = std::env::var("RUSTY_QUEST_MAKEPAD_RECORDED_SEQUENCE_JSON") else {
        return;
    };
    let sequence_json =
        std::fs::read_to_string(&sequence_path).expect("recorded sequence JSON reads");
    let sequence =
        MeshReplaySequence::from_json_str(&sequence_json).expect("recorded sequence parses");
    assert!(sequence.vertex_count() > 8);
    assert!(sequence.triangle_count() > 6);
    assert!(sequence.frame_count() > 1);

    let mut replay = MeshReplayRuntime::from_sequence(
        sequence,
        MeshReplayConfig::normalized(
            true,
            "recorded-meta-quest-hand-sequence".to_owned(),
            1.0,
            1.0,
        ),
    );
    replay.step(0.0);

    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        collision_enabled: true,
        sdf_slice_enabled: false,
        particles_enabled: false,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");
    let frame = runtime
        .step_from_source_frame(
            QuestMakepadMatterSurfaceSourceFrame::from_replay(&replay)
                .expect("source frame builds"),
            1.0 / 60.0,
            &[MatterSurfaceContactProbe::sphere(
                "probe.center",
                replay.sequence().bounds_center(),
                replay.sequence().bounds_radius().max(0.01),
            )],
        )
        .expect("recorded source frame steps");

    assert_eq!(frame.source_id, "recorded-meta-quest-hand-sequence");
    assert_eq!(
        frame.matter_update.vertex_count,
        replay.sequence().vertex_count()
    );
    assert_eq!(
        frame.matter_update.triangle_count,
        replay.sequence().triangle_count()
    );
    assert_eq!(frame.collision_upload.rows.len(), 1);
    let marker = runtime.marker_line("external-recorded-sequence", &frame);
    assert!(marker.contains("nativeMatterRuntime=true"));
    assert!(marker.contains("sourceId=recorded-meta-quest-hand-sequence"));
    assert!(marker.contains("wasmRuntimeUsed=false"));
    assert!(marker.contains("shaderScaffoldUsed=false"));
}

#[test]
fn world_particle_billboard_renderer_identity_is_morphospace_scoped() {
    let values = [
        QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_RENDERER_ID,
        QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_MODE,
        QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_SOURCE,
        QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_REFERENCE,
    ];

    for value in values {
        assert!(!value.contains("rusty.xr"));
        assert!(!value.contains("rustyxr"));
        assert!(!value.contains("RUSTY_XR"));
    }
    assert_eq!(
        QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_RENDERER_ID,
        "makepad-xr-procedural-ring-billboard"
    );
    assert_eq!(
        QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_ANIMATION_SOURCE,
        "rusty-optics-particle-visual-frame"
    );
}

#[test]
fn adapter_can_update_surface_without_high_rate_payloads_enabled() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        collision_enabled: false,
        sdf_slice_enabled: false,
        particles_enabled: false,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 60.0, &[])
        .expect("adapter frame builds");

    assert_eq!(frame.matter_update.vertex_count, 8);
    assert_eq!(frame.collision_upload.rows.len(), 0);
    assert!(frame.sdf_slice_upload.is_none());
    assert!(frame.particle_upload.is_none());
    assert!(frame.particle_step.is_none());
    assert_eq!(frame.particle_snapshot.samples.len(), 0);
}

#[test]
fn world_particle_batch_places_content_center_half_meter_in_front() {
    let upload = QuestMakepadParticleUpload {
        schema_id: QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID.to_owned(),
        source_rows: 2,
        rows: vec![
            QuestMakepadParticleRow {
                position_radius: [0.0, 0.0, 0.0, 0.02],
                color: [0.2, 0.8, 1.0, 1.0],
                normal_frame: [0.0, 0.0, 1.0, 0.5],
                aux: [0.25, 0.0, 0.0, 0.0],
            },
            QuestMakepadParticleRow {
                position_radius: [1.0, 0.0, 0.0, 0.02],
                color: [1.0, 0.5, 0.2, 1.0],
                normal_frame: [1.0, 0.0, 0.0, 0.25],
                aux: [0.75, 0.0, 0.0, 0.0],
            },
        ],
    };

    let batch = world_particle_batch_from_upload(
        &upload,
        [-1.0, -1.0, -1.0],
        [1.0, 1.0, 1.0],
        QuestMakepadWorldParticlePlacement::default(),
        16,
    );

    assert_eq!(batch.instances.len(), 2);
    assert_eq!(
        [
            batch.instances[0].center_radius[0],
            batch.instances[0].center_radius[1],
            batch.instances[0].center_radius[2],
        ],
        DEFAULT_WORLD_CONTENT_CENTER
    );
    assert!(
        (batch.instances[0].center_radius[3] - (0.02 * batch.replay_to_world_scale)).abs()
            < 0.000_001
    );
    assert_eq!(batch.dropped_rows, 0);
}

#[test]
fn world_particle_batch_samples_across_source_rows() {
    let upload = QuestMakepadParticleUpload {
        schema_id: QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID.to_owned(),
        source_rows: 10,
        rows: (0..10)
            .map(|index| QuestMakepadParticleRow {
                position_radius: [index as f32, index as f32 * 0.5, index as f32 * -0.25, 0.02],
                color: [0.2, 0.8, 1.0, 1.0],
                normal_frame: [0.0, 0.0, 1.0, 0.5],
                aux: [index as f32 * 0.01, 0.0, 0.0, 0.0],
            })
            .collect(),
    };

    let batch = world_particle_batch_from_upload(
        &upload,
        [0.0, 0.0, -3.0],
        [9.0, 4.5, 0.0],
        QuestMakepadWorldParticlePlacement::default(),
        4,
    );

    assert_eq!(batch.instances.len(), 4);
    assert_eq!(batch.source_rows, 10);
    assert_eq!(batch.dropped_rows, 6);
    assert!(batch.instances[0].center_radius[0] < batch.instances[1].center_radius[0]);
    assert!(batch.instances[3].center_radius[0] > batch.instances[2].center_radius[0]);
    let marker = batch.marker_line("unit-test");
    assert!(marker.contains("selectionPolicy=evenly-spaced-source-rows"));
    assert!(marker.contains("instanceSpread="));
}
