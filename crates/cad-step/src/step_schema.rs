//! STEP application-protocol preflight.
//!
//! This module deliberately reads only the `FILE_SCHEMA` header value.  It does
//! not build an entity graph, so callers can select semantics before parsing the
//! DATA section.  Unknown protocols retain the conservative AP203 geometry
//! route; they never opt in to AP214/AP242-specific semantics by accident.

use crate::step_part21::{Parser, Tok, Value, lex};
use nurbs_core::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StepProtocol {
    Ap203,
    Ap214,
    Ap242,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepGeometryRoute {
    Ap203,
    Ap214,
    Ap242Mbe,
    /// Geometry is read with the conservative AP203 subset.  No AP203
    /// configuration/effectivity semantics are inferred for an unknown file.
    Ap203GeometryFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepSchemaDiagnosticCode {
    MissingFileSchema,
    MalformedFileSchema,
    MultipleSchemas,
    UnknownSchema,
    PossibleTruncatedSchema,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepDiagnosticSeverity {
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepSchemaDiagnostic {
    pub code: StepSchemaDiagnosticCode,
    pub severity: StepDiagnosticSeverity,
    pub schema: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepSchemaReport {
    pub declared_schemas: Vec<String>,
    /// Recognized protocols, de-duplicated in declaration order.
    pub protocols: Vec<StepProtocol>,
    pub primary_protocol: StepProtocol,
    pub geometry_route: StepGeometryRoute,
    pub diagnostics: Vec<StepSchemaDiagnostic>,
}

impl StepSchemaReport {
    pub fn supports(&self, protocol: StepProtocol) -> bool {
        self.protocols.contains(&protocol)
    }

    pub fn has_diagnostic(&self, code: StepSchemaDiagnosticCode) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code)
    }
}

fn warning(
    code: StepSchemaDiagnosticCode,
    schema: Option<String>,
    message: impl Into<String>,
) -> StepSchemaDiagnostic {
    StepSchemaDiagnostic {
        code,
        severity: StepDiagnosticSeverity::Warning,
        schema,
        message: message.into(),
    }
}

/// Long-form Part 21 schema identifiers are sometimes cut at a column limit.
/// Recognize stable semantic prefixes rather than requiring the full suffix.
fn classify_schema(schema: &str) -> StepProtocol {
    let normalized = schema.trim().to_ascii_uppercase();
    if normalized.starts_with("CONFIG_CONTROL_DESIGN")
        || normalized.starts_with("CONFIG_CONTROL_DESIGN_MIM_LF")
        || normalized.starts_with("AP203")
    {
        StepProtocol::Ap203
    } else if normalized.starts_with("AUTOMOTIVE_DESIGN")
        || normalized.starts_with("AUTOMOTIVE_DESIGN_CC2")
        || normalized.starts_with("AP214")
    {
        StepProtocol::Ap214
    } else if normalized.starts_with("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING")
        || normalized.starts_with("AP242_MANAGED_MODEL_BASED_3D_ENGINEER")
        || normalized.starts_with("AP242_")
    {
        StepProtocol::Ap242
    } else {
        StepProtocol::Unknown
    }
}

fn schema_strings(value: &Value) -> Option<Vec<String>> {
    let Value::Call(name, args) = value else {
        return None;
    };
    if name != "FILE_SCHEMA" || args.len() != 1 {
        return None;
    }
    let Value::List(values) = &args[0] else {
        return None;
    };
    values
        .iter()
        .map(|value| match value {
            Value::String(schema) => Some(schema.clone()),
            _ => None,
        })
        .collect()
}

/// Inspect `FILE_SCHEMA` before parsing any DATA entity.
///
/// Lexing is necessarily over the complete Part 21 payload, but only the header
/// value is parsed here.  Entity values and references remain untouched.
pub fn inspect_step_schema(text: &str) -> Result<StepSchemaReport> {
    let tokens = lex(text)?;
    let mut parsed_schema = None;
    let mut malformed = false;
    for at in 0..tokens.len() {
        if tokens[at] == Tok::Semi {
            // FILE_SCHEMA belongs to the first (HEADER) section.  Looking past
            // DATA would allow an entity named FILE_SCHEMA to affect routing.
            if at > 0 && tokens[..at].contains(&Tok::Ident("ENDSEC".into())) {
                break;
            }
        }
        if tokens[at] != Tok::Ident("FILE_SCHEMA".into()) {
            continue;
        }
        if tokens.get(at + 1) != Some(&Tok::LParen) {
            malformed = true;
            break;
        }
        let mut parser = Parser {
            tokens: tokens.clone(),
            at,
        };
        match parser.value(0) {
            Ok(value) => {
                parsed_schema = schema_strings(&value);
                malformed = parsed_schema.is_none();
            }
            Err(_) => malformed = true,
        }
        break;
    }

    let mut diagnostics = Vec::new();
    if malformed {
        diagnostics.push(warning(
            StepSchemaDiagnosticCode::MalformedFileSchema,
            None,
            "FILE_SCHEMA is malformed; using conservative AP203 geometry fallback",
        ));
    }
    let declared_schemas = parsed_schema.unwrap_or_default();
    if declared_schemas.is_empty() && !malformed {
        diagnostics.push(warning(
            StepSchemaDiagnosticCode::MissingFileSchema,
            None,
            "FILE_SCHEMA is absent or empty; using conservative AP203 geometry fallback",
        ));
    }
    if declared_schemas.len() > 1 {
        diagnostics.push(warning(
            StepSchemaDiagnosticCode::MultipleSchemas,
            None,
            format!(
                "FILE_SCHEMA declares {} schemas; semantic branches must be combined explicitly",
                declared_schemas.len()
            ),
        ));
    }

    let mut protocols = Vec::new();
    for schema in &declared_schemas {
        let protocol = classify_schema(schema);
        if protocol == StepProtocol::Unknown {
            diagnostics.push(warning(
                StepSchemaDiagnosticCode::UnknownSchema,
                Some(schema.clone()),
                format!(
                    "Unknown STEP schema '{schema}'; AP203 geometry fallback remains available"
                ),
            ));
        } else if !protocols.contains(&protocol) {
            protocols.push(protocol);
        }
        // ISO 10303-21 exchange implementations historically wrapped/cut long
        // identifiers near 72 columns.  This is a provenance warning, not a
        // claim that every long identifier is truncated.
        if schema.len() >= 64 {
            diagnostics.push(warning(
                StepSchemaDiagnosticCode::PossibleTruncatedSchema,
                Some(schema.clone()),
                "Long schema identifier may be truncated; routing used its stable prefix",
            ));
        }
    }

    let primary_protocol = protocols.first().copied().unwrap_or(StepProtocol::Unknown);
    let geometry_route = match primary_protocol {
        StepProtocol::Ap203 => StepGeometryRoute::Ap203,
        StepProtocol::Ap214 => StepGeometryRoute::Ap214,
        StepProtocol::Ap242 => StepGeometryRoute::Ap242Mbe,
        StepProtocol::Unknown => StepGeometryRoute::Ap203GeometryFallback,
    };
    Ok(StepSchemaReport {
        declared_schemas,
        protocols,
        primary_protocol,
        geometry_route,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(schemas: &str) -> String {
        format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('x'),'2;1');\nFILE_SCHEMA({schemas});\nENDSEC;\nDATA;\n#1=FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));\nENDSEC;\nEND-ISO-10303-21;\n"
        )
    }

    #[test]
    fn detects_all_protocols_before_data_entities() {
        for (schema, protocol) in [
            ("CONFIG_CONTROL_DESIGN", StepProtocol::Ap203),
            ("AUTOMOTIVE_DESIGN", StepProtocol::Ap214),
            (
                "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF",
                StepProtocol::Ap242,
            ),
        ] {
            let report = inspect_step_schema(&header(&format!("('{schema}')"))).unwrap();
            assert_eq!(report.primary_protocol, protocol);
            assert!(report.supports(protocol));
            assert_eq!(report.declared_schemas, vec![schema]);
        }
    }

    #[test]
    fn multi_schema_is_explicit_and_unknown_falls_back_safely() {
        let report = inspect_step_schema(&header(
            "('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING')",
        ))
        .unwrap();
        assert!(report.has_diagnostic(StepSchemaDiagnosticCode::MultipleSchemas));
        assert!(report.supports(StepProtocol::Ap214));
        assert!(report.supports(StepProtocol::Ap242));

        let report = inspect_step_schema(&header("('VENDOR_PRIVATE_SCHEMA')")).unwrap();
        assert_eq!(report.primary_protocol, StepProtocol::Unknown);
        assert_eq!(
            report.geometry_route,
            StepGeometryRoute::Ap203GeometryFallback
        );
        assert!(report.has_diagnostic(StepSchemaDiagnosticCode::UnknownSchema));
    }

    #[test]
    fn truncated_ap242_prefix_and_missing_schema_do_not_panic() {
        let report = inspect_step_schema(&header(
            "('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF_WITH_A_VERY_LONG_SUFFIX_123456')",
        ))
        .unwrap();
        assert_eq!(report.primary_protocol, StepProtocol::Ap242);
        assert!(report.has_diagnostic(StepSchemaDiagnosticCode::PossibleTruncatedSchema));

        let missing = "ISO-10303-21;HEADER;ENDSEC;DATA;#1=CARTESIAN_POINT('',(0.,0.,0.));ENDSEC;END-ISO-10303-21;";
        let report = inspect_step_schema(missing).unwrap();
        assert_eq!(
            report.geometry_route,
            StepGeometryRoute::Ap203GeometryFallback
        );
        assert!(report.has_diagnostic(StepSchemaDiagnosticCode::MissingFileSchema));
    }
}
