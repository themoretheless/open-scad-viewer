//! Read-only transport verification of an explicitly supplied original JSON.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Supply an original JSON path")?;
    let text = std::fs::read_to_string(path)?;
    let packed = geometry_bridge::dispatch(
        value_codec::json!({"op":"native_geometry_snapshot_pack","json":text}),
    )?;
    let size = value_codec::to_string(&packed)?.len();
    let unpacked = geometry_bridge::dispatch(
        value_codec::json!({"op":"native_geometry_snapshot_unpack","snapshot":packed}),
    )?;
    assert_eq!(unpacked.as_str(), Some(text.as_str()));
    assert!(size <= 4 * 1024 * 1024);
    println!(
        "Original {} bytes, encoded {} bytes; exact original JSON restored",
        text.len(),
        size
    );
    Ok(())
}
