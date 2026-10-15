//! Standalone Rust checker: one JSON request on stdin, no geometry bridge.
use brep_core::profile_certificate::{ProfileCertificate, certify, verify};
use nurbs_core::curve::Curve;
use std::io::{self, Read};
use value_codec::{Value, json};
fn run() -> std::result::Result<Value, Box<dyn std::error::Error>> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "verify".into());
    let mut source = String::new();
    io::stdin().take(2_000_001).read_to_string(&mut source)?;
    if source.len() > 2_000_000 {
        return Err("Profile certificate request exceeds 2 MB".into());
    }
    let v: Value = value_codec::from_str(&source)?;
    let loops: Vec<Vec<Curve>> = value_codec::from_value(v["loops"].clone())?;
    let tolerance: f64 = value_codec::from_value(v["toleranceMm"].clone())?;
    match mode.as_str() {
        "certify" => Ok(value_codec::to_value(certify(&loops, tolerance)?)?),
        "verify" => {
            let c: ProfileCertificate = value_codec::from_value(v["certificate"].clone())?;
            let verified = verify(&loops, tolerance, &c)?;
            Ok(
                json!({"status":"verified","scope":"represented-profile-topology",
                "spans":verified.spans(),"pieces":verified.pieces(),"nestingDepth":verified.nesting_depth()}),
            )
        }
        _ => Err("Use certify or verify".into()),
    }
}
fn main() {
    match run() {
        Ok(v) => println!("{}", value_codec::to_string(&v).unwrap()),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
