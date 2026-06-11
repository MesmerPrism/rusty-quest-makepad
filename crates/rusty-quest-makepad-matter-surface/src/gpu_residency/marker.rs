use super::QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS;

pub(super) fn optional_usize_marker_token(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

pub(super) fn saturating_u32(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

pub(super) fn hex_u32_marker_token(value: u32) -> String {
    format!("0x{value:08X}")
}

pub(super) fn u32_words_marker_token(
    words: &[u32; QUEST_MAKEPAD_GPU_ORACLE_COMPUTE_PROBE_WORDS],
) -> String {
    words
        .iter()
        .map(|word| hex_u32_marker_token(*word))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn finite_f64_marker_token(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.3}")
    } else {
        "unavailable".to_owned()
    }
}

pub(super) fn finite_f32_marker_token(value: f32) -> String {
    if value.is_finite() {
        format!("{value:.6}")
    } else {
        "unavailable".to_owned()
    }
}
