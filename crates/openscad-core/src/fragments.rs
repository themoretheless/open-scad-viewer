//! OpenSCAD fragment rules backed by the shared geometry subdivision API.
pub use geometry_ops::fragment_resolution::*;

#[derive(Debug, Clone, Copy)]
pub struct LegacyResolution {
    pub requested: Option<f64>,
    pub before_cap: f64,
    pub maximum: f64,
    pub segments: f64,
    pub clamped: bool,
    pub reduced: bool,
}
/// Match Math.round, including negative zero and values too large to have fractions.
pub fn legacy_round(value: f64) -> f64 {
    if value.abs() >= 4503599627370496. || !value.is_finite() {
        return value;
    }
    let floor = value.floor();
    let rounded = if value - floor < 0.5 {
        floor
    } else {
        floor + 1.
    };
    if rounded == 0. && value.is_sign_negative() {
        -0.
    } else {
        rounded
    }
}
/// Viewer-subset quality caps. Language validation and warning presentation stay with callers.
pub fn legacy(
    requested: Option<f64>,
    fallback: f64,
    minimum: f64,
    preview: bool,
) -> LegacyResolution {
    let requested = requested.map(legacy_round);
    let maximum = if preview { 48. } else { 256. };
    let before_cap = requested.unwrap_or(if preview { fallback.min(24.) } else { fallback });
    let clamped = before_cap > maximum;
    let segments = before_cap.min(maximum).max(minimum);
    let reduced = preview && segments != requested.unwrap_or(fallback).min(256.).max(minimum);
    LegacyResolution {
        requested,
        before_cap,
        maximum,
        segments,
        clamped,
        reduced,
    }
}
#[cfg(test)]
mod legacy_tests {
    use super::*;
    #[test]
    fn rounding_and_preview_reduction() {
        assert!(legacy_round(-0.5).is_sign_negative());
        assert_eq!(legacy_round(-0.5), 0.);
        assert_eq!(legacy_round(0.49999999999999994), 0.);
        assert_eq!(legacy_round(1.5), 2.);
        assert_eq!(legacy_round(-1.5), -1.);
        let r = legacy(None, 32., 4., true);
        assert_eq!(r.segments, 24.);
        assert!(r.reduced);
        assert!(!r.clamped);
        let r = legacy(Some(1000.), 32., 3., true);
        assert_eq!(r.segments, 48.);
        assert!(r.reduced && r.clamped);
        let r = legacy(Some(-0.5), 32., 3., true);
        assert_eq!(r.segments, 3.);
        assert!(!r.reduced);
    }
}
