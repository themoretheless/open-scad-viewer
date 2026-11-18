//! Reproducible, line-oriented text journal for the minimal feature history.
//!
//! Each JSON Lines record is encoded by `value-codec`: one header, one line per
//! operation, and an optional final B-rep hash. Replay checks the recorded
//! numerical environment before execution and the complete model hash after it.
use crate::direct_edit::direct_edit_model_hash;
use crate::feature_history::{FeatureHistory, HistoryNode, HistoryRebuildReport, RebindPolicy};
use crate::{Error, Result};
use nurbs_core::foundation::guards::Budget;
#[cfg(feature = "codec")]
mod serialization;

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

fn hash_hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;
    use crate::direct_edit::face_plane_fit;
    #[cfg(feature = "codec")]
    use crate::feature_history::HistoryOperation;
    use crate::feature_history::{HistoryNode, TopologyBinding};

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

    #[cfg(feature = "codec")]
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
    #[test]
    fn typed_journal_capture_and_replay_need_no_wire_values() {
        let (history, rebuild, budget) = move_session(3);
        let journal =
            OperationJournal::capture(&history, RebindPolicy::default(), &budget, &rebuild);
        let replayed = journal.replay(RebindPolicy::default(), &budget).unwrap();
        assert_eq!(replayed.operation_count, 3);
        assert_eq!(replayed.verified_hash, rebuild.result_hash);
        assert_eq!(journal.operations.as_slice(), history.nodes());
    }
}
