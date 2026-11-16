use super::*;
use cad_predicates::ToleranceContext;

/// Finite closed-solid author for a positive rational multispan graph and an
/// affine slab exposing at least two disjoint, transverse, complete iso
/// branches. The `/8` successor also authors an exact disjoint-interior
/// partition for union and tool-minus-source difference without fallback.
pub fn author_general_nurbs_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, GeneralNurbsBooleanCertificate)> {
    let operands = select_general_operands(a, b, operation)?;
    let (source_face, top, _, _, _, _) = operands.frame;
    let context = operands
        .source
        .tolerance_context()
        .map_err(|_| refuse("General NURBS source tolerance context is invalid"))?;
    let clear = context.spatial_bounds().clear_mm;
    let source_decomposition =
        decompose_rational_bezier_spans(top, source_face, MULTISPAN_RESOURCE_LIMIT)?;
    let survey = CutterSurvey::collect(&operands, &context, clear, &source_decomposition)?;
    let ss_face_pairs = survey.ss_face_pairs;
    let (branch_graph, axis, roots) =
        survey.into_branch_roots(&context, clear, source_decomposition.spans.len())?;
    let uv = crate::uv_arrangement::arrange_multispan_branch_graph_uv(
        &context,
        [
            &source_decomposition.u_breaks,
            &source_decomposition.v_breaks,
        ],
        &branch_graph,
        0,
        MULTISPAN_RESOURCE_LIMIT,
    )?;
    let slab = SlabFrame::certify(&operands, axis, roots, clear)?;
    let (source_bounds, naming_bounds) =
        partition_bounds(operation, operands.source_is_a, axis, roots)?;
    let mut pieces = slab.partition_pieces(&operands, operation, &source_bounds, clear)?;
    if pieces.is_empty() {
        return Err(refuse("Exact regularized region partition is empty"));
    }
    let partition_cells = pieces.len();
    let mut result = pieces.remove(0);
    for piece in pieces {
        result = crate::boolean_support::separated_union(&result, &piece)?;
    }
    let naming = author_general_operation_naming(
        &mut result,
        operands.source,
        operands.cutter,
        source_face,
        operation,
        &naming_bounds,
        axis,
    )?;
    let audited = LocallyValidatedModel::new(result)?.audit()?;
    let mut audit = audited.certificate().clone();
    audit.notes.push("global_multispan_uv_classification_ok");
    audit
        .notes
        .push("cell_specific_self_intersection_exclusion_ok");
    audit.notes.push("cavity_and_component_separation_ok");
    let result = audited.into_model();
    let certificate = GeneralNurbsBooleanCertificate {
        capability: NURBS_BOOLEAN_SS_CAPABILITY,
        authority: GENERAL_NURBS_BOOLEAN_AUTHORITY,
        status: "Complete",
        operation: operation.into(),
        operand_order: if operands.source_is_a {
            "source-tool"
        } else {
            "tool-source"
        },
        exact_region_membership: true,
        partition_cells,
        cavity_count: 0,
        exact_curve_pcurve_count: branch_graph.certificate.fragment_count * 2,
        branch_graph,
        uv,
        sew: audit.sew.clone(),
        audit,
        change_set: result.1.change_set.clone(),
        naming,
        result_components: result.bodies.len(),
        result_faces: result.faces.len(),
        no_fallback: true,
        ss_reports_complete: true,
        ss_face_pairs,
    };
    if !certificate.permits_topology_change() || !result.persistent_naming_complete() {
        return Err(refuse("General NURBS aggregate authority failed closed"));
    }
    Ok((result, certificate))
}

/// Resource limit shared by every multispan decomposition and SS certificate.
const MULTISPAN_RESOURCE_LIMIT: usize = 64;
/// Exact parallelotope cutter shape (vertices, faces).
const PARALLELOTOPE_VERTICES: usize = 8;
const PARALLELOTOPE_FACES: usize = 6;
/// Required number of strict-interior branch roots for a finite slab.
const SLAB_ROOT_COUNT: usize = 2;

