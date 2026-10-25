//! Reproducible, line-oriented text journal for the minimal feature history.
//!
//! Each JSON Lines record is encoded by `value-codec`: one header, one line per
//! operation, and an optional final B-rep hash. Replay checks the recorded
//! numerical environment before execution and the complete model hash after it.
use crate::analysis::surface_classify::SurfaceClass;
use crate::direct_edit::direct_edit_model_hash;
use crate::feature_history::{
    EdgeSignature, FaceSignature, FeatureHistory, HistoryNode, HistoryOperation,
    HistoryRebuildReport, RebindPolicy, TopologyBinding, TopologySignature,
};
use crate::{Error, Result, TopoKind};
use nurbs_core::foundation::guards::Budget;
use value_codec::{Deserialize, Serialize, Value, json};

pub const JOURNAL_SCHEMA: &str = "brep-feature-journal";
pub const JOURNAL_VERSION: u32 = 1;
pub const NUMERIC_CONTRACT_VERSION: u32 = 1;
pub const JOURNAL_INVALID: &str = "BREP_JOURNAL_INVALID";
pub const JOURNAL_CODEC: &str = "BREP_JOURNAL_CODEC";
pub const JOURNAL_NUMERIC_CONTRACT_MISMATCH: &str = "BREP_JOURNAL_NUMERIC_CONTRACT_MISMATCH";
pub const JOURNAL_HASH_MISMATCH: &str = "BREP_JOURNAL_HASH_MISMATCH";

fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn decode<T: for<'a> Deserialize<'a>>(value: &Value, field: &str) -> value_codec::Result<T> {
    value_codec::from_value(value[field].clone())
        .map_err(|error| value_codec::error(format!("Invalid or missing {field}: {error}")))
}

/// Numerical choices that can change replay results. Wall-clock limits are
/// deliberately excluded: they guard resources, not geometric arithmetic.
#[derive(Clone, Debug, PartialEq)]
pub struct NumericContract {
    pub version: u32,
    pub kernel_version: String,
    pub model_tolerance_mm: f64,
    pub surface_fit_tolerance_mm: f64,
    pub direct_edit_linear_multiplier: f64,
    pub surface_classify_samples: usize,
    pub face_area_relative_tolerance: f64,
    pub face_area_max_divisions: usize,
    pub rebind_policy: RebindPolicy,
    pub guard_max_iterations: usize,
    pub guard_max_depth: usize,
}

impl NumericContract {
    /// Contract of the operations currently used by `FeatureHistory`.
    pub fn current(rebind_policy: RebindPolicy, budget: &Budget) -> Self {
        let model_tolerance_mm = 1e-7;
        Self {
            version: NUMERIC_CONTRACT_VERSION,
            kernel_version: env!("CARGO_PKG_VERSION").into(),
            model_tolerance_mm,
            surface_fit_tolerance_mm: model_tolerance_mm.max(1e-7),
            direct_edit_linear_multiplier: 8.,
            surface_classify_samples: 9,
            face_area_relative_tolerance: 1e-9,
            face_area_max_divisions: 32,
            rebind_policy,
            guard_max_iterations: budget.max_iterations,
            guard_max_depth: budget.max_depth,
        }
    }

