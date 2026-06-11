use super::*;
use rusty_matter_mesh::{HandSkinningMatrixSample, HandSkinningMeshBufferOracle};
use rusty_matter_model::Vec3;
use rusty_quest_makepad_mesh_replay::{MeshReplayConfig, MeshReplayRuntime, MeshReplaySequence};
use std::num::NonZeroUsize;

fn enabled_replay() -> MeshReplayRuntime {
    let mut replay = MeshReplayRuntime::default();
    replay.configure(MeshReplayConfig::normalized(
        true,
        "public-synthetic-hand-sequence".to_owned(),
        1.0,
        0.75,
    ));
    replay.step(0.0);
    replay
}

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

#[test]
fn gpu_residency_proof_preserves_particle_cpu_authority_boundary() {
    let upload = QuestMakepadParticleUpload {
        schema_id: QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID.to_owned(),
        source_rows: 10,
        rows: (0..10)
            .map(|index| QuestMakepadParticleRow {
                position_radius: [index as f32, 0.0, 0.0, 0.02],
                color: [0.2, 0.8, 1.0, 1.0],
                normal_frame: [0.0, 0.0, 1.0, 0.5],
                aux: [0.0, 0.0, 0.0, 0.0],
            })
            .collect(),
    };
    let batch = world_particle_batch_from_upload(
        &upload,
        [0.0, 0.0, -1.0],
        [9.0, 1.0, 1.0],
        QuestMakepadWorldParticlePlacement::default(),
        4,
    );

    let proof = QuestMakepadGpuResidencyProof::from_world_particle_batch(
        &batch,
        4,
        QUEST_MAKEPAD_WORLD_PARTICLE_BILLBOARD_RENDERER_ID,
    );

    assert_eq!(
        proof.payload_kind,
        QuestMakepadGpuResidencyPayloadKind::WorldParticles
    );
    assert_eq!(proof.resident_rows, 4);
    assert_eq!(proof.adapter_row_stride_bytes, 64);
    assert_eq!(proof.adapter_payload_bytes, 256);
    let marker = proof.marker_line("unit-test");
    assert!(marker.contains("schema=rusty.quest.makepad.gpu_residency_proof.v1"));
    assert!(marker.contains("payloadKind=world-particles"));
    assert!(marker.contains("residencyBackend=makepad-xr-instanced-draw-buffer"));
    assert!(marker.contains("computeKernel=false"));
    assert!(marker.contains("matterCpuReferencePreserved=true"));
    assert!(marker.contains("highRateJsonPayload=false"));
    assert!(!marker.contains("rusty.xr"));
    assert!(!marker.contains("RUSTY_XR"));
}

#[test]
fn world_particle_placement_can_target_makepad_content_local_space() {
    let placement = QuestMakepadWorldParticlePlacement::content_local(
        [0.0, 0.58, -0.22],
        DEFAULT_WORLD_CONTENT_TARGET_RADIUS,
    );

    assert_eq!(
        placement.coordinate_space,
        QUEST_MAKEPAD_CONTENT_LOCAL_SPACE
    );
    assert_eq!(placement.center, [0.0, 0.58, -0.22]);

    let batch = QuestMakepadWorldParticleBatch {
        schema_id: QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID.to_owned(),
        source_schema_id: QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID.to_owned(),
        coordinate_space: placement.coordinate_space.to_owned(),
        render_mode: QUEST_MAKEPAD_CENTER_PROJECTED_BILLBOARD_MODE.to_owned(),
        content_center: placement.center,
        content_radius: placement.target_radius,
        replay_to_world_scale: 1.0,
        source_rows: 0,
        dropped_rows: 0,
        instances: Vec::new(),
    };
    assert!(batch
        .marker_line("unit-test")
        .contains("contentCenterDistanceMeters=0.620"));
}

