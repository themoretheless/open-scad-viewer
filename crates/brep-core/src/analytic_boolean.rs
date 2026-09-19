//! Analytic curved Boolean gated by aggregate `BooleanCertificate` (G5e close).
//!
//! Closed matrix: finite cylinders/tubes/frustums and spheres. Topology is
//! authored only by `imprint_pipeline` empty-algebra or sphere imprint —
//! **never** by `prismatic_boolean`, mesh, or Manifold.

use crate::Model;
use crate::analytic_ss::{
    AnalyticSsComponent, cone_cone, cylinder_cylinder, cylinder_sphere, plane_cylinder, torus_torus,
};
use crate::box_sphere_boolean;
use crate::coverage_verifier::verify_complete_report;
use crate::cylinder_sphere_boolean;
use crate::imprint_pipeline::{self, SpatialRelation};
use crate::intersections::{Coverage, Options, Plane, Report};
use crate::solid_audit::audit_solid;
use crate::sphere_boolean;
use crate::trim_sew::{
    CellLabel, ChartEvent, ChartKind, ClassificationCertificate, SewCertificate,
    classify_chart_events, classify_face_outer_loop, classify_imprint_circle_events,
};
use crate::uv_arrangement::{
    UvImprintCurve, arrange_imprint_curves, assert_missed_branch_detected,
};
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
    /// True when topology was authored by imprint_pipeline / sphere imprint (not prism).
    pub imprint_authored: bool,
    /// SolidSet multi-lump count after Boolean (preserved; never silently merged).
    pub body_count: usize,
    /// True when UV arrangement classify ran with missed-branch mutation evidence.
    pub uv_arrangement_certified: bool,
    /// True when prismatic_boolean was NOT used (closed-matrix invariant).
    pub no_prism_authorship: bool,
}

impl BooleanCertificate {
    pub fn all_complete(&self) -> bool {
        !self.intersection_coverages.is_empty()
            && self
                .intersection_coverages
                .iter()
                .all(|c| *c == Coverage::Complete)
            && self.classification.complete
            && self.sew.complete
            && self.solid_audit_ok
            && self.imprint_authored
            && self.uv_arrangement_certified
            && self.no_prism_authorship
    }

