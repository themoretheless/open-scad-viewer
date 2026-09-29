//! Numeric editor input, normalized to millimetres or degrees before preview.
use super::{field, input, json, Result, Value};

pub fn parse(v: Value) -> Result<Value> {
    let text: String = field(&v, "text")?;
    let kind: String = field(&v, "kind")?;
    if !["length", "angle", "scalar"].contains(&kind.as_str()) {
        return Err(input("Unknown quantity kind."));
    }
    let invalid = |reason: &str| Ok(json!({"valid":false,"reason":reason}));
    if text.len() > 128 { return invalid("number"); }
    let normalized = text.trim().to_lowercase().replace(',', ".");
    let end = normalized.find(|c: char| !c.is_ascii_digit() && !matches!(c, '.' | '+' | '-' | 'e')).unwrap_or(normalized.len());
    let Ok(number) = normalized[..end].parse::<f64>() else { return invalid("number"); };
    let unit = normalized[end..].trim();
    let factor = match (kind.as_str(), unit) {
        (_, "") | ("length", "mm") | ("angle", "deg" | "°") => 1.0,
        ("length", "cm") => 10.0,
        ("length", "m") => 1000.0,
        ("length", "in" | "\"") => 25.4,
        ("angle", "rad") => 180.0 / std::f64::consts::PI,
        _ => return invalid("unit"),
    };
    let value = number * factor;
    if !value.is_finite() { return invalid("number"); }
    if v.get("min").and_then(Value::as_f64).is_some_and(|min| value < min)
        || v.get("max").and_then(Value::as_f64).is_some_and(|max| value > max) {
        return invalid("range");
    }
    Ok(json!({"valid":true,"value":value}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_quantities() {
        for (text,kind,expected) in [("2 cm","length",20.0),("1in","length",25.4),("-1,5 mm","length",-1.5),("1e-3 m","length",1.0),("3.141592653589793 rad","angle",180.0),("90°","angle",90.0)] {
            let result=parse(json!({"text":text,"kind":kind})).unwrap();
            assert_eq!(result["valid"],true);
            assert!((result["value"].as_f64().unwrap()-expected).abs()<1e-10);
        }
        for (text,kind) in [("","length"),("1e","length"),("1e999","length"),("2 cm junk","length"),("5 deg","length"),("2mm","angle"),("2cm","scalar"),("1,2,3","length")] {
            assert_eq!(parse(json!({"text":text,"kind":kind})).unwrap()["valid"],false);
        }
        assert_eq!(parse(json!({"text":"-1mm","kind":"length","min":0})).unwrap()["reason"],"range");
    }
}
