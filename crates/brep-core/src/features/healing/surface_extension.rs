//! Atomic surface-only seam healing by bounded analytic surface extension.
//!
//! The supported successful geometry is an affine bilinear plane whose authored
//! pcurves extend just beyond one clamped UV boundary. Edges, vertices, pcurves
//! and persistent topology identities remain unchanged. Projection and stitch
//! are explicit zero-work stages, not hidden fallbacks.

use crate::defects::{DefectBudget, DefectKind, classify_defects};
use crate::healing_report::{
    HealingConversion, HealingGapSummary, HealingReport, HealingRun, HealingStageReport,
    HealingStatus, model_hash,
};
use crate::Model;
use nurbs_core::curve_extension::{Continuity, End};
use nurbs_core::editing::surface_extension::extend_surface;
use nurbs_core::surface::{Axis, Surface};
use nurbs_core::{Error, Result};

pub const EXTEND_BUDGET_EXCEEDED: &str = "BREP_EXTEND_BUDGET_EXCEEDED";
pub const EXTEND_CAP_EXCEEDED: &str = "BREP_EXTEND_CAP_EXCEEDED";
pub const EXTEND_UNSUPPORTED: &str = "BREP_EXTEND_UNSUPPORTED";
pub const EXTEND_UNSTABLE: &str = "BREP_EXTEND_UNSTABLE";
pub const EXTEND_ERROR_GREW: &str = "BREP_EXTEND_ERROR_GREW";
pub const EXTEND_RESIDUAL: &str = "BREP_EXTEND_RESIDUAL";
pub const EXTEND_MEASUREMENT: &str = "BREP_EXTEND_MEASUREMENT";
pub const EXTEND_VALIDATION: &str = "BREP_EXTEND_VALIDATION";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceExtensionBudget {
    pub defects: DefectBudget,
    pub sewing_tolerance_mm: f64,
    pub extension_cap_mm: f64,
    pub max_candidates: usize,
    /// Deterministic samples per coedge used for before/after max-error evidence.
    pub samples_per_use: usize,
}

#[derive(Clone, Copy)]
struct Candidate {
    face: usize,
    axis: Axis,
    end: End,
    length_mm: f64,
}

struct Measurement {
    maximum_mm: f64,
    per_face_mm: Vec<f64>,
}