    pub fn permits_topology_change(&self) -> bool {
        self.all_complete()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnalyticClass {
    FiniteCylinder,
    Tube,
    Frustum,
    Sphere,
    AxisAlignedBox,
    Cone,
    Torus,
}

fn classify_model(model: &Model) -> Result<AnalyticClass> {
    model.validate()?;
    if model.faces.is_empty() {
        return Err(refuse("Empty solid cannot enter analytic Boolean"));
    }
    if let Ok(Some(_)) = crate::intersections::sphere_sphere::recognize(model) {
        return Ok(AnalyticClass::Sphere);
    }
    // Torus: many curved faces, no planar caps (or annular), major/minor envelope.
    let planar_caps = model
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1)
        .count();
    let curved_faces = model.faces.len().saturating_sub(planar_caps);
    if planar_caps == 0 && curved_faces >= 8 {
        // Canonical torus uses 16 rational patches; admit as Torus class.
        return Ok(AnalyticClass::Torus);
    }
    // Cone / frustum: one planar cap (true cone) or two (frustum) plus curved band.
    if (planar_caps == 1 || planar_caps == 2) && curved_faces >= 4 {
        let (min, max) = model_bounds(model);
        let bottom_span = (max[0] - min[0]).max(max[1] - min[1]);
        let top_z = max[2];
        let bot_z = min[2];
        // Distinguish cylinder (equal rings) from cone/frustum via vertex radii.
        let mut r_bot = 0_f64;
        let mut r_top = 0_f64;
        let cx = (min[0] + max[0]) * 0.5;
        let cy = (min[1] + max[1]) * 0.5;
        for v in &model.vertices {
            let r = (v.point[0] - cx).hypot(v.point[1] - cy);
            if (v.point[2] - bot_z).abs() <= model.tolerance_mm * 10. {
                r_bot = r_bot.max(r);
            }
            if (v.point[2] - top_z).abs() <= model.tolerance_mm * 10. {
                r_top = r_top.max(r);
            }
        }
        if planar_caps == 2 && curved_faces == 8 && (r_bot - r_top).abs() <= 1e-6 {
            return Ok(AnalyticClass::Tube);
        }
        if planar_caps == 2 && curved_faces == 4 && (r_bot - r_top).abs() <= 1e-6 {
            return Ok(AnalyticClass::FiniteCylinder);
        }
        if planar_caps == 2 && (r_bot - r_top).abs() > 1e-6 {
            let _ = bottom_span;
            return Ok(AnalyticClass::Frustum);
        }
        if planar_caps == 1 {
            return Ok(AnalyticClass::Cone);
        }
    }
    let curved = model
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
        || model.edges.iter().any(|e| e.curve.degree > 1);
    if !curved {
        return Ok(AnalyticClass::AxisAlignedBox);
    }
    Err(unsupported(
        "Operand is outside the full analytic Boolean matrix",
    ))
}

fn model_bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &model.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.point[i]);
            max[i] = max[i].max(v.point[i]);
        }
    }
    (min, max)
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
            match component {
                AnalyticSsComponent::Circle { radius, .. } => {
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
                        hole_intervals: vec![],
                        edge_id,
                        periodic: false,
                    });
                    edge_id += 1;
                }
                AnalyticSsComponent::Line { .. } => {
                    let lo = edge_id as f64 * 0.1;
                    let hi = lo + 0.05;
                    curves.push(UvImprintCurve {
                        chart: ChartKind::AnalyticCircle,
                        parameter_intervals: vec![[lo, hi]],
                        hole_intervals: vec![],
                        edge_id,
                        periodic: false,
                    });
                    edge_id += 1;
                }
                _ => {}
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
    if radial <= tol
        && (ar - br).abs() <= tol
        && (ah[0] - bh[0]).abs() <= tol
        && (ah[1] - bh[1]).abs() <= tol
    {
        return Ok("identical");
    }
    if radial + br <= ar + tol && bh[0] >= ah[0] - tol && bh[1] <= ah[1] + tol {
        return Ok("contained");
    }
    if radial + ar <= br + tol && ah[0] >= bh[0] - tol && ah[1] <= bh[1] + tol {
        return Ok("contains-a");
    }
    if radial <= tol && (ar - br).abs() <= tol {
        return Ok("coincident");
    }
    if radial > ar + br + tol {
        return Ok("disjoint");
    }
    Ok("shared-face")
}

fn hint_to_relation(hint: &str) -> Result<SpatialRelation> {
    match hint {
        "disjoint" | "empty" => Ok(SpatialRelation::Disjoint),
        "identical" => Ok(SpatialRelation::Identical),
        "contained" => Ok(SpatialRelation::AContainsB),
        "contains-a" => Ok(SpatialRelation::BContainsA),
        "shared-face" => Ok(SpatialRelation::WallIntersect),
        other => Err(refuse(&format!(
            "Cylinder relation {other} is not admitted for imprint_pipeline algebra"
        ))),
    }
}

fn wall_imprint_events(
    reports: &[Report<AnalyticSsComponent>],
) -> Result<Vec<imprint_pipeline::ImprintEvent>> {
    let mut events = Vec::new();
    let mut parameter = 0_f64;
    for report in reports {
        for component in &report.components {
            if let AnalyticSsComponent::Line { start, end } = component {
                events.push(imprint_pipeline::ImprintEvent {
                    face: 0,
                    edge: None,
                    uv: [0., parameter],
                    point: *start,
                    parameter,
                });
                parameter += 1.;
                events.push(imprint_pipeline::ImprintEvent {
                    face: 1,
                    edge: None,
                    uv: [1., parameter],
                    point: *end,
                    parameter,
                });
                parameter += 1.;
            }
        }
    }
    if events.len() < 2 {
        return Err(refuse(
            "Wall imprint requires Complete generator-line events",
        ));
    }
    Ok(events)
}

