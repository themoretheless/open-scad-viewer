//! OpenSCAD compiler front-end ABI adapter (migration stage 1: parse only).
//! Input: `{"source": string, "profile": "openscad-viewer-subset@1" | "openscad/stable-2021.01"}`.
//! Output: `{"ok": true, "ast": [...]}` or
//! `{"ok": false, "diagnostics": [{code, message, start, end, line, column}]}`.
//! Diagnostics keep the exact codes and UTF-16 positions of the TypeScript
//! parser so the language conformance corpus gates parity.
use openscad_core::{LanguageProfile, ParseError, MAX_SOURCE_LENGTH};
use value_codec::{json, Value};

pub fn scad_compile(value: &Value) -> Value {
    let Some(source) = value["source"].as_str() else {
        return input_error("Expected source string");
    };
    let profile_name = value["profile"].as_str().unwrap_or("openscad-viewer-subset@1");
    let Some(profile) = LanguageProfile::parse(profile_name) else {
        return input_error(format!("Unknown OpenSCAD language profile {profile_name}"));
    };
    // Mirrors the MAX_SOURCE_LENGTH guard in `openscadParser.ts` (positions
    // count UTF-16 code units like the TS parser).
    let units: Vec<u16> = source.encode_utf16().collect();
    if units.len() > MAX_SOURCE_LENGTH {
        let diagnostic = ParseError::new(0, "Source exceeds 250,000 characters").resolve(&units);
        return json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&diagnostic)]});
    }
    match openscad_core::compile_units(&units, profile) {
        Ok(statements) => json!({"ok": true, "ast": openscad_core::serialize::program(&statements)}),
        Err(diagnostic) => {
            json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&diagnostic)]})
        }
    }
}

fn input_error(message: impl Into<String>) -> Value {
    json!({"ok": false, "error": {"code": "GEOMETRY_INVALID_INPUT", "message": message.into()}})
}
