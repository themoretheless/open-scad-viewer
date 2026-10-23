//! Measure identical captured native requests; does not qualify browser latency.
use geometry_bridge::dispatch;
use sha2::{Digest, Sha256};
use std::{fs, time::Instant};
use value_codec::{Value, json};

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "Supply captured request and output JSON");
    let input = fs::read(&args[0]).unwrap();
    let request: Value = value_codec::from_str(std::str::from_utf8(&input).unwrap()).unwrap();
    assert_eq!(request["op"], json!("brep_miter_owned_place"));
    let mut timings = vec![];
    let mut reference = None;
    for _ in 0..5 {
        let owned = request.clone();
        let start = Instant::now();
        let report = dispatch(owned).unwrap();
        timings.push(start.elapsed().as_secs_f64() * 1000.);
        assert_eq!(report["resultModelBound"], json!(true));
        assert_eq!(report["solidGeometryCertified"], json!(true));
        let bytes = value_codec::encode_binary(&report).unwrap();
        if let Some(expected) = &reference {
            assert_eq!(&bytes, expected);
        } else {
            fs::write(format!("{}.response.json", args[1]), value_codec::to_string(&report).unwrap()).unwrap();
            reference = Some(bytes);
        }
    }
    let result = json!({"scope":"Native identical-request diagnostic; not a WASM/browser latency gate",
        "requestSha256":sha(&input),"responseBinarySha256":sha(reference.as_ref().unwrap()),
        "milliseconds":timings,"actualModelBound":true,"solidGeometryCertified":true});
    let text = value_codec::to_string(&result).unwrap();
    fs::write(&args[1], &text).unwrap();
    println!("{text}");
}
