//! Makepad-facing world debug rows for Optics ADF debug visuals.
//!
//! This module adapts the renderer-neutral ADF debug frame into bounded
//! world-object rows. It does not build ADF fields, resolve Optics visuals, or
//! own simulation truth.

use crate::{
    sanitize_marker_value, vec3_length, vec3_marker_token, QuestMakepadAdfDebugFrame,
    DEFAULT_WORLD_CONTENT_CENTER, DEFAULT_WORLD_CONTENT_TARGET_RADIUS,
    QUEST_MAKEPAD_CONTENT_LOCAL_SPACE, QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE,
};

/// Quest Makepad world ADF debug batch schema.
pub const QUEST_MAKEPAD_WORLD_ADF_DEBUG_BATCH_SCHEMA_ID: &str =
    "rusty.quest.makepad.world_adf_debug_batch.v1";
/// Quest Makepad world ADF debug marker prefix.
pub const QUEST_MAKEPAD_WORLD_ADF_DEBUG_MARKER_PREFIX: &str = "RUSTY_QUEST_MAKEPAD_WORLD_ADF_DEBUG";
/// Current Makepad-facing ADF debug primitive mode.
pub const QUEST_MAKEPAD_WORLD_ADF_DEBUG_RENDER_MODE: &str = "adf-debug-cell-boxes";
/// Selection policy used when ADF debug cells exceed the draw/debug cap.
pub const QUEST_MAKEPAD_WORLD_ADF_DEBUG_EVEN_SELECTION_POLICY: &str = "evenly-spaced-source-cells";

/// Placement policy for ADF debug cells rendered as Makepad world objects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadWorldAdfDebugPlacement {
    /// Debug content center in the target Makepad/XR coordinate space.
    pub center: [f32; 3],
    /// Target radius for the source ADF root cube.
    pub target_radius: f32,
    /// Coordinate space for `center` and emitted cell centers.
    pub coordinate_space: &'static str,
}

impl Default for QuestMakepadWorldAdfDebugPlacement {
    fn default() -> Self {
        Self {
            center: DEFAULT_WORLD_CONTENT_CENTER,
            target_radius: DEFAULT_WORLD_CONTENT_TARGET_RADIUS,
            coordinate_space: QUEST_MAKEPAD_START_HEAD_LOCAL_SPACE,
        }
    }
}

impl QuestMakepadWorldAdfDebugPlacement {
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

/// One Makepad-facing ADF debug cell row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMakepadWorldAdfDebugCell {
    /// World-space cell center and cubic extent as `[x, y, z, extent]`.
    pub center_extent: [f32; 4],
    /// Distance fields as `[center, min, max, normalized_center]`.
    pub distance: [f32; 4],
    /// Cell metadata as `[level, normalized_range, source_samples, cell_index]`.
    pub meta: [f32; 4],
}

/// Bounded Makepad-facing ADF world debug batch.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadWorldAdfDebugBatch {
    /// Schema identifier.
    pub schema_id: String,
    /// Source Quest-Makepad ADF frame schema identifier.
    pub source_schema_id: String,
    /// Source Optics ADF visual schema identifier.
    pub source_visual_schema_id: String,
    /// Source Matter ADF field id.
    pub source_field_id: String,
    /// Source Matter SDF grid id.
    pub source_grid_id: String,
    /// Coordinate space for the cell centers.
    pub coordinate_space: String,
    /// Renderer-facing primitive mode.
    pub render_mode: String,
    /// Debug content center in the target coordinate space.
    pub content_center: [f32; 3],
    /// Debug content radius in the target coordinate space.
    pub content_radius: f32,
    /// Scale from ADF source units to Makepad world units.
    pub source_to_world_scale: f32,
    /// Full source ADF visual cell count before the batch bound.
    pub source_cells: usize,
    /// Cells dropped by `max_cells`.
    pub dropped_cells: usize,
    /// World-space debug cells.
    pub cells: Vec<QuestMakepadWorldAdfDebugCell>,
}

