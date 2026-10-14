use value_codec::{Value, json};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let fixture: Value =
        value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let mut request = fixture["request"].clone();
    request["op"] = json!("cad_self_intersection");
    let start = std::time::Instant::now();
    let result = geometry_bridge::dispatch(request.clone()).unwrap();
    println!("{}",value_codec::to_string(&json!({"elapsedMs":start.elapsed().as_secs_f64()*1000.,"visited":result["visitedPairs"],"disjoint":result["disjointPairCount"],"shared":result["sharedBoundaryPairCount"],"unresolved":result["unresolvedPairCount"],"absenceProven":result["absenceProven"]})).unwrap());
    std::fs::write(
        &args[2],
        value_codec::to_string(&json!({"request":request,"result":result})).unwrap(),
    )
    .unwrap();
}