#[test]
fn world_adf_debug_placement_can_target_makepad_content_local_space() {
    let placement = QuestMakepadWorldAdfDebugPlacement::content_local(
        [0.0, 0.58, -0.22],
        DEFAULT_WORLD_CONTENT_TARGET_RADIUS,
    );

    assert_eq!(
        placement.coordinate_space,
        QUEST_MAKEPAD_CONTENT_LOCAL_SPACE
    );
    assert_eq!(placement.center, [0.0, 0.58, -0.22]);

    let batch = QuestMakepadWorldAdfDebugBatch {
        schema_id: QUEST_MAKEPAD_WORLD_ADF_DEBUG_BATCH_SCHEMA_ID.to_owned(),
        source_schema_id: QUEST_MAKEPAD_ADF_DEBUG_SCHEMA_ID.to_owned(),
        source_visual_schema_id: "rusty.optics.adf.debug.visual.v1".to_owned(),
        source_field_id: "adf.test".to_owned(),
        source_grid_id: "sdf.test".to_owned(),
        coordinate_space: placement.coordinate_space.to_owned(),
        render_mode: QUEST_MAKEPAD_WORLD_ADF_DEBUG_RENDER_MODE.to_owned(),
        content_center: placement.center,
        content_radius: placement.target_radius,
        source_to_world_scale: 1.0,
        source_cells: 0,
        dropped_cells: 0,
        cells: Vec::new(),
    };
    assert!(batch
        .marker_line("unit-test")
        .contains("contentCenterDistanceMeters=0.620"));
}

#[test]
fn gpu_residency_proof_covers_adf_debug_cells_without_adf_authority() {
    let batch = QuestMakepadWorldAdfDebugBatch {
        schema_id: QUEST_MAKEPAD_WORLD_ADF_DEBUG_BATCH_SCHEMA_ID.to_owned(),
        source_schema_id: QUEST_MAKEPAD_ADF_DEBUG_SCHEMA_ID.to_owned(),
        source_visual_schema_id: "rusty.optics.adf.debug.visual.v1".to_owned(),
        source_field_id: "adf.test".to_owned(),
        source_grid_id: "sdf.test".to_owned(),
        coordinate_space: QUEST_MAKEPAD_CONTENT_LOCAL_SPACE.to_owned(),
        render_mode: QUEST_MAKEPAD_WORLD_ADF_DEBUG_RENDER_MODE.to_owned(),
        content_center: [0.0, 0.58, -0.22],
        content_radius: DEFAULT_WORLD_CONTENT_TARGET_RADIUS,
        source_to_world_scale: 1.0,
        source_cells: 32,
        dropped_cells: 28,
        cells: vec![
            QuestMakepadWorldAdfDebugCell {
                center_extent: [0.0, 0.0, -0.2, 0.02],
                distance: [0.1, -0.1, 0.2, 0.5],
                meta: [1.0, 0.4, 8.0, 0.0],
            };
            4
        ],
    };

    let proof = QuestMakepadGpuResidencyProof::from_world_adf_debug_batch(
        &batch,
        4,
        "hostess-makepad-adf-debug-cell-boxes",
    );

    assert_eq!(
        proof.payload_kind,
        QuestMakepadGpuResidencyPayloadKind::WorldAdfDebugCells
    );
    assert_eq!(proof.source_rows, 32);
    assert_eq!(proof.resident_rows, 4);
    assert_eq!(proof.adapter_row_stride_bytes, 48);
    let marker = proof.marker_line("unit-test");
    assert!(marker.contains("payloadKind=world-adf-debug-cells"));
    assert!(marker.contains("resourcePlane=render-gpu-instance-buffer"));
    assert!(marker.contains("residentRows=4"));
    assert!(marker.contains("adapterPayloadBytes=192"));
    assert!(marker.contains("computeKernel=false"));
    assert!(marker.contains("matterCpuReferencePreserved=true"));
    assert!(marker.contains("highRateJsonPayload=false"));
}