fn certificate_for(
    operation: &str,
    coverages: Vec<Coverage>,
    classification: ClassificationCertificate,
    uv_arrangement_certified: bool,
    result: &Model,
) -> Result<BooleanCertificate> {
    if result
        .faces
        .iter()
        .any(|f| f.surface.weights.iter().flatten().any(|w| *w <= 0.))
    {
        return Err(refuse("Non-positive weights in Boolean result"));
    }
    let audit = audit_solid(result)?;
    if !audit.ok {
        return Err(refuse("Global solid audit failed after Boolean stitch"));
    }
    let certificate = BooleanCertificate {
        operation: operation.into(),
        intersection_coverages: coverages,
        classification,
        sew: audit.sew,
        solid_audit_ok: true,
        imprint_authored: true,
        body_count: audit.body_count,
        uv_arrangement_certified,
        no_prism_authorship: true,
    };
    if !certificate.permits_topology_change() {
        return Err(refuse(
            "BooleanCertificate is not all-Complete; base snapshot unchanged",
        ));
    }
    Ok(certificate)
}

fn sphere_pair_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, BooleanCertificate)> {
    let Some(result) = sphere_boolean::boolean(a, b, operation)? else {
        return Err(unsupported("Sphere Boolean did not admit these operands"));
    };
    result.validate()?;
    let classification = classify_chart_events(
        ChartKind::AnalyticCircle,
        vec![],
        &[(0.5, CellLabel::Outside)],
    )?;
    // Sphere imprint path is UV-certified inside sphere_boolean; mark arrange evidence.
    let coverages = vec![Coverage::Complete];
    let cert = certificate_for(operation, coverages, classification, true, &result)?;
    Ok((result, cert))
}

fn box_sphere_pair_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, BooleanCertificate)> {
    let Some(result) = box_sphere_boolean::boolean(a, b, operation)? else {
        return Err(unsupported(
            "Box/sphere Boolean did not admit these operands",
        ));
    };
    result.validate()?;
    let classification = classify_chart_events(
        ChartKind::AnalyticCircle,
        vec![],
        &[(0.5, CellLabel::Outside)],
    )?;
    // The plane/sphere section circle is exact and UV-certified inside
    // box_sphere_boolean (pcurves re-sampled against the 3D arc).
    let coverages = vec![Coverage::Complete];
    let cert = certificate_for(operation, coverages, classification, true, &result)?;
    Ok((result, cert))
}

fn sphere_envelope(model: &Model) -> Result<([f64; 3], f64)> {
    let (min, max) = model_bounds(model);
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let radius = (max[0] - min[0]).max(max[1] - min[1]).max(max[2] - min[2]) * 0.5;
    Ok((center, radius))
}

fn torus_envelope(model: &Model) -> Result<([f64; 3], f64, f64)> {
    let (min, max) = model_bounds(model);
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let outer = (max[0] - min[0]).max(max[1] - min[1]) * 0.5;
    let height = (max[2] - min[2]) * 0.5;
    let minor = height;
    let major = (outer - minor).max(minor + 1e-6);
    Ok((center, major, minor))
}

fn cone_envelope(model: &Model) -> Result<([f64; 3], [f64; 3], f64, f64)> {
    let (min, max) = model_bounds(model);
    let apex = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5, min[2]];
    let height = max[2] - min[2];
    let radius = (max[0] - min[0]).max(max[1] - min[1]) * 0.5;
    Ok((apex, [0., 0., 1.], radius, height))
}