/// The admitted graph source, the affine cutter, and the graph frame.
struct GeneralOperands<'a> {
    source: &'a Model,
    cutter: &'a Model,
    source_is_a: bool,
    frame: GraphFrame<'a>,
}

fn select_general_operands<'a>(
    a: &'a Model,
    b: &'a Model,
    operation: &str,
) -> Result<GeneralOperands<'a>> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "intersection" | "difference") {
        return Err(refuse(
            "General NURBS author admits union, intersection, or difference only",
        ));
    }
    let a_graph = canonical_graph_frame_cell(a, true, true).ok();
    let b_graph = canonical_graph_frame_cell(b, true, true).ok();
    let (source, cutter, source_is_a, frame) = match (a_graph, b_graph) {
        (Some(frame), None) => (a, b, true, frame),
        (None, Some(frame)) => (b, a, false, frame),
        _ => {
            return Err(refuse(
                "Exactly one operand must be an admitted multispan graph solid",
            ));
        }
    };
    let tolerance = planar_tolerance(cutter);
    if cutter.bodies.len() != 1
        || cutter.shells.len() != 1
        || !cutter.bodies[0].inner_shells.is_empty()
        || cutter.faces.iter().any(|face| {
            planar_support(&face.surface, tolerance).is_none()
                || face.surface.periodic_u
                || face.surface.periodic_v
        })
    {
        return Err(refuse(
            "General NURBS cutter must be one bounded affine closed solid",
        ));
    }
    if cutter.faces.iter().any(|face| {
        face.surface
            .weights
            .iter()
            .flatten()
            .any(|weight| weight.to_bits() != 1f64.to_bits())
    }) {
        return Err(refuse(
            "Unsupported rational cutter boundary parameterization",
        ));
    }
    Ok(GeneralOperands {
        source,
        cutter,
        source_is_a,
        frame,
    })
}

/// Accumulated nurbs-ss/1 reports and certified fragments over all cutter faces.
struct CutterSurvey {
    fragments: Vec<CertifiedBranchFragment>,
    candidate_pairs: usize,
    plane_span_count: usize,
    denominator_lower_bound: f64,
    ss_face_pairs: usize,
    ss_roots: Vec<f64>,
}

impl CutterSurvey {
    fn collect(
        operands: &GeneralOperands<'_>,
        context: &ToleranceContext,
        clear: f64,
        source_decomposition: &RationalBezierDecomposition,
    ) -> Result<Self> {
        let mut survey = Self {
            fragments: Vec::new(),
            candidate_pairs: 0,
            plane_span_count: 0,
            denominator_lower_bound: source_decomposition.denominator_lower_bound,
            ss_face_pairs: 0,
            ss_roots: Vec::new(),
        };
        let tolerance = planar_tolerance(operands.cutter);
        for (cutter_face, face) in operands.cutter.faces.iter().enumerate() {
            let (plane_origin, plane_normal) = planar_support(&face.surface, tolerance)
                .ok_or_else(|| {
                    refuse("General NURBS cutter must be one bounded affine closed solid")
                })?;
            let (_, top, ..) = operands.frame;
            let (minimum, maximum) = signed_distance_range(top, plane_origin, plane_normal);
            if minimum < -clear && maximum > clear {
                survey.add_transverse_face(operands, context, clear, cutter_face, &face.surface)?;
            } else if minimum <= clear && maximum >= -clear {
                return Err(refuse(
                    "Tangent, coincident, or gray-band cutter face is ambiguous",
                ));
            }
        }
        Ok(survey)
    }