    fn differences(&self, actual: &Self) -> Vec<String> {
        let mut differences = Vec::new();
        macro_rules! compare {
            ($field:ident, $label:literal) => {
                if self.$field != actual.$field {
                    differences.push(format!(
                        "{}: recorded {:?}, actual {:?}",
                        $label, self.$field, actual.$field
                    ));
                }
            };
        }
        compare!(version, "version");
        compare!(kernel_version, "kernelVersion");
        compare!(model_tolerance_mm, "modelToleranceMm");
        compare!(surface_fit_tolerance_mm, "surfaceFitToleranceMm");
        compare!(direct_edit_linear_multiplier, "directEditLinearMultiplier");
        compare!(surface_classify_samples, "surfaceClassifySamples");
        compare!(face_area_relative_tolerance, "faceAreaRelativeTolerance");
        compare!(face_area_max_divisions, "faceAreaMaxDivisions");
        compare!(guard_max_iterations, "guardMaxIterations");
        compare!(guard_max_depth, "guardMaxDepth");
        if self.rebind_policy != actual.rebind_policy {
            for (label, recorded, actual) in [
                (
                    "rebind.linearToleranceMm",
                    self.rebind_policy.linear_tolerance_mm,
                    actual.rebind_policy.linear_tolerance_mm,
                ),
                (
                    "rebind.areaRelativeTolerance",
                    self.rebind_policy.area_relative_tolerance,
                    actual.rebind_policy.area_relative_tolerance,
                ),
                (
                    "rebind.angleToleranceRad",
                    self.rebind_policy.angle_tolerance_rad,
                    actual.rebind_policy.angle_tolerance_rad,
                ),
            ] {
                if recorded != actual {
                    differences.push(format!("{label}: recorded {recorded:?}, actual {actual:?}"));
                }
            }
        }
        differences
    }
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

#[derive(Clone, Debug, PartialEq)]
pub struct OperationJournal {
    pub numeric_contract: NumericContract,
    pub operations: Vec<HistoryNode>,
    pub expected_result_hash: Option<[u8; 32]>,
}

impl OperationJournal {
    pub fn new(numeric_contract: NumericContract) -> Self {
        Self {
            numeric_contract,
            operations: Vec::new(),
            expected_result_hash: None,
        }
    }

    pub fn capture(
        history: &FeatureHistory,
        rebind_policy: RebindPolicy,
        budget: &Budget,
        rebuild: &HistoryRebuildReport,
    ) -> Self {
        let mut journal = Self::new(NumericContract::current(rebind_policy, budget));
        for node in history.nodes() {
            journal.record(node.clone());
        }
        journal.seal(rebuild);
        journal
    }

    /// Append one operation in session order. Dependencies remain explicit in
    /// the node, so export also preserves the DAG rather than only a flat log.
    pub fn record(&mut self, node: HistoryNode) -> &mut Self {
        self.operations.push(node);
        self.expected_result_hash = None;
        self
    }

    pub fn seal(&mut self, rebuild: &HistoryRebuildReport) -> &mut Self {
        self.expected_result_hash = Some(rebuild.result_hash);
        self
    }

    pub fn history(&self) -> FeatureHistory {
        FeatureHistory::new(self.operations.clone())
    }

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

    pub fn replay(
        &self,
        execution_policy: RebindPolicy,
        budget: &Budget,
    ) -> Result<JournalReplayReport> {
        let actual = NumericContract::current(execution_policy, budget);
        self.replay_with_contract(actual, budget)
    }

    /// Replay against an explicitly supplied execution contract. This is also
    /// the compatibility probe used to diagnose tolerance/kernel drift.
    pub fn replay_with_contract(
        &self,
        actual_contract: NumericContract,
        budget: &Budget,
    ) -> Result<JournalReplayReport> {
        let differences = self.numeric_contract.differences(&actual_contract);
        if !differences.is_empty() {
            return Err(error(
                JOURNAL_NUMERIC_CONTRACT_MISMATCH,
                format!(
                    "Journal replay refused: numeric contract drift detected ({})",
                    differences.join("; ")
                ),
            ));
        }
        let expected = self.expected_result_hash.ok_or_else(|| {
            error(
                JOURNAL_INVALID,
                "Journal has no expected result hash; seal the captured rebuild first",
            )
        })?;
        let mut history = self.history();
        let rebuild = history.rebuild_and_commit(actual_contract.rebind_policy, budget)?;
        let actual = direct_edit_model_hash(&rebuild.model);
        if actual != expected {
            return Err(error(
                JOURNAL_HASH_MISMATCH,
                format!(
                    "Journal B-rep hash mismatch after {} operations: expected {}, actual {}; numeric contract {}/{}",
                    self.operations.len(),
                    hash_hex(&expected),
                    hash_hex(&actual),
                    JOURNAL_SCHEMA,
                    self.numeric_contract.version
                ),
            ));
        }
        Ok(JournalReplayReport {
            history,
            rebuild,
            operation_count: self.operations.len(),
            expected_hash: expected,
            verified_hash: actual,
        })
    }
}

#[derive(Clone, Debug)]
pub struct JournalReplayReport {
    pub history: FeatureHistory,
    pub rebuild: HistoryRebuildReport,
    pub operation_count: usize,
    pub expected_hash: [u8; 32],
    pub verified_hash: [u8; 32],
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

fn hash_hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;
    use crate::direct_edit::face_plane_fit;
    use crate::feature_history::HistoryNode;

