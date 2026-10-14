use super::*;

/// Aggregate proof for the finite `/2` contacting-solid cell.
#[derive(Clone, Debug)]
pub struct NurbsBooleanImprintCertificate {
    pub capability: &'static str,
    pub complete: bool,
    pub boolean_imprint: bool,
    pub notes: Vec<&'static str>,
    pub curve_component: Option<G6Component>,
    pub operation: String,
    pub context: ToleranceSpecIdentity,
    pub evidence: ComposedEvidence,
    pub arrangements: Vec<ClassificationCertificate>,
    pub sew: SewCertificate,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub no_healing: bool,
    pub no_curved_prism_authorship: bool,
}

/// Audited authority for the finite V3 graph-patch × affine-planar cutter cell.
/// Union is intentionally absent: this successor only authors the one-body
/// retained graph sub-solid for intersection and source-minus-cutter difference.
#[derive(Clone, Debug)]
pub struct CurvedGraphBooleanCertificate {
    pub capability: &'static str,
    pub status: &'static str,
    pub operation: String,
    pub axis: ExactIsoAxis,
    pub fixed_parameter: f64,
    pub source_face: String,
    pub retained_face: String,
    pub deleted_region: &'static str,
    pub intersection_edge: String,
    pub tensor_cells: usize,
    pub exact_correspondence: bool,
    pub sew: SewCertificate,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub naming_complete: bool,
    pub no_fallback: bool,
    pub separation_proof: bool,
    pub homogeneous_root_proof: bool,
    pub denominator_lower_bound: f64,
    pub weight_condition_number: f64,
    pub resource_bound: usize,
}

impl CurvedGraphBooleanCertificate {
    pub fn permits_topology_change(&self) -> bool {
        matches!(
            self.capability,
            NURBS_BOOLEAN_CAPABILITY_V3 | NURBS_BOOLEAN_CAPABILITY_V4 | NURBS_BOOLEAN_CAPABILITY_V5
        ) && self.status == "Complete"
            && matches!(self.operation.as_str(), "intersection" | "difference")
            && self.fixed_parameter > 0.
            && self.fixed_parameter < 1.
            && self.tensor_cells == 3
            && self.exact_correspondence
            && self.sew.complete
            && self.audit.ok
            && self.naming_complete
            && self.no_fallback
            && self.separation_proof
            && (self.capability != NURBS_BOOLEAN_CAPABILITY_V5
                || (self.homogeneous_root_proof
                    && self.denominator_lower_bound >= 0.25
                    && self.weight_condition_number <= 8.
                    && self.resource_bound <= 16))
            && !self.change_set.changes.is_empty()
            && self.change_set.validate().is_ok()
    }
}

#[derive(Clone, Debug)]
pub struct ContainedGraphBooleanCertificate {
    pub capability: &'static str,
    pub status: &'static str,
    pub operation: String,
    pub relation: &'static str,
    pub strict_uv_margin: f64,
    pub floor_clearance: f64,
    pub roof_clearance: f64,
    pub cavity_proof: bool,
    pub separation_proof: bool,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub naming_complete: bool,
    pub no_fallback: bool,
}

impl ContainedGraphBooleanCertificate {
    pub fn permits_topology_change(&self) -> bool {
        self.capability == NURBS_BOOLEAN_CAPABILITY_V4
            && self.status == "Complete"
            && matches!(
                self.operation.as_str(),
                "union" | "intersection" | "difference"
            )
            && self.relation == "graph-strictly-contains-affine-cutter"
            && self.strict_uv_margin > 0.
            && self.floor_clearance > 0.
            && self.roof_clearance > 0.
            && (self.operation != "difference" || self.cavity_proof)
            && self.separation_proof
            && self.audit.ok
            && self.naming_complete
            && self.no_fallback
            && !self.change_set.changes.is_empty()
            && self.change_set.validate().is_ok()
    }
}

