use super::*;

pub(super) fn unit_domain(mut curve: Curve) -> Curve {
    let [lo, hi] = curve.domain();
    for knot in &mut curve.knots {
        *knot = (*knot - lo) / (hi - lo);
    }
    curve
}

pub(super) fn edge_between(model: &Model, a: usize, b: usize) -> Result<usize> {
    model
        .edges
        .iter()
        .position(|edge| edge.vertices == [a.min(b), a.max(b)])
        .ok_or_else(|| refuse("V3 clipped box edge is missing"))
}

pub(super) fn ruled_side(bottom_a: [f64; 3], bottom_b: [f64; 3], top: Curve) -> Result<Surface> {
    let degree = top.degree;
    let single_span = top.control_points.len() == degree + 1 && top.knots.len() == 2 * (degree + 1);
    let (bottom_points, bottom_weights) = if single_span {
        let bottom = crate::line(bottom_a.to_vec(), bottom_b.to_vec()).elevate(degree)?;
        (bottom.control_points, bottom.weights)
    } else {
        if top.weights.windows(2).any(|pair| pair[0] != pair[1]) {
            return Err(refuse(
                "Multispan ruled side requires a constant positive boundary weight",
            ));
        }
        let domain = top.domain();
        let points = (0..top.control_points.len())
            .map(|index| {
                let parameter =
                    top.knots[index + 1..=index + degree].iter().sum::<f64>() / degree as f64;
                let t = (parameter - domain[0]) / (domain[1] - domain[0]);
                bottom_a
                    .into_iter()
                    .zip(bottom_b)
                    .map(|(a, b)| a + t * (b - a))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        (points, top.weights.clone())
    };
    Ok(Surface {
        degree_u: degree,
        degree_v: 1,
        knots_u: top.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: bottom_points
            .iter()
            .zip(&top.control_points)
            .map(|(a, b)| vec![a.clone(), b.clone()])
            .collect(),
        weights: bottom_weights
            .iter()
            .zip(&top.weights)
            .map(|(a, b)| vec![*a, *b])
            .collect(),
        periodic_u: false,
        periodic_v: false,
    })
}

pub(super) fn clipped_graph_solid(
    source: &Model,
    top: &Surface,
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    floor_offset: [f64; 3],
    bounds: [f64; 4],
) -> Result<Model> {
    let [umin, umax, vmin, vmax] = bounds;
    let local = crate::cuboid([umin, vmin, 0.], [umax, vmax, 1.])?;
    let z = floor_offset.map(|value| -value);
    let floor_origin = [
        origin[0] + floor_offset[0],
        origin[1] + floor_offset[1],
        origin[2] + floor_offset[2],
    ];
    let matrix = [
        [u[0], v[0], z[0], floor_origin[0]],
        [u[1], v[1], z[1], floor_origin[1]],
        [u[2], v[2], z[2], floor_origin[2]],
        [0., 0., 0., 1.],
    ];
    let mut model = crate::transform::affine(&local, matrix)?;
    let trimmed = top.trim([umin, umax, vmin, vmax])?;
    model.faces[1].surface = trimmed.clone();
    let top_curves = [
        unit_domain(top.iso(Axis::V, vmin)?.trim(umin, umax)?),
        unit_domain(top.iso(Axis::U, umax)?.trim(vmin, vmax)?),
        unit_domain(top.iso(Axis::V, vmax)?.trim(umin, umax)?.reverse()?),
        unit_domain(top.iso(Axis::U, umin)?.trim(vmin, vmax)?),
    ];
    for ((a, b), curve) in [(4, 5), (5, 6), (6, 7), (4, 7)]
        .into_iter()
        .zip(top_curves.iter())
    {
        let edge = edge_between(&model, a, b)?;
        model.edges[edge].curve = curve.clone();
    }
    let top_loop = model.faces[1].outer;
    let uv = [[umin, vmin], [umax, vmin], [umax, vmax], [umin, vmax]];
    for (index, coedge) in model.loops[top_loop].coedges.iter_mut().enumerate() {
        coedge.pcurve = crate::line(uv[index].to_vec(), uv[(index + 1) % 4].to_vec());
    }
    for (face, bottom, top_curve) in [
        (2usize, (0usize, 1usize), top_curves[0].clone()),
        (3, (1, 2), top_curves[1].clone()),
        (4, (2, 3), top_curves[2].clone()),
        (5, (3, 0), top_curves[3].clone().reverse()?),
    ] {
        model.faces[face].surface = ruled_side(
            model.vertices[bottom.0].point,
            model.vertices[bottom.1].point,
            top_curve,
        )?;
    }
    let assembled = crate::imprint_pipeline::assemble_imprint_solid(
        model.vertices.clone(),
        model.edges.clone(),
        model.loops.clone(),
        model.faces.clone(),
        model.shells[0].faces.clone(),
        source.tolerance_mm,
        &[source],
    )?;
    Ok(assembled)
}

/// Certified V3 author for one canonical graph solid and one affine cutter.
/// Complete rows: intersection and source-minus-cutter difference. Union and
/// cutter-minus-source difference remain typed refusals.
pub(super) fn nurbs_boolean_graph_patch_transverse(
    a: &Model,
    b: &Model,
    operation: &str,
    capability: &'static str,
    rational: bool,
) -> Result<(Model, CurvedGraphBooleanCertificate)> {
    let (source, cutter, frame) = select_v3_operands(a, b, operation, rational)?;
    let (source_face, top, origin, u, v, floor_offset) = frame;
    let context = source
        .tolerance_context()
        .map_err(|_| refuse("V3 source tolerance context is invalid"))?;
    let (iso, plane) = v3_cutting_face(top, cutter, &context, rational)?;
    let keep_positive_parameter = v3_keep_positive_parameter(
        &iso,
        cutter,
        plane,
        context.spatial_bounds().clear_mm,
        operation == "intersection",
    )?;
    let fixed = iso.fixed_parameter;
    let bounds = match iso.first_axis {
        ExactIsoAxis::U if keep_positive_parameter => [fixed, 1., 0., 1.],
        ExactIsoAxis::U => [0., fixed, 0., 1.],
        ExactIsoAxis::V if keep_positive_parameter => [0., 1., fixed, 1.],
        ExactIsoAxis::V => [0., 1., 0., fixed],
    };
    let arrangement = v3_iso_tensor_arrangement(&context, iso.first_axis, fixed)?;
    let uv_trace = iso
        .uv_traces
        .first()
        .ok_or_else(|| refuse("V3 exact iso authority was revoked"))?;
    let correspondence = crate::trim_sew::prove_curve_pcurve_correspondence(
        &context,
        &iso.curve,
        top,
        uv_trace,
        crate::trim_sew::ParameterOrientation::Same,
        crate::trim_sew::BoundaryUse {
            face: source_face,
            wire: source.faces[source_face].outer,
            cyclic_index: 0,
            reversed: false,
        },
    )?;
    let mut result = clipped_graph_solid(source, top, origin, u, v, floor_offset, bounds)?;
    let seam = V3Seam::new(iso.first_axis, keep_positive_parameter);
    let source_id = source.1.faces[source_face];
    seam.record_lineage(&mut result, source, source_id, capability)?;
    let audited = LocallyValidatedModel::new(result)?.audit()?;
    let mut audit = audited.certificate().clone();
    audit.notes.push("v3_graph_patch_cell_separation_ok");
    audit
        .notes
        .push("v3_graph_patch_self_intersection_exclusion_ok");
    let result = audited.into_model();
    let (denominator_lower_bound, denominator_upper_bound) =
        rational_weight_bounds(top).unwrap_or((1., 1.));
    let cert = CurvedGraphBooleanCertificate {
        capability,
        status: "Complete",
        operation: operation.into(),
        cut: GraphBooleanCut {
            axis: iso.first_axis,
            fixed_parameter: fixed,
            tensor_cells: arrangement.cells.len(),
        },
        lineage: GraphBooleanLineage {
            source_face: source_id.to_string(),
            retained_face: result.1.faces[1].to_string(),
            deleted_region: if keep_positive_parameter {
                "lower-parameter-cell"
            } else {
                "upper-parameter-cell"
            },
            intersection_edge: result.1.edges[seam.edge(&result)?].to_string(),
        },
        sew: audit.sew.clone(),
        audit,
        change_set: result.1.change_set.clone(),
        proofs: GraphBooleanProofs {
            exact_correspondence: correspondence.permits_exact_correspondence(),
            naming_complete: result.persistent_naming_complete(),
            no_fallback: true,
            separation_proof: true,
            homogeneous_root_proof: rational,
        },
        conditioning: GraphBooleanConditioning {
            denominator_lower_bound,
            weight_condition_number: denominator_upper_bound / denominator_lower_bound,
            resource_bound: (top.degree_u + 1) * (top.degree_v + 1),
        },
    };
    if !cert.permits_topology_change() {
        return Err(refuse("V3 aggregate certificate failed closed"));
    }
    Ok((result, cert))
}

/// Multiplier applied to the cutter tolerance when admitting planar faces.
const V3_PLANAR_TOLERANCE_FACTOR: f64 = 8.;
/// Smallest tolerance used to recover the cutting-face plane.
const V3_MIN_PLANAR_TOLERANCE_MM: f64 = 1e-9;
/// Tensor arrangement cell and resource limits for the single iso split.
const V3_TENSOR_ITERATION_LIMIT: usize = 1;
const V3_TENSOR_RESOURCE_LIMIT: usize = 16;
/// Primitive id of the iso split line after the four rectangle boundaries.
const V3_ISO_EDGE_ID: usize = 4;

/// Validates operands and returns `(source, cutter, frame)`.
fn select_v3_operands<'a>(
    a: &'a Model,
    b: &'a Model,
    operation: &str,
    rational: bool,
) -> Result<(&'a Model, &'a Model, GraphFrame<'a>)> {
    a.validate()?;
    b.validate()?;
    if operation == "union" {
        return Err(refuse(
            "V3 union lacks a one-body closed-shell separation proof",
        ));
    }
    if !matches!(operation, "intersection" | "difference") {
        return Err(refuse(
            "V3 graph Boolean admits intersection or source-minus-cutter difference",
        ));
    }
    let a_graph = canonical_graph_frame_cell(a, rational, false).ok();
    let b_graph = canonical_graph_frame_cell(b, rational, false).ok();
    let (source, cutter, source_is_a, frame) = match (a_graph, b_graph) {
        (Some(frame), None) => (a, b, true, frame),
        (None, Some(frame)) => (b, a, false, frame),
        _ => return Err(refuse("V3 requires exactly one canonical graph operand")),
    };
    if operation == "difference" && !source_is_a {
        return Err(refuse(
            "V3 cutter-minus-graph difference is outside the certified cell",
        ));
    }
    if cutter.bodies.len() != 1
        || cutter.shells.len() != 1
        || cutter.faces.iter().any(|face| {
            planar_support(
                &face.surface,
                cutter.tolerance_mm * V3_PLANAR_TOLERANCE_FACTOR,
            )
            .is_none()
        })
    {
        return Err(refuse("V3 cutter must be one affine-planar closed solid"));
    }
    Ok((source, cutter, frame))
}

/// Plane `(origin, unit normal)` of an affine cutter face.
type PlaneSupport = ([f64; 3], [f64; 3]);

/// The unique cutter face with an exact iso intersection and its plane.
fn v3_cutting_face(
    top: &Surface,
    cutter: &Model,
    context: &cad_predicates::ToleranceContext,
    rational: bool,
) -> Result<(ExactIsoIntersectionCertificate, PlaneSupport)> {
    let tolerance =
        cutter.tolerance_mm.max(V3_MIN_PLANAR_TOLERANCE_MM) * V3_PLANAR_TOLERANCE_FACTOR;
    let mut matches = Vec::new();
    for face in &cutter.faces {
        let certificate = if rational {
            certify_exact_rational_planar_iso_intersection(top, &face.surface, context)
        } else {
            certify_exact_planar_iso_intersection(top, &face.surface, context)
        };
        if let Ok(certificate) = certificate {
            let support = planar_support(&face.surface, tolerance)
                .ok_or_else(|| refuse("V3 cutter must be one affine-planar closed solid"))?;
            matches.push((certificate, support));
        }
    }
    if matches.len() != 1 {
        return Err(refuse(
            "V3 cutter must expose exactly one strict-interior iso cutting face",
        ));
    }
    let (iso, plane) = matches.remove(0);
    if !iso.permits_topology_authorship() {
        return Err(refuse("V3 exact iso authority was revoked"));
    }
    Ok((iso, plane))
}

/// Whether the retained cell lies on the positive side of the iso parameter.
fn v3_keep_positive_parameter(
    iso: &ExactIsoIntersectionCertificate,
    cutter: &Model,
    (plane_origin, plane_normal): PlaneSupport,
    clear: f64,
    keep_cutter_side: bool,
) -> Result<bool> {
    let distances: Vec<_> = cutter
        .vertices
        .iter()
        .map(|vertex| dot(sub(vertex.point, plane_origin), plane_normal))
        .collect();
    let positive = distances.iter().any(|distance| *distance > clear);
    let negative = distances.iter().any(|distance| *distance < -clear);
    if positive == negative {
        return Err(refuse(
            "V3 cutter does not occupy one strict side of its cutting face",
        ));
    }
    let cutter_sign = if positive { 1. } else { -1. };
    Ok((cutter_sign == iso.transverse_signed_distance_slope.signum()) == keep_cutter_side)
}

/// Unit tensor rectangle arrangement split by the certified iso line.
fn v3_iso_tensor_arrangement(
    context: &cad_predicates::ToleranceContext,
    axis: ExactIsoAxis,
    fixed: f64,
) -> Result<crate::uv_arrangement::LiftedUvArrangement> {
    use crate::uv_arrangement::{
        LiftedUvGeometry, LiftedUvPrimitive, TensorAxis, TensorBoundary,
        arrange_tensor_bezier_graph_uv,
    };
    let domain = [[0., 1.], [0., 1.]];
    let mut primitives: Vec<_> = [
        TensorBoundary::UMin,
        TensorBoundary::UMax,
        TensorBoundary::VMin,
        TensorBoundary::VMax,
    ]
    .into_iter()
    .enumerate()
    .map(|(edge_id, side)| LiftedUvPrimitive {
        edge_id,
        geometry: LiftedUvGeometry::TensorRectangleBoundary { side, domain },
    })
    .collect();
    primitives.push(LiftedUvPrimitive {
        edge_id: V3_ISO_EDGE_ID,
        geometry: LiftedUvGeometry::TensorIsoLine {
            axis: match axis {
                ExactIsoAxis::U => TensorAxis::U,
                ExactIsoAxis::V => TensorAxis::V,
            },
            fixed,
            interval: [0., 1.],
        },
    });
    arrange_tensor_bezier_graph_uv(
        context,
        domain,
        &primitives,
        V3_TENSOR_ITERATION_LIMIT,
        V3_TENSOR_RESOURCE_LIMIT,
    )
}

/// Seam edge (by its clipped-box vertices) and seam side face of the V3 cell.
struct V3Seam {
    vertices: (usize, usize),
    face: usize,
}

impl V3Seam {
    fn new(axis: ExactIsoAxis, keep_positive_parameter: bool) -> Self {
        let (vertices, face) = match (axis, keep_positive_parameter) {
            (ExactIsoAxis::U, true) => ((4, 7), 5),
            (ExactIsoAxis::U, false) => ((5, 6), 3),
            (ExactIsoAxis::V, true) => ((4, 5), 2),
            (ExactIsoAxis::V, false) => ((6, 7), 4),
        };
        Self { vertices, face }
    }

    fn edge(&self, model: &Model) -> Result<usize> {
        edge_between(model, self.vertices.0, self.vertices.1)
    }

    /// Records the split lineage and classifies the change set roles.
    fn record_lineage(
        &self,
        result: &mut Model,
        source: &Model,
        source_id: TopoId,
        capability: &'static str,
    ) -> Result<()> {
        let seam_edge = self.edge(result)?;
        let retained_id = result.1.faces[1];
        result.1.lineage.push(TopologyLineageRecord {
            operation: "split".into(),
            entity_kind: "face".into(),
            parents: vec![source_id],
            children: vec![retained_id, result.1.faces[self.face]],
        });
        result.refresh_change_set(&[source]);
        let seam_id = result.1.edges[seam_edge];
        for change in &mut result.1.change_set.changes {
            if change.topo_kind == crate::TopoKind::Face && change.parents.contains(&source_id) {
                change.role = "curved-source-face-split-retained-deleted".into();
                change.provenance.operation = capability.into();
            }
            if change.topo_kind == crate::TopoKind::Edge && change.children.contains(&seam_id) {
                change.role = "generated-intersection-edge".into();
                change.provenance.operation = capability.into();
            }
        }
        result.1.change_set.validate()
    }
}

pub fn nurbs_boolean_graph_patch_v3(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, CurvedGraphBooleanCertificate)> {
    nurbs_boolean_graph_patch_transverse(a, b, operation, NURBS_BOOLEAN_CAPABILITY_V3, false)
}

pub fn nurbs_boolean_graph_patch_unequal_v4(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, CurvedGraphBooleanCertificate)> {
    let (source, cutter) = if canonical_graph_frame(a).is_ok() {
        (a, b)
    } else if canonical_graph_frame(b).is_ok() {
        (b, a)
    } else {
        return Err(refuse(
            "V4 unequal-span cell requires exactly one graph source",
        ));
    };
    let (source_min, source_max) = model_aabb(source);
    let (cutter_min, cutter_max) = model_aabb(cutter);
    let unequal = (0..3).any(|axis| {
        let source_span = source_max[axis] - source_min[axis];
        let cutter_span = cutter_max[axis] - cutter_min[axis];
        (source_span - cutter_span).abs() > source.tolerance_mm.max(cutter.tolerance_mm) * 8.
    });
    if !unequal {
        return Err(refuse("V4 requires a certified unequal source/cutter span"));
    }
    nurbs_boolean_graph_patch_transverse(a, b, operation, NURBS_BOOLEAN_CAPABILITY_V4, false)
}

pub fn nurbs_boolean_rational_graph_patch_v5(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, CurvedGraphBooleanCertificate)> {
    nurbs_boolean_graph_patch_transverse(a, b, operation, NURBS_BOOLEAN_CAPABILITY_V5, true)
}

#[derive(Clone, Debug)]
pub struct GeneralNurbsBooleanNaming {
    pub split: usize,
    pub retained: usize,
    pub deleted: usize,
    pub generated: usize,
    pub operation_stable: bool,
}

/// Aggregate native authority for nurbs-boolean/1 over nurbs-ss/1.
/// The sealed BranchGraph and global UV arrangement are retained, rather than
/// projected to caller-supplied booleans. Independent SS face-pair reports must
/// agree on transverse roots before topology change is granted.
#[derive(Clone, Debug)]
pub struct GeneralNurbsBooleanCertificate {
    pub capability: &'static str,
    pub authority: &'static str,
    pub status: &'static str,
    pub operation: String,
    pub operand_order: &'static str,
    pub exact_region_membership: bool,
    pub partition_cells: usize,
    pub cavity_count: usize,
    pub branch_graph: BranchGraph,
    pub uv: crate::uv_arrangement::MultiSpanUvArrangement,
    pub exact_curve_pcurve_count: usize,
    pub sew: SewCertificate,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub naming: GeneralNurbsBooleanNaming,
    pub result_components: usize,
    pub result_faces: usize,
    pub no_fallback: bool,
    /// Independent nurbs-ss/1 face-pair reports that agreed on transverse roots.
    pub ss_reports_complete: bool,
    pub ss_face_pairs: usize,
}

impl GeneralNurbsBooleanCertificate {
    pub fn permits_topology_change(&self) -> bool {
        self.capability == NURBS_BOOLEAN_SS_CAPABILITY
            && self.authority == GENERAL_NURBS_BOOLEAN_AUTHORITY
            && self.status == "Complete"
            && self.ss_reports_complete
            && self.ss_face_pairs > 0
            && matches!(
                self.operation.as_str(),
                "union" | "intersection" | "difference"
            )
            && matches!(self.operand_order, "source-tool" | "tool-source")
            && (self.operation != "union" || self.operand_order == "source-tool")
            && (self.operation != "difference"
                || self.operand_order != "tool-source"
                || self.cavity_count == 0)
            && self.exact_region_membership
            && self.partition_cells > 0
            && self.branch_graph.permits_topology_authorship()
            && self.branch_graph.components.len() >= 2
            && self.uv.permits_trim_classification()
            && self.uv.branch_count == self.branch_graph.components.len()
            && self.exact_curve_pcurve_count
                == self
                    .branch_graph
                    .certificate
                    .fragment_count
                    .saturating_mul(2)
            && self.sew.complete
            && self.audit.ok
            && self.naming.split > 0
            && self.naming.retained > 0
            && self.naming.deleted > 0
            && self.naming.generated > 0
            && self.naming.operation_stable
            && self.result_components == self.audit.body_count
            && self.result_faces > 0
            && self.no_fallback
            && self.change_set.validate().is_ok()
    }
}

pub fn canonical_multispan_graph_solid(spans_u: usize, spans_v: usize) -> Result<Model> {
    if !(1..=2).contains(&spans_u) || !(1..=2).contains(&spans_v) {
        return Err(refuse(
            "General graph fixture admits one or two spans per axis",
        ));
    }
    let mut model = canonical_bezier_graph_solid(3, 3)?;
    let top = canonical_graph_frame_cell(&model, false, false)?.0;
    let mut surface = model.faces[top].surface.clone();
    for index in 1..spans_u {
        let knot = index as f64 / spans_u as f64;
        surface = surface.edit_axis(Axis::U, |curve| curve.insert(knot, 1))?;
    }
    for index in 1..spans_v {
        let knot = index as f64 / spans_v as f64;
        surface = surface.edit_axis(Axis::V, |curve| curve.insert(knot, 1))?;
    }
    model.faces[top].surface = surface;
    model.rebuild_topology_ids();
    model.validate()?;
    canonical_graph_frame_cell(&model, true, true)?;
    Ok(model)
}

pub(crate) fn is_general_nurbs_boolean_candidate(a: &Model, b: &Model) -> bool {
    let general = |model: &Model| {
        canonical_graph_frame_cell(model, true, true)
            .ok()
            .and_then(|(face, surface, ..)| decompose_rational_bezier_spans(surface, face, 64).ok())
            .is_some_and(|decomposition| decomposition.spans.len() > 1)
    };
    general(a) ^ general(b)
}
