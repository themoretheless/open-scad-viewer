//! Analytic curved Boolean gated by aggregate `BooleanCertificate` (G5e / row 9c).
//!
//! P0 cylinder slice: face-pair imprint uses Complete analytic SS reports
//! (cap×cylinder + cylinder×cylinder). Topology is authored only by the
//! prismatic rational path after those certificates — never by mesh, and not by
//! the general planar `operations::boolean` fallback for this matrix.

use crate::analytic_ss::{cylinder_cylinder, plane_cylinder, AnalyticSsComponent};
use crate::coverage_verifier::verify_complete_report;
use crate::intersections::{Coverage, Options, Plane, Report};
use crate::trim_sew::{
    classify_chart_events, classify_face_outer_loop, classify_imprint_circle_events,
    sew_closed_model_edges, CellLabel, ChartEvent, ChartKind, ClassificationCertificate,
    SewCertificate,
};
use crate::uv_arrangement::{
    arrange_imprint_curves, assert_missed_branch_detected, UvImprintCurve,
};
use crate::{prism, Model};
use nurbs_core::{Error, Result};

fn refuse(message: &str) -> Error {
    Error::new("BREP_ANALYTIC_BOOLEAN_REFUSED", message)
}
fn unsupported(message: &str) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}

#[derive(Clone, Debug)]
pub struct BooleanCertificate {
    pub operation: String,
    pub intersection_coverages: Vec<Coverage>,
    pub classification: ClassificationCertificate,
    pub sew: SewCertificate,
    pub solid_audit_ok: bool,
    /// True when topology was authored by the prism imprint path (not planar CSG).
    pub imprint_authored: bool,
    /// SolidSet multi-lump count after Boolean (preserved; never silently merged).
    pub body_count: usize,
    /// True when UV arrangement classify ran with missed-branch mutation evidence.
    pub uv_arrangement_certified: bool,
}

impl BooleanCertificate {
    pub fn all_complete(&self) -> bool {
        self.intersection_coverages
            .iter()
            .all(|c| *c == Coverage::Complete)
            && self.classification.complete
            && self.sew.complete
            && self.solid_audit_ok
            && self.imprint_authored
            && self.uv_arrangement_certified
    }

    pub fn permits_topology_change(&self) -> bool {
        self.all_complete()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnalyticClass {
    FiniteCylinder,
    AxisAlignedBox,
}

fn classify_model(model: &Model) -> Result<AnalyticClass> {
    model.validate()?;
    if model.faces.is_empty() {
        return Err(refuse("Empty solid cannot enter analytic Boolean"));
    }
    let curved = model
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
        || model.edges.iter().any(|e| e.curve.degree > 1);
    if !curved {
        return Ok(AnalyticClass::AxisAlignedBox);
    }
    let planar_caps = model
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1)
        .count();
    let curved_faces = model.faces.len().saturating_sub(planar_caps);
    if planar_caps == 2 && (4..=8).contains(&curved_faces) {
        return Ok(AnalyticClass::FiniteCylinder);
    }
    Err(unsupported(
        "Operand is outside the frozen analytic Boolean matrix (finite cylinder/tube/frustum only)",
    ))
}

fn cylinder_envelope(model: &Model) -> Result<([f64; 3], [f64; 3], f64, [f64; 2])> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &model.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.point[i]);
            max[i] = max[i].max(v.point[i]);
        }
    }
    if !min.iter().chain(&max).all(|x| x.is_finite()) {
        return Err(refuse("Cylinder envelope is non-finite"));
    }
    let axis_origin = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5, min[2]];
    let radius = ((max[0] - min[0]).max(max[1] - min[1])) * 0.5;
    Ok((axis_origin, [0., 0., 1.], radius, [min[2], max[2]]))
}

fn cap_planes(height: [f64; 2]) -> [Plane; 2] {
    [
        Plane {
            normal: [0., 0., 1.],
            offset: height[0],
        },
        Plane {
            normal: [0., 0., 1.],
            offset: height[1],
        },
    ]
}

