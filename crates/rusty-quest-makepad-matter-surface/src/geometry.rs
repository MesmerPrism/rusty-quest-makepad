pub(crate) fn vec3_length(value: [f32; 3]) -> f32 {
    (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
}

pub(crate) fn midpoint(minimum: [f32; 3], maximum: [f32; 3]) -> [f32; 3] {
    [
        (minimum[0] + maximum[0]) * 0.5,
        (minimum[1] + maximum[1]) * 0.5,
        (minimum[2] + maximum[2]) * 0.5,
    ]
}

pub(crate) fn bounds_max_half_extent(minimum: [f32; 3], maximum: [f32; 3]) -> f32 {
    let extent_x = maximum[0] - minimum[0];
    let extent_y = maximum[1] - minimum[1];
    let extent_z = maximum[2] - minimum[2];
    extent_x.max(extent_y).max(extent_z).max(0.0) * 0.5
}

pub(crate) fn bounds_radius(minimum: [f32; 3], maximum: [f32; 3]) -> f32 {
    let center = midpoint(minimum, maximum);
    let dx = (maximum[0] - center[0])
        .abs()
        .max((minimum[0] - center[0]).abs());
    let dy = (maximum[1] - center[1])
        .abs()
        .max((minimum[1] - center[1]).abs());
    let dz = (maximum[2] - center[2])
        .abs()
        .max((minimum[2] - center[2]).abs());
    (dx.mul_add(dx, dy.mul_add(dy, dz * dz))).sqrt()
}