    fn add_transverse_face(
        &mut self,
        operands: &GeneralOperands<'_>,
        context: &ToleranceContext,
        clear: f64,
        cutter_face: usize,
        surface: &Surface,
    ) -> Result<()> {
        let (source_face, top, ..) = operands.frame;
        let ss = nurbs_core::ss_intersection::intersect_surface_surface_report(
            top,
            surface,
            operands.source.tolerance_context().ok(),
        )
        .map_err(|error| {
            refuse(&format!(
                "nurbs-ss/1 refused cutter face {cutter_face}: {}",
                error.message
            ))
        })?;
        if !ss.complete() {
            return Err(refuse(
                "nurbs-boolean/1 requires complete nurbs-ss/1 reports without Boolean mutation authority",
            ));
        }
        self.ss_face_pairs = self
            .ss_face_pairs
            .checked_add(1)
            .ok_or_else(|| Error::new("BREP_SS_RESOURCE_LIMIT", "SS face-pair overflow"))?;
        for component in &ss.components {
            if let Some(root) = ss_component_root(component, clear)? {
                self.ss_roots.push(root);
            }
        }
        let graph = certify_multispan_ss(
            top,
            surface,
            [source_face, cutter_face],
            context,
            MULTISPAN_RESOURCE_LIMIT,
        )
        .map_err(|error| {
            refuse(&format!(
                "Cutter face {cutter_face} did not provide a complete transverse branch: {}",
                error.message
            ))
        })?;
        self.candidate_pairs = self
            .candidate_pairs
            .checked_add(graph.certificate.candidate_span_pairs)
            .ok_or_else(|| Error::new("BREP_SS_RESOURCE_LIMIT", "Branch-pair count overflow"))?;
        self.plane_span_count += graph.certificate.source_span_count[1];
        self.denominator_lower_bound = self
            .denominator_lower_bound
            .min(graph.certificate.denominator_lower_bound);
        self.fragments.extend(
            graph
                .components
                .into_iter()
                .flat_map(|component| component.fragments),
        );
        Ok(())
    }

    /// Joins the fragments and checks that SS roots agree with the sealed
    /// BranchGraph roots; returns the graph, the branch axis, and the roots.
    fn into_branch_roots(
        mut self,
        context: &ToleranceContext,
        clear: f64,
        source_span_count: usize,
    ) -> Result<(BranchGraph, ExactIsoAxis, [f64; 2])> {
        let branch_graph = join_certified_multispan_fragments(
            self.fragments,
            context,
            [source_span_count, self.plane_span_count],
            self.candidate_pairs,
            self.denominator_lower_bound,
            MULTISPAN_RESOURCE_LIMIT,
        )?;
        if branch_graph.components.len() < 2 {
            return Err(refuse(
                "General NURBS author requires multiple disjoint transverse branches",
            ));
        }
        let (branch_axis, mut roots) = branch_iso_roots(&branch_graph)?;
        roots.sort_by(f64::total_cmp);
        roots.dedup_by(|left, right| left.to_bits() == right.to_bits());
        let roots: [f64; SLAB_ROOT_COUNT] = match roots[..] {
            [low, high] if low > 0. && high < 1. => [low, high],
            _ => {
                return Err(refuse(
                    "Finite affine slab requires exactly two strict-interior branch roots",
                ));
            }
        };
        self.ss_roots.sort_by(f64::total_cmp);
        self.ss_roots
            .dedup_by(|left, right| (*left - *right).abs() <= clear);
        if self.ss_roots.len() != roots.len()
            || self
                .ss_roots
                .iter()
                .zip(&roots)
                .any(|(ss, branch)| (ss - branch).abs() > clear)
        {
            return Err(refuse(
                "nurbs-ss/1 transverse roots disagree with sealed BranchGraph roots",
            ));
        }
        if self.ss_face_pairs == 0 {
            return Err(refuse(
                "nurbs-boolean/1 requires at least one complete nurbs-ss/1 face pair",
            ));
        }
        let axis = branch_axis.ok_or_else(|| {
            refuse("General NURBS author requires multiple disjoint transverse branches")
        })?;
        Ok((branch_graph, axis, roots))
    }
}

