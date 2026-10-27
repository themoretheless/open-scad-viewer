//! Atomic canonicalization of nearly planar B-spline faces.
//!
//! The successful scope is deliberately plane-only. A best-fit plane is found
//! from the complete control net, every control point must lie strictly inside
//! the requested distance tolerance, and every trim pcurve is re-projected from
//! its oriented 3D edge. Edges, vertices, topology and persistent IDs are never
//! rewritten. Any failed fit, projection, area or validation gate discards the
//! private staged model.

use crate::analysis::surface_classify::{SurfaceClass, classify_surface, face_area};
use crate::healing_report::{
    HealingConversion, HealingGapSummary, HealingReport, HealingRun, HealingStageReport,
    HealingStatus, model_hash,
};
use crate::{Coedge, Model};
use nurbs_core::analysis::ransac_primitives::{
    PrimitiveKind, PrimitiveParams, RansacOptions, detect_primitives,
};
use nurbs_core::curve_pullback;
use nurbs_core::foundation::guards::Budget;
use nurbs_core::surface::Surface;
use nurbs_core::{Error, Result};

pub const CANONICAL_CAST_BUDGET: &str = "BREP_CANONICAL_CAST_BUDGET";
pub const CANONICAL_CAST_UNSUPPORTED: &str = "BREP_CANONICAL_CAST_UNSUPPORTED";
pub const CANONICAL_CAST_NO_FIT: &str = "BREP_CANONICAL_CAST_NO_FIT";
pub const CANONICAL_CAST_DEVIATION: &str = "BREP_CANONICAL_CAST_DEVIATION";
pub const CANONICAL_CAST_PROJECTION: &str = "BREP_CANONICAL_CAST_PROJECTION";
pub const CANONICAL_CAST_AREA_DELTA: &str = "BREP_CANONICAL_CAST_AREA_DELTA";
pub const CANONICAL_CAST_AREA_GREW: &str = "BREP_CANONICAL_CAST_AREA_GREW";
pub const CANONICAL_CAST_VALIDATION: &str = "BREP_CANONICAL_CAST_VALIDATION";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanonicalCastBudget {
    /// Maximum absolute control-net distance to the best-fit plane.
    pub tolerance_mm: f64,
    /// Maximum permitted absolute trimmed-face area change.
    pub area_delta_cap_mm2: f64,
    pub max_candidates: usize,
    pub ransac_iterations: usize,
    /// Per-pcurve continuous curve/surface agreement work limit.
    pub max_agreement_cells: usize,
    /// Shared classification and trimmed-area quadrature budget.
    pub work: Budget,
}

impl Default for CanonicalCastBudget {
    fn default() -> Self {
        Self {
            tolerance_mm: 1e-3,
            area_delta_cap_mm2: 1e-6,
            max_candidates: 64,
            ransac_iterations: 64,
            max_agreement_cells: 100_000,
            work: Budget {
                max_iterations: 20_000_000,
                ..Budget::default()
            },
        }
    }
}

