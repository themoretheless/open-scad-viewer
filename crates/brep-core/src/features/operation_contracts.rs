//! Input/output contracts for Fillet and Sewing (nurbs-priorities 1064).
//!
//! Boolean already declares its tolerance hand-off at its public boundary.
//! Fillet and Sewing additionally get explicit preconditions, checked before
//! any authoring, and postconditions, verified on the exact staged result
//! before it is returned. The wrapped algorithms are unchanged; a contract
//! refusal publishes nothing. Effective tolerances come only from
//! [`crate::tolerance_hierarchy::BodyTolerance`].
use crate::tolerance_hierarchy::BodyTolerance;
use crate::Model;
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;

/// Output tolerance may grow at most this factor over the effective input
/// tolerance (matches the shared non-fillet cap in `cad_predicates`).
pub const TOLERANCE_GROWTH_LIMIT: f64 = 10.;
/// A sewable gap may be at most this multiple of the effective entity tolerance.
pub const SEW_GAP_TOLERANCE_FACTOR: f64 = 10.;

/// Euler-Poincare invariant `V - E + F - H - 2S`, equal to `-2 * genus` for a
/// closed orientable solid. Fillet and sewing must preserve it.
pub fn euler_poincare(model: &Model) -> i64 {
    let holes: usize = model.faces.iter().map(|face| face.holes.len()).sum();
    model.vertices.len() as i64 - model.edges.len() as i64 + model.faces.len() as i64
        - holes as i64
        - 2 * model.shells.len() as i64
}

fn bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &model.vertices {
        for i in 0..3 {
            min[i] = min[i].min(vertex.point[i]);
            max[i] = max[i].max(vertex.point[i]);
        }
    }
    (min, max)
}

fn bounds_within(inner: &Model, outer: &Model, slack: f64) -> bool {
    let (imin, imax) = bounds(inner);
    let (omin, omax) = bounds(outer);
    (0..3).all(|i| imin[i] >= omin[i] - slack && imax[i] <= omax[i] + slack)
}

/// Effective 3D traversal directions of every edge use in shell face loops.
fn edge_uses(model: &Model) -> BTreeMap<usize, Vec<bool>> {
    let mut uses: BTreeMap<usize, Vec<bool>> = BTreeMap::new();
    for shell in &model.shells {
        for face_use in &shell.faces {
            let Some(face) = model.faces.get(face_use.face) else {
                continue;
            };
            for &loop_index in std::iter::once(&face.outer).chain(&face.holes) {
                let Some(lp) = model.loops.get(loop_index) else {
                    continue;
                };
                for coedge in &lp.coedges {
                    uses.entry(coedge.edge)
                        .or_default()
                        .push(coedge.reversed ^ face_use.reversed);
                }
            }
        }
    }
    uses
}

/// Every edge shared by exactly two uses must be traversed in opposite
/// directions; returns the first offending edge index.
pub fn orientation_conflict(model: &Model) -> Option<usize> {
    edge_uses(model)
        .into_iter()
        .find_map(|(edge, dirs)| (dirs.len() == 2 && dirs[0] == dirs[1]).then_some(edge))
}

fn chord(model: &Model, edge: usize) -> f64 {
    let e = &model.edges[edge];
    crate::distance(
        &model.vertices[e.vertices[0]].point,
        &model.vertices[e.vertices[1]].point,
    )
}

fn fillet_refused(code: &'static str, message: String) -> Error {
    // Keep literal codes visible to the kernel error registry scan.
    match code {
        "BREP_FILLET_INPUT_NOT_CLOSED" => Error::new("BREP_FILLET_INPUT_NOT_CLOSED", message),
        "BREP_FILLET_EDGE_CHAIN_INVALID" => Error::new("BREP_FILLET_EDGE_CHAIN_INVALID", message),
        "BREP_FILLET_RADIUS_EXCEEDS_FEATURE" => {
            Error::new("BREP_FILLET_RADIUS_EXCEEDS_FEATURE", message)
        }
        _ => Error::new("BREP_FILLET_POSTCONDITION_FAILED", message),
    }
}

