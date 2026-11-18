use value_codec::json;
#[allow(dead_code)]
#[path = "../../nurbs-core/tests/support/ss_closed_fiber.rs"]
mod intersections;
#[allow(dead_code)]
#[path = "../../brep-core/examples/support/fiber_cut_fixture.rs"]
mod sheets;
fn main() {
    let network = std::env::args().any(|a| a == "--network");
    let s = if network {
        intersections::two_ovals()
    } else {
        intersections::circle()
    };
    let mut a = sheets::sheet(s.first().clone());
    let mut b = sheets::sheet(s.second().clone());
    a.tolerance_mm = if network { 1e-4 } else { 1e-6 };
    b.tolerance_mm = a.tolerance_mm;
    let mut input = json!({"op":"brep_partition_closed_ss","a":a,"b":b,"faceA":0,"faceB":0,"toleranceMm":1e-6,"toleranceUv":1e-6});
    if network {
        input["op"] = json!("brep_partition_all_closed_ss");
        input["maxRepairs"] = json!(32);
        input["toleranceMm"] = json!(1e-4);
        input["toleranceUv"] = json!(1e-4);
    }
    println!("{}", value_codec::to_string(&input).unwrap());
}