#[test]
fn gpu_compute_preflight_identifies_sdf_field_cpu_oracle() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_source: MatterSurfaceParticleForceSource::SdfField,
        particle_force_update_interval_frames: NonZeroUsize::new(2).unwrap(),
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::Disabled,
        sdf_voxel_size: 0.12,
        sdf_max_voxels: 4_096,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 30.0, &[])
        .expect("SDF field frame builds");
    let preflight = QuestMakepadGpuComputePreflight::from_frame(&frame, 64)
        .expect("SDF field frame is compute preflight eligible");

    assert_eq!(
        preflight.resource_kind,
        QuestMakepadGpuComputeResourceKind::SdfParticleForces
    );
    assert_eq!(
        preflight.force_source,
        MatterSurfaceParticleForceSource::SdfField
    );
    assert_eq!(preflight.particle_rows, 16);
    assert_eq!(preflight.readback_probe_count, 16);
    let marker = preflight.marker_line("unit-test");
    assert!(marker.contains("schema=rusty.quest.makepad.gpu_compute_preflight.v1"));
    assert!(marker.contains("status=eligible"));
    assert!(marker.contains("resourceKind=sdf-particle-forces"));
    assert!(marker.contains("particleForceSource=sdf-field"));
    assert!(marker.contains("particleSamplingAuthority=matter-sdf-field-sampler"));
    assert!(marker.contains("cpuOraclePreserved=true"));
    assert!(marker.contains("commandEncoderRequired=true"));
    assert!(marker.contains("makepadComputeBackend=makepad-command-encoder-pending"));
    assert!(marker.contains("gpuComputeReady=false"));
    assert!(marker.contains("computeKernel=false"));
    assert!(marker.contains("highRateJsonPayload=false"));
    assert!(!marker.contains("rusty.xr"));
    assert!(!marker.contains("RUSTY_XR"));
}

#[test]
fn gpu_storage_probe_marker_preserves_cpu_oracle_boundary() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_source: MatterSurfaceParticleForceSource::SdfField,
        particle_force_update_interval_frames: NonZeroUsize::new(2).unwrap(),
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::Disabled,
        sdf_voxel_size: 0.12,
        sdf_max_voxels: 4_096,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 30.0, &[])
        .expect("SDF field frame builds");
    let preflight = QuestMakepadGpuComputePreflight::from_frame(
        &frame,
        QUEST_MAKEPAD_GPU_COMPUTE_DEFAULT_READBACK_PROBE_COUNT,
    )
    .expect("SDF field frame is compute preflight eligible");
    let probe = QuestMakepadGpuStorageProbe::from_preflight(
        &preflight,
        QuestMakepadGpuStorageProbeReadback {
            requested_bytes: QUEST_MAKEPAD_GPU_STORAGE_PROBE_DEFAULT_BYTES,
            storage_buffer_bytes: QUEST_MAKEPAD_GPU_STORAGE_PROBE_DEFAULT_BYTES,
            readback_bytes: QUEST_MAKEPAD_GPU_STORAGE_PROBE_DEFAULT_BYTES,
            pattern: QUEST_MAKEPAD_GPU_STORAGE_PROBE_DEFAULT_PATTERN,
            first_word: QUEST_MAKEPAD_GPU_STORAGE_PROBE_DEFAULT_PATTERN,
            word_count: 16,
            mismatched_words: 0,
            elapsed_ms: 0.25,
        },
    );

    let marker = probe.marker_line("unit-test");
    assert!(marker.contains("schema=rusty.quest.makepad.gpu_storage_probe.v1"));
    assert!(marker.contains("status=ready"));
    assert!(marker.contains("resourcePlane=vulkan-storage-buffer-command-readback"));
    assert!(marker.contains("storageProbeBackend=makepad-vulkan-queue-submit-fill-copy-readback"));
    assert!(marker.contains("resourceKind=sdf-particle-forces"));
    assert!(marker.contains("particleForceSource=sdf-field"));
    assert!(marker.contains("cpuOraclePreserved=true"));
    assert!(marker.contains("preflightSchema=rusty.quest.makepad.gpu_compute_preflight.v1"));
    assert!(marker.contains("readbackPolicy=bounded-cpu-oracle-probes"));
    assert!(marker.contains("readbackMatched=true"));
    assert!(marker.contains("commandEncoderSubmitted=true"));
    assert!(marker.contains("storageBufferResident=true"));
    assert!(marker.contains("gpuCommandExecuted=true"));
    assert!(marker.contains("gpuComputeReady=false"));
    assert!(marker.contains("computeKernel=false"));
    assert!(marker.contains("highRateJsonPayload=false"));
}