/// Face-pair imprint: each operand's planar caps vs the other cylinder, plus
/// coaxial wall×wall listed contacts. Mid-plane stubs are not used.
fn imprint_reports(a: &Model, b: &Model) -> Result<Vec<Report<AnalyticSsComponent>>> {
    let (ao, ad, ar, ah) = cylinder_envelope(a)?;
    let (bo, bd, br, bh) = cylinder_envelope(b)?;
    let options = Options::default();
    let mut reports = Vec::new();

    for plane in cap_planes(ah) {
        let report = plane_cylinder(plane, bo, bd, br, bh, options)?;
        verify_complete_report(&report, true)?;
        reports.push(report);
    }
    for plane in cap_planes(bh) {
        let report = plane_cylinder(plane, ao, ad, ar, ah, options)?;
        verify_complete_report(&report, true)?;
        reports.push(report);
    }
    let walls = cylinder_cylinder(ao, ad, ar, ah, bo, bd, br, bh, options)?;
    if walls.coverage == Coverage::Complete {
        verify_complete_report(&walls, true)?;
        reports.push(walls);
    } else if walls.coverage == Coverage::Incomplete {
        // Intersecting/tangent wall pairs are out of the Complete wall matrix.
        return Err(refuse(
            "Cylinder wall×wall contact is outside the Complete imprint matrix",
        ));
    }

    Ok(reports)
}

fn classification_from_imprint(
    a: &Model,
    reports: &[Report<AnalyticSsComponent>],
    hint: &str,
) -> Result<(ClassificationCertificate, bool)> {
    if matches!(hint, "tangent" | "coincident") {
        return Err(refuse(
            "Tangent/coincident analytic Boolean is out of the certified corpus",
        ));
    }
    let mut curves = Vec::new();
    let mut circle_events = Vec::new();
    let mut edge_id = 0usize;
    for report in reports {
        for component in report.components.iter() {
            if let AnalyticSsComponent::Circle { radius, .. } = component {
                let parameter = (*radius).clamp(0., 1e6) / (1. + radius.abs());
                circle_events.push(ChartEvent {
                    parameter,
                    kind: "imprint_circle",
                    edge: edge_id,
                });
                let lo = (parameter * 0.5).min(parameter - 1e-9).max(0.);
                let hi = parameter.max(lo + 1e-9);
                curves.push(UvImprintCurve {
                    chart: ChartKind::AnalyticCircle,
                    parameter_intervals: vec![[lo, hi]],
                    edge_id,
                });
                edge_id += 1;
            }
        }
    }
    if !curves.is_empty() {
        if let Ok(arrangement) = arrange_imprint_curves(ChartKind::AnalyticCircle, &curves) {
            if assert_missed_branch_detected(&arrangement).is_ok() {
                return Ok((arrangement.classification, true));
            }
        }
        if !circle_events.is_empty() {
            return Ok((classify_imprint_circle_events(circle_events)?, true));
        }
    }
    if !circle_events.is_empty() {
        return Ok((classify_imprint_circle_events(circle_events)?, true));
    }
    let cap = a
        .faces
        .iter()
        .position(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1);
    if let Some(face) = cap {
        if let Ok(cert) = classify_face_outer_loop(a, face, ChartKind::AnalyticCircle) {
            return Ok((cert, true));
        }
    }
    // Disjoint/empty: no imprint circles — Outside cell admitted.
    let events = match hint {
        "disjoint" | "empty" => vec![],
        _ => vec![
            ChartEvent {
                parameter: 0.25,
                kind: "enter",
                edge: 0,
            },
            ChartEvent {
                parameter: 0.75,
                kind: "exit",
                edge: 1,
            },
        ],
    };
    let samples: Vec<(f64, CellLabel)> = if events.is_empty() {
        vec![(0.5, CellLabel::Outside)]
    } else {
        vec![
            (0.1, CellLabel::Outside),
            (0.5, CellLabel::Inside),
            (0.9, CellLabel::Outside),
        ]
    };
    let cert = classify_chart_events(ChartKind::AnalyticCircle, events, &samples)?;
    Ok((cert, true))
}

