//! Golden models for interchange tests (ADR 0011). Kernel tests build a model and compare it
//! with a committed JSON file; `cad-step` tests read the same file and check the STEP round
//! trip. Numbers compare with a relative tolerance of 1e-9 so that libm differences between
//! hosts do not fail the comparison. Set `BREP_UPDATE_EXCHANGE_FIXTURES=1` to rewrite files.
use value_codec::Value;

pub(crate) const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/exchange");

pub(crate) fn assert_matches(name: &str, actual: Value) {
    let path = format!("{DIR}/{name}.json");
    if std::env::var_os("BREP_UPDATE_EXCHANGE_FIXTURES").is_some() {
        std::fs::create_dir_all(DIR).unwrap();
        std::fs::write(&path, value_codec::to_string(&actual).unwrap()).unwrap();
        return;
    }
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e}; run with BREP_UPDATE_EXCHANGE_FIXTURES=1 to create it"));
    let expected: Value = value_codec::from_str(&text).unwrap();
    assert_close(&expected, &actual, name);
}

fn assert_close(expected: &Value, actual: &Value, at: &str) {
    match (expected, actual) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!((a - b).abs() <= 1e-9 * a.abs().max(1.), "{at}: fixture {a}, actual {b}");
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}: array length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                assert_close(x, y, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "{at}: keys");
            for (k, x) in a {
                assert_close(x, &b[k], &format!("{at}.{k}"));
            }
        }
        _ => assert_eq!(expected, actual, "{at}"),
    }
}