#[test]
fn gpu_oracle_compute_probe_marker_preserves_cpu_oracle_boundary() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_source: MatterSurfaceParticleForceSource::SdfField,
        particle_force_update_interval_frames: NonZeroUsize::new(2).unwrap(),
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::Disabled,
        sdf_voxel_size: 0.12,
        sdf_max_voxels: 4_096,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 30.0, &[])
        .expect("SDF field frame builds");
    let preflight = QuestMakepadGpuComputePreflight::from_frame(
        &frame,
        QUEST_MAKEPAD_GPU_COMPUTE_DEFAULT_READBACK_PROBE_COUNT,
    )
    .expect("SDF field frame is compute preflight eligible");
    let input_words = preflight.oracle_compute_probe_words();
    assert_eq!(input_words[0], 0x5DF0_0001);
    assert_eq!(input_words[1], 16);
    assert_eq!(input_words[2], preflight.topology_vertex_count as u32);
    assert_eq!(input_words[3], preflight.topology_triangle_count as u32);

    let expected_words = [0x10, 0x20, 0x30, 0x40];
    let probe = QuestMakepadGpuOracleComputeProbe::from_preflight(
        &preflight,
        QuestMakepadGpuOracleComputeProbeReadback {
            input_words,
            output_words: expected_words,
            expected_words,
            word_count: QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS,
            mismatched_words: 0,
            queue_submit_serial: 7,
            fence_serial: 7,
            resource_generation: 1,
            pending_retire_count: 1,
            retained_resource_count: 1,
            retired_after_fence_count: 0,
            queue_wait_idle_performed: true,
            elapsed_ms: 0.25,
        },
    );

    let marker = probe.marker_line("unit-test");
    assert!(marker.contains("schema=rusty.quest.makepad.gpu_oracle_compute_probe.v1"));
    assert!(marker.contains("status=ready"));
    assert!(marker.contains("proofKind=u32-oracle-compute"));
    assert!(marker.contains("computeStage=field-particle-force-prototype"));
    assert!(marker.contains("resourcePlane=vulkan-compute-storage-buffer-readback"));
    assert!(marker.contains("computeProbeBackend=makepad-vulkan-compute-u32-oracle-probe"));
    assert!(marker.contains("resourceKind=sdf-particle-forces"));
    assert!(marker.contains("particleForceSource=sdf-field"));
    assert!(marker.contains("cpuOraclePreserved=true"));
    assert!(marker.contains("preflightSchema=rusty.quest.makepad.gpu_compute_preflight.v1"));
    assert!(marker.contains("readbackPolicy=bounded-cpu-oracle-probes"));
    assert!(marker.contains("oraclePayload=bounded-matter-frame-u32-probes"));
    assert!(marker.contains("oracleInputWords=0x5DF00001,0x00000010"));
    assert!(marker.contains("gpuOutputWords=0x00000010,0x00000020,0x00000030,0x00000040"));
    assert!(marker.contains("cpuExpectedWords=0x00000010,0x00000020,0x00000030,0x00000040"));
    assert!(marker.contains("mismatchedWords=0"));
    assert!(marker.contains("readbackMatched=true"));
    assert!(marker.contains("commandEncoderSubmitted=true"));
    assert!(marker.contains("storageBufferResident=true"));
    assert!(marker.contains("computeDispatchSubmitted=true"));
    assert!(marker.contains("prototypeComputeKernel=true"));
    assert!(marker.contains("fieldParticleKernel=false"));
    assert!(marker.contains("computeKernel=true"));
    assert!(marker.contains("gpuComputeReady=false"));
    assert!(marker.contains("highRateJsonPayload=false"));
    assert!(marker.contains("queueSubmitSerial=7"));
    assert!(marker.contains("fenceSerial=7"));
    assert!(marker.contains("resourceGeneration=1"));
    assert!(marker.contains("pendingRetireCount=1"));
    assert!(marker.contains("retainedResourceCount=1"));
    assert!(marker.contains("retiredAfterFenceCount=0"));
    assert!(marker.contains("queueWaitIdlePerformed=true"));
    assert!(marker.contains("retirementPolicy=retained-until-vulkan-drop"));
    assert!(marker.contains("hwbAcquiredCount=0"));
    assert!(marker.contains("hwbReleasedAfterFenceCount=0"));
    assert!(marker.contains("kgslFaultsBeforeMarker=unavailable"));
    assert!(marker.contains("kgslFaultsAfterMarker=unavailable"));
}

