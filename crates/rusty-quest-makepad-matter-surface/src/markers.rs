use std::time::Instant;

pub(crate) fn sanitize_marker_value(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn elapsed_ms(started_at: Instant) -> f32 {
    started_at.elapsed().as_secs_f32() * 1000.0
}

pub(crate) fn vec3_marker_token(value: [f32; 3]) -> String {
    format!("{:.6},{:.6},{:.6}", value[0], value[1], value[2])
}

pub(crate) fn optional_usize_marker_token(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}
