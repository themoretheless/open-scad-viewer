// Copyright 2026 Open SCAD Viewer contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SVG 2 geometry properties, kept separate from non-CSS geometry attributes.
use super::{AId, EId};

pub(crate) const PROPERTIES: [AId; 10] = [
    AId::Cx,
    AId::Cy,
    AId::R,
    AId::Rx,
    AId::Ry,
    AId::X,
    AId::Y,
    AId::Width,
    AId::Height,
    AId::D,
];

pub(crate) fn initial(aid: AId) -> &'static str {
    match aid {
        AId::Width | AId::Height | AId::Rx | AId::Ry => "auto",
        AId::D => "",
        _ => "0",
    }
}

/// Whether a geometry property controls this SVG element's geometry.
/// Properties on other elements still have computed values for explicit inherit.
pub fn geometry_property_applies(name: &str, element: &str) -> bool {
    match (AId::from_str(name), EId::from_str(element)) {
        (Some(AId::Cx | AId::Cy), Some(EId::Circle | EId::Ellipse)) => true,
        (Some(AId::R), Some(EId::Circle)) => true,
        (Some(AId::Rx | AId::Ry), Some(EId::Rect | EId::Ellipse)) => true,
        (
            Some(AId::X | AId::Y | AId::Width | AId::Height),
            Some(EId::Svg | EId::Rect | EId::Image | EId::Use | EId::Symbol),
        ) => true,
        (Some(AId::D), Some(EId::Path)) => true,
        _ => false,
    }
}

/// Parse an SVG 2 CSS geometry value into its SVG attribute representation.
/// CSS-wide keywords are preserved for the contextual cascade. Invalid values
/// return None, allowing a preceding valid declaration to win.
pub fn normalize_geometry_property(name: &str, value: &str) -> Option<String> {
    let aid = AId::from_str(name)?;
    if !PROPERTIES.contains(&aid) {
        return None;
    }
    let value = value.trim();
    if ["inherit", "initial", "unset", "revert"]
        .iter()
        .any(|v| value.eq_ignore_ascii_case(v))
    {
        return Some(value.to_ascii_lowercase());
    }
    if aid == AId::D {
        if value.eq_ignore_ascii_case("none") {
            return Some(String::new());
        }
        let data = if value
            .get(..5)
            .is_some_and(|v| v.eq_ignore_ascii_case("path("))
            && value.ends_with(')')
        {
            css_string(value[5..value.len() - 1].trim())?
        } else {
            css_string(value)?
        };
        if svgtypes::PathParser::from(data.as_str()).any(|v| v.is_err()) {
            return None;
        }
        return Some(data);
    }
    if matches!(aid, AId::Width | AId::Height | AId::Rx | AId::Ry)
        && value.eq_ignore_ascii_case("auto")
    {
        return Some("auto".into());
    }
    let lower = value.to_ascii_lowercase();
    let length = if let Some(number) = lower.strip_suffix('q') {
        svgtypes::Length::new(
            number.trim().parse::<f64>().ok()? / 4.,
            svgtypes::LengthUnit::Mm,
        )
    } else {
        lower.parse::<svgtypes::Length>().ok()?
    };
    if !length.number.is_finite() {
        return None;
    }
    if matches!(aid, AId::Width | AId::Height | AId::R | AId::Rx | AId::Ry) && length.number < 0. {
        return None;
    }
    Some(if lower.ends_with('q') {
        format!("{}mm", length.number)
    } else {
        lower
    })
}

fn css_string(value: &str) -> Option<String> {
    let quote = value.chars().next()?;
    if !matches!(quote, '\'' | '"') {
        return None;
    }
    let mut chars = value[quote.len_utf8()..].chars().peekable();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == quote {
            return if chars.all(char::is_whitespace) {
                Some(out)
            } else {
                None
            };
        }
        if matches!(c, '\n' | '\r' | '\u{c}') {
            return None;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        let next = chars.next()?;
        if next == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            continue;
        }
        if matches!(next, '\n' | '\u{c}') {
            continue;
        }
        if next.is_ascii_hexdigit() {
            let mut code = next.to_digit(16)?;
            for _ in 1..6 {
                if let Some(digit) = chars.peek().and_then(|c| c.to_digit(16)) {
                    code = code * 16 + digit;
                    chars.next();
                } else {
                    break;
                }
            }
            if chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
                let whitespace = chars.next();
                if whitespace == Some('\r') && chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            out.push(
                char::from_u32(code)
                    .filter(|c| *c != '\0')
                    .unwrap_or('\u{fffd}'),
            );
        } else {
            out.push(next);
        }
    }
    None
}

/// Canonicalize the case-insensitive CSS important token, preserving all quoted
/// strings, URLs, comments and selector/attribute text byte-for-byte.
pub fn normalize_css_important(css: &str) -> String {
    let bytes = css.as_bytes();
    let mut edits = Vec::new();
    let mut i = 0;
    let mut quote = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = quote {
            if c == b'\\' {
                i += 1;
            } else if c == q {
                quote = None;
            }
        } else if c == b'\'' || c == b'"' {
            quote = Some(c);
        } else if c == b'/' && bytes.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < bytes.len() && &bytes[i..i + 2] != b"*/" {
                i += 1;
            }
            i += 1;
        } else if c == b'!' {
            let mut start = i + 1;
            loop {
                while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
                    start += 1;
                }
                if bytes.get(start..start + 2) == Some(b"/*") {
                    start += 2;
                    while start + 1 < bytes.len() && &bytes[start..start + 2] != b"*/" {
                        start += 1;
                    }
                    start += 2;
                } else {
                    break;
                }
            }
            if css
                .get(start..start + 9)
                .is_some_and(|v| v.eq_ignore_ascii_case("important"))
                && !bytes
                    .get(start + 9)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-')
            {
                edits.push(start..start + 9);
                i = start + 8;
            }
        }
        i += 1;
    }
    let mut output = String::new();
    let mut cursor = 0;
    for range in edits {
        output.push_str(&css[cursor..range.start]);
        output.push_str("important");
        cursor = range.end;
    }
    output.push_str(&css[cursor..]);
    output
}