#[test]
fn gpu_field_force_probe_marker_preserves_cpu_oracle_boundary() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_source: MatterSurfaceParticleForceSource::SdfField,
        particle_force_update_interval_frames: NonZeroUsize::new(2).unwrap(),
        particle_force_compare_probe_count: 4,
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::Disabled,
        sdf_voxel_size: 0.12,
        sdf_max_voxels: 4_096,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 30.0, &[])
        .expect("SDF field frame builds");
    let particle_force_probe = frame
        .particle_step
        .as_ref()
        .and_then(|diagnostics| diagnostics.particle_force_probe.as_ref())
        .expect("Matter CPU force probe is available");
    assert_eq!(particle_force_probe.sampled_count, 4);
    assert!(particle_force_probe.attraction_strength.is_finite());

    let preflight =
        QuestMakepadGpuComputePreflight::from_frame(&frame, particle_force_probe.sampled_count)
            .expect("SDF field frame is compute preflight eligible");
    let probe = QuestMakepadGpuFieldForceProbe::from_preflight(
        &preflight,
        QuestMakepadGpuFieldForceProbeReadback {
            sample_count: 4,
            component_count: 12,
            mismatched_components: 0,
            max_abs_error: 0.000_001,
            tolerance: 0.000_1,
            queue_submit_serial: 9,
            fence_serial: 9,
            resource_generation: 2,
            pending_retire_count: 2,
            retained_resource_count: 2,
            retired_after_fence_count: 0,
            queue_wait_idle_performed: true,
            elapsed_ms: 0.75,
        },
    );

    let marker = probe.marker_line("unit-test");
    assert!(marker.contains("schema=rusty.quest.makepad.gpu_field_force_probe.v1"));
    assert!(marker.contains("status=ready"));
    assert!(marker.contains("proofKind=f32-field-force-arithmetic"));
    assert!(marker.contains("computeStage=field-particle-force-prototype"));
    assert!(marker.contains("resourcePlane=vulkan-compute-storage-buffer-readback"));
    assert!(marker.contains("computeProbeBackend=makepad-vulkan-compute-f32-force-probe"));
    assert!(marker.contains("resourceKind=sdf-particle-forces"));
    assert!(marker.contains("particleForceSource=sdf-field"));
    assert!(marker.contains("cpuOraclePreserved=true"));
    assert!(marker.contains("oraclePayload=bounded-matter-particle-force-probes"));
    assert!(marker.contains("sampleCount=4"));
    assert!(marker.contains("componentCount=12"));
    assert!(marker.contains("mismatchedComponents=0"));
    assert!(marker.contains("maxAbsError=0.000001"));
    assert!(marker.contains("tolerance=0.000100"));
    assert!(marker.contains("readbackMatched=true"));
    assert!(marker.contains("forceArithmeticKernel=true"));
    assert!(marker.contains("fieldSamplingKernel=false"));
    assert!(marker.contains("fieldParticleKernel=false"));
    assert!(marker.contains("computeKernel=true"));
    assert!(marker.contains("gpuComputeReady=false"));
    assert!(marker.contains("highRateJsonPayload=false"));
    assert!(marker.contains("queueSubmitSerial=9"));
    assert!(marker.contains("fenceSerial=9"));
    assert!(marker.contains("resourceGeneration=2"));
    assert!(marker.contains("pendingRetireCount=2"));
    assert!(marker.contains("retainedResourceCount=2"));
    assert!(marker.contains("retiredAfterFenceCount=0"));
    assert!(marker.contains("queueWaitIdlePerformed=true"));
    assert!(marker.contains("retirementPolicy=retained-until-vulkan-drop"));
    assert!(marker.contains("hwbAcquiredCount=0"));
    assert!(marker.contains("hwbReleasedAfterFenceCount=0"));
    assert!(marker.contains("kgslFaultsBeforeMarker=unavailable"));
    assert!(marker.contains("kgslFaultsAfterMarker=unavailable"));
}