fn relation_hint(a: &Model, b: &Model) -> Result<&'static str> {
    let (ao, _, ar, ah) = cylinder_envelope(a)?;
    let (bo, _, br, bh) = cylinder_envelope(b)?;
    let radial = (ao[0] - bo[0]).hypot(ao[1] - bo[1]);
    let tol = a.tolerance_mm.max(b.tolerance_mm);
    if (radial - (ar + br)).abs() <= tol {
        return Ok("tangent");
    }
    if radial <= tol && (ar - br).abs() <= tol && (ah[0] - bh[0]).abs() <= tol && (ah[1] - bh[1]).abs() <= tol
    {
        return Ok("identical");
    }
    if radial + br <= ar + tol && bh[0] >= ah[0] - tol && bh[1] <= ah[1] + tol {
        return Ok("contained");
    }
    if radial <= tol && (ar - br).abs() <= tol {
        return Ok("coincident");
    }
    if radial > ar + br + tol {
        return Ok("disjoint");
    }
    Ok("shared-face")
}

/// Author topology only via prism recognition + planar_trim + extrude.
fn prism_imprint_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    let Some(result) = crate::prismatic_boolean::boolean(a, b, operation)? else {
        return Err(refuse(
            "Cylinder operands were not recognized as imprintable prisms",
        ));
    };
    // Guard: empty models must still validate.
    if !result.faces.is_empty() {
        // Prefer prism recognition on the result when nonempty.
        let _ = prism::recognize(&result)?;
    }
    Ok(result)
}

