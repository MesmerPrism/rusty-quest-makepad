use rusty_matter_particles::{ParticleRenderPayload, ParticleSet};
use rusty_matter_surface_runtime::{
    MatterSurfaceContactProbeBatch, MatterSurfaceRuntime, MatterSurfaceRuntimeError,
};
use rusty_optics_mesh::SdfSliceVisual;
use rusty_optics_particles::ParticleVisualFrame;

use crate::{
    bounds_radius, midpoint, sanitize_marker_value, vec3_length, vec3_marker_token,
    QuestMakepadMatterSurfaceError, DEFAULT_WORLD_CONTENT_CENTER,
    DEFAULT_WORLD_CONTENT_TARGET_RADIUS, QUEST_MAKEPAD_CENTER_PROJECTED_BILLBOARD_MODE,
    QUEST_MAKEPAD_COLLISION_UPLOAD_SCHEMA_ID, QUEST_MAKEPAD_CONTENT_LOCAL_SPACE,
    QUEST_MAKEPAD_DISTANCE_SLICE_UPLOAD_SCHEMA_ID, QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID,
    QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE, QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID,
    QUEST_MAKEPAD_WORLD_PARTICLE_EVEN_SELECTION_POLICY, QUEST_MAKEPAD_WORLD_PARTICLE_MARKER_PREFIX,
};
/// One packed SDF slice row for Makepad-facing upload paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadDistanceSliceRow {
    /// Plane coordinate and normalized distance as `[u, v, normalized, distance]`.
    pub uv_distance: [f32; 4],
    /// Source position as `[x, y, z, 1]`.
    pub position: [f32; 4],
}

/// Bounded SDF slice upload.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadDistanceSliceUpload {
    /// Schema identifier.
    pub schema_id: String,
    /// Slice width.
    pub width: u32,
    /// Slice height.
    pub height: u32,
    /// Packed rows.
    pub rows: Vec<QuestMakepadDistanceSliceRow>,
}

/// One packed collision row for Makepad-facing upload paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadCollisionRow {
    /// Contact point and distance as `[x, y, z, distance]`.
    pub point_distance: [f32; 4],
    /// Contact normal and overlap flag as `[x, y, z, overlaps]`.
    pub normal_overlap: [f32; 4],
}

/// Bounded collision upload.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadCollisionUpload {
    /// Schema identifier.
    pub schema_id: String,
    /// Packed contact rows.
    pub rows: Vec<QuestMakepadCollisionRow>,
}

/// One packed particle row for Makepad-facing upload paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadParticleRow {
    /// Position and visual radius as `[x, y, z, radius]`.
    pub position_radius: [f32; 4],
    /// RGBA visual color.
    pub color: [f32; 4],
    /// Velocity-derived normal and animation frame as `[x, y, z, frame01]`.
    pub normal_frame: [f32; 4],
    /// Rotation, speed, visual envelope, and flags as `[rotation, aux0, aux1, flags]`.
    pub aux: [f32; 4],
}

/// Bounded particle upload.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadParticleUpload {
    /// Schema identifier.
    pub schema_id: String,
    /// Full Matter source row count before visual-row capping.
    pub source_rows: usize,
    /// Packed particle rows.
    pub rows: Vec<QuestMakepadParticleRow>,
}

/// Placement policy for Matter particles rendered as Makepad world objects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadWorldParticlePlacement {
    /// Replay/Matter content center in the target Makepad/XR coordinate space.
    pub center: [f32; 3],
    /// Target radius for the source replay bounds.
    pub target_radius: f32,
    /// Coordinate space for `center` and emitted instance centers.
    pub coordinate_space: &'static str,
}

impl Default for QuestMakepadWorldParticlePlacement {
    fn default() -> Self {
        Self {
            center: DEFAULT_WORLD_CONTENT_CENTER,
            target_radius: DEFAULT_WORLD_CONTENT_TARGET_RADIUS,
            coordinate_space: QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE,
        }
    }
}

impl QuestMakepadWorldParticlePlacement {
    /// Creates a placement for Makepad XR content-local rendering.
    #[must_use]
    pub const fn content_local(center: [f32; 3], target_radius: f32) -> Self {
        Self {
            center,
            target_radius,
            coordinate_space: QUEST_MAKEPAD_CONTENT_LOCAL_SPACE,
        }
    }
}

/// One Makepad-facing world particle instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadWorldParticleInstance {
    /// World-space center and radius as `[x, y, z, radius]`.
    pub center_radius: [f32; 4],
    /// RGBA visual color.
    pub color: [f32; 4],
    /// Source normal and animation frame as `[x, y, z, frame01]`.
    pub normal_frame: [f32; 4],
    /// Renderer-neutral visual animation metadata as `[rotation, aux0, aux1, flags]`.
    pub aux: [f32; 4],
}

