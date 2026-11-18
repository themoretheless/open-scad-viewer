use value_codec::Serialize;
#[path = "support/fiber_cut_fixture.rs"]
mod sheet_fixture;
#[path = "../../nurbs-core/tests/support/ss_closed_fiber.rs"]
mod ss_fixture;
use brep_core::face_cut::{self, ClosedSsLimits};
fn main() {
    let sources = ss_fixture::circle();
    let mut a = sheet_fixture::sheet(sources.first().clone());
    let mut b = sheet_fixture::sheet(sources.second().clone());
    a.tolerance_mm = 1e-6;
    b.tolerance_mm = 1e-6;
    let before = (a.to_value(), b.to_value());
    let result = face_cut::partition_closed_ss(&a, 0, &b, 0, ClosedSsLimits::default()).unwrap();
    result.first.model.validate().unwrap();
    result.second.model.validate().unwrap();
    result.trace.reverify().unwrap();
    assert_eq!(result.first.model.faces.len(), 2);
    assert_eq!(result.second.model.faces.len(), 2);
    assert_eq!(result.first.model.faces[0].holes.len(), 1);
    assert_eq!(before, (a.to_value(), b.to_value()));
    println!(
        "paired closed SS: {} segments; errors world={} UV={:?}; faces={}/{}",
        result.trace.segments().len(),
        result.trace.world_error_upper(),
        result.trace.uv_error_upper(),
        result.first.model.faces.len(),
        result.second.model.faces.len()
    );
}
