//! Native B-rep scene plan: display-policy admission and hash derivation.
//! The display policy hash is a genuine derivation, so it lives behind the
//! WASM ABI; the diagnostic lane re-derives it host-side on purpose so a
//! worker cannot self-certify its own policy.
use super::{Result, Value, field, input};
use sha2::{Digest, Sha256};
use value_codec::json;

fn hex(digest: &[u8]) -> String {
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Canonical `brep-display-policy-v1` preimage; byte-identical to the retired
/// host derivation `sha256Hex('brep-display-policy-v1\n' + JSON.stringify(...))`.
pub fn plan(v: &Value) -> Result<Value> {
    let quality: String = field(v, "quality")?;
    if quality != "preview" && quality != "full" {
        return Err(input(
            "B-rep display requires preview/full quality and 1..32 segments",
        ));
    }
    let segments: usize = field(v, "segments")?;
    if !(1..=32).contains(&segments) {
        return Err(input(
            "B-rep display requires preview/full quality and 1..32 segments",
        ));
    }
    let policy = format!(
        "{{\"version\":1,\"sampling\":\"uniform-patch\",\"quality\":\"{quality}\",\"segments\":{segments}}}"
    );
    let preimage = format!("brep-display-policy-v1\n{policy}");
    let digest = Sha256::digest(preimage.as_bytes());
    Ok(
        json!({"quality":quality,"segments":segments,"policy":policy,"displayPolicyHash":hex(&digest)}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn derives_the_pinned_host_display_policy_hash() {
        let planned = plan(&json!({"quality":"preview","segments":4})).unwrap();
        assert_eq!(
            planned["displayPolicyHash"].as_str().unwrap(),
            // sha256 of 'brep-display-policy-v1\n{"version":1,"sampling":"uniform-patch","quality":"preview","segments":4}'
            "ec733240f798c707d1e39c9028d2483ea9eb929e95a870eeb9165384d8f3a97b"
        );
        assert_eq!(
            planned["policy"].as_str().unwrap(),
            "{\"version\":1,\"sampling\":\"uniform-patch\",\"quality\":\"preview\",\"segments\":4}"
        );
        let full = plan(&json!({"quality":"full","segments":32})).unwrap();
        assert_ne!(
            full["displayPolicyHash"].as_str().unwrap(),
            planned["displayPolicyHash"].as_str().unwrap()
        );
    }
    #[test]
    fn refuses_out_of_contract_display_policies() {
        for value in [
            json!({"quality":"draft","segments":4}),
            json!({"quality":"preview","segments":0}),
            json!({"quality":"preview","segments":33}),
            json!({"quality":"preview","segments":2.5}),
        ] {
            assert!(plan(&value).is_err());
        }
    }
}