fn fail(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn mul(a: [f64; 3], scale: f64) -> [f64; 3] {
    [a[0] * scale, a[1] * scale, a[2] * scale]
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let length = norm(a);
    (length.is_finite() && length > 1e-14).then(|| mul(a, 1. / length))
}

fn control_points(surface: &Surface) -> Result<Vec<[f64; 3]>> {
    let points = surface
        .control_points
        .iter()
        .flatten()
        .map(|point| {
            if point.len() != 3 || point.iter().any(|value| !value.is_finite()) {
                Err(fail(
                    CANONICAL_CAST_UNSUPPORTED,
                    "Canonical cast requires finite 3D surface control points",
                ))
            } else {
                Ok([point[0], point[1], point[2]])
            }
        })
        .collect::<Result<Vec<_>>>()?;
    if points.len() < 3 || points.len() > 65536 {
        return Err(fail(
            CANONICAL_CAST_BUDGET,
            "Plane fitting requires 3..=65536 control points",
        ));
    }
    Ok(points)
}

fn exact_affine_plane(surface: &Surface) -> bool {
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.periodic_u
        || surface.periodic_v
        || surface.control_points.len() != 2
        || surface.control_points.iter().any(|row| row.len() != 2)
        || surface
            .control_points
            .iter()
            .flatten()
            .any(|point| point.len() != 3)
        || surface.weights.iter().flatten().any(|weight| *weight != 1.)
    {
        return false;
    }
    let cp = |i: usize, j: usize| {
        let p = &surface.control_points[i][j];
        [p[0], p[1], p[2]]
    };
    let origin = cp(0, 0);
    let du = sub(cp(1, 0), origin);
    let dv = sub(cp(0, 1), origin);
    let opposite = sub(cp(1, 1), origin);
    let closure = norm(sub(opposite, add(du, dv)));
    let scale = norm(du).max(norm(dv)).max(norm(opposite)).max(1.);
    norm(du) > 0. && norm(dv) > 0. && closure <= 1e-12 * scale
}

#[derive(Clone, Copy)]
struct PlaneFit {
    normal: [f64; 3],
    offset: f64,
    max_deviation_mm: f64,
}

fn fit_plane(surface: &Surface, budget: CanonicalCastBudget) -> Result<PlaneFit> {
    let points = control_points(surface)?;
    let options = RansacOptions {
        threshold: budget.tolerance_mm,
        min_inliers: points.len(),
        max_iterations: budget.ransac_iterations,
        max_primitives: 1,
        early_exit_fraction: 1.,
        seed: 0x814,
        detect_plane: true,
        detect_sphere: false,
        detect_cylinder: false,
        detect_cone: false,
    };
    let detected = detect_primitives(&points, None, &options).map_err(|error| {
        fail(
            CANONICAL_CAST_NO_FIT,
            format!("Best-fit plane detection failed: {}", error.message),
        )
    })?;
    let Some(candidate) = detected.first() else {
        return Err(fail(
            CANONICAL_CAST_DEVIATION,
            "No plane contains the complete control net within the requested tolerance",
        ));
    };
    if candidate.kind != PrimitiveKind::Plane || candidate.inlier_indices.len() != points.len() {
        return Err(fail(
            CANONICAL_CAST_DEVIATION,
            "The best-fit plane does not include every control point",
        ));
    }
    let PrimitiveParams::Plane { normal, offset } = candidate.params else {
        return Err(fail(
            CANONICAL_CAST_NO_FIT,
            "Plane detector returned another primitive",
        ));
    };
    let normal = unit(normal)
        .ok_or_else(|| fail(CANONICAL_CAST_NO_FIT, "Best-fit plane normal is degenerate"))?;
    let max_deviation_mm = points
        .iter()
        .map(|point| (dot(*point, normal) + offset).abs())
        .fold(0., f64::max);
    if !max_deviation_mm.is_finite() {
        return Err(fail(
            CANONICAL_CAST_NO_FIT,
            "Non-finite control-net deviation",
        ));
    }
    if max_deviation_mm >= budget.tolerance_mm {
        return Err(fail(
            CANONICAL_CAST_DEVIATION,
            format!(
                "Maximum control-net deviation {max_deviation_mm} mm is not strictly below {} mm",
                budget.tolerance_mm
            ),
        ));
    }
    Ok(PlaneFit {
        normal,
        offset,
        max_deviation_mm,
    })
}

fn source_normal(surface: &Surface) -> Option<[f64; 3]> {
    let u =
        (surface.knots_u[surface.degree_u] + surface.knots_u[surface.control_points.len()]) * 0.5;
    let v = (surface.knots_v[surface.degree_v] + surface.knots_v[surface.control_points[0].len()])
        * 0.5;
    let evaluation = surface.evaluate(u, v).ok()?;
    let (du, dv) = evaluation.first_derivatives()?;
    unit(cross(du, dv))
}

fn target_plane(model: &Model, face: usize, fit: PlaneFit, tolerance_mm: f64) -> Result<Surface> {
    let surface = &model.faces[face].surface;
    let points = control_points(surface)?;
    let centroid = points.iter().fold([0.; 3], |sum, point| {
        add(sum, mul(*point, 1. / points.len() as f64))
    });
    let mut normal = fit.normal;
    let mut offset = fit.offset;
    if source_normal(surface).is_some_and(|source| dot(source, normal) < 0.) {
        normal = mul(normal, -1.);
        offset = -offset;
    }
    let origin = sub(centroid, mul(normal, dot(centroid, normal) + offset));
    let u =
        (surface.knots_u[surface.degree_u] + surface.knots_u[surface.control_points.len()]) * 0.5;
    let v = (surface.knots_v[surface.degree_v] + surface.knots_v[surface.control_points[0].len()])
        * 0.5;
    let derivatives = surface
        .evaluate(u, v)
        .ok()
        .and_then(|evaluation| evaluation.first_derivatives().map(|(du, dv)| (du, dv)));
    let axis_u = derivatives
        .and_then(|(du, dv)| {
            unit(sub(du, mul(normal, dot(du, normal))))
                .or_else(|| unit(sub(dv, mul(normal, dot(dv, normal)))))
        })
        .or_else(|| {
            let seed = if normal[0].abs() < 0.9 {
                [1., 0., 0.]
            } else {
                [0., 1., 0.]
            };
            unit(cross(normal, seed))
        })
        .ok_or_else(|| fail(CANONICAL_CAST_NO_FIT, "Cannot construct a plane UV frame"))?;
    let axis_v = cross(normal, axis_u);

    let mut minimum = [f64::INFINITY; 2];
    let mut maximum = [f64::NEG_INFINITY; 2];
    for &wire in std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes) {
        for coedge in &model.loops[wire].coedges {
            for point in &model.edges[coedge.edge].curve.control_points {
                if point.len() != 3 || point.iter().any(|value| !value.is_finite()) {
                    return Err(fail(
                        CANONICAL_CAST_UNSUPPORTED,
                        "Canonical cast requires finite 3D edge control points",
                    ));
                }
                let delta = sub([point[0], point[1], point[2]], origin);
                let uv = [dot(delta, axis_u), dot(delta, axis_v)];
                for axis in 0..2 {
                    minimum[axis] = minimum[axis].min(uv[axis]);
                    maximum[axis] = maximum[axis].max(uv[axis]);
                }
            }
        }
    }
    let span = [maximum[0] - minimum[0], maximum[1] - minimum[1]];
    if span
        .iter()
        .any(|value| !value.is_finite() || *value <= 1e-12)
    {
        return Err(fail(
            CANONICAL_CAST_UNSUPPORTED,
            "Trim control hull has a degenerate planar extent",
        ));
    }
    let patch_origin = add(
        origin,
        add(
            mul(axis_u, minimum[0] - tolerance_mm),
            mul(axis_v, minimum[1] - tolerance_mm),
        ),
    );
    nurbs_core::patches::plane(
        patch_origin,
        mul(axis_u, span[0] + 2. * tolerance_mm),
        mul(axis_v, span[1] + 2. * tolerance_mm),
    )
    .map_err(|error| {
        fail(
            CANONICAL_CAST_NO_FIT,
            format!("Canonical plane construction failed: {}", error.message),
        )
    })
}