/// Signed distance range of the source control net from a cutter plane.
fn signed_distance_range(
    top: &Surface,
    plane_origin: [f64; 3],
    plane_normal: [f64; 3],
) -> (f64, f64) {
    top.control_points
        .iter()
        .flatten()
        .map(|point| {
            dot(
                sub([point[0], point[1], point[2]], plane_origin),
                plane_normal,
            )
        })
        .fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(low, high), distance| (low.min(distance), high.max(distance)),
        )
}

/// Source-chart iso root of one nurbs-ss/1 component; overlaps contribute none.
fn ss_component_root(
    component: &nurbs_core::ss_intersection::SurfaceSurfaceComponent,
    clear: f64,
) -> Result<Option<f64>> {
    use nurbs_core::intersection::ContactClass;
    use nurbs_core::ss_intersection::SurfaceSurfaceComponent;
    let [start, end] = match component {
        SurfaceSurfaceComponent::Exact(branch) => branch.first_trace,
        SurfaceSurfaceComponent::Continued(branch)
            if branch.contact == ContactClass::Transverse =>
        {
            [branch.first_uv, branch.last_uv]
        }
        SurfaceSurfaceComponent::Overlap(_) => return Ok(None),
        _ => {
            return Err(refuse(
                "nurbs-boolean/1 admits only transverse curve strata from nurbs-ss/1",
            ));
        }
    };
    let [s0, s1] = start;
    let [e0, e1] = end;
    if (s0 - e0).abs() <= clear {
        Ok(Some(s0))
    } else if (s1 - e1).abs() <= clear {
        Ok(Some(s1))
    } else {
        Err(refuse(
            "nurbs-boolean/1 requires iso-aligned SS pcurves on the source chart",
        ))
    }
}

/// Shared iso axis and fixed parameter of every BranchGraph component.
fn branch_iso_roots(branch_graph: &BranchGraph) -> Result<(Option<ExactIsoAxis>, Vec<f64>)> {
    let mut branch_axis = None;
    let mut roots = Vec::new();
    for component in &branch_graph.components {
        let not_iso = || refuse("Branch pcurve is not an exact source iso");
        let trace = component
            .fragments
            .first()
            .and_then(|fragment| fragment.pcurves.first())
            .ok_or_else(not_iso)?;
        let (Some(a), Some(b)) = (trace.control_points.first(), trace.control_points.last()) else {
            return Err(not_iso());
        };
        let (axis, fixed) = if a[0].to_bits() == b[0].to_bits() {
            (ExactIsoAxis::U, a[0])
        } else if a[1].to_bits() == b[1].to_bits() {
            (ExactIsoAxis::V, a[1])
        } else {
            return Err(not_iso());
        };
        if branch_axis
            .replace(axis)
            .is_some_and(|previous| previous != axis)
        {
            return Err(refuse("Disjoint branch families cross"));
        }
        roots.push(fixed);
    }
    Ok((branch_axis, roots))
}

/// Source-chart `[umin, umax, vmin, vmax]` rectangles.
type PatchBounds = Vec<[f64; 4]>;

/// Source and naming bounds of the exact region partition for `operation`.
fn partition_bounds(
    operation: &str,
    source_is_a: bool,
    axis: ExactIsoAxis,
    roots: [f64; 2],
) -> Result<(PatchBounds, PatchBounds)> {
    let between = match axis {
        ExactIsoAxis::U => [roots[0], roots[1], 0., 1.],
        ExactIsoAxis::V => [0., 1., roots[0], roots[1]],
    };
    let outside = match axis {
        ExactIsoAxis::U => vec![[0., roots[0], 0., 1.], [roots[1], 1., 0., 1.]],
        ExactIsoAxis::V => vec![[0., 1., 0., roots[0]], [0., 1., roots[1], 1.]],
    };
    let source_bounds = match (operation, source_is_a) {
        ("intersection", _) => vec![between],
        ("difference" | "union", true) => outside,
        ("union", false) => return Err(refuse("Union requires graph-source operand order")),
        ("difference", false) => vec![],
        _ => {
            return Err(refuse(
                "General NURBS author admits union, intersection, or difference only",
            ));
        }
    };
    let naming_bounds = if source_bounds.is_empty() {
        vec![between]
    } else {
        source_bounds.clone()
    };
    Ok((source_bounds, naming_bounds))
}

