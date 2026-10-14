use super::*;

/// `toLocaleString()` grouping used by the pinned limit messages.
pub fn locale(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// ECMAScript `Number.prototype.toString` for finite numbers.
pub fn js_number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value == f64::INFINITY {
        return "Infinity".to_string();
    }
    if value == f64::NEG_INFINITY {
        return "-Infinity".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let negative = value < 0.0;
    let rendered = format!("{:e}", value.abs());
    let (mantissa, exponent) = rendered.split_once('e').expect("LowerExp has exponent");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent + 1;
    let body = if k <= n && n <= 21 {
        let mut out = digits;
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
        out
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        let mut out = String::from("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(&digits);
        out
    } else {
        let mut out = String::new();
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if n > 0 { '+' } else { '-' });
        out.push_str(&(n - 1).abs().to_string());
        out
    };
    if negative { format!("-{body}") } else { body }
}

/// ECMAScript `Number.prototype.toPrecision(p)` for finite nonzero numbers.
pub fn js_to_precision(value: f64, precision: usize) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if !value.is_finite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if value == 0.0 {
        return if precision > 1 {
            format!("0.{}", "0".repeat(precision - 1))
        } else {
            "0".to_string()
        };
    }
    let negative = value < 0.0;
    let rendered = format!("{:.*e}", precision - 1, value.abs());
    let (mantissa, exponent) = rendered.split_once('e').expect("LowerExp has exponent");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let body = if exponent < -6 || exponent >= precision as i32 {
        let mut out = String::new();
        out.push_str(&digits[..1]);
        if precision > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if exponent >= 0 { '+' } else { '-' });
        out.push_str(&exponent.abs().to_string());
        out
    } else if exponent >= 0 {
        let n = exponent as usize + 1;
        let mut out = digits[..n].to_string();
        if precision > n {
            out.push('.');
            out.push_str(&digits[n..]);
        }
        out
    } else {
        let mut out = String::from("0.");
        out.extend(std::iter::repeat_n('0', (-exponent - 1) as usize));
        out.push_str(&digits);
        out
    };
    if negative { format!("-{body}") } else { body }
}

/// ECMAScript `Number.prototype.toExponential(fractionDigits)`.
pub fn js_to_exponential(value: f64, fraction_digits: usize) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if !value.is_finite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if value == 0.0 {
        return format!("0.{}e+0", "0".repeat(fraction_digits));
    }
    let negative = value < 0.0;
    let rendered = format!("{:.*e}", fraction_digits, value.abs());
    let (mantissa, exponent) = rendered.split_once('e').expect("LowerExp has exponent");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let body = format!(
        "{mantissa}e{}{}",
        if exponent >= 0 { '+' } else { '-' },
        exponent.abs()
    );
    if negative { format!("-{body}") } else { body }
}

pub(super) fn js_round_to_precision(value: f64, precision: usize) -> f64 {
    js_to_precision(value, precision).parse().unwrap_or(value)
}

/// `formatNumber` from the TS built-ins: six significant digits with the
/// exponent-notation boundary at 1e-6 and trailing-zero trimming.
pub fn format_number(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value == f64::INFINITY {
        return "inf".to_string();
    }
    if value == f64::NEG_INFINITY {
        return "-inf".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let mut rendered = js_to_precision(value, 6);
    // Math.log10 lands exactly on integers at powers of ten in the JS host;
    // snap there so the -6 boundary matches.
    let raw_exponent = value.abs().log10();
    let exponent = if (raw_exponent - raw_exponent.round()).abs() < 1e-9 {
        raw_exponent.round() as i32
    } else {
        raw_exponent.floor() as i32
    };
    if !rendered.contains('e') && exponent <= -6 {
        rendered = js_to_exponential(value, 5);
    }
    match rendered.find('e') {
        None => rendered,
        Some(at) => {
            let suffix = &rendered[at..];
            let mut significand = rendered[..at].to_string();
            if significand.contains('.') {
                while significand.ends_with('0') {
                    significand.pop();
                }
                if significand.ends_with('.') {
                    significand.pop();
                }
            }
            format!("{significand}{suffix}")
        }
    }
}

/// `JSON.stringify` for strings (the escapes JSON emits for control chars).
pub fn json_stringify(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `formatOpenScadValue` (echo/assert formatting; strings are JSON-quoted).
pub fn format_value(value: &Value) -> String {
    match value {
        Value::Undef => "undef".to_string(),
        Value::Str(s) => json_stringify(s),
        Value::Bool(v) => if *v { "true" } else { "false" }.to_string(),
        Value::Number(v) => {
            if v.is_nan() {
                "nan".to_string()
            } else if *v == f64::INFINITY {
                "inf".to_string()
            } else if *v == f64::NEG_INFINITY {
                "-inf".to_string()
            } else if *v == 0.0 {
                "0".to_string()
            } else {
                js_number_to_string(js_round_to_precision(*v, 6))
            }
        }
        Value::Vector(items) => format!(
            "[{}]",
            items
                .iter()
                .map(format_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Range { start, step, end } => format!(
            "[{} : {} : {}]",
            format_value(&Value::Number(*start)),
            format_value(&Value::Number(*step)),
            format_value(&Value::Number(*end))
        ),
        Value::Function(_) => "function(...)".to_string(),
    }
}

/// `compactDiagnosticText` from the TS evaluator.
pub fn compact_diagnostic_text(value: &str, limit: usize) -> String {
    let mut compact = String::new();
    let mut pending_space = false;
    for ch in value.trim().chars() {
        if ch.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space && !compact.is_empty() {
                compact.push(' ');
            }
            pending_space = false;
            compact.push(ch);
        }
    }
    if compact.chars().count() <= limit {
        return compact;
    }
    let truncated: String = compact.chars().take(limit - 1).collect();
    format!("{truncated}…")
}