fn identity_matrix_samples() -> [[[f32; 4]; 4]; 4] {
    let mut matrices = [[[0.0; 4]; 4]; 4];
    matrices[0] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    matrices
}

fn translated_matrix_samples(z_offset: f32) -> [[[f32; 4]; 4]; 4] {
    let mut matrices = identity_matrix_samples();
    matrices[0][2][3] = z_offset;
    matrices
}

#[test]
fn gpu_skinning_probe_marker_preserves_recorded_hand_cpu_oracle_boundary() {
    let matter_samples = [
        HandSkinningMatrixSample {
            vertex_index: 0,
            bind_position: [0.0, 0.0, 0.0, 1.0],
            joint_indices: [0, 0, 0, 0],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
            joint_matrices: identity_matrix_samples(),
            expected_position: [0.0, 0.0, 0.0, 1.0],
        },
        HandSkinningMatrixSample {
            vertex_index: 1,
            bind_position: [1.0, 0.0, -0.5, 1.0],
            joint_indices: [1, 0, 0, 0],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
            joint_matrices: translated_matrix_samples(0.5),
            expected_position: [1.0, 0.0, 0.0, 1.0],
        },
        HandSkinningMatrixSample {
            vertex_index: 2,
            bind_position: [1.0, 1.0, -0.5, 1.0],
            joint_indices: [2, 0, 0, 0],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
            joint_matrices: translated_matrix_samples(0.25),
            expected_position: [1.0, 1.0, -0.25, 1.0],
        },
    ];
    let input = QuestMakepadGpuSkinningProbeInput::from_matter_samples(
        "recorded-hand-synthetic",
        7,
        3,
        1,
        &matter_samples,
    )
    .expect("bounded skinning probe input builds");
    let probe = QuestMakepadGpuSkinningProbe::from_input(
        &input,
        QuestMakepadGpuSkinningProbeReadback {
            sample_count: 3,
            component_count: 9,
            mismatched_components: 0,
            max_abs_error: 0.000_001,
            tolerance: QUEST_MAKEPAD_GPU_SKINNING_PROBE_DEFAULT_TOLERANCE,
            queue_submit_serial: 10,
            fence_serial: 10,
            resource_generation: 1,
            pending_retire_count: 1,
            retained_resource_count: 1,
            retired_after_fence_count: 0,
            queue_wait_idle_performed: true,
            elapsed_ms: 0.6,
        },
    );

    let marker = probe.marker_line("unit-test");
    assert!(marker.contains("schema=rusty.quest.makepad.gpu_skinning_probe.v1"));
    assert!(marker.contains("status=ready"));
    assert!(marker.contains("proofKind=f32-joint-matrix-skinning"));
    assert!(marker.contains("computeStage=hand-skinning-joint-matrix"));
    assert!(marker.contains("sourceId=recorded-hand-synthetic"));
    assert!(marker.contains("sourceFrameIndex=7"));
    assert!(marker.contains("topologyVertexCount=3"));
    assert!(marker.contains("topologyTriangleCount=1"));
    assert!(marker.contains("cpuOracle=matter-recorded-hand-skinning"));
    assert!(marker.contains("cpuOraclePreserved=true"));
    assert!(marker.contains("recordedInputEquivalent=true"));
    assert!(marker.contains("validationInputShape=bind-mesh-plus-compact-joint-frame"));
    assert!(marker.contains("resourcePlane=vulkan-compute-storage-buffer-readback"));
    assert!(marker.contains("computeProbeBackend=makepad-vulkan-compute-f32-skinning-probe"));
    assert!(marker.contains("oraclePayload=bounded-recorded-hand-skinning-probes"));
    assert!(marker.contains("sampleCount=3"));
    assert!(marker.contains("firstSampleVertexIndex=0"));
    assert!(marker.contains("lastSampleVertexIndex=2"));
    assert!(marker.contains("componentCount=9"));
    assert!(marker.contains("mismatchedComponents=0"));
    assert!(marker.contains("maxAbsError=0.000001"));
    assert!(marker.contains("tolerance=0.000100"));
    assert!(marker.contains("readbackMatched=true"));
    assert!(marker.contains("influenceSlotsPerSample=4"));
    assert!(marker.contains("matrixRowsPerInfluence=4"));
    assert!(marker.contains("prototypeComputeKernel=false"));
    assert!(marker.contains("weightedDeltaSkinningKernel=false"));
    assert!(marker.contains("jointMatrixSkinningKernel=true"));
    assert!(marker.contains("meshToSdfKernel=false"));
    assert!(marker.contains("computeKernel=true"));
    assert!(marker.contains("gpuComputeReady=false"));
    assert!(marker.contains("highRateJsonPayload=false"));
}