    fn budget() -> Budget {
        Budget::new(10_000_000, 8, 60_000).unwrap()
    }

    fn wall_binding(model: &crate::Model, budget: &Budget) -> TopologyBinding {
        let wall = (0..model.faces.len())
            .find(|face| {
                face_plane_fit(model, *face).is_ok_and(|plane| {
                    plane.normal[0].abs() > 1. - 1e-9 && plane.offset.abs() < 1e-8
                })
            })
            .unwrap();
        TopologyBinding::from_face(model, wall, "block", budget).unwrap()
    }

    fn move_session(operation_count: usize) -> (FeatureHistory, HistoryRebuildReport, Budget) {
        assert!(operation_count >= 2);
        let budget = budget();
        let block = cuboid([0.; 3], [10., 10., 10.]).unwrap();
        let binding = wall_binding(&block, &budget);
        let mut nodes = vec![HistoryNode::block(
            "block",
            "block",
            [0.; 3],
            [10., 10., 10.],
        )];
        for index in 1..operation_count {
            let upstream = if index == 1 {
                "block".to_string()
            } else {
                format!("move_{:03}", index - 1)
            };
            nodes.push(HistoryNode::move_face(
                format!("move_{index:03}"),
                format!("move_{index:03}"),
                upstream,
                binding.clone(),
                [1., 0., 0.],
                if index % 2 == 1 { 2. } else { -2. },
            ));
        }
        let mut history = FeatureHistory::new(nodes);
        let report = history
            .rebuild_and_commit(RebindPolicy::default(), &budget)
            .unwrap();
        (history, report, budget)
    }

    #[test]
    fn fifty_operation_jsonl_journal_replays_identical_brep() {
        let (history, rebuild, budget) = move_session(50);
        let journal =
            OperationJournal::capture(&history, RebindPolicy::default(), &budget, &rebuild);
        let text = journal.to_text().unwrap();
        assert_eq!(text.lines().count(), 52);
        assert!(text.contains("\"record\":\"operation\""));
        assert!(text.contains("\"kind\":\"moveFace\""));
        assert!(text.contains("\"numericContract\""));
        let first_id = match &journal.operations[1].operation {
            HistoryOperation::MoveFace { face, .. } => face.id.to_string(),
            _ => unreachable!(),
        };
        assert!(text.contains(&first_id));

        let imported = OperationJournal::from_text(&text).unwrap();
        assert_eq!(imported.to_text().unwrap(), text);
        let replayed = imported.replay(RebindPolicy::default(), &budget).unwrap();
        assert_eq!(replayed.operation_count, 50);
        assert_eq!(replayed.verified_hash, rebuild.result_hash);
        assert_eq!(
            direct_edit_model_hash(&replayed.rebuild.model),
            direct_edit_model_hash(&rebuild.model)
        );
    }

    #[test]
    fn replay_reports_numeric_contract_and_hash_drift() {
        let (history, rebuild, budget) = move_session(3);
        let mut journal =
            OperationJournal::capture(&history, RebindPolicy::default(), &budget, &rebuild);
        let mut drifted = journal.clone();
        drifted.numeric_contract.rebind_policy.linear_tolerance_mm *= 2.;
        let error = drifted
            .replay(RebindPolicy::default(), &budget)
            .unwrap_err();
        assert_eq!(error.code, JOURNAL_NUMERIC_CONTRACT_MISMATCH);
        assert!(
            error.message.contains("rebind.linearToleranceMm"),
            "{}",
            error.message
        );
        assert!(error.message.contains("recorded"), "{}", error.message);
        assert!(error.message.contains("actual"), "{}", error.message);

        journal.expected_result_hash = Some([0; 32]);
        let error = journal
            .replay(RebindPolicy::default(), &budget)
            .unwrap_err();
        assert_eq!(error.code, JOURNAL_HASH_MISMATCH);
        assert!(error.message.contains("expected"), "{}", error.message);
        assert!(error.message.contains("actual"), "{}", error.message);
        assert!(error.message.contains("3 operations"), "{}", error.message);
    }
}
