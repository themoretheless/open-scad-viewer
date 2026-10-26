//! Atomic edge-fixing pipeline: snap, same-parameter, safe degenerate removal,
//! and an explicit refusal boundary for topology-changing edge splits.

use crate::defects::{DefectBudget, DefectKind, DefectReport, classify_defects};
use crate::healing_report::{
    HealingRefusal, HealingRun, HealingStageReport, HealingStageSkip, HealingStatus, auto_repair,
    model_hash,
};
use crate::{ChangeKind, ChangeProvenance, Model, TopoKind, TopologyChange};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub const EDGE_FIX_BUDGET: &str = "BREP_EDGE_FIX_BUDGET_EXCEEDED";
pub const EDGE_FIX_AUDIT_INCOMPLETE: &str = "BREP_EDGE_FIX_AUDIT_INCOMPLETE";
pub const EDGE_FIX_INCOMPLETE: &str = "BREP_EDGE_FIX_INCOMPLETE";
pub const EDGE_FIX_UNSUPPORTED: &str = "BREP_EDGE_FIX_UNSUPPORTED";
pub const EDGE_SPLIT_REFUSED: &str = "BREP_HEAL_SPLIT_REFUSED";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeFixBudget {
    pub defects: DefectBudget,
    pub max_candidates_per_stage: usize,
}

impl Default for EdgeFixBudget {
    fn default() -> Self {
        Self {
            defects: DefectBudget::default(),
            max_candidates_per_stage: 64,
        }
    }
}