/// Graph-frame coordinates of the certified affine slab cutter.
struct SlabFrame {
    normal: [f64; 3],
    projected: [[f64; 3]; 2],
    floor_height: f64,
    roots: [f64; 2],
    axis: ExactIsoAxis,
}

/// Inverse Gram projection onto the graph frame `(u, v, normal)`.
struct FrameProjection {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    normal: [f64; 3],
    uu: f64,
    uv: f64,
    vv: f64,
    determinant: f64,
}

impl FrameProjection {
    fn local(&self, point: [f64; 3]) -> [f64; 3] {
        let rhs = sub(point, self.origin);
        let ru = dot(rhs, self.u);
        let rv = dot(rhs, self.v);
        [
            (ru * self.vv - rv * self.uv) / self.determinant,
            (rv * self.uu - ru * self.uv) / self.determinant,
            dot(rhs, self.normal),
        ]
    }
}

impl SlabFrame {
    fn certify(
        operands: &GeneralOperands<'_>,
        axis: ExactIsoAxis,
        roots: [f64; 2],
        clear: f64,
    ) -> Result<Self> {
        let (_, top, origin, u, v, floor_offset) = operands.frame;
        let cutter = operands.cutter;
        let normal = normalize(cross(u, v)).ok_or_else(|| refuse("Graph frame is singular"))?;
        let normal = if dot(floor_offset, normal) < 0. {
            normal
        } else {
            normal.map(|value| -value)
        };
        let uu = dot(u, u);
        let uv = dot(u, v);
        let vv = dot(v, v);
        let determinant = uu * vv - uv * uv;
        if determinant <= SINGULAR_PROJECTION_DETERMINANT {
            return Err(refuse("Graph projection is singular"));
        }
        let projection = FrameProjection {
            origin,
            u,
            v,
            normal,
            uu,
            uv,
            vv,
            determinant,
        };
        let mut projected = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        for vertex in &cutter.vertices {
            let values = projection.local(vertex.point);
            for index in 0..3 {
                projected[0][index] = projected[0][index].min(values[index]);
                projected[1][index] = projected[1][index].max(values[index]);
            }
        }
        let (varying, fixed) = match axis {
            ExactIsoAxis::U => (1, 0),
            ExactIsoAxis::V => (0, 1),
        };
        let floor_height = dot(floor_offset, normal);
        let roof_height = top
            .control_points
            .iter()
            .flatten()
            .map(|point| dot(sub([point[0], point[1], point[2]], origin), normal))
            .fold(f64::NEG_INFINITY, f64::max);
        if cutter.vertices.len() != PARALLELOTOPE_VERTICES
            || cutter.faces.len() != PARALLELOTOPE_FACES
            || cutter.vertices.iter().any(|vertex| {
                let local = projection.local(vertex.point);
                (0..3).any(|index| {
                    (local[index] - projected[0][index]).abs() > clear
                        && (local[index] - projected[1][index]).abs() > clear
                })
            })
        {
            return Err(refuse(
                "Affine cutter is not an exact graph-frame parallelotope",
            ));
        }
        if projected[0][varying] > -clear
            || projected[1][varying] < 1. + clear
            || projected[0][2] > floor_height - clear
            || projected[1][2] < roof_height + clear
            || (projected[0][fixed] - roots[0]).abs() > clear
            || (projected[1][fixed] - roots[1]).abs() > clear
        {
            return Err(refuse(
                "Affine cutter lacks complete varying/floor/roof slab coverage",
            ));
        }
        Ok(Self {
            normal,
            projected,
            floor_height,
            roots,
            axis,
        })
    }

