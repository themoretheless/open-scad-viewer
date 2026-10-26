//! Stable, serializable classification of defects admitted for healing.
//!
//! The classifiers are diagnostic: they do not mutate or pre-validate away a
//! defective model. Full-interval edge/pcurve agreement and shell-closure
//! inspection are shared with the existing auditors. Face/shell intersection
//! classes are explicitly unchecked rather than inferred from absent evidence.

use crate::{Model, TopoId};
use nurbs_core::{Error, Result, curve_surface_agreement::Status};
use std::collections::BTreeSet;
use value_codec::{Deserialize, Serialize, Value, json};

pub const DEFECT_REPORT_SCHEMA: &str = "brep-defect-report";
pub const DEFECT_REPORT_VERSION: u32 = 1;
pub const DEFECT_CLASSIFY_BUDGET: &str = "BREP_DEFECT_BUDGET_EXCEEDED";
pub const DEFECT_CLASSIFY_INVALID: &str = "BREP_DEFECT_INPUT_INVALID";
pub const DEFECT_REPORT_CODEC: &str = "BREP_DEFECT_REPORT_CODEC";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefectKind {
    GapEdges,
    BadPCurve,
    WrongOrientation,
    DegenerateEdge,
    NonManifoldEdge,
    ToleranceInconsistent,
    SelfIntersection,
    OverlapFaces,
}

impl DefectKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::GapEdges => "gapEdges",
            Self::BadPCurve => "badPCurve",
            Self::WrongOrientation => "wrongOrientation",
            Self::DegenerateEdge => "degenerateEdge",
            Self::NonManifoldEdge => "nonManifoldEdge",
            Self::ToleranceInconsistent => "toleranceInconsistent",
            Self::SelfIntersection => "selfIntersection",
            Self::OverlapFaces => "overlapFaces",
        }
    }

    fn parse(value: &Value) -> value_codec::Result<Self> {
        match value.as_str() {
            Some("gapEdges") => Ok(Self::GapEdges),
            Some("badPCurve") => Ok(Self::BadPCurve),
            Some("wrongOrientation") => Ok(Self::WrongOrientation),
            Some("degenerateEdge") => Ok(Self::DegenerateEdge),
            Some("nonManifoldEdge") => Ok(Self::NonManifoldEdge),
            Some("toleranceInconsistent") => Ok(Self::ToleranceInconsistent),
            Some("selfIntersection") => Ok(Self::SelfIntersection),
            Some("overlapFaces") => Ok(Self::OverlapFaces),
            _ => Err(value_codec::error("Unknown defect kind")),
        }
    }
}

