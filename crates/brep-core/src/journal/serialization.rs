//! Journal wire schema only; native capture and replay remain in the parent.
use super::*;
use crate::TopoKind;
use crate::analysis::surface_classify::SurfaceClass;
use crate::feature_history::{
    EdgeSignature, FaceSignature, HistoryOperation, TopologyBinding, TopologySignature,
};
use value_codec::{Deserialize, Serialize, Value, json};
fn decode<T: for<'a> Deserialize<'a>>(value: &Value, field: &str) -> value_codec::Result<T> {
    value_codec::from_value(value[field].clone())
        .map_err(|error| value_codec::error(format!("Invalid or missing {field}: {error}")))
}

impl Serialize for NumericContract {
    fn to_value(&self) -> Value {
        json!({
            "version": self.version,
            "kernelVersion": self.kernel_version,
            "modelToleranceMm": self.model_tolerance_mm,
            "surfaceFitToleranceMm": self.surface_fit_tolerance_mm,
            "directEditLinearMultiplier": self.direct_edit_linear_multiplier,
            "surfaceClassifySamples": self.surface_classify_samples,
            "faceAreaRelativeTolerance": self.face_area_relative_tolerance,
            "faceAreaMaxDivisions": self.face_area_max_divisions,
            "rebind": self.rebind_policy,
            "guardMaxIterations": self.guard_max_iterations,
            "guardMaxDepth": self.guard_max_depth
        })
    }
}

impl<'de> Deserialize<'de> for NumericContract {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let contract = Self {
            version: decode(&value, "version")?,
            kernel_version: decode(&value, "kernelVersion")?,
            model_tolerance_mm: decode(&value, "modelToleranceMm")?,
            surface_fit_tolerance_mm: decode(&value, "surfaceFitToleranceMm")?,
            direct_edit_linear_multiplier: decode(&value, "directEditLinearMultiplier")?,
            surface_classify_samples: decode(&value, "surfaceClassifySamples")?,
            face_area_relative_tolerance: decode(&value, "faceAreaRelativeTolerance")?,
            face_area_max_divisions: decode(&value, "faceAreaMaxDivisions")?,
            rebind_policy: decode(&value, "rebind")?,
            guard_max_iterations: decode(&value, "guardMaxIterations")?,
            guard_max_depth: decode(&value, "guardMaxDepth")?,
        };
        if contract.version != NUMERIC_CONTRACT_VERSION {
            return Err(value_codec::error(format!(
                "Unsupported numeric contract version {}",
                contract.version
            )));
        }
        Ok(contract)
    }
}

impl Serialize for RebindPolicy {
    fn to_value(&self) -> Value {
        json!({
            "linearToleranceMm": self.linear_tolerance_mm,
            "areaRelativeTolerance": self.area_relative_tolerance,
            "angleToleranceRad": self.angle_tolerance_rad
        })
    }
}

impl<'de> Deserialize<'de> for RebindPolicy {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        Ok(Self {
            linear_tolerance_mm: decode(&value, "linearToleranceMm")?,
            area_relative_tolerance: decode(&value, "areaRelativeTolerance")?,
            angle_tolerance_rad: decode(&value, "angleToleranceRad")?,
        })
    }
}

fn surface_class_name(class: SurfaceClass) -> &'static str {
    match class {
        SurfaceClass::Plane => "plane",
        SurfaceClass::Cylinder => "cylinder",
        SurfaceClass::Cone => "cone",
        SurfaceClass::Sphere => "sphere",
        SurfaceClass::Torus => "torus",
        SurfaceClass::Freeform => "freeform",
    }
}

fn parse_surface_class(value: &Value) -> value_codec::Result<SurfaceClass> {
    match value.as_str() {
        Some("plane") => Ok(SurfaceClass::Plane),
        Some("cylinder") => Ok(SurfaceClass::Cylinder),
        Some("cone") => Ok(SurfaceClass::Cone),
        Some("sphere") => Ok(SurfaceClass::Sphere),
        Some("torus") => Ok(SurfaceClass::Torus),
        Some("freeform") => Ok(SurfaceClass::Freeform),
        _ => Err(value_codec::error("Unknown surface class")),
    }
}