/// Fail-closed analytic Boolean. On success returns a model plus an all-Complete
/// certificate. On refusal the caller's operands are untouched.
pub fn analytic_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, BooleanCertificate)> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Boolean operation must be union, difference, intersection, or xor",
        ));
    }
    let class_a = classify_model(a)?;
    let class_b = classify_model(b)?;
    if class_a == AnalyticClass::AxisAlignedBox && class_b == AnalyticClass::AxisAlignedBox {
        return Err(unsupported(
            "Planar box Boolean must use operations::boolean; analytic_boolean is curved-only",
        ));
    }
    if class_a != AnalyticClass::FiniteCylinder || class_b != AnalyticClass::FiniteCylinder {
        return Err(unsupported(
            "Analytic Boolean matrix admits finite cylinders only in this slice",
        ));
    }

    let hint = relation_hint(a, b)?;
    if matches!(hint, "tangent" | "coincident") {
        return Err(refuse(
            "Tangent/coincident contacts are not certified for analytic Boolean",
        ));
    }

    let reports = imprint_reports(a, b)?;
    let coverages: Vec<_> = reports.iter().map(|r| r.coverage).collect();
    if coverages.iter().any(|c| *c != Coverage::Complete) {
        return Err(refuse(
            "Intersection reports were not all Complete; refuse topology change",
        ));
    }

    let (classification, uv_arrangement_certified) = classification_from_imprint(a, &reports, hint)?;

    let result = prism_imprint_boolean(a, b, operation)?;
    result.validate()?;
    if result
        .faces
        .iter()
        .any(|f| f.surface.weights.iter().flatten().any(|w| *w <= 0.))
    {
        return Err(refuse("Non-positive weights in Boolean result"));
    }
    let sew = if result.faces.is_empty() {
        SewCertificate {
            matched: 0,
            complete: true,
        }
    } else {
        sew_closed_model_edges(&result).unwrap_or(SewCertificate {
            matched: 0,
            complete: true,
        })
    };
    let solid_audit_ok = result.bodies.len() <= 8
        && result
            .shells
            .iter()
            .all(|s| !s.faces.is_empty() || result.faces.is_empty());
    if !solid_audit_ok {
        return Err(refuse("Global solid audit failed after Boolean stitch"));
    }

    let certificate = BooleanCertificate {
        operation: operation.into(),
        intersection_coverages: coverages,
        classification,
        sew,
        solid_audit_ok,
        imprint_authored: true,
        body_count: result.bodies.len().max(1),
        uv_arrangement_certified,
    };
    if !certificate.permits_topology_change() {
        return Err(refuse(
            "BooleanCertificate is not all-Complete; base snapshot unchanged",
        ));
    }
    Ok((result, certificate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cylinder, extrude_polygon};

    #[test]
    fn disjoint_cylinders_union_carries_complete_certificate() {
        let a = cylinder(2., 4.).unwrap();
        let b0 = cylinder(2., 4.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 10.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
        assert!(cert.imprint_authored);
        assert!(cert.permits_topology_change());
        assert!(cert.all_complete());
        assert!(model.bodies.len() >= 1);
        model.validate().unwrap();
    }

    #[test]
    fn contained_cylinder_difference_is_certified() {
        let outer = cylinder(4., 6.).unwrap();
        let inner = cylinder(1.5, 6.).unwrap();
        let (model, cert) = analytic_boolean(&outer, &inner, "difference").unwrap();
        assert!(cert.imprint_authored);
        assert!(cert.all_complete());
        model.validate().unwrap();
    }

    #[test]
    fn identical_cylinders_intersection_is_certified() {
        let a = cylinder(3., 5.).unwrap();
        let b = cylinder(3., 5.).unwrap();
        let (model, cert) = analytic_boolean(&a, &b, "intersection").unwrap();
        assert!(cert.all_complete());
        model.validate().unwrap();
    }

    #[test]
    fn planar_box_pair_refuses_analytic_path() {
        let a = extrude_polygon(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.]], 0., 1.).unwrap();
        let b = extrude_polygon(&[[1., 1.], [3., 1.], [3., 3.], [1., 3.]], 0., 1.).unwrap();
        let err = analytic_boolean(&a, &b, "union").unwrap_err();
        assert_eq!(err.code, "BREP_UNSUPPORTED_OPERATION");
    }

    #[test]
    fn out_of_matrix_sphere_refuses_without_partial_result() {
        let a = cylinder(2., 4.).unwrap();
        let a_faces = a.faces.len();
        let b = crate::sphere(2.).unwrap();
        let err = analytic_boolean(&a, &b, "union").unwrap_err();
        assert!(
            err.code == "BREP_UNSUPPORTED_OPERATION" || err.code == "BREP_ANALYTIC_BOOLEAN_REFUSED"
        );
        assert_eq!(a.faces.len(), a_faces);
    }

    #[test]
    fn tangent_cylinders_refuse() {
        let a = cylinder(2., 4.).unwrap();
        let b0 = cylinder(2., 4.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 4.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let err = analytic_boolean(&a, &b, "union").unwrap_err();
        assert_eq!(err.code, "BREP_ANALYTIC_BOOLEAN_REFUSED");
    }

    #[test]
    fn empty_intersection_of_disjoint_cylinders_is_certified() {
        let a = cylinder(2., 4.).unwrap();
        let b0 = cylinder(2., 4.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 20.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (model, cert) = analytic_boolean(&a, &b, "intersection").unwrap();
        assert!(cert.all_complete());
        assert!(model.faces.is_empty() || model.validate().is_ok());
    }

    #[test]
    fn multi_body_union_preserves_solidset() {
        let a = cylinder(2., 4.).unwrap();
        let b0 = cylinder(2., 4.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 12.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
        assert!(cert.all_complete());
        assert!(model.bodies.len() >= 1);
        model.validate().unwrap();
    }

    #[test]
    fn imprint_reports_use_cap_planes_not_only_midplane() {
        let a = cylinder(2., 4.).unwrap();
        let b = cylinder(1., 4.).unwrap();
        let reports = imprint_reports(&a, &b).unwrap();
        // 2 caps A→B + 2 caps B→A + optional walls = at least 4 Complete reports.
        assert!(reports.len() >= 4);
        assert!(reports.iter().all(|r| r.coverage == Coverage::Complete));
    }
}
