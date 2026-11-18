//! Atomic edge/vertex tolerance unification with a versioned healing journal.
//!
//! Missing local values inherit the model tolerance. `RaiseToMax` selects
//! `max(model, every local)`; `ClampToModel` keeps the model scalar; `Custom`
//! selects an explicit common target. Every policy writes the selected target
//! to the model and every edge/vertex entry. A target above the explicit cap is
//! refused before mutation, while lowering a target is permitted.

use crate::healing_report::{
    HealingGapSummary, HealingReport, HealingRun, HealingStatus, model_hash,
};
use crate::{Model, TopoId};
use nurbs_core::{Error, Result};

pub use crate::healing_report::{ToleranceChange, ToleranceUnificationReport, UnifyPolicy};

pub const UNIFY_TOLERANCE_INVALID: &str = "BREP_HEAL_TOLERANCE_INVALID";
pub const UNIFY_TOLERANCE_CAP_EXCEEDED: &str = "BREP_HEAL_TOLERANCE_CAP_EXCEEDED";
pub const UNIFY_TOLERANCE_BUDGET_EXCEEDED: &str = "BREP_HEAL_TOLERANCE_BUDGET_EXCEEDED";

pub type UnifyRun = HealingRun;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnifyBudget {
    /// Maximum permitted common tolerance, not a cumulative geometric budget.
    pub max_tolerance_mm: f64,
    /// Maximum number of visited vertices and edges.
    pub max_entities: usize,
}

impl Default for UnifyBudget {
    fn default() -> Self {
        Self {
            max_tolerance_mm: 1e-2,
            max_entities: crate::MAX_ENTITIES,
        }
    }
}