impl Serialize for FaceSignature {
    fn to_value(&self) -> Value {
        json!({
            "class": surface_class_name(self.class),
            "normalOrAxis": self.normal_or_axis,
            "areaMm2": self.area_mm2,
            "centroid": self.centroid
        })
    }
}

impl<'de> Deserialize<'de> for FaceSignature {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        Ok(Self {
            class: parse_surface_class(&value["class"])?,
            normal_or_axis: decode(&value, "normalOrAxis")?,
            area_mm2: decode(&value, "areaMm2")?,
            centroid: decode(&value, "centroid")?,
        })
    }
}

impl Serialize for EdgeSignature {
    fn to_value(&self) -> Value {
        json!({
            "degree": self.degree,
            "direction": self.direction,
            "midpoint": self.midpoint,
            "lengthMm": self.length_mm
        })
    }
}

impl<'de> Deserialize<'de> for EdgeSignature {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        Ok(Self {
            degree: decode(&value, "degree")?,
            direction: decode(&value, "direction")?,
            midpoint: decode(&value, "midpoint")?,
            length_mm: decode(&value, "lengthMm")?,
        })
    }
}

impl Serialize for TopologySignature {
    fn to_value(&self) -> Value {
        match self {
            Self::Face(face) => json!({"kind": "face", "face": face}),
            Self::Edge(edge) => json!({"kind": "edge", "edge": edge}),
        }
    }
}

impl<'de> Deserialize<'de> for TopologySignature {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        match value["kind"].as_str() {
            Some("face") => Ok(Self::Face(decode(&value, "face")?)),
            Some("edge") => Ok(Self::Edge(decode(&value, "edge")?)),
            _ => Err(value_codec::error("Unknown topology signature kind")),
        }
    }
}

impl Serialize for TopologyBinding {
    fn to_value(&self) -> Value {
        json!({"id": self.id, "producer": self.producer, "signature": self.signature})
    }
}

impl<'de> Deserialize<'de> for TopologyBinding {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let binding = Self {
            id: decode(&value, "id")?,
            producer: decode(&value, "producer")?,
            signature: decode(&value, "signature")?,
        };
        let matches = matches!(
            (binding.id.kind(), &binding.signature),
            (TopoKind::Face, TopologySignature::Face(_))
                | (TopoKind::Edge, TopologySignature::Edge(_))
        );
        if !matches {
            return Err(value_codec::error(
                "Topology binding/signature kind mismatch",
            ));
        }
        Ok(binding)
    }
}

impl Serialize for HistoryOperation {
    fn to_value(&self) -> Value {
        match self {
            Self::Block { min, max } => json!({"kind": "block", "min": min, "max": max}),
            Self::Fillet { edge, radius_mm } => {
                json!({"kind": "fillet", "edge": edge, "radiusMm": radius_mm})
            }
            Self::MoveFace {
                face,
                direction,
                distance_mm,
            } => json!({
                "kind": "moveFace",
                "face": face,
                "direction": direction,
                "distanceMm": distance_mm
            }),
            Self::DeleteFace { face } => json!({"kind": "deleteFace", "face": face}),
        }
    }
}

impl<'de> Deserialize<'de> for HistoryOperation {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        match value["kind"].as_str() {
            Some("block") => Ok(Self::Block {
                min: decode(&value, "min")?,
                max: decode(&value, "max")?,
            }),
            Some("fillet") => Ok(Self::Fillet {
                edge: decode(&value, "edge")?,
                radius_mm: decode(&value, "radiusMm")?,
            }),
            Some("moveFace") => Ok(Self::MoveFace {
                face: decode(&value, "face")?,
                direction: decode(&value, "direction")?,
                distance_mm: decode(&value, "distanceMm")?,
            }),
            Some("deleteFace") => Ok(Self::DeleteFace {
                face: decode(&value, "face")?,
            }),
            _ => Err(value_codec::error("Unknown history operation kind")),
        }
    }
}

impl Serialize for HistoryNode {
    fn to_value(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "dependsOn": self.depends_on.iter().cloned().collect::<Vec<_>>(),
            "operation": self.operation,
            "suppressed": self.suppressed
        })
    }
}