fn fail(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn classify(model: &Model, budget: EdgeFixBudget) -> Result<DefectReport> {
    let report = classify_defects(model, budget.defects)?;
    if !report.complete && report.cells >= budget.defects.max_cells {
        return Err(fail(
            EDGE_FIX_AUDIT_INCOMPLETE,
            "Edge-fix defect audit exhausted its full-interval cell budget",
        ));
    }
    Ok(report)
}

fn defect_count(report: &DefectReport, kind: DefectKind) -> usize {
    report
        .defects
        .iter()
        .filter(|defect| defect.kind == kind)
        .count()
}

fn check_candidate_count(count: usize, budget: EdgeFixBudget) -> Result<()> {
    if count > budget.max_candidates_per_stage {
        Err(fail(
            EDGE_FIX_BUDGET,
            format!("Edge-fix candidates {count} exceed stage budget {budget:?}"),
        ))
    } else {
        Ok(())
    }
}

fn metric(stage: &str) -> HealingStageReport {
    HealingStageReport {
        stage: stage.into(),
        fixed: 0,
        skipped: 0,
        remaining: 0,
        skipped_details: Vec::new(),
    }
}

fn snap_stage(model: &Model, budget: EdgeFixBudget) -> Result<(HealingRun, HealingStageReport)> {
    let initial = classify(model, budget)?;
    let gaps = defect_count(&initial, DefectKind::GapEdges);
    let operations = crate::import_policy::supported_snaps(model, &initial)?;
    check_candidate_count(operations.len(), budget)?;
    let run = auto_repair(model, &operations, None);
    let residual = classify(&run.model, budget)?;
    let mut stage = metric("snap");
    stage.fixed = usize::from(run.report.status == HealingStatus::Complete) * operations.len();
    stage.remaining = defect_count(&residual, DefectKind::GapEdges);
    if run.report.status == HealingStatus::RolledBack {
        for operation in &operations {
            if let Some(entity) = operation.target(&run.model) {
                stage.skipped_details.push(HealingStageSkip {
                    entity,
                    code: run
                        .report
                        .refusal_reasons
                        .first()
                        .map(|reason| reason.code.clone())
                        .unwrap_or_else(|| EDGE_FIX_INCOMPLETE.into()),
                    message: "Authorized endpoint snap rolled back".into(),
                    measured_mm: initial
                        .defects
                        .iter()
                        .find(|defect| defect.kind == DefectKind::GapEdges)
                        .and_then(|defect| defect.measured),
                });
            }
        }
    } else if operations.is_empty() {
        for defect in initial
            .defects
            .iter()
            .filter(|defect| defect.kind == DefectKind::GapEdges)
        {
            if let Some(entity) = defect.entities.first() {
                stage.skipped_details.push(HealingStageSkip {
                    entity: *entity,
                    code: EDGE_FIX_UNSUPPORTED.into(),
                    message: "Gap has no unique authorized endpoint snap".into(),
                    measured_mm: defect.measured,
                });
            }
        }
    }
    stage.skipped = stage.skipped_details.len();
    if gaps > 0 && operations.is_empty() && run.report.status == HealingStatus::Complete {
        stage.fixed = 0;
    }
    Ok((run, stage))
}

fn point3(point: &[f64]) -> Result<[f64; 3]> {
    if point.len() != 3 || point.iter().any(|value| !value.is_finite()) {
        return Err(fail(
            EDGE_FIX_UNSUPPORTED,
            "Same-parameter repair requires finite three-dimensional controls",
        ));
    }
    Ok([point[0], point[1], point[2]])
}

fn affine_planar_lift(surface: &Surface, edge: &Curve) -> Result<Curve> {
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.periodic_u
        || surface.periodic_v
        || surface.control_points.len() != 2
        || surface.control_points.iter().any(|row| row.len() != 2)
        || surface.weights != [[1., 1.], [1., 1.]]
    {
        return Err(fail(
            EDGE_FIX_UNSUPPORTED,
            "Same-parameter repair currently supports only affine bilinear planes",
        ));
    }
    let p00 = point3(&surface.control_points[0][0])?;
    let p10 = point3(&surface.control_points[1][0])?;
    let p01 = point3(&surface.control_points[0][1])?;
    let p11 = point3(&surface.control_points[1][1])?;
    let u = std::array::from_fn(|axis| p10[axis] - p00[axis]);
    let v = std::array::from_fn(|axis| p01[axis] - p00[axis]);
    let fourth: [f64; 3] =
        std::array::from_fn(|axis| p11[axis] - p00[axis] - u[axis] - v[axis]);
    let scale = u
        .iter()
        .chain(&v)
        .fold(0_f64, |maximum, value| maximum.max(value.abs()))
        .max(1.);
    if fourth.iter().map(|value| value * value).sum::<f64>().sqrt() > 1e-12 * scale {
        return Err(fail(
            EDGE_FIX_UNSUPPORTED,
            "Bilinear plane is not affine within the exact repair band",
        ));
    }
    let dot = |a: [f64; 3], b: [f64; 3]| {
        a.into_iter().zip(b).map(|(x, y)| x * y).sum::<f64>()
    };
    let uu = dot(u, u);
    let uv = dot(u, v);
    let vv = dot(v, v);
    let determinant = uu * vv - uv * uv;
    if !determinant.is_finite() || determinant <= f64::EPSILON * uu.max(vv).max(1.) {
        return Err(fail(
            EDGE_FIX_UNSUPPORTED,
            "Plane parameter directions are singular",
        ));
    }
    let domain_u = [surface.knots_u[1], surface.knots_u[2]];
    let domain_v = [surface.knots_v[1], surface.knots_v[2]];
    let mut control_points = Vec::with_capacity(edge.control_points.len());
    for control in &edge.control_points {
        let point = point3(control)?;
        let delta = std::array::from_fn(|axis| point[axis] - p00[axis]);
        let du = dot(delta, u);
        let dv = dot(delta, v);
        let alpha = (du * vv - dv * uv) / determinant;
        let beta = (dv * uu - du * uv) / determinant;
        if !alpha.is_finite() || !beta.is_finite() {
            return Err(fail(
                EDGE_FIX_UNSUPPORTED,
                "Cannot derive finite surface parameters",
            ));
        }
        control_points.push(vec![
            domain_u[0] + alpha * (domain_u[1] - domain_u[0]),
            domain_v[0] + beta * (domain_v[1] - domain_v[0]),
        ]);
    }
    let lifted = Curve {
        degree: edge.degree,
        knots: edge.knots.clone(),
        control_points,
        weights: edge.weights.clone(),
        periodic: edge.periodic,
    };
    lifted.validate()?;
    Ok(lifted)
}

fn bad_uses(model: &Model, report: &DefectReport) -> Vec<(usize, usize, usize, usize)> {
    let mut uses = Vec::new();
    for (face, face_model) in model.faces.iter().enumerate() {
        for &wire in std::iter::once(&face_model.outer).chain(&face_model.holes) {
            for (coedge, use_) in model.loops[wire].coedges.iter().enumerate() {
                let edge_id = model.1.edges[use_.edge];
                let face_id = model.1.faces[face];
                if report.defects.iter().any(|defect| {
                    defect.kind == DefectKind::BadPCurve
                        && defect.entities.contains(&edge_id)
                        && defect.entities.contains(&face_id)
                }) {
                    uses.push((wire, coedge, use_.edge, face));
                }
            }
        }
    }
    uses
}

fn same_parameter_stage(
    model: &Model,
    budget: EdgeFixBudget,
) -> Result<(Model, HealingStageReport)> {
    let initial = classify(model, budget)?;
    let candidates = bad_uses(model, &initial);
    check_candidate_count(candidates.len(), budget)?;
    let mut staged = model.clone();
    let mut stage = metric("sameParameter");
    for (wire, coedge, edge, face) in candidates {
        match affine_planar_lift(&model.faces[face].surface, &model.edges[edge].curve) {
            Ok(lifted) => {
                staged.loops[wire].coedges[coedge].pcurve = lifted;
                stage.fixed += 1;
            }
            Err(error) => {
                stage.skipped_details.push(HealingStageSkip {
                    entity: model.1.edges[edge],
                    code: error.code.into(),
                    message: error.message,
                    measured_mm: initial
                        .defects
                        .iter()
                        .find(|defect| {
                            defect.kind == DefectKind::BadPCurve
                                && defect.entities.contains(&model.1.edges[edge])
                                && defect.entities.contains(&model.1.faces[face])
                        })
                        .and_then(|defect| defect.measured),
                });
            }
        }
    }
    stage.skipped = stage.skipped_details.len();
    let residual = classify(&staged, budget)?;
    stage.remaining = defect_count(&residual, DefectKind::BadPCurve);
    if stage.remaining > stage.skipped {
        return Err(fail(
            EDGE_FIX_INCOMPLETE,
            "A same-parameter replacement did not pass full-interval reclassification",
        ));
    }
    Ok((staged, stage))
}

fn degenerate_candidates(model: &Model, report: &DefectReport) -> Vec<usize> {
    model
        .edges
        .iter()
        .enumerate()
        .filter(|(edge, _)| {
            report.defects.iter().any(|defect| {
                defect.kind == DefectKind::DegenerateEdge
                    && defect.entities.contains(&model.1.edges[*edge])
            })
        })
        .map(|(edge, _)| edge)
        .collect()
}

fn compact_removed_edges(model: &Model, removed: &BTreeSet<usize>) -> Result<Model> {
    let mut prepared = model.clone();
    for wire in &mut prepared.loops {
        wire.coedges.retain(|coedge| !removed.contains(&coedge.edge));
    }
    let mut remap = BTreeMap::new();
    let mut next = 0;
    for old in 0..model.edges.len() {
        if !removed.contains(&old) {
            remap.insert(old, next);
            next += 1;
        }
    }
    let mut staged = prepared.clone();
    staged.edges = prepared
        .edges
        .iter()
        .enumerate()
        .filter(|(edge, _)| !removed.contains(edge))
        .map(|(_, edge)| edge.clone())
        .collect();
    staged.1.edges = prepared
        .1
        .edges
        .iter()
        .enumerate()
        .filter(|(edge, _)| !removed.contains(edge))
        .map(|(_, id)| *id)
        .collect();
    for wire in &mut staged.loops {
        for coedge in &mut wire.coedges {
            coedge.edge = remap[&coedge.edge];
        }
    }
    for &edge in removed {
        let id = model.1.edges[edge];
        staged.1.change_set.changes.push(TopologyChange {
            kind: ChangeKind::Deleted,
            topo_kind: TopoKind::Edge,
            parents: vec![id],
            children: Vec::new(),
            provenance: ChangeProvenance {
                operation: "edge-fix-degenerate".into(),
                operand: None,
                occurrence: edge.to_string(),
            },
            role: "degenerate-removal".into(),
            anchor: None,
        });
    }
    if !removed.is_empty() {
        staged.validate()?;
    }
    Ok(staged)
}

fn degenerate_stage(model: &Model, budget: EdgeFixBudget) -> Result<(Model, HealingStageReport)> {
    let initial = classify(model, budget)?;
    let candidates = degenerate_candidates(model, &initial);
    check_candidate_count(candidates.len(), budget)?;
    let mut stage = metric("degenerateAndWireRebuild");
    let mut removed = BTreeSet::new();
    for edge in candidates {
        let uses: Vec<(usize, usize)> = model
            .loops
            .iter()
            .enumerate()
            .flat_map(|(wire, loop_)| {
                loop_
                    .coedges
                    .iter()
                    .enumerate()
                    .filter(|(_, coedge)| coedge.edge == edge)
                    .map(move |(coedge, _)| (wire, coedge))
            })
            .collect();
        let safe = model.edges[edge].vertices[0] == model.edges[edge].vertices[1]
            && uses.len() == 1
            && model.loops[uses[0].0].coedges.len() >= 3;
        if safe {
            removed.insert(edge);
            stage.fixed += 1;
        } else {
            stage.skipped_details.push(HealingStageSkip {
                entity: model.1.edges[edge],
                code: EDGE_FIX_UNSUPPORTED.into(),
                message: "Only a single-use degenerate self-loop in a wire of at least three coedges is removable".into(),
                measured_mm: None,
            });
        }
    }
    stage.skipped = stage.skipped_details.len();
    let staged = compact_removed_edges(model, &removed)?;
    let residual = classify(&staged, budget)?;
    stage.remaining = defect_count(&residual, DefectKind::DegenerateEdge);
    if stage.remaining > stage.skipped {
        return Err(fail(
            EDGE_FIX_INCOMPLETE,
            "Degenerate edge remained after the safe wire rebuild",
        ));
    }
    Ok((staged, stage))
}

/// Explicit edge-split boundary. The existing authorized kernel correctly
/// refuses split recipes until child-loop authorship is implemented.
pub fn split_edge(model: &Model, edge: usize, parameter: f64) -> Result<HealingStageReport> {
    let Some(edge_model) = model.edges.get(edge) else {
        return Err(fail(EDGE_SPLIT_REFUSED, "Split edge is out of range"));
    };
    let [lo, hi] = edge_model.curve.domain();
    if !parameter.is_finite() || parameter <= lo || parameter >= hi {
        return Err(fail(
            EDGE_SPLIT_REFUSED,
            "Split parameter must be finite and strictly interior",
        ));
    }
    Err(fail(
        EDGE_SPLIT_REFUSED,
        "EdgeSplit lacks topology-preserving child-loop authorship",
    ))
}

fn append_skip_reasons(report: &mut crate::healing_report::HealingReport) {
    let reasons: Vec<HealingRefusal> = report
        .stages
        .iter()
        .flat_map(|stage| {
            stage.skipped_details.iter().map(|skip| HealingRefusal {
                operation: None,
                code: skip.code.clone(),
                message: format!("{}: {}", stage.stage, skip.message),
            })
        })
        .collect();
    report.unresolved_defects.extend(reasons.clone());
    report.refusal_reasons.extend(reasons);
}

/// Run all stages against a private staged model. Any typed error discards the
/// entire staging chain; authorized-operation failures return the original
/// model with a rollback report.
pub fn fix_edges(model: &Model, budget: EdgeFixBudget) -> Result<HealingRun> {
    if budget.max_candidates_per_stage == 0 || budget.max_candidates_per_stage > 4096 {
        return Err(fail(
            EDGE_FIX_BUDGET,
            "Edge-fix stage candidate budget must be in 1..4096",
        ));
    }
    let (mut run, snap) = snap_stage(model, budget)?;
    run.report.stages.push(snap);
    if run.report.status == HealingStatus::RolledBack {
        append_skip_reasons(&mut run.report);
        run.report.validate()?;
        return Ok(run);
    }
    let (staged, same_parameter) = same_parameter_stage(&run.model, budget)?;
    run.model = staged;
    run.report.stages.push(same_parameter);
    let (staged, degenerate) = degenerate_stage(&run.model, budget)?;
    run.model = staged;
    run.report.stages.push(degenerate);
    run.report.stages.push(metric("edgeSplit"));
    run.report.output_hash = model_hash(&run.model);
    append_skip_reasons(&mut run.report);
    run.report.validate()?;
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::healing_report::HEALING_REPORT_VERSION;

    fn stage<'a>(run: &'a HealingRun, name: &str) -> &'a HealingStageReport {
        run.report
            .stages
            .iter()
            .find(|stage| stage.stage == name)
            .unwrap()
    }

    fn bad_pcurve_model() -> Model {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let wire = model.faces[0].outer;
        let coedge = &mut model.loops[wire].coedges[0];
        let a = coedge.pcurve.control_points.first().unwrap().clone();
        let b = coedge.pcurve.control_points.last().unwrap().clone();
        let axis = (0..2).find(|&axis| a[axis] == b[axis]).unwrap();
        let mut middle: Vec<f64> = (0..2).map(|k| (a[k] + b[k]) * 0.5).collect();
        middle[axis] += 0.1;
        coedge.pcurve = Curve::from_polyline(vec![a, middle, b]).unwrap();
        assert!(defect_count(
            &classify_defects(&model, DefectBudget::default()).unwrap(),
            DefectKind::BadPCurve
        ) > 0);
        model
    }

    fn constant_curve(point: Vec<f64>) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![point.clone(), point],
            weights: vec![1., 1.],
            periodic: false,
        }
    }

    fn removable_degenerate_model() -> Model {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let edge_count = model.edges.len();
        let wire = model.faces[0].outer;
        let first = model.loops[wire].coedges[0].clone();
        let old_edge = &model.edges[first.edge];
        let end = usize::from(first.reversed);
        let vertex = old_edge.vertices[end];
        let point = model.vertices[vertex].point.to_vec();
        let uv = first.pcurve.evaluate(0.).unwrap().point;
        model.edges.push(crate::Edge {
            degenerate: true,
            vertices: [vertex, vertex],
            curve: constant_curve(point),
        });
        model.loops[wire].coedges.insert(
            0,
            crate::Coedge::new(edge_count, false, constant_curve(uv)),
        );
        model.rebuild_topology_ids();
        model
    }

    #[test]
    fn same_parameter_success_report_roundtrip_and_idempotence() {
        let run = fix_edges(&bad_pcurve_model(), EdgeFixBudget::default()).unwrap();
        assert_eq!(stage(&run, "sameParameter").fixed, 1);
        assert_eq!(stage(&run, "sameParameter").remaining, 0);
        run.model.validate().unwrap();
        run.report.validate().unwrap();
        assert_eq!(HEALING_REPORT_VERSION, 3);
        let json = run.report.to_json().unwrap();
        assert_eq!(crate::healing_report::HealingReport::from_json(&json).unwrap(), run.report);
        let replay = fix_edges(&run.model, EdgeFixBudget::default()).unwrap();
        assert!(replay.report.stages.iter().all(|stage| stage.fixed == 0));
        assert_eq!(replay.model, run.model);
    }

    #[test]
    fn safe_degenerate_is_removed_and_wire_indices_are_rebuilt() {
        let model = removable_degenerate_model();
        let edge_count = model.edges.len();
        let run = fix_edges(&model, EdgeFixBudget::default()).unwrap();
        assert_eq!(run.model.edges.len(), edge_count - 1);
        assert_eq!(stage(&run, "degenerateAndWireRebuild").fixed, 1);
        assert_eq!(stage(&run, "degenerateAndWireRebuild").remaining, 0);
        run.model.validate().unwrap();
        let replay = fix_edges(&run.model, EdgeFixBudget::default()).unwrap();
        assert_eq!(stage(&replay, "degenerateAndWireRebuild").fixed, 0);
        assert_eq!(replay.model, run.model);
    }

    #[test]
    fn unsafe_degenerate_is_residual_and_split_is_typed_refusal() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        model.edges[0].degenerate = true;
        let run = fix_edges(&model, EdgeFixBudget::default()).unwrap();
        assert_eq!(run.model, model);
        let stage = stage(&run, "degenerateAndWireRebuild");
        assert_eq!(stage.fixed, 0);
        assert_eq!(stage.skipped, 1);
        assert_eq!(stage.remaining, 1);
        assert!(!run.report.unresolved_defects.is_empty());
        let error = split_edge(&model, 0, 0.5).unwrap_err();
        assert_eq!(error.code, EDGE_SPLIT_REFUSED);
    }

    #[test]
    fn failed_snap_rolls_back_whole_pipeline_with_stage_evidence() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let vertex = model.edges[0].vertices[0];
        model.vertices[vertex].point[0] += model.tolerance_mm * 2.;
        let run = fix_edges(&model, EdgeFixBudget::default()).unwrap();
        assert_eq!(run.model, model);
        assert_eq!(run.report.status, HealingStatus::RolledBack);
        assert_eq!(run.report.input_hash, run.report.output_hash);
        assert_eq!(run.report.stages.len(), 1);
        assert_eq!(stage(&run, "snap").fixed, 0);
        assert!(stage(&run, "snap").skipped > 0);
        run.report.validate().unwrap();
    }
}
