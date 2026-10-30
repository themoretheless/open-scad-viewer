//! iges interchange v2 algorithm: tokenize.
use super::*;
pub fn tokenize(text: &str, parameter_delimiter: char, record_delimiter: char) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut current = String::new();
    let bytes = text.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        let ch = text[at..]
            .chars()
            .next()
            .ok_or_else(|| refuse("Invalid IGES UTF-8"))?;
        if ch.is_ascii_digit() {
            let start = at;
            while at < bytes.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
            if at < bytes.len() && matches!(bytes[at], b'H' | b'h') {
                let count: usize = text[start..at]
                    .parse()
                    .map_err(|_| refuse("Invalid Hollerith length"))?;
                at += 1;
                if at + count > bytes.len() {
                    return Err(refuse("Truncated Hollerith string"));
                }
                current.push_str(&text[at..at + count]);
                at += count;
                continue;
            }
            current.push_str(&text[start..at]);
            continue;
        }
        at += ch.len_utf8();
        if ch == parameter_delimiter || ch == record_delimiter {
            out.push(current.trim().to_string());
            current.clear();
            if out.len() > MAX_VALUES {
                return Err(refuse("IGES aggregate value budget exceeded"));
            }
            if ch == record_delimiter {
                break;
            }
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        return Err(refuse("IGES parameter record lacks record delimiter"));
    }
    Ok(out)
}