    /// Disjoint-interior cells whose separated union is the regularized result.
    fn partition_pieces(
        &self,
        operands: &GeneralOperands<'_>,
        operation: &str,
        source_bounds: &[[f64; 4]],
        clear: f64,
    ) -> Result<Vec<Model>> {
        let (_, top, origin, u, v, floor_offset) = operands.frame;
        let source = operands.source;
        let mut pieces = source_bounds
            .iter()
            .map(|bound| clipped_graph_solid(source, top, origin, u, v, floor_offset, *bound))
            .collect::<Result<Vec<_>>>()?;
        if operation == "union" {
            pieces.push(operands.cutter.clone());
        } else if operation == "difference" && !operands.source_is_a {
            self.push_tool_minus_source(operands, clear, &mut pieces)?;
        }
        Ok(pieces)
    }

    fn push_tool_minus_source(
        &self,
        operands: &GeneralOperands<'_>,
        clear: f64,
        pieces: &mut Vec<Model>,
    ) -> Result<()> {
        let (_, top, origin, u, v, _) = operands.frame;
        let (source, cutter) = (operands.source, operands.cutter);
        let (normal, projected, floor_height) = (self.normal, self.projected, self.floor_height);
        let [fixed_min, fixed_max] = self.roots;
        let (u_bounds, v_bounds) = match self.axis {
            ExactIsoAxis::U => ([fixed_min, fixed_max], [0., 1.]),
            ExactIsoAxis::V => ([0., 1.], [fixed_min, fixed_max]),
        };
        let tolerance = source.tolerance_mm.max(cutter.tolerance_mm);
        let frame_box =
            |bounds: [[f64; 2]; 3]| graph_frame_box(origin, u, v, normal, bounds, tolerance);
        let roof_offset = normal.map(|value| value * projected[1][2]);
        pieces.push(clipped_graph_solid(
            source,
            top,
            origin,
            u,
            v,
            roof_offset,
            [u_bounds[0], u_bounds[1], v_bounds[0], v_bounds[1]],
        )?);
        if projected[0][2] < floor_height - clear {
            pieces.push(frame_box([
                u_bounds,
                [projected[0][1], projected[1][1]],
                [projected[0][2], floor_height],
            ])?);
        }
        for outside_v in [[projected[0][1], 0.], [1., projected[1][1]]] {
            if outside_v[1] > outside_v[0] + clear {
                pieces.push(frame_box([
                    u_bounds,
                    outside_v,
                    [floor_height, projected[1][2]],
                ])?);
            }
        }
        for outside_u in [
            [projected[0][0], u_bounds[0]],
            [u_bounds[1], projected[1][0]],
        ] {
            if outside_u[1] > outside_u[0] + clear {
                pieces.push(frame_box([
                    outside_u,
                    [projected[0][1], projected[1][1]],
                    [floor_height, projected[1][2]],
                ])?);
            }
        }
        Ok(())
    }
}

pub(crate) fn graph_affine_strict_containment_bounds(
    graph: &Model,
    cutter: &Model,
) -> Result<Option<[f64; 3]>> {
    let Ok(frame) = canonical_graph_frame(graph) else {
        return Ok(None);
    };
    if !is_single_affine_body(cutter) {
        return Ok(None);
    }
    let bounds = match graph_frame_clearances(frame, cutter) {
        Ok(bounds) => bounds,
        Err(ClearanceFault::SingularFrame) => {
            return Err(refuse("V4 containment graph frame is singular"));
        }
        Err(ClearanceFault::SingularProjection) => return Ok(None),
    };
    let clear = graph
        .tolerance_context()
        .map_err(|_| refuse("V4 containment tolerance context is invalid"))?
        .spatial_bounds()
        .clear_mm;
    Ok((bounds.iter().all(|value| *value > clear)).then_some(bounds))
}
