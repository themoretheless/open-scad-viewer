use geometry_bridge::dispatch;
use value_codec::json;
fn main() {
    let a = brep_core::analytic::sphere(2.).unwrap();
    let b = brep_core::analytic::sphere(5.).unwrap();
    let result=dispatch(json!({"op":"cad_shell_distance","a":a,"b":b,"toleranceMm":0.001,"toleranceUv":1e-7,"maxCells":1000000,"maxDomainCells":8000000})).unwrap();
    let ma = dispatch(json!({"op":"brep_nurbs_tessellate","model":a,"segments":4})).unwrap();
    let mb = dispatch(json!({"op":"brep_nurbs_tessellate","model":b,"segments":4})).unwrap();
    println!(
        "{}",
        json!({"document":{"version":1,"sketches":[],"bodies":[{"id":"inner","name":"Inner sphere","brep":a,"mesh":ma},{"id":"outer","name":"Outer sphere","brep":b,"mesh":mb}]},"expectedMm":3.,"result":result})
    );
}