impl Serialize for DefectKind {
    fn to_value(&self) -> Value {
        json!(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DefectKind {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        Self::parse(&value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefectSeverity {
    Error,
    Warning,
}

impl DefectSeverity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }

    fn parse(value: &Value) -> value_codec::Result<Self> {
        match value.as_str() {
            Some("error") => Ok(Self::Error),
            Some("warning") => Ok(Self::Warning),
            _ => Err(value_codec::error("Unknown defect severity")),
        }
    }
}

impl Serialize for DefectSeverity {
    fn to_value(&self) -> Value {
        json!(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DefectSeverity {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        Self::parse(&value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Defect {
    pub kind: DefectKind,
    pub entities: Vec<TopoId>,
    /// Millimetres for geometric/tolerance defects; use count for incidence defects.
    pub measured: Option<f64>,
    pub severity: DefectSeverity,
}

impl Defect {
    fn validate(&self) -> Result<()> {
        let unique = self.entities.iter().collect::<BTreeSet<_>>();
        if self.entities.is_empty()
            || unique.len() != self.entities.len()
            || !self
                .entities
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || self
                .measured
                .is_some_and(|value| !value.is_finite() || value < 0.)
        {
            return Err(Error::new(
                DEFECT_CLASSIFY_INVALID,
                "Defect entities must be sorted and unique; measurement must be finite",
            ));
        }
        Ok(())
    }
}

impl Serialize for Defect {
    fn to_value(&self) -> Value {
        json!({
            "kind": self.kind,
            "entities": self.entities,
            "measured": self.measured,
            "severity": self.severity
        })
    }
}

impl<'de> Deserialize<'de> for Defect {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(&value, &["kind", "entities", "measured", "severity"])?;
        Ok(Self {
            kind: decode(&value, "kind")?,
            entities: decode(&value, "entities")?,
            measured: decode(&value, "measured")?,
            severity: decode(&value, "severity")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefectReport {
    pub defects: Vec<Defect>,
    /// Whether every edge/pcurve use received a full-interval decision.
    pub complete: bool,
    pub cells: usize,
    /// Classes for which this iteration deliberately provides no verdict.
    pub unchecked: Vec<DefectKind>,
}

impl DefectReport {
    pub fn validate(&self) -> Result<()> {
        if self.cells > 1_000_000 || self.unchecked.iter().collect::<BTreeSet<_>>().len() != self.unchecked.len() {
            return Err(Error::new(
                DEFECT_CLASSIFY_INVALID,
                "Invalid defect report work count or unchecked classes",
            ));
        }
        for defect in &self.defects {
            defect.validate()?;
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String> {
        self.validate()?;
        value_codec::to_string(&self.to_value())
            .map_err(|error| Error::new(DEFECT_REPORT_CODEC, error.to_string()))
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let value: Value = value_codec::from_str_strict(text)
            .map_err(|error| Error::new(DEFECT_REPORT_CODEC, error.to_string()))?;
        let report = Self::from_value(value)
            .map_err(|error| Error::new(DEFECT_REPORT_CODEC, error.to_string()))?;
        report.validate()?;
        Ok(report)
    }
}

impl Serialize for DefectReport {
    fn to_value(&self) -> Value {
        json!({
            "schema": DEFECT_REPORT_SCHEMA,
            "version": DEFECT_REPORT_VERSION,
            "defects": self.defects,
            "complete": self.complete,
            "cells": self.cells,
            "unchecked": self.unchecked
        })
    }
}

impl<'de> Deserialize<'de> for DefectReport {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(
            &value,
            &["schema", "version", "defects", "complete", "cells", "unchecked"],
        )?;
        let schema: String = decode(&value, "schema")?;
        let version: u32 = decode(&value, "version")?;
        if schema != DEFECT_REPORT_SCHEMA || version != DEFECT_REPORT_VERSION {
            return Err(value_codec::error("Unsupported defect report schema/version"));
        }
        Ok(Self {
            defects: decode(&value, "defects")?,
            complete: decode(&value, "complete")?,
            cells: decode(&value, "cells")?,
            unchecked: decode(&value, "unchecked")?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefectBudget {
    /// Shared full-interval same-parameter work limit.
    pub max_cells: usize,
    /// Maximum number of emitted defects; clean reports may use zero.
    pub max_defects: usize,
}

impl Default for DefectBudget {
    fn default() -> Self {
        Self {
            max_cells: 100_000,
            max_defects: 4096,
        }
    }
}

fn decode<T: for<'a> Deserialize<'a>>(value: &Value, field: &str) -> value_codec::Result<T> {
    let field_value = value
        .as_object()
        .and_then(|object| object.get(field))
        .ok_or_else(|| value_codec::error(format!("Missing field {field}")))?;
    value_codec::from_value(field_value.clone())
        .map_err(|error| value_codec::error(format!("Invalid {field}: {error}")))
}

fn require_keys(value: &Value, expected: &[&str]) -> value_codec::Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| value_codec::error("Expected object"))?;
    if object.len() != expected.len() || expected.iter().any(|field| !object.contains_key(*field)) {
        return Err(value_codec::error("Unknown or missing defect report fields"));
    }
    Ok(())
}

fn make_defect(
    kind: DefectKind,
    entities: impl IntoIterator<Item = TopoId>,
    measured: Option<f64>,
    severity: DefectSeverity,
) -> Defect {
    Defect {
        kind,
        entities: entities.into_iter().collect::<BTreeSet<_>>().into_iter().collect(),
        measured,
        severity,
    }
}

fn push(
    defects: &mut Vec<Defect>,
    defect: Defect,
    budget: DefectBudget,
) -> Result<()> {
    if defects.len() >= budget.max_defects {
        return Err(Error::new(
            DEFECT_CLASSIFY_BUDGET,
            format!("Defect count exceeds budget {}", budget.max_defects),
        ));
    }
    defects.push(defect);
    Ok(())
}

/// Classify defects without changing the input model. Unsupported intersection
/// detection is reported only in `unchecked`, never as a false clean verdict.
pub fn classify_defects(model: &Model, budget: DefectBudget) -> Result<DefectReport> {
    if budget.max_cells == 0 || budget.max_cells > 1_000_000 {
        return Err(Error::new(
            DEFECT_CLASSIFY_BUDGET,
            "Defect classification cells must be in 1..1000000",
        ));
    }
    if !model.tolerance_mm.is_finite() || model.tolerance_mm <= 0. {
        return Err(Error::new(
            DEFECT_CLASSIFY_INVALID,
            "Model tolerance must be finite and positive",
        ));
    }
    let mut defects = Vec::new();

    // Existing curve evaluation and distance primitives measure endpoint gaps
    // even when ordinary model admission would reject those same gaps.
    for (edge_index, edge) in model.edges.iter().enumerate() {
        let mut entities = vec![model.1.edges[edge_index]];
        let mut maximum = 0_f64;
        for (end, &vertex_index) in edge.vertices.iter().enumerate() {
            let measured = crate::distance(
                &crate::curve_point(&edge.curve, end as f64)?,
                &model.vertices[vertex_index].point,
            );
            if measured > model.tolerance_mm {
                maximum = maximum.max(measured);
                entities.push(model.1.vertices[vertex_index]);
            }
        }
        if maximum > model.tolerance_mm {
            push(
                &mut defects,
                make_defect(DefectKind::GapEdges, entities, Some(maximum), DefectSeverity::Error),
                budget,
            )?;
        }
    }

    let agreement = crate::boundary_agreement::verify_for_defects(model, budget.max_cells)?;
    for use_ in &agreement.uses {
        if use_.status == Status::Mismatch {
            push(
                &mut defects,
                make_defect(
                    DefectKind::BadPCurve,
                    [model.1.edges[use_.edge], model.1.faces[use_.face]],
                    use_.witness,
                    DefectSeverity::Error,
                ),
                budget,
            )?;
        }
    }

    for (edge, edge_model) in model.edges.iter().enumerate() {
        if edge_model.degenerate {
            push(
                &mut defects,
                make_defect(
                    DefectKind::DegenerateEdge,
                    [model.1.edges[edge]],
                    None,
                    DefectSeverity::Error,
                ),
                budget,
            )?;
        }
    }

    for (&entity, &local) in &model.1.local_tolerances_mm {
        if !local.is_finite() || local < 0. {
            return Err(Error::new(
                DEFECT_CLASSIFY_INVALID,
                "Local tolerance must be finite and nonnegative",
            ));
        }
        if local != model.tolerance_mm {
            push(
                &mut defects,
                make_defect(
                    DefectKind::ToleranceInconsistent,
                    [entity],
                    Some((local - model.tolerance_mm).abs()),
                    DefectSeverity::Warning,
                ),
                budget,
            )?;
        }
    }

    for shell in crate::watertight::inspect_shell_closures(model) {
        let shell_uses = model.shells[shell.shell]
            .faces
            .iter()
            .flat_map(|face_use| {
                let face = &model.faces[face_use.face];
                std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .flat_map(|wire| model.loops[wire].coedges.iter())
            })
            .fold(std::collections::BTreeMap::<usize, usize>::new(), |mut uses, coedge| {
                *uses.entry(coedge.edge).or_default() += 1;
                uses
            });
        for edge in shell.boundary_edges {
            push(
                &mut defects,
                make_defect(
                    DefectKind::NonManifoldEdge,
                    [model.1.edges[edge]],
                    Some(1.),
                    DefectSeverity::Error,
                ),
                budget,
            )?;
        }
        for edge in shell.incompatible_edges {
            let uses = shell_uses.get(&edge).copied().unwrap_or(0);
            let kind = if uses == 2 {
                DefectKind::WrongOrientation
            } else {
                DefectKind::NonManifoldEdge
            };
            push(
                &mut defects,
                make_defect(kind, [model.1.edges[edge]], Some(uses as f64), DefectSeverity::Error),
                budget,
            )?;
        }
    }

    Ok(DefectReport {
        defects,
        complete: agreement.complete,
        cells: agreement.cells,
        unchecked: vec![DefectKind::SelfIntersection, DefectKind::OverlapFaces],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(report: &DefectReport, kind: DefectKind) -> &Defect {
        report
            .defects
            .iter()
            .find(|defect| defect.kind == kind)
            .unwrap()
    }

    #[test]
    fn endpoint_gap_is_classified_with_measured_distance() {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        model.vertices[0].point[0] += model.tolerance_mm * 10.;
        let report = classify_defects(&model, DefectBudget::default()).unwrap();
        let defect = one(&report, DefectKind::GapEdges);
        assert!(defect.measured.unwrap() > model.tolerance_mm);
        assert!(defect.entities.contains(&model.1.edges[0]));
        assert!(defect.entities.iter().any(|id| model.1.vertices.contains(id)));
    }

    #[test]
    fn full_interval_pcurve_mismatch_is_bad_pcurve() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let wire = model.faces[0].outer;
        let coedge = &mut model.loops[wire].coedges[0];
        let a = coedge.pcurve.control_points.first().unwrap().clone();
        let b = coedge.pcurve.control_points.last().unwrap().clone();
        let axis = (0..2).find(|&axis| a[axis] == b[axis]).unwrap();
        let mut power = vec![1.];
        for j in 0..=8 {
            let root = j as f64 / 8.;
            let mut next = vec![0.; power.len() + 1];
            for (i, &value) in power.iter().enumerate() {
                next[i] -= root * value;
                next[i + 1] += value;
            }
            power = next;
        }
        fn choose(n: usize, k: usize) -> f64 {
            (0..k).fold(1., |value, i| value * (n - i) as f64 / (i + 1) as f64)
        }
        coedge.pcurve.degree = 9;
        coedge.pcurve.knots = [vec![0.; 10], vec![1.; 10]].concat();
        coedge.pcurve.weights = vec![1.; 10];
        coedge.pcurve.control_points = (0..=9)
            .map(|i| {
                let mut point: Vec<f64> = (0..2)
                    .map(|k| a[k] + (b[k] - a[k]) * i as f64 / 9.)
                    .collect();
                point[axis] += (0..=i)
                    .map(|k| power[k] * choose(i, k) / choose(9, k))
                    .sum::<f64>();
                point
            })
            .collect();
        let report = classify_defects(&model, DefectBudget::default()).unwrap();
        let defect = one(&report, DefectKind::BadPCurve);
        assert!(defect.measured.unwrap() > model.tolerance_mm);
        assert!(defect.entities.contains(&model.1.faces[0]));
    }

    #[test]
    fn degenerate_orientation_and_nonmanifold_incidence_are_distinct() {
        let mut degenerate = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        degenerate.edges[0].degenerate = true;
        assert!(one(
            &classify_defects(&degenerate, DefectBudget::default()).unwrap(),
            DefectKind::DegenerateEdge
        )
        .entities
        .contains(&degenerate.1.edges[0]));

        let mut oriented = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        oriented.shells[0].faces[0].reversed = !oriented.shells[0].faces[0].reversed;
        let report = classify_defects(&oriented, DefectBudget::default()).unwrap();
        assert_eq!(one(&report, DefectKind::WrongOrientation).measured, Some(2.));

        let mut nonmanifold = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let repeated = nonmanifold.shells[0].faces[0].clone();
        nonmanifold.shells[0].faces.push(repeated);
        let report = classify_defects(&nonmanifold, DefectBudget::default()).unwrap();
        assert_eq!(one(&report, DefectKind::NonManifoldEdge).measured, Some(3.));
    }

    #[test]
    fn tolerance_clean_model_and_json_round_trip() {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        model.1.local_tolerances_mm.insert(model.1.vertices[0], 1e-5);
        model.1.local_tolerances_mm.insert(model.1.edges[0], model.tolerance_mm);
        let report = classify_defects(&model, DefectBudget::default()).unwrap();
        assert_eq!(report.defects.len(), 1);
        assert_eq!(report.defects[0].kind, DefectKind::ToleranceInconsistent);
        assert_eq!(report.defects[0].severity, DefectSeverity::Warning);
        assert!(report.defects[0].measured.unwrap() > 0.);
        assert_eq!(
            report.unchecked,
            vec![DefectKind::SelfIntersection, DefectKind::OverlapFaces]
        );
        let json = report.to_json().unwrap();
        assert!(json.contains("\"version\":1"));
        assert_eq!(DefectReport::from_json(&json).unwrap(), report);
        assert_eq!(DefectReport::from_json(&json).unwrap().to_json().unwrap(), json);

        let clean = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let report = classify_defects(&clean, DefectBudget::default()).unwrap();
        assert!(report.defects.is_empty());
        assert!(report.complete);
    }

    #[test]
    fn cell_and_output_budgets_are_enforced() {
        let clean = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let limited = classify_defects(
            &clean,
            DefectBudget {
                max_cells: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!limited.complete);
        assert!(limited.defects.is_empty());
        assert_eq!(
            classify_defects(
                &clean,
                DefectBudget {
                    max_cells: 0,
                    ..Default::default()
                },
            )
            .unwrap_err()
            .code,
            DEFECT_CLASSIFY_BUDGET
        );

        let mut mixed = clean.clone();
        mixed.1.local_tolerances_mm.insert(mixed.1.vertices[0], 1e-5);
        assert_eq!(
            classify_defects(
                &mixed,
                DefectBudget {
                    max_defects: 0,
                    ..Default::default()
                },
            )
            .unwrap_err()
            .code,
            DEFECT_CLASSIFY_BUDGET
        );
    }
}
