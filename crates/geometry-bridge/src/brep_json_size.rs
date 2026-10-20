//! UTF-16 JSON character accounting for the legacy browser snapshot contract.
use super::Value;
fn string_size(s: &str) -> usize {
    2 + s
        .chars()
        .map(|c| match c {
            '"' | '\\' | '\n' | '\r' | '\t' | '\u{8}' | '\u{c}' => 2,
            c if c < ' ' => 6,
            c => c.len_utf16(),
        })
        .sum::<usize>()
}
#[inline]
fn decimal_digits_u32(n: u32) -> usize {
    if n < 10 {
        1
    } else if n < 100 {
        2
    } else if n < 1_000 {
        3
    } else {
        (n.ilog10() + 1) as usize
    }
}
fn number_size(x: f64) -> usize {
    if !x.is_finite() {
        return 4; // JSON serializes non-finite numbers as "null"
    }
    if x == 0. {
        return 1;
    }
    let sign = usize::from(x < 0.);
    let abs = x.abs();
    // Fast path for exact non-negative small integers up to 999,999,999
    if abs.fract() == 0.0 && abs < 1_000_000_000.0 {
        return sign + decimal_digits_u32(abs as u32);
    }
    use std::io::Write;
    let mut buf = [0u8; 32];
    let mut cursor = std::io::Cursor::new(&mut buf[..]);
    let _ = write!(cursor, "{:e}", abs);
    let len = cursor.position() as usize;
    let raw = std::str::from_utf8(&buf[..len]).unwrap_or("0e0");
    let (mantissa, exponent) = raw.split_once('e').unwrap();
    let exponent: i32 = exponent.parse().unwrap();
    let digits = mantissa.bytes().filter(|b| *b != b'.').count();
    if !(-6..21).contains(&exponent) {
        sign + digits + usize::from(digits > 1) + 2 + decimal_digits_u32(exponent.unsigned_abs())
    } else if exponent >= 0 {
        sign + if digits <= exponent as usize + 1 {
            exponent as usize + 1
        } else {
            digits + 1
        }
    } else {
        sign + 2 + (-exponent - 1) as usize + digits
    }
}
pub fn characters(value: &Value) -> usize {
    match value {
        Value::Null => 4,
        Value::Bool(v) => {
            if *v {
                4
            } else {
                5
            }
        }
        Value::Number(v) => number_size(v.as_f64().unwrap()),
        Value::String(s) => string_size(s),
        Value::Array(a) => 2 + a.len().saturating_sub(1) + a.iter().map(characters).sum::<usize>(),
        Value::Object(o) => {
            2 + o.len().saturating_sub(1)
                + o.iter()
                    .map(|(key, value)| string_size(key) + 1 + characters(value))
                    .sum::<usize>()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;
    #[test]
    fn browser_json_conventions() {
        for (x, n) in [
            (0., 1),
            (-0., 1),
            (1., 1),
            (1e-6, 8),
            (1e-7, 4),
            (1e20, 21),
            (1e21, 5),
            (-1.25, 5),
            (1.23e-7, 7),
        ] {
            assert_eq!(number_size(x), n, "{x}");
        }
        assert_eq!(
            characters(&json!({"a":"😀\n","b":[1.0,true,Value::Null]})),
            30
        );
    }
    #[test]
    fn independent_browser_number_length_oracle() {
        let rows: Value = value_codec::from_str(include_str!(
            "../tests/fixtures/brep-json-number-lengths.json"
        ))
        .unwrap();
        for row in rows.as_array().unwrap() {
            assert_eq!(
                characters(&row[0]),
                row[1].as_u64().unwrap() as usize,
                "{}",
                row[0]
            );
        }
    }
}
