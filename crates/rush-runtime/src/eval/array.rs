use super::*;

pub(super) fn array(value: &Json) -> &[Json] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub(super) fn string(value: &Json) -> &str {
    value.as_str().unwrap_or("")
}
pub(super) fn number(value: &Json) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}
pub(super) fn unique(items: &[Json]) -> bool {
    let mut seen = HashSet::with_capacity(items.len());
    items.iter().all(|item| seen.insert(string(item)))
}