impl NurbsBooleanImprintCertificate {
    pub fn permits_topology_change(&self) -> bool {
        self.capability == NURBS_BOOLEAN_CAPABILITY
            && self.complete
            && self.boolean_imprint
            && matches!(
                self.operation.as_str(),
                "union" | "difference" | "intersection"
            )
            && self.context == self.evidence.context
            && !self.arrangements.is_empty()
            && self
                .arrangements
                .iter()
                .all(|cert| cert.complete && cert.chart == ChartKind::PlanePoly)
            && self
                .evidence
                .claims
                .iter()
                .any(|claim| matches!(claim, EvidenceClaim::Positional { .. }))
            && self.evidence.claims.iter().any(|claim| {
                matches!(
                    claim,
                    EvidenceClaim::TopologyPreservation { invariant }
                        if invariant == "profile_arrangement_split_retrim_sew_audit"
                )
            })
            && self.sew.complete
            && self.sew.displacement_budget_ok
            && self.sew.matched == self.audit.sew.matched
            && self.sew.displacement_budget_ok == self.audit.sew.displacement_budget_ok
            && self.audit.ok
            && self.audit.sew.complete
            && self.no_healing
            && self.no_curved_prism_authorship
            && !self.change_set.changes.is_empty()
            && self.change_set.validate().is_ok()
    }
}

pub(super) fn mix_pt(a: &[f64], b: &[f64], t: f64) -> Vec<f64> {
    a.iter()
        .zip(b.iter())
        .map(|(u, v)| (1. - t) * u + t * v)
        .collect()
}

/// Elevate an admitted Bezier-le3 patch (or already-uniform bicubic) to R1 bicubic.
/// Keeps the historical bilinear 1×1 elevation path; deg 2 and mixed use Bernstein
/// elevation on U then V until 3×3.
pub(super) fn as_uniform_bicubic(surface: &Surface) -> Option<Surface> {
    if is_uniform_bicubic_positive(surface) {
        return Some(surface.clone());
    }
    if !is_bezier_le3_positive(surface) {
        return None;
    }
    if surface.degree_u == 1 && surface.degree_v == 1 {
        return elevate_bilinear_1x1(surface);
    }
    let elevated = surface
        .edit_axis(Axis::U, |c| c.elevate(3))
        .ok()?
        .edit_axis(Axis::V, |c| c.elevate(3))
        .ok()?;
    if is_uniform_bicubic_positive(&elevated) {
        Some(elevated)
    } else {
        None
    }
}

pub(super) fn elevate_bilinear_1x1(surface: &Surface) -> Option<Surface> {
    let p00 = &surface.control_points[0][0];
    let p10 = &surface.control_points[0][1];
    let p01 = &surface.control_points[1][0];
    let p11 = &surface.control_points[1][1];
    let row = |a: &[f64], b: &[f64]| -> Vec<Vec<f64>> {
        vec![
            a.to_vec(),
            mix_pt(a, b, 1. / 3.),
            mix_pt(a, b, 2. / 3.),
            b.to_vec(),
        ]
    };
    let r0 = row(p00, p10);
    let r1 = row(p01, p11);
    let mut control_points = Vec::with_capacity(4);
    for j in 0..4 {
        let t = j as f64 / 3.;
        control_points.push((0..4).map(|i| mix_pt(&r0[i], &r1[i], t)).collect());
    }
    Some(Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    })
}

pub fn nurbs_boolean_transverse_bicubic(
    a: &Surface,
    b: &Surface,
    options: Options,
) -> Result<NurbsBooleanImprintCertificate> {
    let _ = narrow_transverse_bicubic(a, b, options)?;
    Err(refuse(
        "nurbs-boolean-bezier-le3/1 is Unavailable; use the finite /2 contacting-solid author",
    ))
}