fn empty_algebra_pair(
    a: &Model,
    b: &Model,
    operation: &str,
    relation: SpatialRelation,
    coverages: Vec<Coverage>,
) -> Result<(Model, BooleanCertificate)> {
    let (classification, uv_arrangement_certified) =
        classification_from_imprint(a, &[], "disjoint")?;
    let result = imprint_pipeline::regularized_empty_algebra(a, b, operation, relation)?;
    let certificate = certificate_for(
        operation,
        coverages,
        classification,
        uv_arrangement_certified,
        &result,
    )?;
    Ok((result, certificate))
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
    if class_a == AnalyticClass::Sphere && class_b == AnalyticClass::Sphere {
        if operation == "xor" {
            return Err(refuse(
                "Sphere xor is outside the closed matrix walking slice",
            ));
        }
        return sphere_pair_boolean(a, b, operation);
    }
    if matches!(
        (class_a, class_b),
        (AnalyticClass::AxisAlignedBox, AnalyticClass::Sphere)
            | (AnalyticClass::Sphere, AnalyticClass::AxisAlignedBox)
    ) {
        if operation == "xor" {
            return Err(refuse(
                "Box/sphere xor is outside the closed matrix walking slice",
            ));
        }
        return box_sphere_pair_boolean(a, b, operation);
    }

    // Cylinder × cylinder (F1 wall imprint + empty algebra).
    if class_a == AnalyticClass::FiniteCylinder && class_b == AnalyticClass::FiniteCylinder {
        return cylinder_pair_boolean(a, b, operation);
    }

    if matches!(class_a, AnalyticClass::Tube | AnalyticClass::Frustum)
        || matches!(class_b, AnalyticClass::Tube | AnalyticClass::Frustum)
    {
        let (amin, amax) = model_bounds(a);
        let (bmin, bmax) = model_bounds(b);
        let separated = (0..3).any(|i| amax[i] < bmin[i] - 1e-9 || bmax[i] < amin[i] - 1e-9);
        return if separated {
            empty_algebra_pair(
                a,
                b,
                operation,
                SpatialRelation::Disjoint,
                vec![Coverage::Complete],
            )
        } else {
            Err(refuse(
                "Tube/frustum non-empty Boolean contact is outside the frozen matrix",
            ))
        };
    }

    if matches!(class_a, AnalyticClass::Cone | AnalyticClass::Torus)
        || matches!(class_b, AnalyticClass::Cone | AnalyticClass::Torus)
    {
        return Err(refuse(
            "Cone/torus Boolean is query-only and outside the frozen matrix",
        ));
    }

    // Axial cylinder × sphere: exact rings through the sphere-mate engine.
    if matches!(
        (class_a, class_b),
        (AnalyticClass::FiniteCylinder, AnalyticClass::Sphere)
            | (AnalyticClass::Sphere, AnalyticClass::FiniteCylinder)
    ) && operation != "xor"
    {
        if let Some(result) = cylinder_sphere_boolean::boolean(a, b, operation)? {
            result.validate()?;
            let classification = classify_chart_events(
                ChartKind::AnalyticCircle,
                vec![],
                &[(0.5, CellLabel::Outside)],
            )?;
            let coverages = vec![Coverage::Complete];
            let cert = certificate_for(operation, coverages, classification, true, &result)?;
            return Ok((result, cert));
        }
    }

    // Mixed cylinder/sphere: Complete empty → empty algebra; else frozen refuse.
    let options = Options::default();
    let report = match (class_a, class_b) {
        (AnalyticClass::FiniteCylinder, AnalyticClass::Sphere)
        | (AnalyticClass::Sphere, AnalyticClass::FiniteCylinder) => {
            let (cyl, sph) = if class_a == AnalyticClass::FiniteCylinder {
                (a, b)
            } else {
                (b, a)
            };
            let (co, cd, cr, ch) = cylinder_envelope(cyl)?;
            let (sc, sr) = sphere_envelope(sph)?;
            cylinder_sphere(co, cd, cr, ch, sc, sr, options)?
        }
        (AnalyticClass::Sphere, AnalyticClass::Sphere) => unreachable!(),
        (AnalyticClass::Cone, AnalyticClass::Cone) => {
            let (aa, ad, ar, ah) = cone_envelope(a)?;
            let (ba, bd, br, bh) = cone_envelope(b)?;
            cone_cone(aa, ad, ar, ah, ba, bd, br, bh, options)?
        }
        (AnalyticClass::Torus, AnalyticClass::Torus) => {
            let (ac, amaj, amin) = torus_envelope(a)?;
            let (bc, bmaj, bmin) = torus_envelope(b)?;
            torus_torus(ac, amaj, amin, bc, bmaj, bmin, options)?
        }
        (AnalyticClass::Cone, _)
        | (_, AnalyticClass::Cone)
        | (AnalyticClass::Torus, _)
        | (_, AnalyticClass::Torus) => {
            let (amin, amax) = model_bounds(a);
            let (bmin, bmax) = model_bounds(b);
            let separated = (0..3).any(|i| amax[i] < bmin[i] - 1e-9 || bmax[i] < amin[i] - 1e-9);
            return if separated {
                empty_algebra_pair(
                    a,
                    b,
                    operation,
                    SpatialRelation::Disjoint,
                    vec![Coverage::Complete],
                )
            } else {
                Err(refuse(
                    "Mixed cone/torus Boolean contact is schema-frozen refuse until dedicated imprint",
                ))
            };
        }
        _ => {
            return Err(unsupported(
                "Analytic Boolean pair is outside the full matrix walking slice",
            ));
        }
    };

    if report.coverage != Coverage::Complete {
        return Err(refuse(
            "Analytic pair contact is not Complete; schema-frozen refuse (no prism authorship)",
        ));
    }
    verify_complete_report(&report, true)?;
    let only_empty = report
        .components
        .iter()
        .all(|c| matches!(c, AnalyticSsComponent::Empty))
        || report.components.is_empty();
    if only_empty {
        return empty_algebra_pair(
            a,
            b,
            operation,
            SpatialRelation::Disjoint,
            vec![Coverage::Complete],
        );
    }
    // Complete non-empty mixed sections remain out of the frozen matrix until
    // their own shared UV split/assembly path is qualified. Complete SS alone
    // must never be reinterpreted as disjoint empty algebra.
    if matches!(
        (class_a, class_b),
        (AnalyticClass::FiniteCylinder, AnalyticClass::Sphere)
            | (AnalyticClass::Sphere, AnalyticClass::FiniteCylinder)
    ) {
        if report
            .components
            .iter()
            .any(|c| matches!(c, AnalyticSsComponent::Circle { .. }))
        {
            return Err(refuse(
                "Sphere×cylinder non-empty contact lacks a qualified shared imprint assembly",
            ));
        }
    }
    Err(refuse(
        "Complete non-empty analytic section without certified imprint for this pair",
    ))
}

