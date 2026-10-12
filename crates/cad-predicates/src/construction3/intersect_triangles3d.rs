use super::*;

/// Closed triangle intersection, retaining exact endpoint/vertex recipes.
/// Coplanar area overlap yields a convex contour; no B-rep topology is emitted.
pub fn intersect_triangles3d(
    ctx: &mut PredicateContext<'_>,
    a: [Point3Input<'_>; 3],
    b: [Point3Input<'_>; 3],
) -> Result<TriangleTriangleIntersection3Report, InputError> {
    let inputs = [a[0], a[1], a[2], b[0], b[1], b[2]];
    let admitted = inventory(ctx, &inputs)?;
    let outcome = (|| {
        ctx.charge(admitted.work)?;
        if admitted.depth > MAX_CONSTRUCTION_DEPTH || admitted.nodes > MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        let mut evaluation = Evaluation::default();
        let exact = [
            evaluation.point(a[0], ctx)?,
            evaluation.point(a[1], ctx)?,
            evaluation.point(a[2], ctx)?,
            evaluation.point(b[0], ctx)?,
            evaluation.point(b[1], ctx)?,
            evaluation.point(b[2], ctx)?,
        ];
        let pa = plane(&exact[0], &exact[1], &exact[2], ctx)?;
        if zero(&pa) {
            return Ok(TriangleTriangleIntersection3::DegenerateTriangle(0));
        }
        let pb = plane(&exact[3], &exact[4], &exact[5], ctx)?;
        if zero(&pb) {
            return Ok(TriangleTriangleIntersection3::DegenerateTriangle(1));
        }
        let signs = [
            dot(&pa, &exact[3], ctx)?.sign(),
            dot(&pa, &exact[4], ctx)?.sign(),
            dot(&pa, &exact[5], ctx)?.sign(),
        ];
        let coplanar = signs.iter().all(|s| *s == Sign::Zero);
        let mut points: Vec<ConstructedPoint3> = Vec::new();
        for (source, target) in [(a, b), (b, a)] {
            for i in 0..3 {
                let result = intersect_segment_triangle3d(
                    ctx,
                    source[i],
                    source[(i + 1) % 3],
                    target[0],
                    target[1],
                    target[2],
                )
                .map_err(|_| Reason::MissingProof)?
                .outcome;
                let candidates = match result {
                    SegmentTriangleIntersection3::Empty => Vec::new(),
                    SegmentTriangleIntersection3::Point(p) => vec![p.point],
                    SegmentTriangleIntersection3::Segment(p) => {
                        let [lo, hi] = *p;
                        vec![lo.point, hi.point]
                    }
                    SegmentTriangleIntersection3::Indeterminate(r) => return Err(r),
                    _ => return Err(Reason::MissingProof),
                };
                for candidate in candidates {
                    let mut position = points.len();
                    let mut duplicate = false;
                    for (j, existing) in points.iter().enumerate() {
                        match compare_points3d(
                            ctx,
                            Point3Input::Constructed(&candidate),
                            Point3Input::Constructed(existing),
                        )
                        .map_err(|_| Reason::MissingProof)?
                        .outcome
                        {
                            Outcome::Sign(Sign::Zero) => {
                                duplicate = true;
                                break;
                            }
                            Outcome::Sign(Sign::Negative) => {
                                position = j;
                                break;
                            }
                            Outcome::Sign(Sign::Positive) => {}
                            Outcome::Indeterminate(r) => return Err(r),
                        }
                    }
                    if !duplicate {
                        points.insert(position, candidate);
                    }
                }
            }
        }
        if coplanar && points.len() >= 3 {
            let axes = if pa[2].sign() != Sign::Zero {
                [0, 1]
            } else if pa[1].sign() != Sign::Zero {
                [0, 2]
            } else {
                [1, 2]
            };
            points = convex_contour3(ctx, points, axes)?;
            if points.len() >= 3 {
                if points.len() > 6 {
                    return Err(Reason::MissingProof);
                }
                return Ok(TriangleTriangleIntersection3::Polygon {
                    vertices: points,
                    projection_axes: axes,
                });
            }
        }
        // Convex triangles in distinct planes intersect in a convex subset of
        // their common line. Intermediate boundary points do not split it.
        match points.len() {
            0 => Ok(TriangleTriangleIntersection3::Empty),
            1 => Ok(TriangleTriangleIntersection3::Point(Box::new(
                points.remove(0),
            ))),
            _ => {
                let hi = points.pop().ok_or(Reason::MissingProof)?;
                let lo = points.remove(0);
                let graph = inventory(
                    ctx,
                    &[Point3Input::Constructed(&lo), Point3Input::Constructed(&hi)],
                )
                .map_err(|_| Reason::MissingProof)?;
                ctx.charge(graph.work)?;
                if graph.nodes > MAX_CONSTRUCTION_NODES {
                    return Err(Reason::ResourceLimit);
                }
                Ok(TriangleTriangleIntersection3::Segment(Box::new([lo, hi])))
            }
        }
    })()
    .and_then(|outcome| {
        ctx.charge(0)?;
        Ok(outcome)
    });
    Ok(TriangleTriangleIntersection3Report {
        outcome: outcome.unwrap_or_else(TriangleTriangleIntersection3::Indeterminate),
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}

fn convex_contour3(
    ctx: &mut PredicateContext<'_>,
    points: Vec<ConstructedPoint3>,
    axes: [usize; 2],
) -> Result<Vec<ConstructedPoint3>, Reason> {
    let inputs: Vec<_> = points.iter().map(Point3Input::Constructed).collect();
    let graph = inventory(ctx, &inputs).map_err(|_| Reason::MissingProof)?;
    ctx.charge(graph.work)?;
    if graph.nodes > MAX_CONSTRUCTION_NODES {
        return Err(Reason::ResourceLimit);
    }
    let mut evaluation = Evaluation::default();
    let exact: Vec<_> = inputs
        .into_iter()
        .map(|p| evaluation.point(p, ctx))
        .collect::<Result<_, _>>()?;
    let turn = |i: usize, j: usize, k: usize, ctx: &mut PredicateContext<'_>| {
        let row = |n: usize| [&exact[n][axes[0]], &exact[n][axes[1]], &exact[n][3]];
        Ok::<_, Reason>(determinant3(row(i), row(j), row(k), ctx)?.sign())
    };
    // The first nonsingular coordinate pair preserves lexicographic XYZ order
    // on this plane, including constant leading coordinates.
    let mut lower: Vec<usize> = Vec::new();
    let mut upper: Vec<usize> = Vec::new();
    for (chain, indices) in [
        (&mut lower, (0..points.len()).collect::<Vec<_>>()),
        (&mut upper, (0..points.len()).rev().collect()),
    ] {
        for i in indices {
            ctx.charge(1)?;
            while chain.len() >= 2
                && turn(chain[chain.len() - 2], chain[chain.len() - 1], i, ctx)? != Sign::Positive
            {
                chain.pop();
            }
            chain.push(i);
        }
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    Ok(lower.into_iter().map(|i| points[i].clone()).collect())
}

pub(super) fn triangle_location_from_signs(signs: [Sign; 3]) -> Result<TriangleLocation3, Reason> {
    let triangle = if signs.contains(&Sign::Negative) {
        TriangleLocation3::Outside
    } else {
        let zero_count = signs.iter().filter(|s| **s == Sign::Zero).count();
        match zero_count {
            0 => TriangleLocation3::Interior,
            1 => TriangleLocation3::Edge(
                signs
                    .iter()
                    .position(|s| *s == Sign::Zero)
                    .ok_or(Reason::MissingProof)?,
            ),
            2 => TriangleLocation3::Vertex(
                signs
                    .iter()
                    .position(|s| *s == Sign::Positive)
                    .ok_or(Reason::MissingProof)?,
            ),
            _ => return Err(Reason::MissingProof),
        }
    };
    Ok(triangle)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointTriangleLocation3 {
    OnPlane(TriangleLocation3),
    OffPlane(Sign),
    DegenerateTriangle,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct PointTriangleLocation3Report {
    pub outcome: PointTriangleLocation3,
    pub context: ContextIdentity,
    pub work_used: u64,
}
/// Locate an authored or constructed point on an arbitrary oriented triangle.
/// OffPlane reports its exact plane-side sign; no modelling tolerance snaps it.
pub fn classify_point_triangle3d(
    ctx: &mut PredicateContext<'_>,
    p: Point3Input<'_>,
    triangle: [Point3Input<'_>; 3],
) -> Result<PointTriangleLocation3Report, InputError> {
    let [a, b, c] = triangle;
    let admitted = inventory(ctx, &[p, a, b, c])?;
    let outcome = (|| {
        ctx.charge(admitted.work)?;
        if admitted.depth > MAX_CONSTRUCTION_DEPTH || admitted.nodes > MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        let mut evaluation = Evaluation::default();
        let p = evaluation.point(p, ctx)?;
        let a = evaluation.point(a, ctx)?;
        let b = evaluation.point(b, ctx)?;
        let c = evaluation.point(c, ctx)?;
        let support = plane(&a, &b, &c, ctx)?;
        if zero(&support) {
            return Ok(PointTriangleLocation3::DegenerateTriangle);
        }
        let side = dot(&support, &p, ctx)?.sign();
        if side != Sign::Zero {
            return Ok(PointTriangleLocation3::OffPlane(side));
        }
        let [wa, wb, wc, _] = barycentric_fraction(&p, &a, &b, &c, ctx)?;
        Ok(PointTriangleLocation3::OnPlane(
            triangle_location_from_signs([wa.sign(), wb.sign(), wc.sign()])?,
        ))
    })()
    .and_then(|outcome| {
        ctx.charge(0)?;
        Ok(outcome)
    });
    Ok(PointTriangleLocation3Report {
        outcome: outcome.unwrap_or_else(PointTriangleLocation3::Indeterminate),
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}

/// Portable data only. Importers must authenticate the authored source and
/// revalidate/replay every node; these records confer no admission authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeInput3Record {
    Authored {
        indices: [usize; 3],
        values: [AuthoredScalar; 3],
    },
    /// Dependency index, strictly less than its containing node index.
    Node(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeNode3Record {
    pub kind: ConstructionKind3,
    pub inputs: [RecipeInput3Record; 5],
}
#[derive(Clone, Debug)]
pub struct RecipeGraph3Record {
    pub schema_version: u32,
    pub implementation: String,
    pub source_id: String,
    pub source_revision: u64,
    pub tolerance: crate::ToleranceSpec,
    pub nodes: Vec<RecipeNode3Record>,
    pub root: usize,
}
#[derive(Clone, Debug)]
pub struct RecipeGraph3Report {
    pub outcome: Result<RecipeGraph3Record, Reason>,
    pub context: ContextIdentity,
    pub work_used: u64,
}
/// Export the bounded source recipe DAG in deterministic dependency-first order.
/// Only authored bits/rationals and operations are exported, never box centers,
/// cached expansions, in-process arena identities or pointer addresses.
pub fn export_point3d(
    ctx: &mut PredicateContext<'_>,
    point: &ConstructedPoint3,
) -> Result<RecipeGraph3Report, InputError> {
    let admitted = inventory(ctx, &[Point3Input::Constructed(point)])?;
    fn visit(
        point: &ConstructedPoint3,
        ctx: &mut PredicateContext<'_>,
        seen: &mut HashMap<*const Node3, usize>,
        nodes: &mut Vec<RecipeNode3Record>,
    ) -> Result<usize, Reason> {
        ctx.charge(1)?;
        if let Some(index) = seen.get(&point.key()) {
            return Ok(*index);
        }
        let mut inputs = Vec::with_capacity(5);
        for input in point.inputs() {
            inputs.push(match input {
                Point3Input::Constructed(child) => {
                    RecipeInput3Record::Node(visit(child, ctx, seen, nodes)?)
                }
                Point3Input::Authored(refs) => {
                    ctx.charge(3)?;
                    let indices = [
                        ctx.source_index(refs[0]),
                        ctx.source_index(refs[1]),
                        ctx.source_index(refs[2]),
                    ];
                    let [a, b, c] = indices;
                    RecipeInput3Record::Authored {
                        indices: [
                            a.map_err(|_| Reason::MissingProof)?,
                            b.map_err(|_| Reason::MissingProof)?,
                            c.map_err(|_| Reason::MissingProof)?,
                        ],
                        values: values(ctx, refs).map_err(|_| Reason::MissingProof)?,
                    }
                }
            });
        }
        let index = nodes.len();
        if index >= MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        nodes.push(RecipeNode3Record {
            kind: point.kind(),
            inputs: inputs.try_into().map_err(|_| Reason::MissingProof)?,
        });
        seen.insert(point.key(), index);
        Ok(index)
    }
    let outcome = (|| {
        ctx.charge(admitted.work)?;
        if admitted.depth > MAX_CONSTRUCTION_DEPTH || admitted.nodes > MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        let mut nodes = Vec::with_capacity(admitted.nodes);
        let mut seen = HashMap::new();
        let root = visit(point, ctx, &mut seen, &mut nodes)?;
        let (source_id, source_revision) = ctx.source_descriptor();
        let result = RecipeGraph3Record {
            schema_version: 1,
            implementation: crate::IMPLEMENTATION_VERSION.into(),
            source_id: source_id.into(),
            source_revision,
            tolerance: ctx.tolerance.specification().clone(),
            nodes,
            root,
        };
        ctx.charge(0)?;
        Ok(result)
    })();
    Ok(RecipeGraph3Report {
        outcome,
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}

#[derive(Clone, Debug)]
pub struct Point3ReplayReport {
    pub outcome: Result<ConstructedPoint3, Reason>,
    pub context: ContextIdentity,
    pub work_used: u64,
}
/// Replay portable records against an already admitted authoritative source.
/// Does not mint an arena from untrusted embedded values or trust cached geometry.
pub fn replay_point3d(
    ctx: &mut PredicateContext<'_>,
    graph: &RecipeGraph3Record,
) -> Result<Point3ReplayReport, InputError> {
    let invalid = |m| InputError::InvalidInput(m);
    if graph.schema_version != 1 || graph.implementation != crate::IMPLEMENTATION_VERSION {
        return Err(invalid("Unsupported recipe schema or implementation"));
    }
    let (source_id, revision) = ctx.source_descriptor();
    if graph.source_id != source_id || graph.source_revision != revision {
        return Err(InputError::ProvenanceMismatch);
    }
    let bits = |s: &crate::ToleranceSpec| {
        [
            s.linear_abs,
            s.linear_rel,
            s.on_tol,
            s.clear_tol,
            s.angular,
            s.param_floor,
            s.max_entity_error,
        ]
        .map(f64::to_bits)
    };
    let expected = ctx.tolerance.specification();
    if bits(&graph.tolerance) != bits(expected)
        || graph.tolerance.ulp_guard != expected.ulp_guard
        || graph.tolerance.policy != expected.policy
    {
        return Err(InputError::ContextMismatch);
    }
    if graph.nodes.is_empty()
        || graph.nodes.len() > MAX_CONSTRUCTION_NODES
        || graph.root != graph.nodes.len() - 1
    {
        return Err(invalid("Invalid recipe graph size or root"));
    }
    let mut depths = Vec::with_capacity(graph.nodes.len());
    for (index, node) in graph.nodes.iter().enumerate() {
        let mut depth = 1;
        for input in &node.inputs {
            match input {
                RecipeInput3Record::Node(dependency) => {
                    if *dependency >= index {
                        return Err(invalid("Recipe dependency is not earlier than its node"));
                    }
                    depth = depth.max(depths[*dependency] + 1);
                }
                RecipeInput3Record::Authored { indices, values } => {
                    for i in 0..3 {
                        let leaf = ctx.source_leaf(indices[i])?;
                        if ctx.resolve(leaf)? != &values[i] {
                            return Err(InputError::ProvenanceMismatch);
                        }
                    }
                }
            }
        }
        if depth > MAX_CONSTRUCTION_DEPTH {
            return Err(invalid("Recipe depth exceeds limit"));
        }
        depths.push(depth);
    }
    let mut reachable = vec![false; graph.nodes.len()];
    let mut pending = vec![graph.root];
    while let Some(index) = pending.pop() {
        if std::mem::replace(&mut reachable[index], true) {
            continue;
        }
        for input in &graph.nodes[index].inputs {
            if let RecipeInput3Record::Node(dependency) = input {
                pending.push(*dependency);
            }
        }
    }
    if reachable.contains(&false) {
        return Err(invalid("Recipe graph contains unreachable nodes"));
    }
    let outcome = (|| {
        ctx.charge((graph.nodes.len() * 32) as u64)?;
        let mut constructed: Vec<ConstructedPoint3> = Vec::with_capacity(graph.nodes.len());
        let mut exact_points: Vec<HPoint> = Vec::with_capacity(graph.nodes.len());
        let mut cached_terms = 0usize;
        for node in &graph.nodes {
            let mut inputs = Vec::with_capacity(5);
            let mut exact_inputs = Vec::with_capacity(5);
            for input in &node.inputs {
                match input {
                    RecipeInput3Record::Node(index) => {
                        inputs.push(Point3Input::Constructed(&constructed[*index]));
                        let exact = &exact_points[*index];
                        ctx.charge(exact.iter().map(Expansion::term_count).sum::<usize>() as u64)?;
                        exact_inputs.push(exact.clone());
                    }
                    RecipeInput3Record::Authored { indices, values } => {
                        inputs.push(Point3Input::Authored([
                            ctx.source_leaf(indices[0])
                                .map_err(|_| Reason::MissingProof)?,
                            ctx.source_leaf(indices[1])
                                .map_err(|_| Reason::MissingProof)?,
                            ctx.source_leaf(indices[2])
                                .map_err(|_| Reason::MissingProof)?,
                        ]));
                        // Every embedded value was matched to the authoritative
                        // arena in admission above; no new authored leaf is minted.
                        exact_inputs.push(authored(values, ctx)?);
                    }
                }
            }
            let inputs: [Point3Input<'_>; 5] =
                inputs.try_into().map_err(|_| Reason::MissingProof)?;
            let exact_inputs: [HPoint; 5] =
                exact_inputs.try_into().map_err(|_| Reason::MissingProof)?;
            let (exact, source) = match node.kind {
                ConstructionKind3::LinePlane => match solve(exact_inputs, ctx)? {
                    ExactResult::Unique { point, inputs } => (point, *inputs),
                    _ => return Err(Reason::MissingProof),
                },
                ConstructionKind3::CoplanarEndpoint(endpoint) => {
                    let exact = clip_endpoint(&exact_inputs, endpoint, ctx)?;
                    (exact, exact_inputs)
                }
            };
            let terms = exact.iter().map(Expansion::term_count).sum::<usize>();
            ctx.charge(terms as u64)?;
            if cached_terms + terms > MAX_CACHED_TERMS {
                return Err(Reason::ResourceLimit);
            }
            cached_terms += terms;
            let admitted = inventory(ctx, &inputs).map_err(|_| Reason::MissingProof)?;
            ctx.charge(admitted.work)?;
            let point = publish_point3(ctx, inputs, &admitted, &exact, &source, node.kind)?;
            ctx.charge(0)?;
            constructed.push(point);
            exact_points.push(exact);
        }
        ctx.charge(0)?;
        constructed.pop().ok_or(Reason::MissingProof)
    })();
    Ok(Point3ReplayReport {
        outcome,
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}