#[test]
fn gpu_skinning_mesh_probe_marker_preserves_full_recorded_hand_buffer_boundary() {
    let oracle = HandSkinningMeshBufferOracle {
        vertices: vec![
            HandSkinningMatrixSample {
                vertex_index: 0,
                bind_position: [0.0, 0.0, 0.0, 1.0],
                joint_indices: [0, 0, 0, 0],
                joint_weights: [1.0, 0.0, 0.0, 0.0],
                joint_matrices: identity_matrix_samples(),
                expected_position: [0.0, 0.0, 0.0, 1.0],
            },
            HandSkinningMatrixSample {
                vertex_index: 1,
                bind_position: [1.0, 0.0, -0.5, 1.0],
                joint_indices: [1, 0, 0, 0],
                joint_weights: [1.0, 0.0, 0.0, 0.0],
                joint_matrices: translated_matrix_samples(0.5),
                expected_position: [1.0, 0.0, 0.0, 1.0],
            },
            HandSkinningMatrixSample {
                vertex_index: 2,
                bind_position: [1.0, 1.0, -0.5, 1.0],
                joint_indices: [2, 0, 0, 0],
                joint_weights: [1.0, 0.0, 0.0, 0.0],
                joint_matrices: translated_matrix_samples(0.25),
                expected_position: [1.0, 1.0, -0.25, 1.0],
            },
        ],
        triangles: vec![[0, 1, 2]],
    };
    let input = QuestMakepadGpuSkinningMeshProbeInput::from_matter_oracle(
        "recorded-hand-synthetic",
        7,
        &oracle,
    )
    .expect("full skinning mesh probe input builds");
    let probe = QuestMakepadGpuSkinningMeshProbe::from_input(
        &input,
        QuestMakepadGpuSkinningMeshProbeReadback {
            vertex_count: 3,
            triangle_count: 1,
            index_count: 3,
            checked_position_components: 9,
            mismatched_position_components: 0,
            mismatched_triangle_indices: 0,
            max_abs_error: 0.000_001,
            tolerance: QUEST_MAKEPAD_GPU_SKINNING_MESH_PROBE_DEFAULT_TOLERANCE,
            sample_count: 3,
            sample_vertex_indices: input.sample_vertex_indices,
            queue_submit_serial: 11,
            fence_serial: 11,
            resource_generation: 1,
            pending_retire_count: 1,
            retained_resource_count: 1,
            retired_after_fence_count: 0,
            queue_wait_idle_performed: true,
            elapsed_ms: 1.2,
        },
    );

    let marker = probe.marker_line("unit-test");
    assert!(marker.contains("schema=rusty.quest.makepad.gpu_skinning_mesh_residency.v1"));
    assert!(marker.contains("status=ready"));
    assert!(marker.contains("proofKind=full-recorded-hand-skinning-mesh-residency"));
    assert!(marker.contains("computeStage=hand-skinning-full-vertex-buffer"));
    assert!(marker.contains("sourceId=recorded-hand-synthetic"));
    assert!(marker.contains("sourceFrameIndex=7"));
    assert!(marker.contains("topologyVertexCount=3"));
    assert!(marker.contains("topologyTriangleCount=1"));
    assert!(marker.contains("topologyIndexCount=3"));
    assert!(marker.contains("cpuOracle=matter-recorded-hand-skinning"));
    assert!(marker.contains("cpuOraclePreserved=true"));
    assert!(marker.contains("recordedInputEquivalent=true"));
    assert!(marker.contains("validationInputShape=bind-mesh-plus-compact-joint-frame"));
    assert!(marker.contains("resourcePlane=vulkan-compute-storage-buffer-readback"));
    assert!(
        marker.contains("computeProbeBackend=makepad-vulkan-compute-full-f32-skinning-mesh-probe")
    );
    assert!(marker.contains("oraclePayload=full-recorded-hand-skinning-mesh-buffer"));
    assert!(marker.contains("vertexCount=3"));
    assert!(marker.contains("triangleCount=1"));
    assert!(marker.contains("indexCount=3"));
    assert!(marker.contains("sampleCount=3"));
    assert!(marker.contains("checkedPositionComponents=9"));
    assert!(marker.contains("mismatchedPositionComponents=0"));
    assert!(marker.contains("mismatchedTriangleIndices=0"));
    assert!(marker.contains("readbackMatched=true"));
    assert!(marker.contains("fullVertexBufferResident=true"));
    assert!(marker.contains("fullIndexBufferResident=true"));
    assert!(marker.contains("skinnedVertexBufferResident=true"));
    assert!(marker.contains("indexBufferConsumedByGpu=true"));
    assert!(marker.contains("fullBufferGpuResidency=true"));
    assert!(marker.contains("boundedSampleOnly=false"));
    assert!(marker.contains("meshToSdfKernel=false"));
    assert!(marker.contains("computeKernel=true"));
    assert!(marker.contains("gpuComputeReady=false"));
    assert!(marker.contains("highRateJsonPayload=false"));
}

