//! Profile the exact native volume continuation captured from a failing UI build.
use geometry_bridge::dispatch;
use sha2::{Digest, Sha256};
use std::{fs, time::Instant};
use value_codec::{Value, json};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        2,
        "Supply captured request JSON and output prefix"
    );
    let bytes = fs::read(&args[0]).unwrap();
    let request: Value = value_codec::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
    assert_eq!(request["op"], json!("brep_sweep_volume_begin"));
    let started = Instant::now();
    let session = dispatch(request).unwrap()["session"].clone();
    let mut report = None;
    let mut steps = 0;
    for _ in 0..10000 {
        let step = dispatch(json!({"op":"brep_sweep_volume_step","session":session,"maxUnits":32}))
            .unwrap();
        steps += 1;
        if step["done"] == json!(true) {
            report = Some(step["report"].clone());
            break;
        }
    }
    dispatch(json!({"op":"brep_sweep_volume_drop","session":session})).unwrap();
    let milliseconds = started.elapsed().as_secs_f64() * 1000.;
    let report = report.expect("Original continuation allowance did not finish");
    assert_eq!(report["solidGeometryCertified"], json!(true));
    assert_eq!(report["boundary"]["allFacesInjective"], json!(true));
    assert_eq!(report["boundary"]["allPairsClassified"], json!(true));
    fs::write(
        format!("{}.report.json", args[1]),
        value_codec::to_string(&report).unwrap(),
    )
    .unwrap();
    let result = json!({
        "scope":"Native timing/profiling of an unchanged captured request; not browser qualification",
        "requestSha256":Sha256::digest(&bytes).iter().map(|b|format!("{b:02x}")).collect::<String>(),
        "milliseconds":milliseconds,"steps":steps,"solidGeometryCertified":true
    });
    let text = value_codec::to_string(&result).unwrap();
    fs::write(&args[1], &text).unwrap();
    println!("{text}");
}
