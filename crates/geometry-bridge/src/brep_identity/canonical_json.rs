use super::*;

/// ECMAScript Number::toString for a finite binary64 (JSON.stringify number form).
pub(super) fn js_f64(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned(); // JSON.stringify(-0) is "0"
    }
    let negative = value.is_sign_negative();
    let absolute = value.abs();
    // Rust LowerExp emits the shortest round-trip decimal, matching the digits
    // ECMAScript selects; only the exponent/decimal-point layout differs.
    let scientific = format!("{absolute:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("LowerExp always carries an exponent");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let digit_count = digits.len() as i32;
    let point = exponent + 1; // n in the spec: decimal point position
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if point > 0 && point <= 21 {
        if digit_count <= point {
            out.push_str(&digits);
            out.extend(std::iter::repeat_n('0', (point - digit_count) as usize));
        } else {
            out.push_str(&digits[..point as usize]);
            out.push('.');
            out.push_str(&digits[point as usize..]);
        }
    } else if point > -6 && point <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-point) as usize));
        out.push_str(&digits);
    } else {
        out.push_str(&digits[..1]);
        if digit_count > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        let exponent = point - 1;
        out.push(if exponent >= 0 { '+' } else { '-' });
        out.push_str(&exponent.unsigned_abs().to_string());
    }
    out
}

pub(crate) fn js_number(number: &Number) -> String {
    match number {
        // Integers within the exactly-representable range print as digits; a
        // wider wire integer could only have reached the host as a rounded
        // binary64, so it is formatted through the float path for parity.
        Number::Unsigned(v) if *v <= (1u64 << 53) => v.to_string(),
        Number::Signed(v) if v.unsigned_abs() <= (1u64 << 53) => v.to_string(),
        Number::Unsigned(v) => js_f64(*v as f64),
        Number::Signed(v) => js_f64(*v as f64),
        Number::Float(v) => js_f64(*v),
    }
}

/// JSON.stringify string escaping (controls, quote and backslash only).
pub(super) fn js_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// stableJson from src/core/semanticProgram.ts: keys sorted by raw UTF-8 bytes.
/// BTreeMap<String, _> iteration order is exactly that byte order.
pub(crate) fn stable_json(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Value::Number(v) => out.push_str(&js_number(v)),
        Value::String(v) => js_string(v, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                stable_json(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (index, (key, item)) in map.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                js_string(key, out);
                out.push(':');
                stable_json(item, out);
            }
            out.push('}');
        }
    }
}
