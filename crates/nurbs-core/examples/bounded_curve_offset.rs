//! Export a bounded offset request for an independent geometric verifier.
fn main() {
    let file = std::env::args().nth(1).expect("Usage: bounded_curve_offset request.json");
    let request = std::fs::read_to_string(file).expect("Read request");
    let response = nurbs_core::execute(&request);
    println!("{response}");
    let parsed: value_codec::Value = value_codec::from_str(&response).expect("Kernel response");
    if parsed["ok"].as_bool() != Some(true) { std::process::exit(1); }
}
