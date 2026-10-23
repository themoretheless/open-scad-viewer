//! Lossless native transport codec. It never modifies or certifies geometry.
use super::{Result, Value, field, input};
use base64::Engine;
use flate2::{Compression, Decompress, FlushDecompress, Status, write::DeflateEncoder};
use sha2::{Digest, Sha256};
use std::io::Write;
use value_codec::json;

const FORMAT: &str = "native-deflate-json/1";
const MAX_ENCODED: usize = 4 * 1024 * 1024;
// The stored artifact retains its 4 MiB ceiling. Decoding also has a finite
// output ceiling, matching existing bounded WASM decompression carriers.
const MAX_DECODED: usize = 16 * 1024 * 1024;
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn pack(v: Value) -> Result<Value> {
    let text: String = field(&v, "json")?;
    if text.len() > MAX_DECODED {
        return Err(input("Native snapshot decoded size limit"));
    }
    value_codec::from_str::<Value>(&text)
        .map_err(|e| input(format!("Invalid snapshot JSON: {e}")))?;
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(text.as_bytes())
        .map_err(|e| input(e.to_string()))?;
    let bytes = encoder.finish().map_err(|e| input(e.to_string()))?;
    let packed = json!({"encoding":FORMAT,"decodedBytes":text.len(),"sha256":digest(text.as_bytes()),
        "data":base64::engine::general_purpose::STANDARD.encode(bytes)});
    if value_codec::to_string(&packed)
        .map_err(|e| input(e.to_string()))?
        .len()
        > MAX_ENCODED
    {
        return Err(input("Native snapshot encoded size limit"));
    }
    Ok(packed)
}
pub fn unpack(v: Value) -> Result<Value> {
    let packet = &v["snapshot"];
    let object = packet
        .as_object()
        .ok_or_else(|| input("Invalid native snapshot carrier"))?;
    if object.len() != 4 || field::<String>(packet, "encoding")? != FORMAT {
        return Err(input("Invalid native snapshot encoding"));
    }
    let length: usize = field(packet, "decodedBytes")?;
    let encoded: String = field(packet, "data")?;
    let hash: String = field(packet, "sha256")?;
    if length > MAX_DECODED || encoded.len() > MAX_ENCODED || hash.len() != 64 {
        return Err(input("Native snapshot size limit"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .map_err(|e| input(format!("Invalid native snapshot base64: {e}")))?;
    if base64::engine::general_purpose::STANDARD.encode(&bytes) != encoded {
        return Err(input("Noncanonical native snapshot base64"));
    }
    // One bounded allocation; neither a forged header nor a zip bomb can
    // expand beyond its declared size. Require complete stream consumption.
    let mut decoded = vec![0; length + 1];
    let mut decoder = Decompress::new(false);
    let status = decoder
        .decompress(&bytes, &mut decoded, FlushDecompress::Finish)
        .map_err(|e| input(format!("Invalid native snapshot deflate: {e}")))?;
    if status != Status::StreamEnd
        || decoder.total_in() != bytes.len() as u64
        || decoder.total_out() != length as u64
    {
        return Err(input("Native snapshot stream or length mismatch"));
    }
    decoded.truncate(length);
    if digest(&decoded) != hash {
        return Err(input("Native snapshot digest mismatch"));
    }
    let text = String::from_utf8(decoded).map_err(|e| input(e.to_string()))?;
    value_codec::from_str::<Value>(&text)
        .map_err(|e| input(format!("Invalid snapshot JSON: {e}")))?;
    Ok(json!(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_json_bytes_and_extreme_binary64_values_survive_native_transport() {
        let text = "{\"geometry\":{\"values\":[-0.0,1.0000000000000002,2.2250738585072014e-308,1.7976931348623157e308]},\"owner\":\"λ\"}";
        let encoded = pack(json!({"json":text})).unwrap();
        assert_eq!(unpack(json!({"snapshot":encoded})).unwrap(), json!(text));
    }
    #[test]
    fn size_digest_truncation_trailing_bytes_and_noncanonical_carriers_are_refused() {
        let good = pack(json!({"json":"{\"geometry\":[1,2,3]}"})).unwrap();
        for key in ["decodedBytes", "sha256", "encoding"] {
            let mut bad = good.clone();
            bad[key] = match key {
                "decodedBytes" => json!(1),
                "sha256" => json!("0".repeat(64)),
                _ => json!("unknown"),
            };
            assert!(unpack(json!({"snapshot":bad})).is_err());
        }
        let raw = base64::engine::general_purpose::STANDARD
            .decode(good["data"].as_str().unwrap())
            .unwrap();
        for bytes in [
            raw[..raw.len() - 1].to_vec(),
            [raw.as_slice(), &[0]].concat(),
        ] {
            let mut bad = good.clone();
            bad["data"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
            assert!(unpack(json!({"snapshot":bad})).is_err());
        }
        let mut bad = good.clone();
        bad["decodedBytes"] = json!(MAX_DECODED + 1);
        assert!(unpack(json!({"snapshot":bad})).is_err());
        let mut bad = good;
        bad["extra"] = json!(true);
        assert!(unpack(json!({"snapshot":bad})).is_err());
        assert!(pack(json!({"json":"not JSON"})).is_err());
    }
}
