#[allow(dead_code)]
#[path = "../../nurbs-core/tests/support/ss_closed_fiber.rs"]
mod intersections;
#[allow(dead_code)]
#[path = "support/fiber_cut_fixture.rs"]
mod sheets;
use brep_core::face_cut::{self, ClosedSsLimits};
use value_codec::Serialize;
fn main() {
    let s = intersections::two_ovals();
    let mut a = sheets::sheet(s.first().clone());
    let mut b = sheets::sheet(s.second().clone());
    a.tolerance_mm = 1e-4;
    b.tolerance_mm = 1e-4;
    let before = [a.to_value(), b.to_value()];
    let mut limits = ClosedSsLimits::default();
    limits.branches.max_repairs = 32;
    limits.trace.tolerance_world = 1e-4;
    limits.trace.tolerance_uv = 1e-4;
    let result = face_cut::partition_all_closed_ss(&a, 0, &b, 0, limits).unwrap();
    assert_eq!(result.traces.len(), 2);
    for m in &result.models {
        m.validate().unwrap();
        assert_eq!(m.faces.len(), 3);
        assert_eq!(m.faces[0].holes.len(), 2);
        assert!(m.persistent_naming_complete());
    }
    assert_eq!(before, [a.to_value(), b.to_value()]);
    println!(
        "two complete SS components; faces={:?}; placements={}; segments={:?}",
        result.faces,
        result.placement_attempts,
        result
            .traces
            .iter()
            .map(|t| t.segments().len())
            .collect::<Vec<_>>()
    );
}