pub(super) fn model_aabb(model: &Model) -> ([f64; 3], [f64; 3]) {
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

pub(super) fn affine_planar_carrier(model: &Model) -> Result<Model> {
    let tolerance = model.tolerance_mm.max(1e-9);
    let mut carrier = model.clone();
    for (face_index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        if !is_bezier_le3_positive(surface) || planar_support(surface, tolerance).is_none() {
            return Err(refuse(
                "NURBS /2 requires non-periodic unit-weight single-span planar faces of degree <=3",
            ));
        }
        let du = surface.degree_u;
        let dv = surface.degree_v;
        let p00 = &surface.control_points[0][0];
        let p10 = &surface.control_points[du][0];
        let p01 = &surface.control_points[0][dv];
        let p11 = &surface.control_points[du][dv];
        for (i, row) in surface.control_points.iter().enumerate().take(du + 1) {
            for (j, actual) in row.iter().enumerate().take(dv + 1) {
                let u = i as f64 / du as f64;
                let v = j as f64 / dv as f64;
                let expected = (0..3)
                    .map(|axis| {
                        (1. - u) * (1. - v) * p00[axis]
                            + u * (1. - v) * p10[axis]
                            + (1. - u) * v * p01[axis]
                            + u * v * p11[axis]
                    })
                    .collect::<Vec<_>>();
                let residual = (0..3)
                    .map(|axis| (actual[axis] - expected[axis]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                if residual > tolerance {
                    return Err(refuse(
                        "NURBS /2 refuses planar but non-affine Bezier parameterizations",
                    ));
                }
            }
        }
        carrier.faces[face_index].surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![p00.clone(), p01.clone()],
                vec![p10.clone(), p11.clone()],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
    }
    carrier.validate()?;
    Ok(carrier)
}

pub(super) fn strict_partial_contact(a: &Model, b: &Model) -> Result<()> {
    let (amin, amax) = model_aabb(a);
    let (bmin, bmax) = model_aabb(b);
    let tolerance = a.tolerance_mm.max(b.tolerance_mm).max(1e-9);
    let overlap = std::array::from_fn::<_, 3, _>(|axis| {
        amax[axis].min(bmax[axis]) - amin[axis].max(bmin[axis])
    });
    if overlap.iter().any(|width| *width <= tolerance * 8.) {
        return Err(refuse(
            "NURBS /2 requires strict positive-volume contact; tangent and gray-band contacts refuse",
        ));
    }
    let contains =
        |outer_min: [f64; 3], outer_max: [f64; 3], inner_min: [f64; 3], inner_max: [f64; 3]| {
            (0..3).all(|axis| {
                inner_min[axis] >= outer_min[axis] - tolerance
                    && inner_max[axis] <= outer_max[axis] + tolerance
            })
        };
    if contains(amin, amax, bmin, bmax) || contains(bmin, bmax, amin, amax) {
        return Err(refuse(
            "NURBS /2 admits partial contacting solids only; containment and identity remain unavailable",
        ));
    }
    Ok(())
}

/// `/2` successor cell: exact Boolean authorship for strict partial contact
/// between affine-planar Bezier profile prisms. Generic nonplanar contact,
/// empty algebra, containment, tangency, and fallback construction all refuse.
pub fn nurbs_boolean_imprint_solids(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, NurbsBooleanImprintCertificate)> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Err(refuse(
            "nurbs-boolean-bezier-le3/2 admits union, difference, or intersection only",
        ));
    }
    strict_partial_contact(a, b)?;
    let carrier_a = affine_planar_carrier(a)?;
    let carrier_b = affine_planar_carrier(b)?;
    let result =
        crate::profile_imprint::exact_planar_contact_boolean(&carrier_a, &carrier_b, operation)?;
    let audited = LocallyValidatedModel::new(result)?.audit()?;
    let audit = audited.certificate().clone();
    let result = audited.into_model();
    if result
        .faces
        .iter()
        .any(|face| !is_bezier_le3_positive(&face.surface))
    {
        return Err(refuse(
            "NURBS /2 result escaped the non-periodic unit-weight single-span degree<=3 cell",
        ));
    }
    let arrangements = (0..result.faces.len())
        .map(|face| crate::trim_sew::classify_face_outer_loop(&result, face, ChartKind::PlanePoly))
        .collect::<Result<Vec<_>>>()?;
    let context = result
        .tolerance_context()
        .map_err(|_| refuse("NURBS /2 result tolerance context is invalid"))?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::positional(&context, 0., 1.)?,
            PredicateEvidence::topology_preservation(
                &context,
                "profile_arrangement_split_retrim_sew_audit",
                true,
            )?,
        ],
    )?;
    let cert = NurbsBooleanImprintCertificate {
        capability: NURBS_BOOLEAN_CAPABILITY,
        complete: true,
        boolean_imprint: true,
        notes: vec![
            "finite_affine_planar_contact_cell",
            "exact_profile_arrangement",
            "split_retrim_sew_audit_complete",
        ],
        curve_component: None,
        operation: operation.into(),
        context: context.spec_identity(),
        evidence,
        arrangements,
        sew: audit.sew.clone(),
        audit,
        change_set: result.1.change_set.clone(),
        no_healing: true,
        no_curved_prism_authorship: true,
    };
    if !cert.permits_topology_change() {
        return Err(refuse(
            "NURBS Boolean certificate does not permit topology change",
        ));
    }
    Ok((result, cert))
}

