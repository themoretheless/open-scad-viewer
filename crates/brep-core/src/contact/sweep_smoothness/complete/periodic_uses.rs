//! Whole opposite-side uses of one periodic source chart.
use crate::Surface;

pub(super) fn opposite_sides(surface: &Surface, a: &str, b: &str) -> bool {
    match (a, b) {
        ("uMin", "uMax") | ("uMax", "uMin") => surface.periodic_u,
        ("vMin", "vMax") | ("vMax", "vMin") => surface.periodic_v,
        _ => false,
    }
}
