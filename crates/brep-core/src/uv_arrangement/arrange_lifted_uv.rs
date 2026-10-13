use super::*;

fn arrange_lifted_uv_impl(
    context: &ToleranceContext,
    chart: ChartKind,
    primitives: &[LiftedUvPrimitive],
    resource_limit: usize,
    tensor_admitted: bool,
) -> Result<LiftedUvArrangement> {
    if chart == ChartKind::Freeform
        && !primitives
            .iter()
            .all(|p| matches!(p.geometry, LiftedUvGeometry::RationalCurvedTrace { .. }))
    {
        return Err(refuse(
            "Generic Freeform curves are not admitted for Complete UV arrangement",
        ));
    }
    if resource_limit == 0 || primitives.len() > resource_limit {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Lifted UV primitive budget exceeded",
        ));
    }
    let mut segments: Vec<(usize, [f64; 2], [f64; 2])> = Vec::new();
    let mut arcs: Vec<(usize, [f64; 2], f64, [f64; 2])> = Vec::new();
    for primitive in primitives {
        match (&primitive.geometry, chart) {
            (LiftedUvGeometry::PlaneSegment { start, end }, ChartKind::PlanePoly) => {
                if !start.iter().chain(end).all(|v| v.is_finite()) || start == end {
                    return Err(refuse("PlanePoly segment must be finite and nondegenerate"));
                }
                segments.push((primitive.edge_id, *start, *end));
            }
            (
                LiftedUvGeometry::AnalyticCircleArc {
                    center,
                    radius,
                    interval,
                },
                ChartKind::AnalyticCircle,
            ) => {
                if !center.iter().chain(interval).all(|v| v.is_finite())
                    || !radius.is_finite()
                    || *radius <= 0.
                    || interval[1] <= interval[0]
                    || interval[1] - interval[0] >= std::f64::consts::TAU
                {
                    return Err(refuse(
                        "AnalyticCircle arc must be finite and span less than TAU",
                    ));
                }
                arcs.push((primitive.edge_id, *center, *radius, *interval));
            }
            (
                LiftedUvGeometry::TensorIsoLine {
                    axis,
                    fixed,
                    interval,
                },
                ChartKind::TensorBezierGraph,
            ) if tensor_admitted => {
                let (start, end) = match axis {
                    TensorAxis::U => ([*fixed, interval[0]], [*fixed, interval[1]]),
                    TensorAxis::V => ([interval[0], *fixed], [interval[1], *fixed]),
                };
                segments.push((primitive.edge_id, start, end));
            }
            (
                LiftedUvGeometry::TensorRectangleBoundary { side, domain },
                ChartKind::TensorBezierGraph,
            ) if tensor_admitted => {
                let [[umin, umax], [vmin, vmax]] = *domain;
                let (start, end) = match side {
                    TensorBoundary::UMin => ([umin, vmax], [umin, vmin]),
                    TensorBoundary::UMax => ([umax, vmin], [umax, vmax]),
                    TensorBoundary::VMin => ([umin, vmin], [umax, vmin]),
                    TensorBoundary::VMax => ([umax, vmax], [umin, vmax]),
                };
                segments.push((primitive.edge_id, start, end));
            }
            (
                LiftedUvGeometry::RationalCurvedTrace {
                    samples,
                    closed,
                    correspondence: _,
                    overlap: _,
                    singular,
                },
                ChartKind::PlanePoly | ChartKind::Freeform,
            ) => {
                if *singular {
                    return Err(refuse(
                        "Singular UV strata refuse Complete DCEL until regularized",
                    ));
                }
                if samples.len() < 2 || samples.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(refuse(
                        "Rational curved UV trace needs at least two finite samples",
                    ));
                }
                for window in samples.windows(2) {
                    if window[0] != window[1] {
                        segments.push((primitive.edge_id, window[0], window[1]));
                    }
                }
                if *closed && samples.len() > 2 {
                    let first = samples[0];
                    let last = *samples.last().unwrap();
                    if first != last {
                        segments.push((primitive.edge_id, last, first));
                    }
                }
            }
            _ => return Err(refuse("Lifted primitive does not match its admitted chart")),
        }
    }

    // Split PlanePoly segments at every constructed event vertex.
    let mut split_parameters = vec![vec![0., 1.]; segments.len()];
    let mut intersections: BTreeMap<(u64, u64), [usize; 2]> = BTreeMap::new();
    for i in 0..segments.len() {
        for j in (i + 1)..segments.len() {
            if let Some((ti, tj, point)) =
                segment_intersection(segments[i].1, segments[i].2, segments[j].1, segments[j].2)?
            {
                split_parameters[i].push(ti);
                split_parameters[j].push(tj);
                intersections.insert(
                    point_key(point),
                    [
                        segments[i].0.min(segments[j].0),
                        segments[i].0.max(segments[j].0),
                    ],
                );
            }
        }
    }
    // Circle arcs may meet at authored endpoints, but overlapping angular
    // interiors are not a unique branch and are refused.
    for i in 0..arcs.len() {
        for j in (i + 1)..arcs.len() {
            if arcs[i].1 == arcs[j].1 && arcs[i].2.to_bits() == arcs[j].2.to_bits() {
                let lo = arcs[i].3[0].max(arcs[j].3[0]);
                let hi = arcs[i].3[1].min(arcs[j].3[1]);
                if hi > lo {
                    return Err(refuse("Overlapping periodic arc branches are ambiguous"));
                }
            } else {
                return Err(refuse(
                    "Intersecting arcs from different analytic circles are outside the admitted matrix",
                ));
            }
        }
    }

    let mut vertices = Vec::<UvVertex>::new();
    let mut vertex_ids = BTreeMap::<(u64, u64), usize>::new();
    let add_vertex = |point: [f64; 2],
                      origin: UvVertexOrigin,
                      vertices: &mut Vec<UvVertex>,
                      ids: &mut BTreeMap<(u64, u64), usize>| {
        let key = point_key(point);
        if let Some(&id) = ids.get(&key) {
            if !matches!(&origin, UvVertexOrigin::AuthoredEndpoint) {
                vertices[id].origin = origin;
            }
            id
        } else {
            let id = vertices.len();
            vertices.push(UvVertex { point, origin });
            ids.insert(key, id);
            id
        }
    };
    let mut pieces: Vec<(usize, usize, usize, UvHalfedgeGeometry)> = Vec::new();
    for (index, (edge, start, end)) in segments.iter().enumerate() {
        let mut ts = split_parameters[index].clone();
        ts.sort_by(|a, b| a.total_cmp(b));
        ts.dedup_by(|a, b| (*a - *b).abs() <= 1e-14);
        for pair in ts.windows(2) {
            if pair[1] - pair[0] <= 1e-14 {
                continue;
            }
            let point = |t: f64| {
                [
                    start[0] + t * (end[0] - start[0]),
                    start[1] + t * (end[1] - start[1]),
                ]
            };
            let pa = point(pair[0]);
            let pb = point(pair[1]);
            let origin_for = |p: [f64; 2]| {
                intersections
                    .get(&point_key(p))
                    .copied()
                    .map(|edges| UvVertexOrigin::ConstructedIntersection { edges })
                    .unwrap_or(UvVertexOrigin::AuthoredEndpoint)
            };
            let a = add_vertex(pa, origin_for(pa), &mut vertices, &mut vertex_ids);
            let b = add_vertex(pb, origin_for(pb), &mut vertices, &mut vertex_ids);
            pieces.push((*edge, a, b, UvHalfedgeGeometry::Segment));
        }
    }
    for (edge, center, radius, interval) in arcs {
        let point = |angle: f64| {
            let snap = |value: f64| {
                if value.abs() <= 32. * f64::EPSILON {
                    0.
                } else if (value - 1.).abs() <= 32. * f64::EPSILON {
                    1.
                } else if (value + 1.).abs() <= 32. * f64::EPSILON {
                    -1.
                } else {
                    value
                }
            };
            [
                center[0] + radius * snap(angle.cos()),
                center[1] + radius * snap(angle.sin()),
            ]
        };
        let shift = (interval[0] / std::f64::consts::TAU).floor() as i32;
        let a = add_vertex(
            point(interval[0]),
            if shift != 0 {
                UvVertexOrigin::PeriodicSeam { edge, shift }
            } else {
                UvVertexOrigin::AuthoredEndpoint
            },
            &mut vertices,
            &mut vertex_ids,
        );
        let end_shift = (interval[1] / std::f64::consts::TAU).floor() as i32;
        let b = add_vertex(
            point(interval[1]),
            if end_shift != shift {
                UvVertexOrigin::PeriodicSeam {
                    edge,
                    shift: end_shift,
                }
            } else {
                UvVertexOrigin::AuthoredEndpoint
            },
            &mut vertices,
            &mut vertex_ids,
        );
        pieces.push((
            edge,
            a,
            b,
            UvHalfedgeGeometry::CircleArc {
                center,
                radius,
                interval,
            },
        ));
    }
    if pieces.len() > resource_limit.saturating_mul(4) {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Constructed UV halfedge budget exceeded",
        ));
    }
    let mut halfedges = Vec::with_capacity(pieces.len() * 2);
    for (source_edge, a, b, geometry) in pieces {
        let forward = halfedges.len();
        let reverse = forward + 1;
        halfedges.push(UvHalfedge {
            origin: a,
            destination: b,
            twin: reverse,
            next: usize::MAX,
            cell: usize::MAX,
            source_edge,
            geometry: geometry.clone(),
        });
        let reverse_geometry = match geometry {
            UvHalfedgeGeometry::CircleArc {
                center,
                radius,
                interval,
            } => UvHalfedgeGeometry::CircleArc {
                center,
                radius,
                interval: [interval[1], interval[0]],
            },
            UvHalfedgeGeometry::Segment => UvHalfedgeGeometry::Segment,
        };
        halfedges.push(UvHalfedge {
            origin: b,
            destination: a,
            twin: forward,
            next: usize::MAX,
            cell: usize::MAX,
            source_edge,
            geometry: reverse_geometry,
        });
    }
    let mut outgoing = vec![Vec::<usize>::new(); vertices.len()];
    for (id, halfedge) in halfedges.iter().enumerate() {
        outgoing[halfedge.origin].push(id);
    }
    for edges in &mut outgoing {
        if edges.len() > 4 {
            return Err(refuse("UV event has ambiguous branch degree"));
        }
        edges.sort_by(|&a, &b| {
            let direction = |id: usize| {
                let h = &halfedges[id];
                let p = vertices[h.origin].point;
                let q = vertices[h.destination].point;
                (q[1] - p[1]).atan2(q[0] - p[0])
            };
            direction(a)
                .total_cmp(&direction(b))
                .then_with(|| a.cmp(&b))
        });
    }
    for halfedge in halfedges.iter_mut() {
        let destination = halfedge.destination;
        let twin = halfedge.twin;
        let around = &outgoing[destination];
        let twin_position = around
            .iter()
            .position(|candidate| *candidate == twin)
            .ok_or_else(|| refuse("DCEL twin is absent from destination star"))?;
        halfedge.next = around[(twin_position + around.len() - 1) % around.len()];
    }
    let mut cells = Vec::new();
    for start in 0..halfedges.len() {
        if halfedges[start].cell != usize::MAX {
            continue;
        }
        let cell_id = cells.len();
        let mut boundary = Vec::new();
        let mut current = start;
        loop {
            if boundary.len() > halfedges.len() {
                return Err(refuse("DCEL next relation does not close"));
            }
            if halfedges[current].cell != usize::MAX {
                if current != start {
                    return Err(refuse("DCEL branches merge before closing a cell"));
                }
                break;
            }
            halfedges[current].cell = cell_id;
            boundary.push(current);
            current = halfedges[current].next;
        }
        let mut twice_area = 0.;
        for &id in &boundary {
            let h = &halfedges[id];
            let p = vertices[h.origin].point;
            let q = vertices[h.destination].point;
            twice_area += match h.geometry {
                UvHalfedgeGeometry::Segment => cross(p, q),
                UvHalfedgeGeometry::CircleArc {
                    center,
                    radius,
                    interval,
                } => {
                    cross(center, q) - cross(center, p)
                        + radius * radius * (interval[1] - interval[0])
                }
            };
        }
        let signed_area = 0.5 * twice_area;
        let winding = if signed_area > 0. { 1 } else { 0 };
        cells.push(UvCell {
            boundary,
            signed_area,
            winding,
            label: if winding == 0 {
                WindingLabel::Exterior
            } else {
                WindingLabel::Material(winding)
            },
        });
    }
    if !cells.iter().any(|cell| cell.winding != 0) {
        return Err(refuse("UV primitives do not bound a material cell"));
    }
    let coverage = certify_lifted_uv_coverage(
        context,
        primitives.len(),
        vertices.len(),
        halfedges.len(),
        cells.len(),
        true,
        resource_limit,
    )?;
    Ok(LiftedUvArrangement {
        chart,
        context: context.spec_identity(),
        vertices,
        halfedges,
        cells,
        coverage,
    })
}