pub(super) fn near_point(a: [f64; 3], b: [f64; 3], tolerance: f64) -> bool {
    norm(sub(a, b)) <= tolerance
}

pub(super) fn point3(surface: &Surface, u: f64, v: f64) -> Result<[f64; 3]> {
    let p = surface.evaluate(u, v)?.point;
    Ok([p[0], p[1], p[2]])
}

/// Canonical one-body input for the finite V3 cell.  Its only non-planar face
/// is a unit-weight single-span degree-2/3 tensor graph with straight boundary.
pub fn canonical_bezier_graph_solid(degree_u: usize, degree_v: usize) -> Result<Model> {
    if !(2..=3).contains(&degree_u) || !(2..=3).contains(&degree_v) {
        return Err(refuse("V3 graph solid requires degree 2 or 3 in U and V"));
    }
    let mut model = crate::cuboid([0., 0., 0.], [1., 1., 1.])?;
    let mut control_points = Vec::with_capacity(degree_u + 1);
    for i in 0..=degree_u {
        let u = i as f64 / degree_u as f64;
        control_points.push(
            (0..=degree_v)
                .map(|j| {
                    let v = j as f64 / degree_v as f64;
                    let interior = i > 0 && i < degree_u && j > 0 && j < degree_v;
                    vec![u, v, 1. + if interior { 0.25 } else { 0. }]
                })
                .collect(),
        );
    }
    model.faces[1].surface = Surface {
        degree_u,
        degree_v,
        knots_u: [vec![0.; degree_u + 1], vec![1.; degree_u + 1]].concat(),
        knots_v: [vec![0.; degree_v + 1], vec![1.; degree_v + 1]].concat(),
        control_points,
        weights: vec![vec![1.; degree_v + 1]; degree_u + 1],
        periodic_u: false,
        periodic_v: false,
    };
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}

/// Canonical bounded positive-weight graph for the finite `/5` cell. Weights
/// vary only along V, so the homogeneous plane numerator factors into a
/// positive denominator polynomial and one affine U root.
pub fn canonical_rational_graph_solid(degree_u: usize, degree_v: usize) -> Result<Model> {
    let base = canonical_bezier_graph_solid(degree_u, degree_v)?;
    let (_, source_top, origin, u, v, floor_offset) = canonical_graph_frame(&base)?;
    let mut top = source_top.clone();
    let weights: Vec<f64> = (0..=degree_v)
        .map(|j| {
            if j == 0 || j == degree_v {
                1.
            } else {
                1. + 0.25 * j as f64
            }
        })
        .collect();
    top.weights = (0..=degree_u).map(|_| weights.clone()).collect();
    let model = clipped_graph_solid(&base, &top, origin, u, v, floor_offset, [0., 1., 0., 1.])?;
    model.validate()?;
    let rational_face = canonical_graph_frame_cell(&model, true, false)?.0;
    if rational_weight_bounds(&model.faces[rational_face].surface).is_none() {
        return Err(refuse(
            "Canonical rational graph exceeded finite weight bounds",
        ));
    }
    Ok(model)
}

