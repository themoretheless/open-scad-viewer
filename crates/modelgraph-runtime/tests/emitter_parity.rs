//! Captured results from the original TypeScript compiler cover geometry emission,
//! instance identity, solved constraints and failure diagnostics during migration.
use value_codec::Value;
fn close(a: f64, b: f64, path: &str) {
    assert!(
        (a - b).abs() <= 1e-8 * (1. + a.abs().max(b.abs())),
        "numeric mismatch {path}: {a} != {b}"
    );
}
fn source_tokens(s: &str) -> (String, Vec<f64>) {
    let bytes = s.as_bytes();
    let mut text = String::new();
    let mut numbers = vec![];
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'-' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit())
        {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                i += 1;
                if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if let Ok(n) = s[start..i].parse() {
                numbers.push(n);
                text.push('#');
            } else {
                text.push_str(&s[start..i]);
            }
        } else {
            let ch = s[i..].chars().next().unwrap();
            text.push(ch);
            i += ch.len_utf8();
        }
    }
    (text, numbers)
}
fn approx(a: &Value, b: &Value, path: &str) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            close(a.as_f64().unwrap(), b.as_f64().unwrap(), path)
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "array length {path}");
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                approx(a, b, &format!("{path}/{i}"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len(), "object length {path}: {a:?} vs {b:?}");
            for (k, v) in a {
                approx(
                    v,
                    b.get(k).unwrap_or_else(|| panic!("missing {path}/{k}")),
                    &format!("{path}/{k}"),
                );
            }
        }
        (Value::String(a), Value::String(b)) if path.ends_with("/source") => {
            let (ta, na) = source_tokens(a);
            let (tb, nb) = source_tokens(b);
            assert_eq!(ta, tb, "source tokens {path}");
            assert_eq!(na.len(), nb.len(), "source coordinate length {path}");
            for (i, (a, b)) in na.iter().zip(nb.iter()).enumerate() {
                close(*a, *b, &format!("{path}/coordinate/{i}"));
            }
        }
        _ => assert_eq!(a, b, "mismatch {path}"),
    }
}
#[test]
fn geometry_and_diagnostics_match_original_compiler() {
    let cases: Value = value_codec::from_str(include_str!("fixtures/emitter-parity.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        match modelgraph_runtime::compile(case["document"].clone()) {
            Ok(result) => {
                assert!(case.get("result").is_some(), "unexpected success {name}");
                let mut expected = case["result"].clone();
                // The archived compiler fixture used the external kernel.
                // Source and graph semantics remain identical; routing changes.
                expected["execution_target"] = value_codec::json!("legacy/current+own-rust-cad");
                approx(&result, &expected, name);
            }
            Err(error) => {
                assert!(
                    case.get("error").is_some(),
                    "unexpected failure {name}: {error:?}"
                );
                approx(&value_codec::to_value(error).unwrap(), &case["error"], name);
            }
        }
    }
}