/// Build a finite context-bound DCEL for the admitted 2D primitive matrix.
/// TensorBezierGraph is intentionally reachable only through the stricter
/// rectangle validator below.
pub fn arrange_lifted_uv(
    context: &ToleranceContext,
    chart: ChartKind,
    primitives: &[LiftedUvPrimitive],
    resource_limit: usize,
) -> Result<LiftedUvArrangement> {
    arrange_lifted_uv_impl(context, chart, primitives, resource_limit, false)
}

/// DCEL for certified rational curved SS pcurve traces (overlaps, loops, holes).
/// Does not use the graph-patch iso fixture path.
pub fn arrange_rational_curved_ss_uv(
    context: &ToleranceContext,
    primitives: &[LiftedUvPrimitive],
    resource_limit: usize,
) -> Result<LiftedUvArrangement> {
    arrange_lifted_uv_impl(
        context,
        ChartKind::PlanePoly,
        primitives,
        resource_limit,
        false,
    )
}

/// Exact finite chart for one tensor rectangle and its complete set of
/// constant-U/V iso branches. Diagonals, partial branches and generic
/// Freeform curves are not representable in this entry point.
pub fn arrange_tensor_bezier_graph_uv(
    context: &ToleranceContext,
    domain: [[f64; 2]; 2],
    primitives: &[LiftedUvPrimitive],
    expected_iso_branches: usize,
    resource_limit: usize,
) -> Result<LiftedUvArrangement> {
    let [[umin, umax], [vmin, vmax]] = domain;
    if !domain.iter().flatten().all(|value| value.is_finite())
        || umin >= umax
        || vmin >= vmax
        || expected_iso_branches == 0
        || expected_iso_branches > resource_limit
    {
        return Err(refuse("Tensor chart domain or branch budget is invalid"));
    }
    if primitives.len() > resource_limit {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Tensor UV primitive budget exceeded",
        ));
    }
    let mut boundaries = BTreeMap::<TensorBoundary, usize>::new();
    let mut branches = BTreeMap::<(TensorAxis, u64), usize>::new();
    for primitive in primitives {
        match &primitive.geometry {
            LiftedUvGeometry::TensorRectangleBoundary {
                side,
                domain: actual,
            } if *actual == domain => {
                if boundaries.insert(*side, primitive.edge_id).is_some() {
                    return Err(refuse("Tensor rectangle boundary is duplicated"));
                }
            }
            LiftedUvGeometry::TensorIsoLine {
                axis,
                fixed,
                interval,
            } => {
                let (fixed_domain, varying_domain) = match axis {
                    TensorAxis::U => ([umin, umax], [vmin, vmax]),
                    TensorAxis::V => ([vmin, vmax], [umin, umax]),
                };
                if !fixed.is_finite()
                    || !(*fixed > fixed_domain[0] && *fixed < fixed_domain[1])
                    || *interval != varying_domain
                    || branches
                        .insert((*axis, fixed.to_bits()), primitive.edge_id)
                        .is_some()
                {
                    return Err(refuse(
                        "Tensor iso branch must be unique, strict-interior and span the whole rectangle",
                    ));
                }
            }
            _ => {
                return Err(refuse(
                    "Tensor chart accepts exact rectangle boundaries and U/V iso lines only",
                ));
            }
        }
    }
    if boundaries.len() != 4
        || branches.len() != expected_iso_branches
        || ![
            TensorBoundary::UMin,
            TensorBoundary::UMax,
            TensorBoundary::VMin,
            TensorBoundary::VMax,
        ]
        .iter()
        .all(|side| boundaries.contains_key(side))
    {
        return Err(refuse(
            "Tensor chart missed a rectangle boundary or certified iso branch",
        ));
    }
    let arrangement = arrange_lifted_uv_impl(
        context,
        ChartKind::TensorBezierGraph,
        primitives,
        resource_limit,
        true,
    )?;
    if arrangement.coverage.primitive_count != primitives.len() {
        return Err(refuse("Tensor chart coverage missed an authored branch"));
    }
    Ok(arrangement)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MultiSpanUvAuthority {
    pub(super) context: ToleranceSpecIdentity,
    pub(super) knot_bits: [Vec<u64>; 2],
    pub(super) branch_keys: Vec<(TensorAxis, u64, usize)>,
    pub(super) primitive_count: usize,
    pub(super) cell_count: usize,
    pub(super) tensor_cell_count: usize,
    pub(super) material_cell_count: usize,
    pub(super) hole_cell_count: usize,
}

