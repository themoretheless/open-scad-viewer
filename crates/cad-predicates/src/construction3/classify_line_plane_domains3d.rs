use super::*;

/// Re-evaluate a unique retained line/plane intersection and classify it against
/// its original closed segment and triangle, using exact parameter numerators.
/// Coplanar overlap and degenerate primitives require separate algorithms.
pub fn classify_line_plane_domains3d(
    ctx: &mut PredicateContext<'_>,
    recipe: &ConstructedPoint3,
) -> Result<FiniteLocation3Report, InputError> {
    let inventory = inventory(ctx, &[Point3Input::Constructed(recipe)])?;
    let outcome = (|| {
        ctx.charge(inventory.work)?;
        if inventory.depth > MAX_CONSTRUCTION_DEPTH || inventory.nodes > MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        let mut evaluation = Evaluation::default();
        let point = evaluation.point(Point3Input::Constructed(recipe), ctx)?;
        let [a, b, c, d, e] = recipe.inputs();
        let a = evaluation.point(a, ctx)?;
        let b = evaluation.point(b, ctx)?;
        let c = evaluation.point(c, ctx)?;
        let d = evaluation.point(d, ctx)?;
        let e = evaluation.point(e, ctx)?;
        let [mut tn, mut td] = line_parameter_fraction(&point, &a, &b, ctx)?;
        if td.sign() == Sign::Negative {
            tn = tn.negated();
            td = td.negated();
        }
        if td.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        let end = td.sub(&tn, ctx)?.sign();
        let segment = match (tn.sign(), end) {
            (Sign::Negative, _) => SegmentLocation3::BeforeStart,
            (Sign::Zero, _) => SegmentLocation3::Start,
            (_, Sign::Negative) => SegmentLocation3::AfterEnd,
            (_, Sign::Zero) => SegmentLocation3::End,
            _ => SegmentLocation3::Interior,
        };
        let [mut un, mut vn, mut den] = plane_parameter_fractions(&point, &c, &d, &e, ctx)?;
        if den.sign() == Sign::Negative {
            un = un.negated();
            vn = vn.negated();
            den = den.negated();
        }
        if den.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        let signs = [
            den.sub(&un, ctx)?.sub(&vn, ctx)?.sign(),
            un.sign(),
            vn.sign(),
        ];
        let triangle = triangle_location_from_signs(signs)?;
        ctx.charge(0)?;
        Ok(FiniteLocation3 { segment, triangle })
    })();
    Ok(FiniteLocation3Report {
        outcome,
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}

/// An exact coplanar overlap, parameterized on the original segment A+t(B-A).
/// Only outward parameter bounds are exposed; they are not authored coordinates.
#[derive(Clone)]
pub struct CoplanarOverlap3 {
    inputs: [StoredPoint3; 5],
    context: ContextIdentity,
    bounds: [[f64; 2]; 2],
    singleton: bool,
}
impl CoplanarOverlap3 {
    pub fn inputs(&self) -> [Point3Input<'_>; 5] {
        self.inputs.each_ref().map(StoredPoint3::borrowed)
    }
    pub fn context(&self) -> ContextIdentity {
        self.context
    }
    pub fn parameter_enclosures(&self) -> [[f64; 2]; 2] {
        self.bounds
    }
    /// Exact equality of the two clipped parameters, independent of box width.
    pub fn is_singleton(&self) -> bool {
        self.singleton
    }
}
impl std::fmt::Debug for CoplanarOverlap3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoplanarOverlap3")
            .field("context", &self.context)
            .field("bounds", &self.bounds)
            .field("singleton", &self.singleton)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug)]