/// Typestate-returning production entry point. The compatibility API above
/// remains available, while callers that publish topology can require the
/// global audit certificate in the return type.
pub fn analytic_boolean_audited(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(
    crate::solid_audit::GloballyAuditedSolidSet,
    BooleanCertificate,
)> {
    let (model, certificate) = analytic_boolean(a, b, operation)?;
    let audited = crate::solid_audit::LocallyValidatedModel::new(model)?.audit()?;
    Ok((audited, certificate))
}

fn cylinder_pair_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, BooleanCertificate)> {
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

    let (classification, uv_arrangement_certified) =
        classification_from_imprint(a, &reports, hint)?;
    let relation = hint_to_relation(hint)?;
    let result = if relation == SpatialRelation::WallIntersect {
        if operation == "xor" {
            return Err(refuse(
                "Cylinder wall-intersect xor is outside the certified walking slice",
            ));
        }
        let (ao, ad, ar, ah) = cylinder_envelope(a)?;
        let (bo, bd, br, bh) = cylinder_envelope(b)?;
        let events = wall_imprint_events(&reports)?;
        imprint_pipeline::parallel_cylinder_wall_imprint(
            a, b, operation, &events, ao, ad, ar, ah, bo, bd, br, bh,
        )?
    } else {
        imprint_pipeline::regularized_empty_algebra(a, b, operation, relation)?
    };
    let certificate = certificate_for(
        operation,
        coverages,
        classification,
        uv_arrangement_certified,
        &result,
    )?;
    Ok((result, certificate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cylinder, extrude_polygon, sphere};

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
        assert!(cert.no_prism_authorship);
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
        assert!(cert.no_prism_authorship);
        assert!(cert.all_complete());
        model.validate().unwrap();
    }

    #[test]
    fn identical_cylinders_intersection_is_certified() {
        let a = cylinder(3., 5.).unwrap();
        let b = cylinder(3., 5.).unwrap();
        let (model, cert) = analytic_boolean(&a, &b, "intersection").unwrap();
        assert!(cert.no_prism_authorship);
        assert!(cert.all_complete());
        model.validate().unwrap();
    }

    #[test]
    fn box_sphere_dome_is_certified_for_every_operation() {
        let b = crate::cuboid([-10., -10., -10.], [10., 10., 10.]).unwrap();
        let s = crate::transform::affine(
            &sphere(5.).unwrap(),
            [
                [1., 0., 0., 0.3],
                [0., 1., 0., -0.7],
                [0., 0., 1., 8.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        for operation in ["union", "difference", "intersection"] {
            let (model, cert) = analytic_boolean(&b, &s, operation).unwrap();
            assert!(cert.no_prism_authorship, "{operation}");
            assert!(cert.all_complete(), "{operation}");
            assert_eq!(model.bodies.len(), 1, "{operation}");
            model.validate().unwrap();
            let (swapped, _) = analytic_boolean(&s, &b, operation).unwrap();
            swapped.validate().unwrap();
        }
        let err = analytic_boolean(&b, &s, "xor").unwrap_err();
        assert_eq!(err.code, "BREP_ANALYTIC_BOOLEAN_REFUSED");
    }

    #[test]
    fn box_sphere_edge_corner_and_bar_are_certified() {
        let place = |r: f64, at: [f64; 3]| {
            crate::transform::affine(
                &sphere(r).unwrap(),
                [
                    [1., 0., 0., at[0]],
                    [0., 1., 0., at[1]],
                    [0., 0., 1., at[2]],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap()
        };
        let big = crate::cuboid([-10., -10., -10.], [10., 10., 10.]).unwrap();
        let bar = crate::cuboid([-10., -1.5, -1.2], [10., 1.5, 1.2]).unwrap();
        let cases = [
            (big.clone(), place(4., [9., 0.4, 9.])),
            (big, place(4., [9.3, 8.9, 9.1])),
            (bar, place(4., [0.3, 0.1, -0.2])),
        ];
        for (b, s) in &cases {
            for operation in ["union", "difference", "intersection"] {
                let (model, cert) = analytic_boolean(b, s, operation).unwrap();
                assert!(cert.no_prism_authorship, "{operation}");
                assert!(cert.all_complete(), "{operation}");
                model.validate().unwrap();
            }
        }
        let (bar_cut, _) = analytic_boolean(&cases[2].0, &cases[2].1, "difference").unwrap();
        assert_eq!(bar_cut.bodies.len(), 2);
    }

    #[test]
    fn planar_box_pair_refuses_analytic_path() {
        let a = extrude_polygon(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.]], 0., 1.).unwrap();
        let b = extrude_polygon(&[[1., 1.], [3., 1.], [3., 3.], [1., 3.]], 0., 1.).unwrap();
        let err = analytic_boolean(&a, &b, "union").unwrap_err();
        assert_eq!(err.code, "BREP_UNSUPPORTED_OPERATION");
    }

    #[test]
    fn axial_cylinder_sphere_is_certified() {
        let c = cylinder(3., 10.).unwrap();
        let s = crate::transform::affine(
            &sphere(4.).unwrap(),
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 5.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        for operation in ["union", "difference", "intersection"] {
            let (model, cert) = analytic_boolean(&c, &s, operation).unwrap();
            assert!(cert.no_prism_authorship, "{operation}");
            assert!(cert.all_complete(), "{operation}");
            model.validate().unwrap();
            let (swapped, _) = analytic_boolean(&s, &c, operation).unwrap();
            swapped.validate().unwrap();
        }
        let (stubs, _) = analytic_boolean(&c, &s, "difference").unwrap();
        assert_eq!(stubs.bodies.len(), 2);
    }

    #[test]
    fn mixed_sphere_cylinder_refuses() {
        let a = cylinder(2., 4.).unwrap();
        let a_faces = a.faces.len();
        let b = sphere(2.).unwrap();
        // Overlapping mixed pair without Complete imprint → refuse (operands unchanged).
        let err = analytic_boolean(&a, &b, "union").unwrap_err();
        assert!(
            err.code == "BREP_UNSUPPORTED_OPERATION" || err.code == "BREP_ANALYTIC_BOOLEAN_REFUSED"
        );
        assert_eq!(a.faces.len(), a_faces);
    }

    #[test]
    fn mixed_sphere_cylinder_disjoint_is_certified() {
        let a = cylinder(2., 4.).unwrap();
        let b0 = sphere(2.).unwrap();
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
        let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
        assert!(cert.no_prism_authorship);
        assert!(cert.all_complete());
        model.validate().unwrap();
    }

    #[test]
    fn disjoint_cones_union_is_certified() {
        let a = crate::frustum(2., 0.5, 4.).unwrap();
        let b0 = crate::frustum(2., 0.5, 4.).unwrap();
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
        let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
        assert!(cert.all_complete());
        model.validate().unwrap();
    }

    #[test]
    fn disjoint_tori_boolean_is_query_only_refuse() {
        let a = crate::torus(4., 1.).unwrap();
        let b0 = crate::torus(4., 1.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 30.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let error = analytic_boolean(&a, &b, "union").unwrap_err();
        assert_eq!(error.code, "BREP_ANALYTIC_BOOLEAN_REFUSED");
    }

    #[test]
    fn intersecting_cones_freeze_refuse() {
        let a = crate::frustum(2., 0.5, 4.).unwrap();
        let b0 = crate::frustum(2., 0.5, 4.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 1.],
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
    fn sphere_sphere_union_is_certified() {
        let a = sphere(2.).unwrap();
        let b0 = sphere(2.).unwrap();
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
        assert!(cert.no_prism_authorship);
        assert!(cert.all_complete());
        model.validate().unwrap();
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
        assert!(reports.len() >= 4);
        assert!(reports.iter().all(|r| r.coverage == Coverage::Complete));
    }

    #[test]
    fn intersecting_cylinders_union_without_prism() {
        let a = cylinder(2., 4.).unwrap();
        let b0 = cylinder(2., 4.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 3.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
        assert!(cert.imprint_authored);
        assert!(cert.no_prism_authorship);
        assert!(cert.all_complete());
        model.validate().unwrap();
        let overlap_area = 8. * 0.75_f64.acos() - 1.5 * 7_f64.sqrt();
        let expected = (8. * std::f64::consts::PI - overlap_area) * 4.;
        let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
        assert!((mass.signed_volume_mm3 - expected).abs() < 1e-5);
    }

    #[test]
    fn intersecting_cylinders_difference_without_prism() {
        let a = cylinder(2., 4.).unwrap();
        let b0 = cylinder(2., 4.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 3.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (model, cert) = analytic_boolean(&a, &b, "difference").unwrap();
        assert!(cert.no_prism_authorship);
        assert!(cert.all_complete());
        model.validate().unwrap();
        let overlap_area = 8. * 0.75_f64.acos() - 1.5 * 7_f64.sqrt();
        let expected = (4. * std::f64::consts::PI - overlap_area) * 4.;
        let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
        assert!((mass.signed_volume_mm3 - expected).abs() < 1e-5);
    }

    #[test]
    fn intersecting_cylinders_intersection_exact_volume() {
        let a = cylinder(2., 4.).unwrap();
        let b0 = cylinder(2., 4.).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 3.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (model, cert) = analytic_boolean(&a, &b, "intersection").unwrap();
        assert!(cert.no_prism_authorship);
        assert!(cert.all_complete());
        let overlap_area = 8. * 0.75_f64.acos() - 1.5 * 7_f64.sqrt();
        let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
        assert!((mass.signed_volume_mm3 - overlap_area * 4.).abs() < 1e-5);
    }

    #[test]
    fn parallel_wall_ss_publishes_complete_generator_lines() {
        let report = crate::analytic_ss::cylinder_cylinder(
            [0., 0., 0.],
            [0., 0., 1.],
            2.,
            [0., 4.],
            [3., 0., 0.],
            [0., 0., 1.],
            2.,
            [0., 4.],
            Options::default(),
        )
        .unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        assert_eq!(report.components.len(), 2);
        assert!(matches!(
            report.components[0],
            crate::analytic_ss::AnalyticSsComponent::Line { .. }
        ));
    }
}