impl QuestMakepadWorldAdfDebugBatch {
    /// Builds a compact evidence marker without logging high-rate cell rows.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        format!(
            "{} schema={} phase={} status={} renderMode={} coordinateSpace={} sourceSchema={} sourceVisualSchema={} sourceFieldId={} sourceGridId={} sourceCells={} cellRows={} droppedCells={} selectionPolicy={} contentCenter={} contentRadius={:.6} sourceToWorldScale={:.6} contentCenterDistanceMeters={:.3} cellSpread={} dataPlane=makepad-world-adf-debug-cells",
            QUEST_MAKEPAD_WORLD_ADF_DEBUG_MARKER_PREFIX,
            QUEST_MAKEPAD_WORLD_ADF_DEBUG_BATCH_SCHEMA_ID,
            sanitize_marker_value(phase),
            if self.cells.is_empty() { "empty" } else { "ready" },
            sanitize_marker_value(&self.render_mode),
            sanitize_marker_value(&self.coordinate_space),
            sanitize_marker_value(&self.source_schema_id),
            sanitize_marker_value(&self.source_visual_schema_id),
            sanitize_marker_value(&self.source_field_id),
            sanitize_marker_value(&self.source_grid_id),
            self.source_cells,
            self.cells.len(),
            self.dropped_cells,
            QUEST_MAKEPAD_WORLD_ADF_DEBUG_EVEN_SELECTION_POLICY,
            vec3_marker_token(self.content_center),
            self.content_radius,
            self.source_to_world_scale,
            vec3_length(self.content_center),
            cell_spread_token(&self.cells),
        )
    }
}

/// Converts an ADF debug frame into bounded Makepad world debug cells.
#[must_use]
pub fn world_adf_debug_batch_from_frame(
    frame: &QuestMakepadAdfDebugFrame,
    placement: QuestMakepadWorldAdfDebugPlacement,
    max_cells: usize,
) -> QuestMakepadWorldAdfDebugBatch {
    let visual = &frame.visual;
    let source_cells = visual.cell_count;
    let source_rows = visual.cells.len();
    let cell_count = source_rows.min(max_cells);
    let root_extent = visual.root_extent.max(0.001);
    let root_center = [
        visual.root_origin.x + root_extent * 0.5,
        visual.root_origin.y + root_extent * 0.5,
        visual.root_origin.z + root_extent * 0.5,
    ];
    let root_radius = ((root_extent * 0.5).powi(2) * 3.0).sqrt().max(0.001);
    let placement_radius = placement.target_radius.max(0.001);
    let scale = placement_radius / root_radius;
    let cells = (0..cell_count)
        .filter_map(|selection_index| {
            let source_index =
                evenly_spaced_source_index(selection_index, cell_count, source_rows)?;
            visual.cells.get(source_index)
        })
        .map(|cell| {
            let centered = [
                cell.center.x - root_center[0],
                cell.center.y - root_center[1],
                cell.center.z - root_center[2],
            ];
            QuestMakepadWorldAdfDebugCell {
                center_extent: [
                    placement.center[0] + centered[0] * scale,
                    placement.center[1] + centered[1] * scale,
                    placement.center[2] + centered[2] * scale,
                    cell.extent.max(0.001) * scale,
                ],
                distance: [
                    cell.center_distance,
                    cell.min_distance,
                    cell.max_distance,
                    cell.normalized_center_distance,
                ],
                meta: [
                    cell.level as f32,
                    cell.normalized_range,
                    cell.source_sample_count as f32,
                    cell.cell_index as f32,
                ],
            }
        })
        .collect::<Vec<_>>();
    QuestMakepadWorldAdfDebugBatch {
        schema_id: QUEST_MAKEPAD_WORLD_ADF_DEBUG_BATCH_SCHEMA_ID.to_owned(),
        source_schema_id: frame.schema_id.clone(),
        source_visual_schema_id: frame.visual_schema_id.clone(),
        source_field_id: frame.source_field_id.clone(),
        source_grid_id: frame.source_grid_id.clone(),
        coordinate_space: placement.coordinate_space.to_owned(),
        render_mode: QUEST_MAKEPAD_WORLD_ADF_DEBUG_RENDER_MODE.to_owned(),
        content_center: placement.center,
        content_radius: placement_radius,
        source_to_world_scale: scale,
        source_cells,
        dropped_cells: source_cells.saturating_sub(cells.len()),
        cells,
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

fn cell_spread_token(cells: &[QuestMakepadWorldAdfDebugCell]) -> String {
    let Some(first) = cells.first() else {
        return "0.000000,0.000000,0.000000".to_owned();
    };
    let mut minimum = [
        first.center_extent[0],
        first.center_extent[1],
        first.center_extent[2],
    ];
    let mut maximum = minimum;
    for cell in cells.iter().skip(1) {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(cell.center_extent[axis]);
            maximum[axis] = maximum[axis].max(cell.center_extent[axis]);
        }
    }
    vec3_marker_token([
        maximum[0] - minimum[0],
        maximum[1] - minimum[1],
        maximum[2] - minimum[2],
    ])
}
