use super::*;

/// Finite closed-solid author for a positive rational multispan graph and an
/// affine slab exposing at least two disjoint, transverse, complete iso
/// branches. The `/8` successor also authors an exact disjoint-interior
/// partition for union and tool-minus-source difference without fallback.
pub fn author_general_nurbs_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, GeneralNurbsBooleanCertificate)> {
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
    if cutter.bodies.len() != 1
        || cutter.shells.len() != 1
        || !cutter.bodies[0].inner_shells.is_empty()
        || cutter.faces.iter().any(|face| {
            planar_support(&face.surface, cutter.tolerance_mm.max(1e-9) * 8.).is_none()
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
    let (source_face, top, origin, u, v, floor_offset) = frame;
    let context = source
        .tolerance_context()
        .map_err(|_| refuse("General NURBS source tolerance context is invalid"))?;
    let clear = context.spatial_bounds().clear_mm;
    let source_decomposition = decompose_rational_bezier_spans(top, source_face, 64)?;
    let mut fragments = Vec::new();
    let mut candidate_pairs = 0usize;
    let mut plane_span_count = 0usize;
    let mut denominator_lower_bound = source_decomposition.denominator_lower_bound;
    let mut ss_face_pairs = 0usize;
    let mut ss_roots = Vec::<f64>::new();
    for (cutter_face, face) in cutter.faces.iter().enumerate() {
        let Some((plane_origin, plane_normal)) =
            planar_support(&face.surface, cutter.tolerance_mm.max(1e-9) * 8.)
        else {
            unreachable!()
        };
        let distances = top
            .control_points
            .iter()
            .flatten()
            .map(|point| {
                dot(
                    sub([point[0], point[1], point[2]], plane_origin),
                    plane_normal,
                )
            })
            .collect::<Vec<_>>();
        let minimum = distances.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = distances.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if minimum < -clear && maximum > clear {
            let ss = nurbs_core::ss_intersection::intersect_surface_surface_report(
                top,
                &face.surface,
                source.tolerance_context().ok(),
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
            ss_face_pairs = ss_face_pairs
                .checked_add(1)
                .ok_or_else(|| Error::new("BREP_SS_RESOURCE_LIMIT", "SS face-pair overflow"))?;
            for component in &ss.components {
                use nurbs_core::intersection::ContactClass;
                use nurbs_core::ss_intersection::SurfaceSurfaceComponent;
                let [start, end] = match component {
                    SurfaceSurfaceComponent::Exact(branch) => branch.first_trace,
                    SurfaceSurfaceComponent::Continued(branch)
                        if branch.contact == ContactClass::Transverse =>
                    {
                        [branch.first_uv, branch.last_uv]
                    }
                    SurfaceSurfaceComponent::Overlap(_) => continue,
                    _ => {
                        return Err(refuse(
                            "nurbs-boolean/1 admits only transverse curve strata from nurbs-ss/1",
                        ));
                    }
                };
                let [s0, s1] = start;
                let [e0, e1] = end;
                if (s0 - e0).abs() <= clear {
                    ss_roots.push(s0);
                } else if (s1 - e1).abs() <= clear {
                    ss_roots.push(s1);
                } else {
                    return Err(refuse(
                        "nurbs-boolean/1 requires iso-aligned SS pcurves on the source chart",
                    ));
                }
            }
            let graph = certify_multispan_ss(
                top,
                &face.surface,
                [source_face, cutter_face],
                &context,
                64,
            )
            .map_err(|error| {
                refuse(&format!(
                    "Cutter face {cutter_face} did not provide a complete transverse branch: {}",
                    error.message
                ))
            })?;
            candidate_pairs = candidate_pairs
                .checked_add(graph.certificate.candidate_span_pairs)
                .ok_or_else(|| {
                    Error::new("BREP_SS_RESOURCE_LIMIT", "Branch-pair count overflow")
                })?;
            plane_span_count += graph.certificate.source_span_count[1];
            denominator_lower_bound =
                denominator_lower_bound.min(graph.certificate.denominator_lower_bound);
            fragments.extend(
                graph
                    .components
                    .into_iter()
                    .flat_map(|component| component.fragments),
            );
        } else if minimum <= clear && maximum >= -clear {
            return Err(refuse(
                "Tangent, coincident, or gray-band cutter face is ambiguous",
            ));
        }
    }
    let branch_graph = join_certified_multispan_fragments(
        fragments,
        &context,
        [source_decomposition.spans.len(), plane_span_count],
        candidate_pairs,
        denominator_lower_bound,
        64,
    )?;
    if branch_graph.components.len() < 2 {
        return Err(refuse(
            "General NURBS author requires multiple disjoint transverse branches",
        ));
    }
    let mut branch_axis = None;
    let mut roots = Vec::new();
    for component in &branch_graph.components {
        let trace = &component.fragments[0].pcurves[0];
        let a = &trace.control_points[0];
        let b = trace.control_points.last().unwrap();
        let (axis, fixed) = if a[0].to_bits() == b[0].to_bits() {
            (ExactIsoAxis::U, a[0])
        } else if a[1].to_bits() == b[1].to_bits() {
            (ExactIsoAxis::V, a[1])
        } else {
            return Err(refuse("Branch pcurve is not an exact source iso"));
        };
        if branch_axis
            .replace(axis)
            .is_some_and(|previous| previous != axis)
        {
            return Err(refuse("Disjoint branch families cross"));
        }
        roots.push(fixed);
    }
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|left, right| left.to_bits() == right.to_bits());
    if roots.len() != 2 || roots[0] <= 0. || roots[1] >= 1. {
        return Err(refuse(
            "Finite affine slab requires exactly two strict-interior branch roots",
        ));
    }
    ss_roots.sort_by(f64::total_cmp);
    ss_roots.dedup_by(|left, right| (*left - *right).abs() <= clear);
    if ss_roots.len() != roots.len()
        || ss_roots
            .iter()
            .zip(&roots)
            .any(|(ss, branch)| (ss - branch).abs() > clear)
    {
        return Err(refuse(
            "nurbs-ss/1 transverse roots disagree with sealed BranchGraph roots",
        ));
    }
    if ss_face_pairs == 0 {
        return Err(refuse(
            "nurbs-boolean/1 requires at least one complete nurbs-ss/1 face pair",
        ));
    }
    let axis = branch_axis.unwrap();
    let uv = crate::uv_arrangement::arrange_multispan_branch_graph_uv(
        &context,
        [
            &source_decomposition.u_breaks,
            &source_decomposition.v_breaks,
        ],
        &branch_graph,
        0,
        64,
    )?;

    let normal = normalize(cross(u, v)).ok_or_else(|| refuse("Graph frame is singular"))?;
    let normal = if dot(floor_offset, normal) < 0. {
        normal
    } else {
        normal.map(|value| -value)
    };
    let uu = dot(u, u);
    let uv_dot = dot(u, v);
    let vv = dot(v, v);
    let determinant = uu * vv - uv_dot * uv_dot;
    if determinant <= 1e-18 {
        return Err(refuse("Graph projection is singular"));
    }
    let mut projected = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for vertex in &cutter.vertices {
        let rhs = sub(vertex.point, origin);
        let ru = dot(rhs, u);
        let rv = dot(rhs, v);
        let values = [
            (ru * vv - rv * uv_dot) / determinant,
            (rv * uu - ru * uv_dot) / determinant,
            dot(rhs, normal),
        ];
        for index in 0..3 {
            projected[0][index] = projected[0][index].min(values[index]);
            projected[1][index] = projected[1][index].max(values[index]);
        }
    }
    let varying = if axis == ExactIsoAxis::U { 1 } else { 0 };
    let fixed = if axis == ExactIsoAxis::U { 0 } else { 1 };
    let floor_height = dot(floor_offset, normal);
    let roof_height = top
        .control_points
        .iter()
        .flatten()
        .map(|point| dot(sub([point[0], point[1], point[2]], origin), normal))
        .fold(f64::NEG_INFINITY, f64::max);
    if cutter.vertices.len() != 8
        || cutter.faces.len() != 6
        || cutter.vertices.iter().any(|vertex| {
            let rhs = sub(vertex.point, origin);
            let ru = dot(rhs, u);
            let rv = dot(rhs, v);
            let local = [
                (ru * vv - rv * uv_dot) / determinant,
                (rv * uu - ru * uv_dot) / determinant,
                dot(rhs, normal),
            ];
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
    let source_bounds = match (operation, source_is_a, axis) {
        ("intersection", _, ExactIsoAxis::U) => vec![[roots[0], roots[1], 0., 1.]],
        ("intersection", _, ExactIsoAxis::V) => vec![[0., 1., roots[0], roots[1]]],
        ("difference", true, ExactIsoAxis::U) => {
            vec![[0., roots[0], 0., 1.], [roots[1], 1., 0., 1.]]
        }
        ("difference", true, ExactIsoAxis::V) => {
            vec![[0., 1., 0., roots[0]], [0., 1., roots[1], 1.]]
        }
        ("union", true, ExactIsoAxis::U) => {
            vec![[0., roots[0], 0., 1.], [roots[1], 1., 0., 1.]]
        }
        ("union", true, ExactIsoAxis::V) => {
            vec![[0., 1., 0., roots[0]], [0., 1., roots[1], 1.]]
        }
        ("union", false, _) => return Err(refuse("Union requires graph-source operand order")),
        ("difference", false, _) => vec![],
        _ => unreachable!(),
    };
    let naming_bounds = if source_bounds.is_empty() {
        match axis {
            ExactIsoAxis::U => vec![[roots[0], roots[1], 0., 1.]],
            ExactIsoAxis::V => vec![[0., 1., roots[0], roots[1]]],
        }
    } else {
        source_bounds.clone()
    };
    let mut pieces = source_bounds
        .iter()
        .map(|bound| clipped_graph_solid(source, top, origin, u, v, floor_offset, *bound))
        .collect::<Result<Vec<_>>>()?;
    if operation == "union" {
        pieces.push(cutter.clone());
    } else if operation == "difference" && !source_is_a {
        let [fixed_min, fixed_max] = [roots[0], roots[1]];
        let (u_bounds, v_bounds) = match axis {
            ExactIsoAxis::U => ([fixed_min, fixed_max], [0., 1.]),
            ExactIsoAxis::V => ([0., 1.], [fixed_min, fixed_max]),
        };
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
            pieces.push(graph_frame_box(
                origin,
                u,
                v,
                normal,
                [
                    u_bounds,
                    [projected[0][1], projected[1][1]],
                    [projected[0][2], floor_height],
                ],
                source.tolerance_mm.max(cutter.tolerance_mm),
            )?);
        }
        for outside_v in [[projected[0][1], 0.], [1., projected[1][1]]] {
            if outside_v[1] > outside_v[0] + clear {
                pieces.push(graph_frame_box(
                    origin,
                    u,
                    v,
                    normal,
                    [u_bounds, outside_v, [floor_height, projected[1][2]]],
                    source.tolerance_mm.max(cutter.tolerance_mm),
                )?);
            }
        }
        for outside_u in [
            [projected[0][0], u_bounds[0]],
            [u_bounds[1], projected[1][0]],
        ] {
            if outside_u[1] > outside_u[0] + clear {
                pieces.push(graph_frame_box(
                    origin,
                    u,
                    v,
                    normal,
                    [
                        outside_u,
                        [projected[0][1], projected[1][1]],
                        [floor_height, projected[1][2]],
                    ],
                    source.tolerance_mm.max(cutter.tolerance_mm),
                )?);
            }
        }
    }
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
        source,
        cutter,
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
        operand_order: if source_is_a {
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

pub(crate) fn graph_affine_strict_containment_bounds(
    graph: &Model,
    cutter: &Model,
) -> Result<Option<[f64; 3]>> {
    let Ok((_, top, origin, u, v, floor_offset)) = canonical_graph_frame(graph) else {
        return Ok(None);
    };
    if cutter.bodies.len() != 1
        || cutter.shells.len() != 1
        || !cutter.bodies[0].inner_shells.is_empty()
        || cutter
            .faces
            .iter()
            .any(|face| planar_support(&face.surface, cutter.tolerance_mm.max(1e-9) * 8.).is_none())
    {
        return Ok(None);
    }
    let normal =
        normalize(cross(u, v)).ok_or_else(|| refuse("V4 containment graph frame is singular"))?;
    let normal = if dot(floor_offset, normal) < 0. {
        normal
    } else {
        normal.map(|value| -value)
    };
    let floor_height = dot(floor_offset, normal);
    let roof_lower_bound = top
        .control_points
        .iter()
        .flatten()
        .map(|point| dot(sub([point[0], point[1], point[2]], origin), normal))
        .fold(f64::INFINITY, f64::min);
    let uu = dot(u, u);
    let uv = dot(u, v);
    let vv = dot(v, v);
    let determinant = uu * vv - uv * uv;
    if determinant <= 1e-18 {
        return Ok(None);
    }
    let clear = graph
        .tolerance_context()
        .map_err(|_| refuse("V4 containment tolerance context is invalid"))?
        .spatial_bounds()
        .clear_mm;
    let mut bounds = [f64::INFINITY; 3];
    for vertex in &cutter.vertices {
        let rhs = sub(vertex.point, origin);
        let ru = dot(rhs, u);
        let rv = dot(rhs, v);
        let su = (ru * vv - rv * uv) / determinant;
        let sv = (rv * uu - ru * uv) / determinant;
        let height = dot(rhs, normal);
        bounds[0] = bounds[0].min(su).min(1. - su).min(sv).min(1. - sv);
        bounds[1] = bounds[1].min(height - floor_height);
        bounds[2] = bounds[2].min(roof_lower_bound - height);
    }
    Ok((bounds.iter().all(|value| *value > clear)).then_some(bounds))
}