fn reproject_trims(
    model: &Model,
    face: usize,
    target: &Surface,
    budget: CanonicalCastBudget,
) -> Result<Vec<(usize, usize, Coedge)>> {
    let mut replacements = Vec::new();
    for &wire in std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes) {
        for (index, coedge) in model.loops[wire].coedges.iter().enumerate() {
            if coedge.trim().is_some() {
                return Err(fail(
                    CANONICAL_CAST_UNSUPPORTED,
                    "Subcurve coedge trims are outside the canonical-cast projection scope",
                ));
            }
            let edge = &model.edges[coedge.edge];
            let oriented = if coedge.reversed {
                edge.curve.reverse().map_err(|error| {
                    fail(
                        CANONICAL_CAST_PROJECTION,
                        format!("Edge reversal failed: {}", error.message),
                    )
                })?
            } else {
                edge.curve.clone()
            };
            let projection = curve_pullback::project(
                &oriented,
                target,
                curve_pullback::Limits {
                    tolerance: model.tolerance_mm,
                    max_segments: 256,
                    max_iterations: 32,
                    agreement_cells: budget.max_agreement_cells,
                },
            )
            .map_err(|error| {
                fail(
                    CANONICAL_CAST_PROJECTION,
                    format!("Pcurve projection failed: {}", error.message),
                )
            })?;
            if !projection.within_tolerance {
                return Err(fail(
                    CANONICAL_CAST_PROJECTION,
                    format!(
                        "Full-interval projection of edge {} was not certified",
                        coedge.edge
                    ),
                ));
            }
            let pcurve = projection.pcurve.ok_or_else(|| {
                fail(
                    CANONICAL_CAST_PROJECTION,
                    "Certified projection has no pcurve",
                )
            })?;
            replacements.push((
                wire,
                index,
                Coedge::new(coedge.edge, coedge.reversed, pcurve),
            ));
        }
    }
    Ok(replacements)
}