/// Global tensor-chart result for a certified BranchGraph. Internal knot lines
/// are represented explicitly, so coverage and cell classification span the
/// complete tensor grid rather than being repeated independently per patch.
#[derive(Clone, Debug)]
pub struct MultiSpanUvArrangement {
    pub arrangement: LiftedUvArrangement,
    pub tensor_cell_count: usize,
    pub branch_count: usize,
    pub material_cell_count: usize,
    pub hole_cell_count: usize,
    pub global_coverage_complete: bool,
    pub context: ToleranceSpecIdentity,
    pub(super) authority: MultiSpanUvAuthority,
}

impl MultiSpanUvArrangement {
    pub fn permits_trim_classification(&self) -> bool {
        let material = self
            .arrangement
            .cells
            .iter()
            .filter(|cell| matches!(cell.label, WindingLabel::Material(_)))
            .count();
        self.global_coverage_complete
            && self.context == self.authority.context
            && self.arrangement.context == self.context
            && self.arrangement.coverage.complete
            && self.arrangement.coverage.primitive_count == self.authority.primitive_count
            && self.arrangement.cells.len() == self.authority.cell_count
            && self.material_cell_count == material
            && self.material_cell_count == self.authority.material_cell_count
            && self.hole_cell_count == self.authority.hole_cell_count
            && self.branch_count == self.authority.branch_keys.len()
            && self.tensor_cell_count == self.authority.tensor_cell_count
            && self.tensor_cell_count
                == (self.authority.knot_bits[0].len() - 1) * (self.authority.knot_bits[1].len() - 1)
    }
}

pub(super) fn exact_line_trace(curve: &nurbs_core::curve::Curve) -> Result<(TensorAxis, f64, [f64; 2])> {
    curve.validate()?;
    if curve.degree != 1 || curve.control_points.len() != 2 {
        return Err(refuse(
            "Multi-span tensor arrangement accepts exact linear iso pcurves only",
        ));
    }
    let a = [curve.control_points[0][0], curve.control_points[0][1]];
    let b = [curve.control_points[1][0], curve.control_points[1][1]];
    if a[0].to_bits() == b[0].to_bits() && a[1] != b[1] {
        Ok((TensorAxis::U, a[0], [a[1], b[1]]))
    } else if a[1].to_bits() == b[1].to_bits() && a[0] != b[0] {
        Ok((TensorAxis::V, a[1], [a[0], b[0]]))
    } else {
        Err(refuse(
            "Multi-span tensor branch is not an exact constant-U/V trace",
        ))
    }
}