#[test]
fn gpu_compute_preflight_identifies_adf_field_cpu_oracle() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_source: MatterSurfaceParticleForceSource::AdfField,
        particle_force_update_interval_frames: NonZeroUsize::new(2).unwrap(),
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::Disabled,
        sdf_voxel_size: 0.12,
        sdf_max_voxels: 4_096,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 30.0, &[])
        .expect("ADF field frame builds");
    let preflight = QuestMakepadGpuComputePreflight::from_frame(
        &frame,
        QUEST_MAKEPAD_GPU_COMPUTE_DEFAULT_READBACK_PROBE_COUNT,
    )
    .expect("ADF field frame is compute preflight eligible");

    assert_eq!(
        preflight.resource_kind,
        QuestMakepadGpuComputeResourceKind::AdfParticleForces
    );
    assert_eq!(
        preflight.force_source,
        MatterSurfaceParticleForceSource::AdfField
    );
    let marker = preflight.marker_line("unit-test");
    assert!(marker.contains("resourceKind=adf-particle-forces"));
    assert!(marker.contains("fieldResourceId=quest.makepad.gpu_compute.adf_force_field"));
    assert!(marker.contains("particleForceSource=adf-field"));
    assert!(marker.contains("particleSamplingAuthority=matter-adf-field-sampler"));
    assert!(marker.contains("readbackPolicy=bounded-cpu-oracle-probes"));
    assert!(marker.contains("readbackProbeCount=16"));
    assert!(marker.contains("gpuComputeReady=false"));
}

#[test]
fn gpu_compute_preflight_rejects_mesh_distance_authority() {
    let replay = enabled_replay();
    let mut runtime = QuestMakepadMatterSurfaceRuntime::new(QuestMakepadMatterSurfaceConfig {
        enabled: true,
        particles_enabled: true,
        particle_count: 16,
        particle_force_source: MatterSurfaceParticleForceSource::MeshDistance,
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::Disabled,
        ..QuestMakepadMatterSurfaceConfig::default()
    })
    .expect("runtime builds");

    let frame = runtime
        .step_from_replay(&replay, 1.0 / 30.0, &[])
        .expect("mesh-distance frame builds");

    assert!(QuestMakepadGpuComputePreflight::from_frame(&frame, 16).is_none());
}