/// Facts captured by the fillet precondition check and reused after the operation.
#[derive(Clone, Debug, PartialEq)]
pub struct FilletPreconditions {
    pub radius: f64,
    pub edges: Vec<usize>,
    pub tolerance: BodyTolerance,
    pub input_tolerance_mm: f64,
    pub euler_poincare: i64,
    pub volume_mm3: f64,
    /// Smallest local feature size over the selected edges.
    pub local_feature_mm: f64,
    /// Upper bound on |volume change|: sum of `L * r^2 * (1 - pi/4)`.
    pub max_volume_change_mm3: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilletPostconditions {
    pub volume_change_mm3: f64,
    pub output_tolerance_mm: f64,
    pub euler_poincare: i64,
}

const VOLUME_RELATIVE: f64 = 1e-7;

fn volume(model: &Model) -> Result<f64> {
    crate::analysis::mass_properties(model, VOLUME_RELATIVE, 400_000)
        .map(|mass| mass.signed_volume_mm3)
}

fn closed_manifold(model: &Model, code: &'static str) -> Result<()> {
    model
        .validate()
        .map_err(|error| fillet_refused(code, format!("input is not valid: {}", error.message)))?;
    if model.bodies.is_empty() || model.shells.iter().any(|shell| !shell.closed) {
        return Err(fillet_refused(code, "input must be closed solid bodies".into()));
    }
    let uses = edge_uses(model);
    if let Some((edge, dirs)) = (0..model.edges.len())
        .map(|edge| (edge, uses.get(&edge).map_or(0, Vec::len)))
        .find(|(_, count)| *count != 2)
    {
        return Err(fillet_refused(
            code,
            format!("edge {edge} has {dirs} face uses; a closed 2-manifold needs exactly 2"),
        ));
    }
    if let Some(edge) = orientation_conflict(model) {
        return Err(fillet_refused(
            code,
            format!("edge {edge} is traversed in the same direction by both faces"),
        ));
    }
    crate::solid_audit::audit_solid(model)
        .map_err(|error| fillet_refused(code, format!("solid audit failed: {}", error.message)))?;
    Ok(())
}

/// Preconditions: closed oriented 2-manifold input; finite positive radius
/// below the local feature size of every selected edge; a non-empty,
/// duplicate-free chain of in-range, non-degenerate manifold edges.
pub fn check_fillet_preconditions(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<FilletPreconditions> {
    closed_manifold(model, "BREP_FILLET_INPUT_NOT_CLOSED")?;
    let tolerance = BodyTolerance::of(model)?;
    if edges.is_empty() || edges.len() > 64 {
        return Err(fillet_refused(
            "BREP_FILLET_EDGE_CHAIN_INVALID",
            "fillet edge chain must contain 1..64 edges".into(),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for &edge in edges {
        if edge >= model.edges.len() {
            return Err(fillet_refused(
                "BREP_FILLET_EDGE_CHAIN_INVALID",
                format!("edge index {edge} is out of range"),
            ));
        }
        if !seen.insert(edge) {
            return Err(fillet_refused(
                "BREP_FILLET_EDGE_CHAIN_INVALID",
                format!("edge {edge} is selected twice"),
            ));
        }
        if model.edges[edge].degenerate || chord(model, edge) <= tolerance.edge_mm(model, edge) {
            return Err(fillet_refused(
                "BREP_FILLET_EDGE_CHAIN_INVALID",
                format!("edge {edge} is degenerate at the effective tolerance"),
            ));
        }
    }
    if !(radius.is_finite() && radius > tolerance.body_mm) {
        return Err(fillet_refused(
            "BREP_FILLET_RADIUS_EXCEEDS_FEATURE",
            format!(
                "radius must be finite and above the effective tolerance {} mm",
                tolerance.body_mm
            ),
        ));
    }
    // Local feature size: half the shortest edge bounding any face adjacent
    // to a selected edge. A larger radius would consume a whole face.
    let mut face_edges: Vec<Vec<usize>> = vec![Vec::new(); model.faces.len()];
    for (index, face) in model.faces.iter().enumerate() {
        for &loop_index in std::iter::once(&face.outer).chain(&face.holes) {
            face_edges[index].extend(model.loops[loop_index].coedges.iter().map(|c| c.edge));
        }
    }
    let mut local_feature_mm = f64::INFINITY;
    let mut max_volume_change_mm3 = 0.;
    for &edge in edges {
        let lfs = face_edges
            .iter()
            .filter(|list| list.contains(&edge))
            .flatten()
            .map(|&other| chord(model, other) / 2.)
            .fold(f64::INFINITY, f64::min);
        local_feature_mm = local_feature_mm.min(lfs);
        max_volume_change_mm3 +=
            chord(model, edge) * radius * radius * (1. - std::f64::consts::FRAC_PI_4);
    }
    if radius >= local_feature_mm {
        return Err(fillet_refused(
            "BREP_FILLET_RADIUS_EXCEEDS_FEATURE",
            format!("radius {radius} mm reaches the local feature size {local_feature_mm} mm"),
        ));
    }
    let volume_mm3 = volume(model).map_err(|error| {
        fillet_refused(
            "BREP_FILLET_INPUT_NOT_CLOSED",
            format!("input volume is not measurable: {}", error.message),
        )
    })?;
    Ok(FilletPreconditions {
        radius,
        edges: edges.to_vec(),
        tolerance,
        input_tolerance_mm: model.tolerance_mm,
        euler_poincare: euler_poincare(model),
        volume_mm3,
        local_feature_mm,
        max_volume_change_mm3,
    })
}

/// Postconditions: closed oriented output with the same Euler-Poincare
/// invariant; bounding box not grown; non-zero volume change bounded by the
/// swept corner area; tolerance not tightened and grown at most
/// [`TOLERANCE_GROWTH_LIMIT`] times.
pub fn verify_fillet_postconditions(
    pre: &FilletPreconditions,
    input: &Model,
    output: &Model,
) -> Result<FilletPostconditions> {
    const CODE: &str = "BREP_FILLET_POSTCONDITION_FAILED";
    closed_manifold(output, CODE)?;
    let euler = euler_poincare(output);
    if euler != pre.euler_poincare {
        return Err(fillet_refused(
            CODE,
            format!("Euler-Poincare changed {} -> {euler}", pre.euler_poincare),
        ));
    }
    if !bounds_within(output, input, pre.tolerance.body_mm) {
        return Err(fillet_refused(CODE, "result grew beyond the input bounds".into()));
    }
    let out_tol = output.tolerance_mm;
    let limit = pre.input_tolerance_mm.max(pre.tolerance.body_mm) * TOLERANCE_GROWTH_LIMIT;
    if !(out_tol >= pre.input_tolerance_mm && out_tol <= limit) {
        return Err(fillet_refused(
            CODE,
            format!(
                "output tolerance {out_tol} mm outside [{}, {limit}] mm",
                pre.input_tolerance_mm
            ),
        ));
    }
    let after = volume(output)
        .map_err(|error| fillet_refused(CODE, format!("output volume: {}", error.message)))?;
    let change = after - pre.volume_mm3;
    let slack = (pre.volume_mm3.abs() + after.abs()) * VOLUME_RELATIVE * 10.;
    if change.abs() <= slack || change.abs() > pre.max_volume_change_mm3 + slack {
        return Err(fillet_refused(
            CODE,
            format!(
                "volume change {change} mm3 outside (0, {}] mm3",
                pre.max_volume_change_mm3
            ),
        ));
    }
    Ok(FilletPostconditions {
        volume_change_mm3: change,
        output_tolerance_mm: out_tol,
        euler_poincare: euler,
    })
}

/// Run any fillet author between its contract checks.
pub fn with_fillet_contract<T>(
    model: &Model,
    edges: &[usize],
    radius: f64,
    author: impl FnOnce(&Model) -> Result<(Model, T)>,
) -> Result<(Model, T, FilletPostconditions)> {
    let pre = check_fillet_preconditions(model, edges, radius)?;
    let (output, evidence) = author(model)?;
    let post = verify_fillet_postconditions(&pre, model, &output)?;
    Ok((output, evidence, post))
}

/// `analytic_fillet_chain` under the Fillet contract.
pub fn contracted_analytic_fillet_chain(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<(Model, crate::FeatureCertificate, FilletPostconditions)> {
    with_fillet_contract(model, edges, radius, |m| {
        crate::analytic_fillet_chain(m, edges, radius)
    })
}

#[cfg(feature = "codec")]
pub use sewing::*;

#[cfg(feature = "codec")]
mod sewing {
    use super::*;
    use crate::defects::{DefectKind, classify_defects};
    use crate::healing::tolerant_sew::{CertifiedSewingRun, SewingBudget, sew_tolerant_certified};
    use crate::healing_report::HealingStatus;

    #[derive(Clone, Debug, PartialEq)]
    pub struct SewingPreconditions {
        pub tolerance: BodyTolerance,
        pub input_tolerance_mm: f64,
        pub euler_poincare: i64,
        pub max_gap_mm: f64,
        pub gap_count: usize,
    }

    /// Preconditions: complete defect audit; every endpoint gap is at most the
    /// user cap and [`SEW_GAP_TOLERANCE_FACTOR`] times the effective entity
    /// tolerance; edges shared by two face uses have compatible orientation.
    pub fn check_sewing_preconditions(
        model: &Model,
        budget: &SewingBudget,
    ) -> Result<SewingPreconditions> {
        let tolerance = BodyTolerance::of(model)?;
        if let Some(edge) = orientation_conflict(model) {
            return Err(Error::new(
                "BREP_SEW_ORIENTATION_INCOMPATIBLE",
                format!("edge {edge} is traversed in the same direction by both faces"),
            ));
        }
        let report = classify_defects(model, budget.defects)?;
        if !report.complete {
            return Err(Error::new(
                "BREP_SEW_AUDIT_INCOMPLETE",
                "Sewing contract requires a complete defect audit",
            ));
        }
        let limit_entity = tolerance.max_entity_mm(model) * SEW_GAP_TOLERANCE_FACTOR;
        let limit = limit_entity.min(budget.user_cap_mm);
        let mut max_gap_mm: f64 = 0.;
        let mut gap_count = 0;
        for defect in report
            .defects
            .iter()
            .filter(|defect| defect.kind == DefectKind::GapEdges)
        {
            gap_count += 1;
            let gap = defect.measured.unwrap_or(f64::INFINITY);
            if !(gap <= limit) {
                return Err(Error::new(
                    "BREP_SEW_GAP_EXCEEDS_TOLERANCE",
                    format!(
                        "gap {gap} mm exceeds min(user cap {}, {SEW_GAP_TOLERANCE_FACTOR} x tolerance {}) mm",
                        budget.user_cap_mm, limit_entity
                    ),
                ));
            }
            max_gap_mm = max_gap_mm.max(gap);
        }
        Ok(SewingPreconditions {
            tolerance,
            input_tolerance_mm: model.tolerance_mm,
            euler_poincare: euler_poincare(model),
            max_gap_mm,
            gap_count,
        })
    }

    /// Postconditions: a rolled-back run publishes the input unchanged; a
    /// complete run is valid, has closed shells, the same Euler-Poincare
    /// invariant, no residual gaps, vertices moved at most the largest gap,
    /// and tolerance within `[input, max(input, max gap)]`.
    pub fn verify_sewing_postconditions(
        pre: &SewingPreconditions,
        input: &Model,
        run: &CertifiedSewingRun,
        budget: &SewingBudget,
    ) -> Result<()> {
        let fail = |message: String| Error::new("BREP_SEW_POSTCONDITION_FAILED", message);
        let output = &run.run.model;
        if run.run.report.status != HealingStatus::Complete {
            return if output.0 == input.0 {
                Ok(())
            } else {
                Err(fail("rolled-back sewing changed the model".into()))
            };
        }
        output
            .validate()
            .map_err(|error| fail(format!("sewn model invalid: {}", error.message)))?;
        if output.shells.iter().any(|shell| !shell.closed) {
            return Err(fail("sewn model has an open shell".into()));
        }
        let euler = euler_poincare(output);
        if euler != pre.euler_poincare {
            return Err(fail(format!(
                "Euler-Poincare changed {} -> {euler}",
                pre.euler_poincare
            )));
        }
        if orientation_conflict(output).is_some() {
            return Err(fail("sewn model has inconsistent orientation".into()));
        }
        let residual = classify_defects(output, budget.defects)?;
        if residual
            .defects
            .iter()
            .any(|defect| defect.kind == DefectKind::GapEdges)
        {
            return Err(fail("sewn model has residual gaps".into()));
        }
        let moved = input
            .vertices
            .iter()
            .zip(&output.vertices)
            .map(|(a, b)| crate::distance(&a.point, &b.point))
            .fold(0., f64::max);
        if input.vertices.len() != output.vertices.len()
            || moved > pre.max_gap_mm + pre.tolerance.body_mm
        {
            return Err(fail(format!(
                "vertex displacement {moved} mm exceeds the largest gap {} mm",
                pre.max_gap_mm
            )));
        }
        let out_tol = output.tolerance_mm;
        let ceiling = pre
            .input_tolerance_mm
            .max(pre.max_gap_mm)
            .min(pre.input_tolerance_mm * TOLERANCE_GROWTH_LIMIT);
        if !(out_tol >= pre.input_tolerance_mm && out_tol <= ceiling) {
            return Err(fail(format!(
                "output tolerance {out_tol} mm outside [{}, {ceiling}] mm",
                pre.input_tolerance_mm
            )));
        }
        Ok(())
    }

    /// `sew_tolerant_certified` under the Sewing contract.
    pub fn contracted_sew(model: &Model, budget: SewingBudget) -> Result<CertifiedSewingRun> {
        let pre = check_sewing_preconditions(model, &budget)?;
        let run = sew_tolerant_certified(model, budget)?;
        verify_sewing_postconditions(&pre, model, &run, &budget)?;
        Ok(run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> Model {
        crate::cuboid([0.; 3], [10.; 3]).unwrap()
    }
    fn vertical_edges(model: &Model) -> Vec<usize> {
        (0..model.edges.len())
            .filter(|&i| {
                let e = &model.edges[i];
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                (a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12
            })
            .collect()
    }

    #[test]
    fn cuboid_is_oriented_closed_genus_zero() {
        let model = cube();
        assert_eq!(euler_poincare(&model), 0);
        assert_eq!(orientation_conflict(&model), None);
    }

    #[test]
    fn fillet_contract_passes_and_bounds_volume() {
        let model = cube();
        let edges = vertical_edges(&model);
        let (out, cert, post) = contracted_analytic_fillet_chain(&model, &edges[..1], 1.).unwrap();
        assert_eq!(cert.capability, "analytic-fillet/1");
        assert_eq!(post.euler_poincare, 0);
        let expected = -10. * (1. - std::f64::consts::FRAC_PI_4);
        assert!(
            (post.volume_change_mm3 - expected).abs() < 1e-3,
            "{post:?}"
        );
        assert_eq!(post.output_tolerance_mm, out.tolerance_mm);
        let (_, _, two) = contracted_analytic_fillet_chain(&model, &edges[..2], 1.).unwrap();
        assert!((two.volume_change_mm3 - 2. * expected).abs() < 1e-3);
    }

    #[test]
    fn fillet_refuses_invalid_chains() {
        let model = cube();
        let edges = vertical_edges(&model);
        let code = |e: &[usize], r: f64| check_fillet_preconditions(&model, e, r).unwrap_err().code;
        assert_eq!(code(&[], 1.), "BREP_FILLET_EDGE_CHAIN_INVALID");
        assert_eq!(code(&[999], 1.), "BREP_FILLET_EDGE_CHAIN_INVALID");
        assert_eq!(code(&[edges[0], edges[0]], 1.), "BREP_FILLET_EDGE_CHAIN_INVALID");
        assert_eq!(code(&edges[..1], 5.), "BREP_FILLET_RADIUS_EXCEEDS_FEATURE");
        assert_eq!(code(&edges[..1], f64::NAN), "BREP_FILLET_RADIUS_EXCEEDS_FEATURE");
        assert_eq!(code(&edges[..1], 0.), "BREP_FILLET_RADIUS_EXCEEDS_FEATURE");
        // The contract refuses before the author runs.
        let mut ran = false;
        let error = with_fillet_contract(&model, &edges[..1], 6., |m| {
            ran = true;
            Ok((m.clone(), ()))
        })
        .unwrap_err();
        assert_eq!(error.code, "BREP_FILLET_RADIUS_EXCEEDS_FEATURE");
        assert!(!ran);
    }

    #[test]
    fn fillet_refuses_open_or_misoriented_input() {
        let mut open = cube();
        open.shells[0].closed = false;
        assert_eq!(
            check_fillet_preconditions(&open, &[0], 1.).unwrap_err().code,
            "BREP_FILLET_INPUT_NOT_CLOSED"
        );
        let mut flipped = cube();
        flipped.shells[0].faces[0].reversed ^= true;
        assert_eq!(
            check_fillet_preconditions(&flipped, &[0], 1.).unwrap_err().code,
            "BREP_FILLET_INPUT_NOT_CLOSED"
        );
    }

    #[test]
    fn fillet_postconditions_reject_identity_and_tolerance_blowup() {
        let model = cube();
        let edges = vertical_edges(&model);
        // An author that does nothing changes no volume.
        let error = with_fillet_contract(&model, &edges[..1], 1., |m| Ok((m.clone(), ())))
            .unwrap_err();
        assert_eq!(error.code, "BREP_FILLET_POSTCONDITION_FAILED");
        // An author that inflates the tolerance past the growth limit.
        let error = with_fillet_contract(&model, &edges[..1], 1., |m| {
            let (mut out, cert) = crate::analytic_fillet_chain(m, &edges[..1], 1.)?;
            out.tolerance_mm = m.tolerance_mm * 1e3;
            Ok((out, cert))
        })
        .unwrap_err();
        assert_eq!(error.code, "BREP_FILLET_POSTCONDITION_FAILED");
        // An author that removes far more material than a fillet can.
        let error = with_fillet_contract(&model, &edges[..1], 1., |_| {
            Ok((crate::cuboid([0.; 3], [10., 10., 5.]).unwrap(), ()))
        })
        .unwrap_err();
        assert_eq!(error.code, "BREP_FILLET_POSTCONDITION_FAILED");
    }

    #[cfg(feature = "codec")]
    mod sewing {
        use super::super::*;
        use crate::defects::DefectBudget;
        use crate::healing::tolerant_sew::SewingBudget;
        use crate::healing_report::HealingStatus;

        fn gap_model(multiplier: f64) -> Model {
            let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
            let vertex = model.edges[0].vertices[0];
            model.vertices[vertex].point[0] += model.tolerance_mm * multiplier;
            model
        }
        fn budget(model: &Model, user: f64) -> SewingBudget {
            SewingBudget {
                defects: DefectBudget::default(),
                user_cap_mm: model.tolerance_mm * user,
                cumulative_cap_mm: model.tolerance_mm * 100.,
                max_candidates: 64,
            }
        }

        #[test]
        fn sewing_contract_passes_and_bounds_tolerance() {
            let model = gap_model(2.);
            let pre = check_sewing_preconditions(&model, &budget(&model, 3.)).unwrap();
            assert!(pre.gap_count >= 1, "{pre:?}");
            assert!(pre.max_gap_mm > 0.);
            let run = contracted_sew(&model, budget(&model, 3.)).unwrap();
            assert_eq!(run.run.report.status, HealingStatus::Complete);
            assert!(run.run.model.tolerance_mm >= model.tolerance_mm);
            assert!(run.run.model.tolerance_mm <= model.tolerance_mm * TOLERANCE_GROWTH_LIMIT);
            assert_eq!(euler_poincare(&run.run.model), euler_poincare(&model));
        }

        #[test]
        fn clean_input_sews_nothing_and_passes() {
            let model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
            let run = contracted_sew(&model, budget(&model, 3.)).unwrap();
            assert_eq!(run.run.model.0, model.0);
        }

        #[test]
        fn gap_beyond_tolerance_or_cap_is_refused_up_front() {
            let model = gap_model(20.);
            assert_eq!(
                contracted_sew(&model, budget(&model, 30.)).unwrap_err().code,
                "BREP_SEW_GAP_EXCEEDS_TOLERANCE"
            );
            let model = gap_model(2.);
            assert_eq!(
                contracted_sew(&model, budget(&model, 1.)).unwrap_err().code,
                "BREP_SEW_GAP_EXCEEDS_TOLERANCE"
            );
        }

        #[test]
        fn incompatible_orientation_is_refused() {
            let mut model = gap_model(2.);
            model.shells[0].faces[0].reversed ^= true;
            assert_eq!(
                contracted_sew(&model, budget(&model, 3.)).unwrap_err().code,
                "BREP_SEW_ORIENTATION_INCOMPATIBLE"
            );
        }

        #[test]
        fn postcondition_rejects_tampered_run() {
            let model = gap_model(2.);
            let b = budget(&model, 3.);
            let pre = check_sewing_preconditions(&model, &b).unwrap();
            let mut run = crate::healing::tolerant_sew::sew_tolerant_certified(&model, b).unwrap();
            run.run.model.tolerance_mm = model.tolerance_mm * 1e3;
            assert_eq!(
                verify_sewing_postconditions(&pre, &model, &run, &b).unwrap_err().code,
                "BREP_SEW_POSTCONDITION_FAILED"
            );
            let mut run = crate::healing::tolerant_sew::sew_tolerant_certified(&model, b).unwrap();
            run.run.model.vertices[0].point[2] += 1.;
            assert_eq!(
                verify_sewing_postconditions(&pre, &model, &run, &b).unwrap_err().code,
                "BREP_SEW_POSTCONDITION_FAILED"
            );
        }
    }
}
