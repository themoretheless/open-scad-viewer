use value_codec::json;
#[allow(dead_code)]
#[path = "../../nurbs-core/tests/support/ss_closed_fiber.rs"]
mod intersections;
#[allow(dead_code)]
#[path = "../../brep-core/examples/support/fiber_cut_fixture.rs"]
mod sheets;
fn main() {
    let s = intersections::circle();
    let mut a = sheets::sheet(s.first().clone());
    let mut b = sheets::sheet(s.second().clone());
    a.tolerance_mm = 1e-6;
    b.tolerance_mm = 1e-6;
    let input = json!({"op":"brep_partition_closed_ss","a":a,"b":b,"faceA":0,"faceB":0,"toleranceMm":1e-6,"toleranceUv":1e-6});
    println!("{}", value_codec::to_string(&input).unwrap());
}