fn fail(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn valid_tolerance(value: f64) -> bool {
    value.is_finite() && (1e-10..=1e-2).contains(&value)
}

fn effective(model: &Model, id: TopoId) -> f64 {
    model
        .1
        .local_tolerances_mm
        .get(&id)
        .copied()
        .unwrap_or(model.tolerance_mm)
}

/// Unify all edge/vertex tolerances and return the same healing artifact used
/// by auto-repair. The input model is never mutated; any error discards staging.
pub fn unify_tolerances(
    model: &Model,
    policy: UnifyPolicy,
    budget: UnifyBudget,
) -> Result<UnifyRun> {
    let entity_count = model.vertices.len() + model.edges.len();
    if entity_count > budget.max_entities {
        return Err(fail(
            UNIFY_TOLERANCE_BUDGET_EXCEEDED,
            format!(
                "Tolerance unification visits {entity_count} entities, exceeding budget {}",
                budget.max_entities
            ),
        ));
    }
    if !valid_tolerance(budget.max_tolerance_mm) {
        return Err(fail(
            UNIFY_TOLERANCE_INVALID,
            "Tolerance cap must be finite and within 1e-10..=1e-2 mm",
        ));
    }
    model.validate()?;
    let mut entities: Vec<TopoId> = model
        .1
        .vertices
        .iter()
        .chain(&model.1.edges)
        .copied()
        .collect();
    entities.sort();
    let target_mm = match policy {
        UnifyPolicy::RaiseToMax => entities.iter().fold(model.tolerance_mm, |target, &id| {
            target.max(effective(model, id))
        }),
        UnifyPolicy::ClampToModel => model.tolerance_mm,
        UnifyPolicy::Custom(target) => target,
    };
    if target_mm > budget.max_tolerance_mm {
        return Err(fail(
            UNIFY_TOLERANCE_CAP_EXCEEDED,
            format!(
                "Unified tolerance {target_mm} mm exceeds cap {} mm",
                budget.max_tolerance_mm
            ),
        ));
    }
    if !valid_tolerance(target_mm) {
        return Err(fail(
            UNIFY_TOLERANCE_INVALID,
            "Unified tolerance must be finite and within 1e-10..=1e-2 mm",
        ));
    }

    let changes: Vec<ToleranceChange> = entities
        .iter()
        .filter_map(|&entity_id| {
            let before_mm = effective(model, entity_id);
            (before_mm.to_bits() != target_mm.to_bits()).then_some(ToleranceChange {
                entity_id,
                before_mm,
                after_mm: target_mm,
            })
        })
        .collect();
    let input_hash = model_hash(model);
    let mut staged = model.clone();
    staged.tolerance_mm = target_mm;
    for id in entities {
        staged.1.local_tolerances_mm.insert(id, target_mm);
    }
    staged.validate()?;
    let output_hash = model_hash(&staged);
    Ok(HealingRun {
        model: staged,
        report: HealingReport {
            input_hash,
            output_hash,
            status: HealingStatus::Complete,
            operations: Vec::new(),
            gaps: HealingGapSummary {
                before_mm: 0.,
                after_mm: 0.,
            },
            sewn_edges: Vec::new(),
            conversions: Vec::new(),
            displacement: Vec::new(),
            certificates: Vec::new(),
            displacement_budget_mm: 0.,
            cumulative_displacement_mm: 0.,
            unresolved_defects: Vec::new(),
            refusal_reasons: Vec::new(),
            tolerance_unification: Some(ToleranceUnificationReport {
                policy,
                target_mm,
                cap_mm: budget.max_tolerance_mm,
                changes,
            }),
            stages: Vec::new(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mixed_model() -> (Model, TopoId, TopoId, TopoId) {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        model.tolerance_mm = 1e-6;
        let vertex = model.1.vertices[0];
        let raised_edge = model.1.edges[0];
        let unchanged_edge = model.1.edges[1];
        model.1.local_tolerances_mm.insert(vertex, 1e-7);
        model
            .1
            .local_tolerances_mm
            .insert(raised_edge, 1e-4);
        model
            .1
            .local_tolerances_mm
            .insert(unchanged_edge, 1e-6);
        (model, vertex, raised_edge, unchanged_edge)
    }

    fn budget(cap: f64) -> UnifyBudget {
        UnifyBudget {
            max_tolerance_mm: cap,
            ..Default::default()
        }
    }

    #[test]
    fn raise_to_max_unifies_every_entity_and_round_trips_journal() {
        let (model, vertex, raised_edge, normalized_edge) = mixed_model();
        let run = unify_tolerances(&model, UnifyPolicy::RaiseToMax, budget(1e-3)).unwrap();
        assert_eq!(run.model.tolerance_mm, 1e-4);
        assert!(run.model.1.vertices.iter().chain(&run.model.1.edges).all(|id| {
            run.model.1.local_tolerances_mm.get(id) == Some(&1e-4)
        }));
        let journal = run.report.tolerance_unification.as_ref().unwrap();
        assert_eq!(journal.policy, UnifyPolicy::RaiseToMax);
        assert_eq!(journal.target_mm, 1e-4);
        assert!(!journal.changes.iter().any(|change| change.entity_id == raised_edge));
        let normalized_change = journal
            .changes
            .iter()
            .find(|change| change.entity_id == normalized_edge)
            .unwrap();
        assert_eq!(normalized_change.before_mm, 1e-6);
        assert_eq!(normalized_change.after_mm, 1e-4);
        let vertex_change = journal
            .changes
            .iter()
            .find(|change| change.entity_id == vertex)
            .unwrap();
        assert_eq!(vertex_change.before_mm, 1e-7);
        assert_eq!(vertex_change.after_mm, 1e-4);
        assert!(journal.changes.iter().all(|change| change.after_mm == 1e-4));
        assert!(journal.changes.windows(2).all(|pair| pair[0].entity_id < pair[1].entity_id));

        let json = run.report.to_json().unwrap();
        assert!(json.contains(&format!(
            "\"version\":{}",
            crate::healing_report::HEALING_REPORT_VERSION
        )));
        assert_eq!(HealingReport::from_json(&json).unwrap(), run.report);
        let encoded = value_codec::to_string(&run.model).unwrap();
        let decoded: Model = value_codec::from_str_strict(&encoded).unwrap();
        assert_eq!(decoded.1.local_tolerances_mm, run.model.1.local_tolerances_mm);
    }

    #[test]
    fn repeated_unification_is_bit_identical_with_empty_journal() {
        let (model, ..) = mixed_model();
        let first = unify_tolerances(&model, UnifyPolicy::RaiseToMax, budget(1e-3)).unwrap();
        let before = first.model.clone();
        let second = unify_tolerances(&first.model, UnifyPolicy::RaiseToMax, budget(1e-3)).unwrap();
        assert_eq!(second.model, before);
        assert_eq!(second.report.input_hash, second.report.output_hash);
        assert!(
            second
                .report
                .tolerance_unification
                .unwrap()
                .changes
                .is_empty()
        );
    }

    #[test]
    fn cap_and_work_budget_refusals_leave_model_untouched() {
        let (model, ..) = mixed_model();
        let before = model.clone();
        let error = unify_tolerances(&model, UnifyPolicy::RaiseToMax, budget(1e-5)).unwrap_err();
        assert_eq!(error.code, UNIFY_TOLERANCE_CAP_EXCEEDED);
        assert_eq!(model, before);
        let error = unify_tolerances(
            &model,
            UnifyPolicy::ClampToModel,
            UnifyBudget {
                max_tolerance_mm: 1e-3,
                max_entities: 1,
            },
        )
        .unwrap_err();
        assert_eq!(error.code, UNIFY_TOLERANCE_BUDGET_EXCEEDED);
        assert_eq!(model, before);
    }

    #[test]
    fn clamp_and_custom_lower_targets_without_cap_refusal() {
        let (model, ..) = mixed_model();
        let clamped = unify_tolerances(&model, UnifyPolicy::ClampToModel, budget(1e-3)).unwrap();
        assert_eq!(clamped.model.tolerance_mm, 1e-6);
        assert!(
            clamped
                .model
                .1
                .local_tolerances_mm
                .values()
                .all(|tolerance| *tolerance == 1e-6)
        );
        let custom = unify_tolerances(&model, UnifyPolicy::Custom(1e-7), budget(1e-3)).unwrap();
        assert_eq!(custom.model.tolerance_mm, 1e-7);
        assert!(
            custom
                .model
                .1
                .local_tolerances_mm
                .values()
                .all(|tolerance| *tolerance == 1e-7)
        );
        custom.model.validate().unwrap();
    }
}