/// Bounded Makepad-facing world particle batch.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadWorldParticleBatch {
    /// Schema identifier.
    pub schema_id: String,
    /// Source upload schema identifier.
    pub source_schema_id: String,
    /// Coordinate space for the instance centers.
    pub coordinate_space: String,
    /// Renderer-facing primitive mode.
    pub render_mode: String,
    /// Replay/Matter content center in the target coordinate space.
    pub content_center: [f32; 3],
    /// Replay/Matter content radius in the target coordinate space.
    pub content_radius: f32,
    /// Scale from replay-local units to Makepad world units.
    pub replay_to_world_scale: f32,
    /// Source particle rows before the batch bound.
    pub source_rows: usize,
    /// Rows dropped by `max_instances`.
    pub dropped_rows: usize,
    /// World-space instances.
    pub instances: Vec<QuestMakepadWorldParticleInstance>,
}

impl QuestMakepadWorldParticleBatch {
    /// Builds a compact evidence marker without logging high-rate particle rows.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        let spread = instance_spread_token(&self.instances);
        format!(
            "{} schema={} phase={} status={} renderMode={} coordinateSpace={} sourceSchema={} sourceRows={} instanceRows={} droppedRows={} selectionPolicy={} contentCenter={} contentRadius={:.6} replayToWorldScale={:.6} contentCenterDistanceMeters={:.3} instanceSpread={} dataPlane=makepad-world-particle-instances",
            QUEST_MAKEPAD_WORLD_PARTICLE_MARKER_PREFIX,
            QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID,
            sanitize_marker_value(phase),
            if self.instances.is_empty() { "empty" } else { "ready" },
            sanitize_marker_value(&self.render_mode),
            sanitize_marker_value(&self.coordinate_space),
            sanitize_marker_value(&self.source_schema_id),
            self.source_rows,
            self.instances.len(),
            self.dropped_rows,
            QUEST_MAKEPAD_WORLD_PARTICLE_EVEN_SELECTION_POLICY,
            vec3_marker_token(self.content_center),
            self.content_radius,
            self.replay_to_world_scale,
            vec3_length(self.content_center),
            spread,
        )
    }
}

pub(crate) fn distance_slice_upload_from_visual(
    visual: &SdfSliceVisual,
) -> QuestMakepadDistanceSliceUpload {
    let rows = visual
        .cells
        .iter()
        .map(|cell| QuestMakepadDistanceSliceRow {
            uv_distance: [
                cell.plane[0] as f32,
                cell.plane[1] as f32,
                cell.normalized_distance,
                cell.distance,
            ],
            position: [cell.position.x, cell.position.y, cell.position.z, 1.0],
        })
        .collect();
    QuestMakepadDistanceSliceUpload {
        schema_id: QUEST_MAKEPAD_DISTANCE_SLICE_UPLOAD_SCHEMA_ID.to_owned(),
        width: visual.width,
        height: visual.height,
        rows,
    }
}

pub(crate) fn collision_upload_from_batch(
    batch: &MatterSurfaceContactProbeBatch,
) -> QuestMakepadCollisionUpload {
    let rows = batch
        .results
        .iter()
        .filter_map(|result| {
            let contact = result.contact.as_ref()?;
            Some(QuestMakepadCollisionRow {
                point_distance: [
                    contact.point.x,
                    contact.point.y,
                    contact.point.z,
                    contact.distance,
                ],
                normal_overlap: [
                    contact.normal.x,
                    contact.normal.y,
                    contact.normal.z,
                    if result.overlaps { 1.0 } else { 0.0 },
                ],
            })
        })
        .collect();
    QuestMakepadCollisionUpload {
        schema_id: QUEST_MAKEPAD_COLLISION_UPLOAD_SCHEMA_ID.to_owned(),
        rows,
    }
}

pub(crate) fn particle_render_payload_for_visual_limit(
    matter: &MatterSurfaceRuntime,
    payload_id: &'static str,
    visual_row_limit: Option<usize>,
) -> Result<ParticleRenderPayload, QuestMakepadMatterSurfaceError> {
    let Some(limit) = visual_row_limit else {
        return matter
            .particle_render_payload(payload_id)
            .map_err(Into::into);
    };

    let source_particles = matter.particle_runtime().particles();
    let source_count = source_particles.particles.len();
    let visual_count = source_count.min(limit);
    if visual_count == source_count {
        return matter
            .particle_render_payload(payload_id)
            .map_err(Into::into);
    }

    let mut sampled = ParticleSet::with_capacity(source_particles.set_id.clone(), visual_count);
    sampled.time_seconds = source_particles.time_seconds;
    for selection_index in 0..visual_count {
        if let Some(source_index) =
            evenly_spaced_source_index(selection_index, visual_count, source_count)
        {
            if let Some(particle) = source_particles.particles.get(source_index) {
                sampled.push(particle.clone());
            }
        }
    }

    ParticleRenderPayload::from_particle_set(payload_id, &sampled)
        .map_err(MatterSurfaceRuntimeError::from)
        .map_err(Into::into)
}