pub enum CoplanarIntersection3 {
    Overlap(Box<CoplanarOverlap3>),
    Empty,
    NotCoplanar,
    DegenerateSegment,
    DegenerateTriangle,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct CoplanarIntersection3Report {
    pub outcome: CoplanarIntersection3,
    pub context: ContextIdentity,
    pub work_used: u64,
}
pub(super) fn barycentric_fraction(
    p: &HPoint,
    c: &HPoint,
    d: &HPoint,
    e: &HPoint,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Expansion; 4], Reason> {
    let [mut u, mut v, mut den] = plane_parameter_fractions(p, c, d, e, ctx)?;
    if den.sign() == Sign::Negative {
        u = u.negated();
        v = v.negated();
        den = den.negated();
    }
    if den.sign() != Sign::Positive {
        return Err(Reason::MissingProof);
    }
    Ok([den.sub(&u, ctx)?.sub(&v, ctx)?, u, v, den])
}
fn ratio_order(
    a: &[Expansion; 2],
    b: &[Expansion; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<Sign, Reason> {
    Ok(a[0]
        .mul(&b[1], ctx)?
        .sub(&b[0].mul(&a[1], ctx)?, ctx)?
        .sign())
}
/// Clip a coplanar segment against a closed triangle with exact half-space signs.
/// A noncoplanar input is explicitly distinguished from an empty intersection.
/// Zero-length segments and degenerate triangles are explicit unsupported cases.
pub fn intersect_coplanar_segment_triangle3d(
    ctx: &mut PredicateContext<'_>,
    a: Point3Input<'_>,
    b: Point3Input<'_>,
    c: Point3Input<'_>,
    d: Point3Input<'_>,
    e: Point3Input<'_>,
) -> Result<CoplanarIntersection3Report, InputError> {
    let inputs = [a, b, c, d, e];
    let inventory = inventory(ctx, &inputs)?;
    let outcome = (|| {
        ctx.charge(inventory.work)?;
        if inventory.depth > MAX_CONSTRUCTION_DEPTH || inventory.nodes > MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        let mut evaluation = Evaluation::default();
        let a = evaluation.point(a, ctx)?;
        let b = evaluation.point(b, ctx)?;
        let c = evaluation.point(c, ctx)?;
        let d = evaluation.point(d, ctx)?;
        let e = evaluation.point(e, ctx)?;
        let (lower, upper) = match solve_clip(&[a, b, c, d, e], ctx)? {
            ExactClip3::Overlap { lower, upper } => (lower, upper),
            ExactClip3::Empty => return Ok(CoplanarIntersection3::Empty),
            ExactClip3::NotCoplanar => return Ok(CoplanarIntersection3::NotCoplanar),
            ExactClip3::DegenerateSegment => return Ok(CoplanarIntersection3::DegenerateSegment),
            ExactClip3::DegenerateTriangle => return Ok(CoplanarIntersection3::DegenerateTriangle),
        };
        let singleton = ratio_order(&lower, &upper, ctx)? == Sign::Zero;
        let bounds = [
            quotient(&lower[0], &lower[1], ctx)?,
            quotient(&upper[0], &upper[1], ctx)?,
        ]
        .map(|b| [b.lo, b.hi]);
        Ok(CoplanarIntersection3::Overlap(Box::new(CoplanarOverlap3 {
            inputs: inputs.map(StoredPoint3::owned),
            context: ctx.identity(),
            bounds,
            singleton,
        })))
    })()
    .and_then(|outcome| {
        ctx.charge(0)?;
        Ok(outcome)
    });
    Ok(CoplanarIntersection3Report {
        outcome: outcome.unwrap_or_else(CoplanarIntersection3::Indeterminate),
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}

enum ExactClip3 {
    Overlap {
        lower: [Expansion; 2],
        upper: [Expansion; 2],
    },
    Empty,
    NotCoplanar,
    DegenerateSegment,
    DegenerateTriangle,
}
fn solve_clip(inputs: &[HPoint; 5], ctx: &mut PredicateContext<'_>) -> Result<ExactClip3, Reason> {
    let [a, b, c, d, e] = inputs;
    if zero(&delta(b, a, ctx)?) {
        return Ok(ExactClip3::DegenerateSegment);
    }
    let support = plane(c, d, e, ctx)?;
    if zero(&support) {
        return Ok(ExactClip3::DegenerateTriangle);
    }
    if dot(&support, a, ctx)?.sign() != Sign::Zero || dot(&support, b, ctx)?.sign() != Sign::Zero {
        return Ok(ExactClip3::NotCoplanar);
    }
    let wa = barycentric_fraction(a, c, d, e, ctx)?;
    let wb = barycentric_fraction(b, c, d, e, ctx)?;
    let mut lower = [Expansion::scalar(0.), Expansion::scalar(1.)];
    let mut upper = [Expansion::scalar(1.), Expansion::scalar(1.)];
    for i in 0..3 {
        // Positive common denominator: each half-space is alpha+t*beta >= 0.
        let alpha = wa[i].mul(&wb[3], ctx)?;
        let beta = wb[i].mul(&wa[3], ctx)?.sub(&alpha, ctx)?;
        match beta.sign() {
            Sign::Zero => {
                if alpha.sign() == Sign::Negative {
                    return Ok(ExactClip3::Empty);
                }
            }
            Sign::Positive => {
                let boundary = normalize([alpha.negated(), beta], ctx)?;
                if ratio_order(&boundary, &lower, ctx)? == Sign::Positive {
                    lower = boundary;
                }
            }
            Sign::Negative => {
                let boundary = normalize([alpha, beta.negated()], ctx)?;
                if ratio_order(&boundary, &upper, ctx)? == Sign::Negative {
                    upper = boundary;
                }
            }
        }
        if ratio_order(&lower, &upper, ctx)? == Sign::Positive {
            return Ok(ExactClip3::Empty);
        }
    }
    Ok(ExactClip3::Overlap { lower, upper })
}

/// Select a bound in increasing parameter order on the original segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlapEndpoint3 {
    Lower,
    Upper,
}
pub(super) fn clip_endpoint(
    inputs: &[HPoint; 5],
    endpoint: OverlapEndpoint3,
    ctx: &mut PredicateContext<'_>,
) -> Result<HPoint, Reason> {
    let ExactClip3::Overlap { lower, upper } = solve_clip(inputs, ctx)? else {
        return Err(Reason::MissingProof);
    };
    let [n, d] = match endpoint {
        OverlapEndpoint3::Lower => lower,
        OverlapEndpoint3::Upper => upper,
    };
    let [a, b, ..] = inputs;
    let left = d.sub(&n, ctx)?.mul(&b[3], ctx)?;
    let right = n.mul(&a[3], ctx)?;
    let mut point = std::array::from_fn(|_| Expansion::scalar(0.));
    for i in 0..4 {
        point[i] = a[i].mul(&left, ctx)?.add(&b[i].mul(&right, ctx)?, ctx)?;
    }
    let point = normalize(point, ctx)?;
    if point[3].sign() != Sign::Positive {
        return Err(Reason::MissingProof);
    }
    Ok(point)
}
#[derive(Clone, Debug)]
pub struct OverlapEndpoint3Report {
    pub outcome: Result<ConstructedPoint3, Reason>,
    pub context: ContextIdentity,
    pub work_used: u64,
}
/// Materialize an overlap bound as an immutable exact recipe, never as a
/// rounded authored point. Both bounds of a singleton remain exactly equal.
pub fn construct_overlap_endpoint3d(
    ctx: &mut PredicateContext<'_>,
    overlap: &CoplanarOverlap3,
    endpoint: OverlapEndpoint3,
) -> Result<OverlapEndpoint3Report, InputError> {
    if overlap.context() != ctx.identity() {
        return Err(InputError::ContextMismatch);
    }
    let inputs = overlap.inputs();
    let inventory = inventory(ctx, &inputs)?;
    let outcome = (|| {
        ctx.charge(inventory.work)?;
        if inventory.depth >= MAX_CONSTRUCTION_DEPTH || inventory.nodes >= MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        let mut evaluation = Evaluation::default();
        let [a, b, c, d, e] = inputs;
        let exact = [
            evaluation.point(a, ctx)?,
            evaluation.point(b, ctx)?,
            evaluation.point(c, ctx)?,
            evaluation.point(d, ctx)?,
            evaluation.point(e, ctx)?,
        ];
        let point = clip_endpoint(&exact, endpoint, ctx)?;
        let result = publish_point3(
            ctx,
            inputs,
            &inventory,
            &point,
            &exact,
            ConstructionKind3::CoplanarEndpoint(endpoint),
        )?;
        ctx.charge(0)?;
        Ok(result)
    })();
    Ok(OverlapEndpoint3Report {
        outcome,
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}

/// A retained intersection point and its exact original finite-domain location.
#[derive(Clone, Debug)]
pub struct SegmentTrianglePoint3 {
    pub point: ConstructedPoint3,
    pub location: FiniteLocation3,
}
#[derive(Clone, Debug)]
pub enum SegmentTriangleIntersection3 {
    Empty,
    Point(Box<SegmentTrianglePoint3>),
    /// Nonzero coplanar overlap, ordered by increasing original segment parameter.
    Segment(Box<[SegmentTrianglePoint3; 2]>),
    DegenerateSegment,
    DegenerateTriangle,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct SegmentTriangleIntersection3Report {
    pub outcome: SegmentTriangleIntersection3,
    pub context: ContextIdentity,
    pub work_used: u64,
}
fn locate_segment_triangle_point(
    ctx: &mut PredicateContext<'_>,
    point: ConstructedPoint3,
) -> Result<SegmentTrianglePoint3, Reason> {
    // The immutable inputs/context have already passed the public admission.
    let location = classify_line_plane_domains3d(ctx, &point)
        .map_err(|_| Reason::MissingProof)?
        .outcome?;
    Ok(SegmentTrianglePoint3 { point, location })
}
/// Intersect a closed segment A/B with a closed triangle C/D/E. Both transverse
/// and coplanar cases return retained exact constructions. Degenerate input
/// primitives remain explicit; the query does not create B-rep topology.
/// A failure constructing either overlap endpoint publishes no partial geometry.
pub fn intersect_segment_triangle3d(
    ctx: &mut PredicateContext<'_>,
    a: Point3Input<'_>,
    b: Point3Input<'_>,
    c: Point3Input<'_>,
    d: Point3Input<'_>,
    e: Point3Input<'_>,
) -> Result<SegmentTriangleIntersection3Report, InputError> {
    // Validate the entire immutable input graph before any cancellation/resource
    // shortcut. Internal revalidation cannot introduce a new foreign reference.
    let admission = inventory(ctx, &[a, b, c, d, e])?;
    let outcome = (|| {
        ctx.charge(admission.work)?;
        let line = intersect_line_plane3d(ctx, a, b, c, d, e)
            .map_err(|_| Reason::MissingProof)?
            .outcome;
        match line {
            LinePlaneIntersection3::Indeterminate(reason) => Err(reason),
            LinePlaneIntersection3::DegenerateLine => {
                Ok(SegmentTriangleIntersection3::DegenerateSegment)
            }
            LinePlaneIntersection3::DegeneratePlane => {
                Ok(SegmentTriangleIntersection3::DegenerateTriangle)
            }
            LinePlaneIntersection3::Parallel => Ok(SegmentTriangleIntersection3::Empty),
            LinePlaneIntersection3::Unique(point) => {
                let point = locate_segment_triangle_point(ctx, *point)?;
                if point.location.is_closed_domain_hit() {
                    Ok(SegmentTriangleIntersection3::Point(Box::new(point)))
                } else {
                    Ok(SegmentTriangleIntersection3::Empty)
                }
            }
            LinePlaneIntersection3::Contained => {
                let overlap = match intersect_coplanar_segment_triangle3d(ctx, a, b, c, d, e)
                    .map_err(|_| Reason::MissingProof)?
                    .outcome
                {
                    CoplanarIntersection3::Overlap(overlap) => overlap,
                    CoplanarIntersection3::Empty => return Ok(SegmentTriangleIntersection3::Empty),
                    CoplanarIntersection3::Indeterminate(reason) => return Err(reason),
                    _ => return Err(Reason::MissingProof),
                };
                let lower = construct_overlap_endpoint3d(ctx, &overlap, OverlapEndpoint3::Lower)
                    .map_err(|_| Reason::MissingProof)?
                    .outcome?;
                let lower = locate_segment_triangle_point(ctx, lower)?;
                if !lower.location.is_closed_domain_hit() {
                    return Err(Reason::MissingProof);
                }
                if overlap.is_singleton() {
                    return Ok(SegmentTriangleIntersection3::Point(Box::new(lower)));
                }
                let upper = construct_overlap_endpoint3d(ctx, &overlap, OverlapEndpoint3::Upper)
                    .map_err(|_| Reason::MissingProof)?
                    .outcome?;
                let upper = locate_segment_triangle_point(ctx, upper)?;
                if !upper.location.is_closed_domain_hit() {
                    return Err(Reason::MissingProof);
                }
                let combined = inventory(
                    ctx,
                    &[
                        Point3Input::Constructed(&lower.point),
                        Point3Input::Constructed(&upper.point),
                    ],
                )
                .map_err(|_| Reason::MissingProof)?;
                ctx.charge(combined.work)?;
                if combined.nodes > MAX_CONSTRUCTION_NODES {
                    return Err(Reason::ResourceLimit);
                }
                Ok(SegmentTriangleIntersection3::Segment(Box::new([
                    lower, upper,
                ])))
            }
        }
    })()
    .and_then(|outcome| {
        ctx.charge(0)?;
        Ok(outcome)
    });
    Ok(SegmentTriangleIntersection3Report {
        outcome: outcome.unwrap_or_else(SegmentTriangleIntersection3::Indeterminate),
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}

/// Exact lexicographic coordinate comparison (x, then y, then z).
/// Zero means algebraically equal coordinates, not tolerance-based coincidence
/// or persistent topological identity. No pointer/source ID breaks a geometric tie.
pub fn compare_points3d(
    ctx: &mut PredicateContext<'_>,
    a: Point3Input<'_>,
    b: Point3Input<'_>,
) -> Result<Decision, InputError> {
    let admitted = inventory(ctx, &[a, b])?;
    let mut stage = Stage::Admission;
    let outcome = (|| {
        ctx.charge(admitted.work)?;
        if admitted.depth > MAX_CONSTRUCTION_DEPTH || admitted.nodes > MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        stage = Stage::ExactExpansion;
        let mut evaluation = Evaluation::default();
        let a = evaluation.point(a, ctx)?;
        let b = evaluation.point(b, ctx)?;
        for i in 0..3 {
            let sign = a[i]
                .mul(&b[3], ctx)?
                .sub(&b[i].mul(&a[3], ctx)?, ctx)?
                .sign();
            if sign != Sign::Zero {
                return Ok(sign);
            }
        }
        Ok(Sign::Zero)
    })()
    .and_then(|sign| {
        ctx.charge(0)?;
        Ok(sign)
    });
    Ok(Decision {
        outcome: match outcome {
            Ok(sign) => Outcome::Sign(sign),
            Err(reason) => Outcome::Indeterminate(reason),
        },
        stage,
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}

#[derive(Clone, Debug)]
pub enum TriangleTriangleIntersection3 {
    Empty,
    Point(Box<ConstructedPoint3>),
    /// Ordered lexicographically by exact coordinates.
    Segment(Box<[ConstructedPoint3; 2]>),
    /// Implicitly closed convex contour, CCW in projection_axes, starting at
    /// the exact lexicographic minimum. No repeated closing vertex.
    Polygon {
        vertices: Vec<ConstructedPoint3>,
        projection_axes: [usize; 2],
    },
    DegenerateTriangle(usize),
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct TriangleTriangleIntersection3Report {
    pub outcome: TriangleTriangleIntersection3,
    pub context: ContextIdentity,
    pub work_used: u64,
}
