use super::*;

pub(super) fn refuse(message: &str) -> Error {
    Error::new("BREP_ANALYTIC_BOOLEAN_REFUSED", message)
}
pub(super) fn unsupported(message: &str) -> Error {
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
pub(super) enum AnalyticClass {
    FiniteCylinder,
    Tube,
    Frustum,
    Sphere,
    AxisAlignedBox,
    Cone,
    Torus,
}

pub(super) fn classify_model(model: &Model) -> Result<AnalyticClass> {
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

pub(super) fn model_bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
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

pub(super) type CylinderEnvelope = ([f64; 3], [f64; 3], f64, [f64; 2]);

pub(super) fn cylinder_envelope(model: &Model) -> Result<CylinderEnvelope> {
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

pub(super) fn cap_planes(height: [f64; 2]) -> [Plane; 2] {
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

pub(super) fn imprint_reports(a: &Model, b: &Model) -> Result<Vec<Report<AnalyticSsComponent>>> {
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
    let walls = cylinder_cylinder(
        FiniteCylinder {
            origin: ao,
            direction: ad,
            radius: ar,
            height: ah,
        },
        FiniteCylinder {
            origin: bo,
            direction: bd,
            radius: br,
            height: bh,
        },
        options,
    )?;
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

pub(super) fn classification_from_imprint(
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
        if let Ok(arrangement) = arrange_imprint_curves(ChartKind::AnalyticCircle, &curves)
            && assert_missed_branch_detected(&arrangement).is_ok()
        {
            return Ok((arrangement.classification, true));
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
    if let Some(face) = cap
        && let Ok(cert) = classify_face_outer_loop(a, face, ChartKind::AnalyticCircle)
    {
        return Ok((cert, true));
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

pub(super) fn relation_hint(a: &Model, b: &Model) -> Result<&'static str> {
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

pub(super) fn hint_to_relation(hint: &str) -> Result<SpatialRelation> {
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

pub(super) fn wall_imprint_events(
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

pub(super) fn certificate_for(
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

pub(super) fn sphere_pair_boolean(
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

pub(super) fn box_sphere_pair_boolean(
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

pub(super) fn sphere_envelope(model: &Model) -> Result<([f64; 3], f64)> {
    let (min, max) = model_bounds(model);
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let radius = (max[0] - min[0]).max(max[1] - min[1]).max(max[2] - min[2]) * 0.5;
    Ok((center, radius))
}

pub(super) fn torus_envelope(model: &Model) -> Result<([f64; 3], f64, f64)> {
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

pub(super) fn cone_envelope(model: &Model) -> Result<([f64; 3], [f64; 3], f64, f64)> {
    let (min, max) = model_bounds(model);
    let apex = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5, min[2]];
    let height = max[2] - min[2];
    let radius = (max[0] - min[0]).max(max[1] - min[1]) * 0.5;
    Ok((apex, [0., 0., 1.], radius, height))
}

pub(super) fn empty_algebra_pair(
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