fn unsupported_rotational(class: SurfaceClass) -> Result<()> {
    if matches!(class, SurfaceClass::Cylinder | SurfaceClass::Cone) {
        Err(fail(
            CANONICAL_CAST_UNSUPPORTED,
            format!(
                "Near-{class:?} canonical cast is not implemented; no pcurves or topology were changed"
            ),
        ))
    } else {
        Ok(())
    }
}

fn check_area_delta(before: f64, after: f64, cap: f64) -> Result<f64> {
    let delta = after - before;
    if !delta.is_finite() || !before.is_finite() || !after.is_finite() {
        return Err(fail(
            CANONICAL_CAST_AREA_DELTA,
            "Non-finite trimmed-face area",
        ));
    }
    if delta > cap {
        return Err(fail(
            CANONICAL_CAST_AREA_GREW,
            format!("Face area grew by {delta} mm^2, above cap {cap} mm^2; cast rolled back"),
        ));
    }
    if delta.abs() > cap {
        return Err(fail(
            CANONICAL_CAST_AREA_DELTA,
            format!(
                "Absolute face-area delta {} mm^2 exceeds cap {cap} mm^2; cast rolled back",
                delta.abs()
            ),
        ));
    }
    Ok(delta)
}

/// Cast eligible near-plane faces to exact affine plane carriers.
///
/// The input is never mutated. Cylinder/cone targets and subcurve coedge trims
/// are typed unsupported refusals. Existing exact affine planes and already
/// analytic rotational faces are left untouched, making replay idempotent.
pub fn cast_canonical_surfaces(model: &Model, budget: CanonicalCastBudget) -> Result<HealingRun> {
    if !budget.tolerance_mm.is_finite()
        || budget.tolerance_mm <= 0.
        || !budget.area_delta_cap_mm2.is_finite()
        || budget.area_delta_cap_mm2 < 0.
        || !(1..=4096).contains(&budget.max_candidates)
        || !(1..=4096).contains(&budget.ransac_iterations)
        || !(1..=1_000_000).contains(&budget.max_agreement_cells)
    {
        return Err(fail(
            CANONICAL_CAST_BUDGET,
            "Require positive finite tolerance, non-negative area cap and bounded candidate/RANSAC/projection work",
        ));
    }
    model.validate().map_err(|error| {
        fail(
            CANONICAL_CAST_VALIDATION,
            format!("Invalid source model: {}", error.message),
        )
    })?;

    let mut candidates = Vec::new();
    for face in 0..model.faces.len() {
        let surface = &model.faces[face].surface;
        if exact_affine_plane(surface) {
            continue;
        }
        let candidate = classify_surface(surface, budget.tolerance_mm, &budget.work)?;
        if matches!(candidate.class, SurfaceClass::Cylinder | SurfaceClass::Cone) {
            let strict = classify_surface(surface, model.tolerance_mm.max(1e-7), &budget.work)?;
            if strict.class == candidate.class {
                continue;
            }
            unsupported_rotational(candidate.class)?;
        }
        if candidate.class == SurfaceClass::Plane {
            candidates.push(face);
        }
    }
    if candidates.len() > budget.max_candidates {
        return Err(fail(
            CANONICAL_CAST_BUDGET,
            format!(
                "Canonical-cast candidates {} exceed budget {}",
                candidates.len(),
                budget.max_candidates
            ),
        ));
    }

    let mut staged = model.clone();
    let mut conversions = Vec::with_capacity(candidates.len());
    let mut before_deviation = 0_f64;
    let mut after_deviation = 0_f64;
    for face in candidates {
        let fit = fit_plane(&staged.faces[face].surface, budget)?;
        before_deviation = before_deviation.max(fit.max_deviation_mm);
        let target = target_plane(&staged, face, fit, budget.tolerance_mm)?;
        let replacements = reproject_trims(&staged, face, &target, budget)?;
        let area_before = face_area(&staged, face, &budget.work)?;
        let previous_surface = std::mem::replace(&mut staged.faces[face].surface, target);
        let previous_pcurves = replacements
            .iter()
            .map(|(wire, index, _)| {
                (
                    *wire,
                    *index,
                    staged.loops[*wire].coedges[*index].pcurve.clone(),
                )
            })
            .collect::<Vec<_>>();
        for (wire, index, replacement) in replacements {
            staged.loops[wire].coedges[index] = replacement;
        }
        let area_after = match face_area(&staged, face, &budget.work) {
            Ok(area) => area,
            Err(error) => {
                staged.faces[face].surface = previous_surface;
                for (wire, index, pcurve) in previous_pcurves {
                    staged.loops[wire].coedges[index].pcurve = pcurve;
                }
                return Err(error);
            }
        };
        let delta = check_area_delta(area_before, area_after, budget.area_delta_cap_mm2)?;
        staged.validate().map_err(|error| {
            fail(
                CANONICAL_CAST_VALIDATION,
                format!("Canonical cast validation failed: {}", error.message),
            )
        })?;
        let target_normal = fit.normal;
        let post = staged.faces[face]
            .surface
            .control_points
            .iter()
            .flatten()
            .map(|point| (dot([point[0], point[1], point[2]], target_normal) + fit.offset).abs())
            .fold(0., f64::max);
        if !post.is_finite() {
            return Err(fail(
                CANONICAL_CAST_VALIDATION,
                "Non-finite post-cast deviation",
            ));
        }
        after_deviation = after_deviation.max(post);
        conversions.push(HealingConversion {
            entity_id: model.1.faces[face],
            from: "nurbsBSpline".into(),
            to: "plane".into(),
            rationale: format!(
                "Best-fit max control deviation {:.9} mm; trimmed area delta {:.9} mm^2; pcurves re-projected",
                fit.max_deviation_mm, delta
            ),
        });
    }

    staged.validate().map_err(|error| {
        fail(
            CANONICAL_CAST_VALIDATION,
            format!("Final canonical-cast validation failed: {}", error.message),
        )
    })?;
    let fixed = conversions.len();
    if fixed == 0 {
        before_deviation = 0.;
        after_deviation = 0.;
    }
    let report = HealingReport {
        input_hash: model_hash(model),
        output_hash: model_hash(&staged),
        status: HealingStatus::Complete,
        operations: Vec::new(),
        gaps: HealingGapSummary {
            before_mm: before_deviation,
            after_mm: after_deviation,
        },
        sewn_edges: Vec::new(),
        conversions,
        displacement: Vec::new(),
        certificates: Vec::new(),
        displacement_budget_mm: 0.,
        cumulative_displacement_mm: 0.,
        unresolved_defects: Vec::new(),
        refusal_reasons: Vec::new(),
        tolerance_unification: None,
        stages: vec![HealingStageReport {
            stage: "canonicalCast".into(),
            fixed,
            skipped: 0,
            remaining: 0,
            skipped_details: Vec::new(),
        }],
    };
    report.validate()?;
    Ok(HealingRun {
        model: staged,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::healing_report::HEALING_REPORT_VERSION;

    fn options(tolerance_mm: f64, area_delta_cap_mm2: f64) -> CanonicalCastBudget {
        CanonicalCastBudget {
            tolerance_mm,
            area_delta_cap_mm2,
            max_candidates: 8,
            ..CanonicalCastBudget::default()
        }
    }

    /// A cubic patch whose authored boundary is exactly the original bottom
    /// square. Four symmetric interior poles make the best-fit plane z = 0,
    /// while the carrier itself is not planar.
    fn near_plane(amplitude: f64, interior_weight: f64) -> Model {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        model.faces[0].surface = Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: [vec![0.; 4], vec![1.; 4]].concat(),
            knots_v: [vec![0.; 4], vec![1.; 4]].concat(),
            control_points: (0..4)
                .map(|i| {
                    (0..4)
                        .map(|j| {
                            let interior = matches!((i, j), (1, 1) | (1, 2) | (2, 1) | (2, 2));
                            let sign = if (i + j) % 2 == 0 { 1. } else { -1. };
                            vec![
                                2. * j as f64 / 3.,
                                2. * i as f64 / 3.,
                                if interior { sign * amplitude } else { 0. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: (0..4)
                .map(|i| {
                    (0..4)
                        .map(|j| {
                            if matches!((i, j), (1, 1) | (1, 2) | (2, 1) | (2, 2)) {
                                interior_weight
                            } else {
                                1.
                            }
                        })
                        .collect()
                })
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        model.validate().unwrap();
        model
    }

    #[test]
    fn near_plane_success_reprojects_trims_preserves_ids_and_round_trips() {
        let model = near_plane(1e-4, 1.);
        let ids = model.1.clone();
        let edges = model.edges.clone();
        let vertices = model.vertices.clone();
        let old_pcurves = model.loops[model.faces[0].outer].coedges.clone();
        let area_before = face_area(&model, 0, &CanonicalCastBudget::default().work).unwrap();
        let run = cast_canonical_surfaces(&model, options(1e-3, 1e-6)).unwrap();
        let area_after = face_area(&run.model, 0, &CanonicalCastBudget::default().work).unwrap();

        assert!(exact_affine_plane(&run.model.faces[0].surface));
        assert!((area_after - area_before).abs() < 1e-6);
        assert!(
            area_after <= area_before + 1e-9,
            "{area_before} -> {area_after}"
        );
        assert_eq!(run.model.1, ids);
        assert_eq!(run.model.edges, edges);
        assert_eq!(run.model.vertices, vertices);
        assert_eq!(run.report.conversions.len(), 1);
        assert_eq!(run.report.conversions[0].entity_id, model.1.faces[0]);
        assert_eq!(run.report.conversions[0].to, "plane");
        assert_eq!(run.report.stages[0].stage, "canonicalCast");
        assert_eq!(run.report.stages[0].fixed, 1);
        assert!(run.report.gaps.before_mm >= 1e-4);
        assert!(run.report.gaps.after_mm <= 1e-12);
        assert!(
            run.model.loops[run.model.faces[0].outer]
                .coedges
                .iter()
                .zip(&old_pcurves)
                .any(|(after, before)| after.pcurve != before.pcurve)
        );
        run.model.validate().unwrap();
        run.model.validate_continuous(100_000).unwrap();
        assert_eq!(HEALING_REPORT_VERSION, 3);
        let json = run.report.to_json().unwrap();
        assert_eq!(HealingReport::from_json(&json).unwrap(), run.report);
    }

    #[test]
    fn replay_is_bit_identical_and_converts_nothing() {
        let model = near_plane(1e-4, 1.);
        let first = cast_canonical_surfaces(&model, options(1e-3, 1.)).unwrap();
        let second = cast_canonical_surfaces(&first.model, options(1e-3, 1.)).unwrap();
        assert!(second.report.conversions.is_empty());
        assert_eq!(second.report.stages[0].fixed, 0);
        assert_eq!(second.model, first.model);
        assert_eq!(model_hash(&second.model), model_hash(&first.model));
    }

    #[test]
    fn excessive_control_deviation_is_refused_atomically() {
        // Small interior weights keep evaluated samples near z = 0 while the
        // complete control net is deliberately much farther from the plane.
        let model = near_plane(1e-2, 1e-3);
        let before = model.clone();
        let error = cast_canonical_surfaces(&model, options(1e-3, 1.)).unwrap_err();
        assert_eq!(error.code, CANONICAL_CAST_DEVIATION);
        assert_eq!(model, before);
    }

    #[test]
    fn measured_area_delta_cap_rolls_back_atomically() {
        let model = near_plane(1e-2, 1.);
        let before = model.clone();
        let error = cast_canonical_surfaces(&model, options(1e-1, 0.)).unwrap_err();
        assert_eq!(error.code, CANONICAL_CAST_AREA_DELTA);
        assert_eq!(model, before);
    }

    #[test]
    fn area_growth_guard_is_typed_and_nan_guards_are_active() {
        // Orthogonal reprojection of a graph-like near-plane cannot grow its
        // trimmed area. Exercise the shared growth branch directly; the real
        // measured before/after integration gate is covered above.
        assert_eq!(
            check_area_delta(4., 4.1, 0.01).unwrap_err().code,
            CANONICAL_CAST_AREA_GREW
        );
        assert_eq!(
            check_area_delta(4., f64::NAN, 1.).unwrap_err().code,
            CANONICAL_CAST_AREA_DELTA
        );
        let model = near_plane(1e-4, 1.);
        let error = cast_canonical_surfaces(&model, options(f64::NAN, 1.)).unwrap_err();
        assert_eq!(error.code, CANONICAL_CAST_BUDGET);
    }

    #[test]
    fn cylinder_and_cone_targets_are_explicitly_unsupported() {
        for class in [SurfaceClass::Cylinder, SurfaceClass::Cone] {
            let error = unsupported_rotational(class).unwrap_err();
            assert_eq!(error.code, CANONICAL_CAST_UNSUPPORTED);
        }
        assert!(unsupported_rotational(SurfaceClass::Plane).is_ok());
    }
}
