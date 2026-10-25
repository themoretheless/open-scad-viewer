//! Replay one saved inspection request without changing its budgets or recipe.
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, io::Write, time::Instant};
use value_codec::{Value, json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let input = args.next().ok_or("provide request JSON and new output path")?;
    let output = args.next().ok_or("provide a new output path")?;
    if args.next().is_some() {
        return Err("expected exactly two paths".into());
    }
    let bytes = std::fs::read(&input)?;
    let request: Value = value_codec::from_str(std::str::from_utf8(&bytes)?)
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    if request["op"].as_str() != Some("cad_expression_shell_inspect") {
        return Err("request must use cad_expression_shell_inspect".into());
    }
    // Check early, then use create_new again to protect against races.
    if std::path::Path::new(&output).try_exists()? {
        return Err("output already exists".into());
    }
    let started = Instant::now();
    let response = geometry_bridge::dispatch(request.clone())
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.;
    let report = json!({
        "schema":"expression-inspection-native-replay/1",
        "requestSha256":Sha256::digest(&bytes).iter().map(|byte|format!("{byte:02x}")).collect::<String>(),
        "requestBytes":bytes.len(),"request":request,"response":response,
        "elapsedMs":elapsed_ms,
        "scope":"One native dispatch of the unchanged saved request; no additional geometry admission or latency guarantee."
    });
    let encoded = value_codec::to_string(&report)
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(output)?;
    file.write_all(encoded.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