impl<'de> Deserialize<'de> for HistoryNode {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        Ok(Self {
            id: decode(&value, "id")?,
            name: decode(&value, "name")?,
            depends_on: decode::<Vec<String>>(&value, "dependsOn")?
                .into_iter()
                .collect(),
            operation: decode(&value, "operation")?,
            // Version-1 journals written before history-tree suppression did
            // not contain this field; their nodes remain active on replay.
            suppressed: decode::<Option<bool>>(&value, "suppressed")?.unwrap_or(false),
        })
    }
}

fn encode_line(value: &Value) -> Result<String> {
    value_codec::to_string(value).map_err(|codec_error| {
        error(
            JOURNAL_CODEC,
            format!("Cannot encode journal record: {codec_error}"),
        )
    })
}

fn decode_line(line: &str, line_number: usize) -> Result<Value> {
    value_codec::from_str(line).map_err(|error| codec(line_number, error))
}

fn codec(line_number: usize, error: impl std::fmt::Display) -> Error {
    crate::Error::new(
        JOURNAL_CODEC,
        format!("Journal line {line_number}: {error}"),
    )
}

fn parse_hash(text: &str) -> value_codec::Result<[u8; 32]> {
    if text.len() != 64 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(value_codec::error(
            "Result hash must contain 64 hexadecimal digits",
        ));
    }
    let mut hash = [0u8; 32];
    for (index, byte) in hash.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .map_err(|_| value_codec::error("Invalid result hash"))?;
    }
    Ok(hash)
}

impl OperationJournal {
    /// Deterministic JSON Lines: a header, exactly one operation per line, and
    /// an optional result. One edited feature therefore produces a local diff.
    pub fn to_text(&self) -> Result<String> {
        let mut lines = Vec::with_capacity(self.operations.len() + 2);
        lines.push(encode_line(&json!({
            "record": "header",
            "schema": JOURNAL_SCHEMA,
            "version": JOURNAL_VERSION,
            "numericContract": self.numeric_contract
        }))?);
        for operation in &self.operations {
            lines.push(encode_line(
                &json!({"record": "operation", "node": operation}),
            )?);
        }
        if let Some(hash) = self.expected_result_hash {
            lines.push(encode_line(
                &json!({"record": "result", "resultHash": hash_hex(&hash)}),
            )?);
        }
        lines.push(String::new());
        Ok(lines.join("\n"))
    }

    pub fn from_text(text: &str) -> Result<Self> {
        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty());
        let (line_number, header) = lines
            .next()
            .ok_or_else(|| error(JOURNAL_CODEC, "Journal is empty; expected a header record"))?;
        let header: Value = decode_line(header, line_number + 1)?;
        if header["record"].as_str() != Some("header")
            || header["schema"].as_str() != Some(JOURNAL_SCHEMA)
            || header["version"].as_u64() != Some(JOURNAL_VERSION as u64)
        {
            return Err(error(
                JOURNAL_CODEC,
                format!("Line {}: unsupported journal header", line_number + 1),
            ));
        }
        let numeric_contract = decode::<NumericContract>(&header, "numericContract")
            .map_err(|error| codec(line_number + 1, error))?;
        let mut journal = Self::new(numeric_contract);
        for (line_number, line) in lines {
            let value: Value = decode_line(line, line_number + 1)?;
            match value["record"].as_str() {
                Some("operation") if journal.expected_result_hash.is_none() => {
                    let node =
                        decode(&value, "node").map_err(|error| codec(line_number + 1, error))?;
                    journal.operations.push(node);
                }
                Some("result") if journal.expected_result_hash.is_none() => {
                    let hash: String = decode(&value, "resultHash")
                        .map_err(|error| codec(line_number + 1, error))?;
                    journal.expected_result_hash =
                        Some(parse_hash(&hash).map_err(|error| codec(line_number + 1, error))?);
                }
                _ => {
                    return Err(error(
                        JOURNAL_CODEC,
                        format!(
                            "Line {}: unexpected or out-of-order record",
                            line_number + 1
                        ),
                    ));
                }
            }
        }
        Ok(journal)
    }
}