fn fail(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn domain(surface: &Surface, axis: Axis) -> [f64; 2] {
    match axis {
        Axis::U => [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        Axis::V => [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    }
}

fn measure(model: &Model, samples: usize) -> Result<Measurement> {
    let mut per_face_mm = vec![0_f64; model.faces.len()];
    for (face_index, face) in model.faces.iter().enumerate() {
        let domains = [domain(&face.surface, Axis::U), domain(&face.surface, Axis::V)];
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            for coedge in &model.loops[wire].coedges {
                let edge = &model.edges[coedge.edge].curve;
                let edge_domain = edge.domain();
                let pcurve_domain = coedge.pcurve.domain();
                for sample in 0..samples {
                    let fraction = sample as f64 / (samples - 1) as f64;
                    let edge_fraction = if coedge.reversed { 1. - fraction } else { fraction };
                    let edge_parameter = edge_domain[0]
                        + (edge_domain[1] - edge_domain[0]) * edge_fraction;
                    let pcurve_parameter = pcurve_domain[0]
                        + (pcurve_domain[1] - pcurve_domain[0]) * fraction;
                    let edge_point = edge.evaluate(edge_parameter).map_err(|error| {
                        fail(EXTEND_MEASUREMENT, format!("Edge evaluation failed: {}", error.message))
                    })?.point;
                    let uv = coedge.pcurve.evaluate(pcurve_parameter).map_err(|error| {
                        fail(EXTEND_MEASUREMENT, format!("Pcurve evaluation failed: {}", error.message))
                    })?.point;
                    if uv.len() != 2 || uv.iter().any(|value| !value.is_finite()) {
                        return Err(fail(EXTEND_MEASUREMENT, "Surface seam requires finite 2D pcurves"));
                    }
                    let clamped: [f64; 2] = std::array::from_fn(|axis| {
                        uv[axis].clamp(domains[axis][0], domains[axis][1])
                    });
                    let surface_point = face.surface.evaluate(clamped[0], clamped[1]).map_err(|error| {
                        fail(EXTEND_MEASUREMENT, format!("Surface evaluation failed: {}", error.message))
                    })?.point;
                    let error = crate::distance(&edge_point, &surface_point);
                    if !error.is_finite() {
                        return Err(fail(EXTEND_MEASUREMENT, "Non-finite surface seam error"));
                    }
                    per_face_mm[face_index] = per_face_mm[face_index].max(error);
                }
            }
        }
    }
    Ok(Measurement {
        maximum_mm: per_face_mm.iter().copied().fold(0., f64::max),
        per_face_mm,
    })
}

fn sub(a: &[f64], b: &[f64]) -> [f64; 3] {
    std::array::from_fn(|axis| a[axis] - b[axis])
}

fn norm(vector: [f64; 3]) -> f64 {
    vector.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn is_affine_plane(surface: &Surface) -> bool {
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.periodic_u
        || surface.periodic_v
        || surface.control_points.len() != 2
        || surface.control_points.iter().any(|row| row.len() != 2)
        || surface.control_points.iter().flatten().any(|point| point.len() != 3)
        || surface.weights.iter().flatten().any(|weight| *weight != 1.)
    {
        return false;
    }
    let origin = &surface.control_points[0][0];
    let a = sub(&surface.control_points[1][0], origin);
    let b = sub(&surface.control_points[0][1], origin);
    let opposite = sub(&surface.control_points[1][1], origin);
    let residual = norm(std::array::from_fn(|axis| opposite[axis] - a[axis] - b[axis]));
    let scale = norm(a).max(norm(b)).max(norm(opposite)).max(1.);
    norm(a) > 0. && norm(b) > 0. && residual <= 1e-12 * scale
}

fn candidate(model: &Model, face: usize) -> Result<Candidate> {
    let surface = &model.faces[face].surface;
    let domains = [domain(surface, Axis::U), domain(surface, Axis::V)];
    let mut minimum = [f64::INFINITY; 2];
    let mut maximum = [f64::NEG_INFINITY; 2];
    for &wire in std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes) {
        for coedge in &model.loops[wire].coedges {
            if coedge.trim().is_some() {
                return Err(fail(
                    EXTEND_UNSUPPORTED,
                    "Surface extension does not infer subcurve trim intervals",
                ));
            }
            for point in &coedge.pcurve.control_points {
                if point.len() != 2 || point.iter().any(|value| !value.is_finite()) {
                    return Err(fail(EXTEND_UNSUPPORTED, "Surface extension requires finite 2D pcurves"));
                }
                for axis in 0..2 {
                    minimum[axis] = minimum[axis].min(point[axis]);
                    maximum[axis] = maximum[axis].max(point[axis]);
                }
            }
        }
    }
    let mut requests = Vec::new();
    for (axis_index, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
        let below = (domains[axis_index][0] - minimum[axis_index]).max(0.);
        let above = (maximum[axis_index] - domains[axis_index][1]).max(0.);
        if below > 0. && above > 0. {
            return Err(fail(
                EXTEND_UNSUPPORTED,
                "One-pass extension cannot cover both ends of a surface axis",
            ));
        }
        if below > 0. {
            requests.push((axis, End::Start, below));
        }
        if above > 0. {
            requests.push((axis, End::Stop, above));
        }
    }
    if requests.len() != 1 {
        return Err(fail(
            EXTEND_UNSUPPORTED,
            "Surface error is not a single clamped UV-domain extension gap",
        ));
    }
    let (axis, end, parameter_step) = requests[0];
    let other = match axis { Axis::U => Axis::V, Axis::V => Axis::U };
    let other_domain = domain(surface, other);
    let other_mid = (other_domain[0] + other_domain[1]) * 0.5;
    let own_domain = domain(surface, axis);
    let own_end = match end { End::Start => own_domain[0], End::Stop => own_domain[1] };
    let (u, v) = match axis { Axis::U => (own_end, other_mid), Axis::V => (other_mid, own_end) };
    let evaluation = surface.evaluate(u, v).map_err(|error| {
        fail(EXTEND_UNSUPPORTED, format!("Cannot evaluate extension jet: {}", error.message))
    })?;
    let (du, dv) = evaluation.first_derivatives().ok_or_else(|| {
        fail(EXTEND_UNSUPPORTED, "Surface extension requires a nonzero boundary jet")
    })?;
    let speed = norm(match axis { Axis::U => du, Axis::V => dv });
    let length_mm = speed * parameter_step;
    if !length_mm.is_finite() || length_mm <= 0. {
        return Err(fail(EXTEND_UNSUPPORTED, "Surface extension requires nonzero finite end speed"));
    }
    Ok(Candidate { face, axis, end, length_mm })
}

fn metric(stage: &str, fixed: usize) -> HealingStageReport {
    HealingStageReport {
        stage: stage.into(),
        fixed,
        skipped: 0,
        remaining: 0,
        skipped_details: Vec::new(),
    }
}

/// Extend eligible surfaces without moving edge or vertex geometry. Any typed
/// error discards the private staged model and returns no partial repair.
pub fn extend_surface_seams(model: &Model, budget: SurfaceExtensionBudget) -> Result<HealingRun> {
    if !budget.sewing_tolerance_mm.is_finite()
        || budget.sewing_tolerance_mm <= 0.
        || !budget.extension_cap_mm.is_finite()
        || budget.extension_cap_mm <= 0.
        || budget.extension_cap_mm > budget.sewing_tolerance_mm
        || budget.max_candidates == 0
        || budget.max_candidates > 4096
        || !(8..=256).contains(&budget.samples_per_use)
    {
        return Err(fail(
            EXTEND_BUDGET_EXCEEDED,
            "Require 0 < extension cap <= sewing tolerance, 1..4096 candidates and 8..256 samples",
        ));
    }
    model.validate_boundary_diagnostic_inputs().map_err(|error| {
        fail(EXTEND_VALIDATION, format!("Invalid seam diagnostic input: {}", error.message))
    })?;
    let before = measure(model, budget.samples_per_use)?;
    let candidates = before
        .per_face_mm
        .iter()
        .enumerate()
        .filter_map(|(face, error)| (*error > model.tolerance_mm).then_some(face))
        .map(|face| candidate(model, face))
        .collect::<Result<Vec<_>>>()?;
    if candidates.len() > budget.max_candidates {
        return Err(fail(
            EXTEND_BUDGET_EXCEEDED,
            format!("Surface extension candidates {} exceed budget {}", candidates.len(), budget.max_candidates),
        ));
    }

    let mut staged = model.clone();
    let mut conversions = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if candidate.length_mm > budget.extension_cap_mm {
            return Err(fail(
                EXTEND_CAP_EXCEEDED,
                format!("Required extension {} mm exceeds cap {} mm", candidate.length_mm, budget.extension_cap_mm),
            ));
        }
        let source = staged.faces[candidate.face].surface.clone();
        let affine_plane = is_affine_plane(&source);
        let extension = extend_surface(&source, candidate.axis, candidate.end, candidate.length_mm, Continuity::C2)
            .map_err(|error| fail(EXTEND_UNSUPPORTED, format!("Surface axis extension failed: {}", error.message)))?;
        if !affine_plane {
            if !extension.stable {
                return Err(fail(
                    EXTEND_UNSTABLE,
                    format!("Extrapolation growth {} exceeds the stability limit", extension.max_extrapolation_growth),
                ));
            }
            return Err(fail(
                EXTEND_UNSUPPORTED,
                "Successful healing is restricted to affine bilinear planar surfaces",
            ));
        }
        staged.faces[candidate.face].surface = extension.surface;
        conversions.push(HealingConversion {
            entity_id: model.1.faces[candidate.face],
            from: "affineBilinearPlane".into(),
            to: "extendedAffineBilinearPlane".into(),
            rationale: format!("Extended one clamped UV boundary by {} mm without stitch", candidate.length_mm),
        });
    }

    if !conversions.is_empty() {
        let after = measure(&staged, budget.samples_per_use)?;
        if after.maximum_mm > before.maximum_mm {
            return Err(fail(
                EXTEND_ERROR_GREW,
                format!("Max seam error grew from {} to {} mm; extension rolled back", before.maximum_mm, after.maximum_mm),
            ));
        }
        if after.maximum_mm > model.tolerance_mm {
            return Err(fail(
                EXTEND_RESIDUAL,
                format!("Residual max seam error {} mm exceeds model tolerance {} mm", after.maximum_mm, model.tolerance_mm),
            ));
        }
        staged.validate().map_err(|error| {
            fail(EXTEND_VALIDATION, format!("Extended model validation failed: {}", error.message))
        })?;
        let residual = classify_defects(&staged, budget.defects)?;
        if residual.defects.iter().any(|defect| defect.kind == DefectKind::BadPCurve) {
            return Err(fail(EXTEND_RESIDUAL, "Full-interval BadPCurve evidence remains after extension"));
        }
    }

    let extended = conversions.len();
    let after_mm = if extended == 0 { before.maximum_mm } else { measure(&staged, budget.samples_per_use)?.maximum_mm };
    let report = HealingReport {
        input_hash: model_hash(model),
        output_hash: model_hash(&staged),
        status: HealingStatus::Complete,
        operations: Vec::new(),
        gaps: HealingGapSummary { before_mm: before.maximum_mm, after_mm },
        sewn_edges: Vec::new(),
        conversions,
        displacement: Vec::new(),
        certificates: Vec::new(),
        displacement_budget_mm: budget.extension_cap_mm,
        cumulative_displacement_mm: 0.,
        unresolved_defects: Vec::new(),
        refusal_reasons: Vec::new(),
        tolerance_unification: None,
        stages: vec![metric("surfaceExtension", extended), metric("project", 0), metric("stitch", 0)],
    };
    report.validate()?;
    Ok(HealingRun { model: staged, report })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::healing_report::HEALING_REPORT_VERSION;

    fn budget(model: &Model, cap: f64) -> SurfaceExtensionBudget {
        SurfaceExtensionBudget {
            defects: DefectBudget::default(),
            sewing_tolerance_mm: model.tolerance_mm * 4.,
            extension_cap_mm: model.tolerance_mm * cap,
            max_candidates: 8,
            samples_per_use: 33,
        }
    }

    fn trimmed_plane_model(gap: f64) -> Model {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let surface = model.faces[0].surface.clone();
        let [u0, u1] = domain(&surface, Axis::U);
        let [_, v1] = domain(&surface, Axis::V);
        let speed = norm(surface.evaluate(u1, v1).unwrap().first_derivatives().unwrap().0);
        model.faces[0].surface = surface.trim([u0, u1 - gap / speed, domain(&surface, Axis::V)[0], v1]).unwrap();
        model
    }

    fn stage<'a>(run: &'a HealingRun, name: &str) -> &'a HealingStageReport {
        run.report.stages.iter().find(|stage| stage.stage == name).unwrap()
    }

    #[test]
    fn geometric_extension_closes_gap_without_stitch_and_round_trips_report() {
        let model = trimmed_plane_model(2e-7);
        let ids = model.1.clone();
        let original_edges = model.edges.clone();
        let run = extend_surface_seams(&model, budget(&model, 4.)).unwrap();
        assert_eq!(stage(&run, "surfaceExtension").fixed, 1);
        assert_eq!(stage(&run, "project").fixed, 0);
        assert_eq!(stage(&run, "stitch").fixed, 0);
        assert!(run.report.sewn_edges.is_empty());
        assert_eq!(run.report.conversions.len(), 1);
        assert!(run.report.gaps.before_mm > model.tolerance_mm);
        assert!(run.report.gaps.after_mm <= model.tolerance_mm);
        assert!(run.report.gaps.after_mm < run.report.gaps.before_mm);
        assert_eq!(run.model.edges, original_edges);
        assert_eq!(run.model.1, ids);
        run.model.validate().unwrap();
        let end = domain(&run.model.faces[0].surface, Axis::U)[1];
        let point = run.model.faces[0].surface.evaluate(end, 0.5).unwrap().point;
        let reference = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let reference_end = domain(&reference.faces[0].surface, Axis::U)[1];
        let expected = reference.faces[0].surface.evaluate(reference_end, 0.5).unwrap().point;
        for axis in 0..3 {
            assert!((point[axis] - expected[axis]).abs() <= model.tolerance_mm);
        }
        assert_eq!(HEALING_REPORT_VERSION, 3);
        let json = run.report.to_json().unwrap();
        assert_eq!(HealingReport::from_json(&json).unwrap(), run.report);
    }

    #[test]
    fn extension_above_cap_is_typed_and_atomic() {
        let model = trimmed_plane_model(2e-7);
        let error = extend_surface_seams(&model, budget(&model, 1.)).unwrap_err();
        assert_eq!(error.code, EXTEND_CAP_EXCEEDED);
        assert_eq!(model, trimmed_plane_model(2e-7));
    }

    #[test]
    fn unstable_extrapolation_rolls_back_with_typed_error() {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let degree = 20;
        let surface = Surface {
            degree_u: degree,
            degree_v: 1,
            knots_u: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..=degree)
                .map(|i| [0., 1.].map(|z| vec![i as f64 / degree as f64, if i % 2 == 0 { 0.5 } else { -0.5 }, z]).to_vec())
                .collect(),
            weights: vec![vec![1.; 2]; degree + 1],
            periodic_u: false,
            periodic_v: false,
        };
        model.faces[0].surface = surface.trim([0., 1. - 1e-6, 0., 1.]).unwrap();
        let before = model.clone();
        let mut unsafe_budget = budget(&model, 1e9);
        unsafe_budget.sewing_tolerance_mm = model.tolerance_mm * 1e9;
        let error = extend_surface_seams(&model, unsafe_budget).unwrap_err();
        assert_eq!(error.code, EXTEND_UNSTABLE);
        assert_eq!(model, before);
    }

    #[test]
    fn extension_replay_is_a_bit_identical_no_op() {
        let model = trimmed_plane_model(2e-7);
        let run = extend_surface_seams(&model, budget(&model, 4.)).unwrap();
        let replay = extend_surface_seams(&run.model, budget(&model, 4.)).unwrap();
        assert_eq!(stage(&replay, "surfaceExtension").fixed, 0);
        assert_eq!(replay.model, run.model);
        assert_eq!(model_hash(&replay.model), model_hash(&run.model));
        assert!(replay.report.conversions.is_empty());
    }

    #[test]
    fn non_domain_surface_mismatch_is_unsupported_not_projected_or_stitched() {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        model.faces[0].surface = nurbs_core::patches::plane([0., 0., 10.], [2., 0., 0.], [0., 2., 0.]).unwrap();
        let before = model.clone();
        let error = extend_surface_seams(&model, budget(&model, 4.)).unwrap_err();
        assert_eq!(error.code, EXTEND_UNSUPPORTED);
        assert_eq!(model, before);
    }
}