pub(crate) fn particle_upload_from_visual_frame(
    frame: &ParticleVisualFrame,
    source_rows: usize,
) -> QuestMakepadParticleUpload {
    let rows = frame
        .samples
        .iter()
        .map(|sample| QuestMakepadParticleRow {
            position_radius: [
                sample.position.x,
                sample.position.y,
                sample.position.z,
                sample.radius,
            ],
            color: [
                sample.color.r,
                sample.color.g,
                sample.color.b,
                sample.color.a,
            ],
            normal_frame: [
                sample.normal.x,
                sample.normal.y,
                sample.normal.z,
                sample.frame01,
            ],
            aux: [
                sample.rotation_radians,
                sample.aux0,
                sample.aux1,
                sample.flags as f32,
            ],
        })
        .collect();
    QuestMakepadParticleUpload {
        schema_id: QUEST_MAKEPAD_PARTICLE_UPLOAD_SCHEMA_ID.to_owned(),
        source_rows,
        rows,
    }
}

/// Converts a particle upload into bounded Makepad world-particle instances.
#[must_use]
pub fn world_particle_batch_from_upload(
    upload: &QuestMakepadParticleUpload,
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    placement: QuestMakepadWorldParticlePlacement,
    max_instances: usize,
) -> QuestMakepadWorldParticleBatch {
    let bounds_center = midpoint(bounds_min, bounds_max);
    let bounds_radius = bounds_radius(bounds_min, bounds_max).max(0.001);
    let placement_radius = placement.target_radius.max(0.001);
    let scale = placement_radius / bounds_radius;
    let upload_rows = upload.rows.len();
    let instance_count = upload_rows.min(max_instances);
    let instances = (0..instance_count)
        .filter_map(|selection_index| {
            let source_index =
                evenly_spaced_source_index(selection_index, instance_count, upload_rows)?;
            upload.rows.get(source_index)
        })
        .map(|row| {
            let source = [
                row.position_radius[0],
                row.position_radius[1],
                row.position_radius[2],
            ];
            let centered = [
                source[0] - bounds_center[0],
                source[1] - bounds_center[1],
                source[2] - bounds_center[2],
            ];
            QuestMakepadWorldParticleInstance {
                center_radius: [
                    placement.center[0] + centered[0] * scale,
                    placement.center[1] + centered[1] * scale,
                    placement.center[2] + centered[2] * scale,
                    row.position_radius[3].max(0.001) * scale,
                ],
                color: row.color,
                normal_frame: row.normal_frame,
                aux: row.aux,
            }
        })
        .collect::<Vec<_>>();
    QuestMakepadWorldParticleBatch {
        schema_id: QUEST_MAKEPAD_WORLD_PARTICLE_BATCH_SCHEMA_ID.to_owned(),
        source_schema_id: upload.schema_id.clone(),
        coordinate_space: placement.coordinate_space.to_owned(),
        render_mode: QUEST_MAKEPAD_CENTER_PROJECTED_BILLBOARD_MODE.to_owned(),
        content_center: placement.center,
        content_radius: placement_radius,
        replay_to_world_scale: scale,
        source_rows: upload.source_rows,
        dropped_rows: upload.source_rows.saturating_sub(instances.len()),
        instances,
    }
}

fn evenly_spaced_source_index(
    selection_index: usize,
    selection_count: usize,
    source_count: usize,
) -> Option<usize> {
    if source_count == 0 || selection_count == 0 || selection_index >= selection_count {
        return None;
    }
    if selection_count >= source_count {
        return Some(selection_index);
    }
    if selection_count == 1 {
        return Some(source_count / 2);
    }
    let numerator = selection_index
        .saturating_mul(source_count.saturating_sub(1))
        .saturating_add((selection_count - 1) / 2);
    Some((numerator / (selection_count - 1)).min(source_count - 1))
}

fn instance_spread_token(instances: &[QuestMakepadWorldParticleInstance]) -> String {
    let Some(first) = instances.first() else {
        return "0.000000,0.000000,0.000000".to_owned();
    };
    let mut minimum = [
        first.center_radius[0],
        first.center_radius[1],
        first.center_radius[2],
    ];
    let mut maximum = minimum;
    for instance in instances.iter().skip(1) {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(instance.center_radius[axis]);
            maximum[axis] = maximum[axis].max(instance.center_radius[axis]);
        }
    }
    vec3_marker_token([
        maximum[0] - minimum[0],
        maximum[1] - minimum[1],
        maximum[2] - minimum[2],
    ])
}
