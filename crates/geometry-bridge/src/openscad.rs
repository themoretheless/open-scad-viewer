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
    let profile_name = value["profile"]
        .as_str()
        .unwrap_or("openscad-viewer-subset@1");
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
        Ok(statements) => {
            json!({"ok": true, "ast": openscad_core::serialize::program(&statements)})
        }
        Err(diagnostic) => {
            json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&diagnostic)]})
        }
    }
}

fn input_error(message: impl Into<String>) -> Value {
    json!({"ok": false, "error": {"code": "GEOMETRY_INVALID_INPUT", "message": message.into()}})
}

/// Stage-2 migration ABI: evaluate the source with the Rust value evaluator.
/// Geometry modules return accounted shape descriptors (`{name, dimension}`)
/// until stage 3 wires the shared cad handle store. Output:
/// `{"ok": true, "shapes": [...], "warnings": [...], "reduced": bool}` or
/// `{"ok": false, "diagnostics": [{code, message, start, end, line, column}]}`
/// (`aborted: true` marks host cancellation instead of a diagnostic).
pub fn scad_eval(value: &Value) -> Value {
    let Some(source) = value["source"].as_str() else {
        return input_error("Expected source string");
    };
    let profile_name = value["profile"]
        .as_str()
        .unwrap_or("openscad-viewer-subset@1");
    let Some(profile) = LanguageProfile::parse(profile_name) else {
        return input_error(format!("Unknown OpenSCAD language profile {profile_name}"));
    };
    let units: Vec<u16> = source.encode_utf16().collect();
    if units.len() > MAX_SOURCE_LENGTH {
        let diagnostic = ParseError::new(0, "Source exceeds 250,000 characters").resolve(&units);
        return json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&diagnostic)]});
    }
    let statements = match openscad_core::compile_units(&units, profile) {
        Ok(statements) => statements,
        Err(diagnostic) => {
            return json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&diagnostic)]})
        }
    };
    let options = openscad_core::eval::EvaluatorOptions::default();
    match openscad_core::eval::evaluate_program(&statements, &units, profile, options) {
        Ok(evaluation) => {
            let shapes: Vec<Value> = evaluation
                .shapes
                .iter()
                .map(|shape| json!({"name": shape.name, "dimension": shape.dimension}))
                .collect();
            json!({
                "ok": true,
                "shapes": shapes,
                "warnings": evaluation.warnings,
                "reduced": evaluation.reduced,
            })
        }
        Err(openscad_core::value::EvalFailure::Error(error)) => {
            json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&error.resolve(&units))]})
        }
        Err(openscad_core::value::EvalFailure::Aborted) => json!({"ok": false, "aborted": true}),
        Err(openscad_core::value::EvalFailure::ViewportRoot(_)) => {
            unreachable!("root selection is caught by the top-level loop")
        }
    }
}