pub(super) fn canonical_graph_frame_cell(
    source: &Model,
    rational: bool,
    multispan: bool,
) -> Result<GraphFrame<'_>> {
    if source.bodies.len() != 1
        || source.shells.len() != 1
        || source.faces.len() != 6
        || source.vertices.len() != 8
        || !source.bodies[0].inner_shells.is_empty()
    {
        return Err(refuse(
            "Graph source must be the canonical one-body graph solid",
        ));
    }
    let tolerance = source.tolerance_mm.max(1e-9) * 8.;
    let nonplanar: Vec<_> = source
        .faces
        .iter()
        .enumerate()
        .filter(|(_, face)| planar_support(&face.surface, tolerance).is_none())
        .collect();
    if nonplanar.len() != 1 {
        return Err(refuse(
            "Graph source must contain exactly one non-planar face",
        ));
    }
    let (face_id, top) = (nonplanar[0].0, &nonplanar[0].1.surface);
    let admitted_top = if multispan {
        positive_multispan_graph(top)
    } else if rational {
        rational_weight_bounds(top).is_some()
    } else {
        is_bezier_le3_positive(top)
    };
    if !(2..=3).contains(&top.degree_u)
        || !(2..=3).contains(&top.degree_v)
        || !admitted_top
        || !source.faces[face_id].holes.is_empty()
    {
        return Err(refuse(
            "Curved face is outside the admitted degree-2/3 graph cell",
        ));
    }
    let o = point3(top, 0., 0.)?;
    let pu = point3(top, 1., 0.)?;
    let pv = point3(top, 0., 1.)?;
    let p11 = point3(top, 1., 1.)?;
    let u = sub(pu, o);
    let v = sub(pv, o);
    if !near_point(
        p11,
        [o[0] + u[0] + v[0], o[1] + u[1] + v[1], o[2] + u[2] + v[2]],
        tolerance,
    ) {
        return Err(refuse(
            "V3 graph corners do not define one affine projection",
        ));
    }
    let mut normal = normalize(cross(u, v)).ok_or_else(|| refuse("V3 graph frame is singular"))?;
    let corners = [o, pu, p11, pv];
    let last_u = top.control_points.len() - 1;
    let last_v = top.control_points[0].len() - 1;
    for (i, row) in top.control_points.iter().enumerate().take(last_u + 1) {
        for (j, p) in row.iter().enumerate().take(last_v + 1) {
            if (i == 0 || i == last_u || j == 0 || j == last_v)
                && dot(sub([p[0], p[1], p[2]], o), normal).abs() > tolerance
            {
                return Err(refuse(
                    "V3 graph boundary must remain on its affine corner plane",
                ));
            }
        }
    }
    let mut offset = None;
    'candidate: for vertex in &source.vertices {
        let w = sub(vertex.point, o);
        if norm(w) <= tolerance
            || dot(w, u).abs() > tolerance * norm(u)
            || dot(w, v).abs() > tolerance * norm(v)
        {
            continue;
        }
        for corner in corners {
            let target = [corner[0] + w[0], corner[1] + w[1], corner[2] + w[2]];
            if !source
                .vertices
                .iter()
                .any(|candidate| near_point(candidate.point, target, tolerance))
            {
                continue 'candidate;
            }
        }
        offset = Some(w);
        break;
    }
    let w = offset.ok_or_else(|| refuse("V3 graph solid has no exact parallel affine floor"))?;
    if dot(w, normal) > 0. {
        normal = normal.map(|value| -value);
    }
    if top.control_points.iter().flatten().any(|point| {
        let height = dot(sub([point[0], point[1], point[2]], o), normal);
        let floor_height = dot(w, normal);
        height <= floor_height + tolerance
    }) {
        return Err(refuse("V3 graph-to-floor separation proof failed"));
    }
    Ok((face_id, top, o, u, v, w))
}

pub(super) fn canonical_graph_frame(source: &Model) -> Result<GraphFrame<'_>> {
    canonical_graph_frame_cell(source, false, false)
}

pub(crate) fn is_nurbs_boolean_v3_candidate(a: &Model, b: &Model) -> bool {
    canonical_graph_frame(a).is_ok() ^ canonical_graph_frame(b).is_ok()
}

pub(crate) fn canonical_graph_is_first(model: &Model) -> bool {
    canonical_graph_frame(model).is_ok()
}

pub(crate) fn is_nurbs_boolean_v5_candidate(a: &Model, b: &Model) -> bool {
    canonical_graph_frame_cell(a, true, false).is_ok()
        ^ canonical_graph_frame_cell(b, true, false).is_ok()
}
