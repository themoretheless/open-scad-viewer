//! Native measurement of one original retained-domain request, independent of WASM packaging.
use value_codec::{Value, json};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let request: Value = value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let original = request["definition"].clone();
    let started = std::time::Instant::now();
    let result = geometry_bridge::dispatch(request.clone()).unwrap();
    assert_eq!(result["admitted"], json!(true));
    assert_eq!(result["sourceBody"], original);
    assert_eq!(result["wallCoverage"]["request"], request["wallCoverage"]);
    std::fs::write(&args[2], value_codec::to_string(&json!({
        "elapsedMs": started.elapsed().as_secs_f64()*1000., "result": result,
        "scope": "One native original-body request; packaging and visual UI acceptance remain separate."
    })).unwrap()).unwrap();
}
