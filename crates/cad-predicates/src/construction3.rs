//! Exact projective line/plane recipes over retained authored inputs in 3D.
//! Includes bounded segment/triangle queries; no trim topology or solid certificate.
use crate::arithmetic::{Expansion, Interval};
use crate::construction::{MAX_CACHED_TERMS, fraction, normalize};
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Decision, InputError, LeafRef,
    MAX_CONSTRUCTION_DEPTH, MAX_CONSTRUCTION_NODES, Outcome, PredicateContext, Reason, Sign, Stage,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

type HPoint = [Expansion; 4];

#[derive(Clone)]
pub struct ConstructedPoint3 {
    node: Arc<Node3>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructionKind3 {
    LinePlane,
    CoplanarEndpoint(OverlapEndpoint3),
}
struct Node3 {
    recipe: ConstructionKind3,
    inputs: [StoredPoint3; 5],
    context: ContextIdentity,
    enclosure: [[f64; 2]; 3],
    line_parameter: [f64; 2],
    plane_parameters: [[f64; 2]; 2],
    diameter: f64,
    depth: usize,
    node_count: usize,
}
#[derive(Clone)]
enum StoredPoint3 {
    Authored([LeafRef; 3]),
    Constructed(ConstructedPoint3),
}
impl StoredPoint3 {
    fn borrowed(&self) -> Point3Input<'_> {
        match self {
            Self::Authored(p) => Point3Input::Authored(*p),
            Self::Constructed(p) => Point3Input::Constructed(p),
        }
    }
    fn owned(input: Point3Input<'_>) -> Self {
        match input {
            Point3Input::Authored(p) => Self::Authored(p),
            Point3Input::Constructed(p) => Self::Constructed(p.clone()),
        }
    }
}
impl std::fmt::Debug for ConstructedPoint3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstructedPoint3")
            .field("kind", &self.kind())
            .field("context", &self.context())
            .field("enclosure", &self.enclosure())
            .field("depth", &self.recipe_depth())
            .field("nodes", &self.recipe_node_count())
            .finish_non_exhaustive()
    }
}
impl ConstructedPoint3 {
    pub fn kind(&self) -> ConstructionKind3 {
        self.node.recipe
    }
    pub fn context(&self) -> ContextIdentity {
        self.node.context
    }
    /// Ordered inputs: line A/B followed by plane C/D/E.
    pub fn inputs(&self) -> [Point3Input<'_>; 5] {
        self.node.inputs.each_ref().map(StoredPoint3::borrowed)
    }
    pub fn enclosure(&self) -> [[f64; 2]; 3] {
        self.node.enclosure
    }
    /// t in A+t(B-A), not clamped to a finite segment.
    pub fn line_parameter_enclosure(&self) -> [f64; 2] {
        self.node.line_parameter
    }
    /// u/v in C+u(D-C)+v(E-C), not clamped to a triangle or trim.
    pub fn plane_parameter_enclosures(&self) -> [[f64; 2]; 2] {
        self.node.plane_parameters
    }
    pub fn diameter_bound(&self) -> f64 {
        self.node.diameter
    }
    pub fn recipe_depth(&self) -> usize {
        self.node.depth
    }
    pub fn recipe_node_count(&self) -> usize {
        self.node.node_count
    }
    fn key(&self) -> *const Node3 {
        Arc::as_ptr(&self.node)
    }
}
#[derive(Clone, Copy)]
pub enum Point3Input<'a> {
    Authored([LeafRef; 3]),
    Constructed(&'a ConstructedPoint3),
}
#[derive(Clone, Debug)]
pub enum LinePlaneIntersection3 {
    Unique(Box<ConstructedPoint3>),
    Parallel,
    Contained,
    DegenerateLine,
    DegeneratePlane,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct Construction3Report {
    pub outcome: LinePlaneIntersection3,
    pub context: ContextIdentity,
    pub work_used: u64,
}
fn values(ctx: &PredicateContext<'_>, p: [LeafRef; 3]) -> Result<[AuthoredScalar; 3], InputError> {
    Ok([
        ctx.resolve(p[0])?.clone(),
        ctx.resolve(p[1])?.clone(),
        ctx.resolve(p[2])?.clone(),
    ])
}
struct Inventory {
    nodes: usize,
    depth: usize,
    work: u64,
}
fn inventory(
    ctx: &PredicateContext<'_>,
    inputs: &[Point3Input<'_>],
) -> Result<Inventory, InputError> {
    // At most twelve already-admitted roots, each with at most 256 nodes. Private
    // immutable constructors guarantee acyclicity before this bounded admission.
    let mut seen = HashSet::new();
    let mut pending = inputs.to_vec();
    let mut depth = 0;
    let mut work = 0;
    while let Some(input) = pending.pop() {
        work += 1;
        match input {
            Point3Input::Authored(p) => {
                values(ctx, p)?;
                work += 3;
            }
            Point3Input::Constructed(p) => {
                if p.context() != ctx.identity() {
                    return Err(InputError::ContextMismatch);
                }
                depth = depth.max(p.recipe_depth());
                if seen.insert(p.key()) {
                    pending.extend(p.inputs());
                }
            }
        }
    }
    Ok(Inventory {
        nodes: seen.len(),
        depth,
        work,
    })
}
fn authored(value: &[AuthoredScalar; 3], ctx: &mut PredicateContext<'_>) -> Result<HPoint, Reason> {
    let [xn, xd] = fraction(&value[0], ctx)?;
    let [yn, yd] = fraction(&value[1], ctx)?;
    let [zn, zd] = fraction(&value[2], ctx)?;
    normalize(
        [
            xn.mul(&yd, ctx)?.mul(&zd, ctx)?,
            yn.mul(&xd, ctx)?.mul(&zd, ctx)?,
            zn.mul(&xd, ctx)?.mul(&yd, ctx)?,
            xd.mul(&yd, ctx)?.mul(&zd, ctx)?,
        ],
        ctx,
    )
}
fn determinant3(
    a: [&Expansion; 3],
    b: [&Expansion; 3],
    c: [&Expansion; 3],
    ctx: &mut PredicateContext<'_>,
) -> Result<Expansion, Reason> {
    let x = b[1].mul(c[2], ctx)?.sub(&b[2].mul(c[1], ctx)?, ctx)?;
    let y = b[2].mul(c[0], ctx)?.sub(&b[0].mul(c[2], ctx)?, ctx)?;
    let z = b[0].mul(c[1], ctx)?.sub(&b[1].mul(c[0], ctx)?, ctx)?;
    a[0].mul(&x, ctx)?
        .add(&a[1].mul(&y, ctx)?, ctx)?
        .add(&a[2].mul(&z, ctx)?, ctx)
}
fn plane(
    a: &HPoint,
    b: &HPoint,
    c: &HPoint,
    ctx: &mut PredicateContext<'_>,
) -> Result<HPoint, Reason> {
    // Signed 3x3 minors give coefficients n.x*X+n.y*Y+n.z*Z+d*W=0.
    let minor = |indices: [usize; 3], ctx: &mut PredicateContext<'_>| {
        determinant3(
            indices.map(|i| &a[i]),
            indices.map(|i| &b[i]),
            indices.map(|i| &c[i]),
            ctx,
        )
    };
    normalize(
        [
            minor([1, 2, 3], ctx)?,
            minor([0, 2, 3], ctx)?.negated(),
            minor([0, 1, 3], ctx)?,
            minor([0, 1, 2], ctx)?.negated(),
        ],
        ctx,
    )
}
fn dot(a: &HPoint, b: &HPoint, ctx: &mut PredicateContext<'_>) -> Result<Expansion, Reason> {
    let mut result = a[0].mul(&b[0], ctx)?;
    for i in 1..4 {
        result = result.add(&a[i].mul(&b[i], ctx)?, ctx)?;
    }
    Ok(result)
}
fn delta(b: &HPoint, a: &HPoint, ctx: &mut PredicateContext<'_>) -> Result<[Expansion; 3], Reason> {
    Ok([
        b[0].mul(&a[3], ctx)?.sub(&a[0].mul(&b[3], ctx)?, ctx)?,
        b[1].mul(&a[3], ctx)?.sub(&a[1].mul(&b[3], ctx)?, ctx)?,
        b[2].mul(&a[3], ctx)?.sub(&a[2].mul(&b[3], ctx)?, ctx)?,
    ])
}
fn zero<const N: usize>(value: &[Expansion; N]) -> bool {
    value.iter().all(|x| x.sign() == Sign::Zero)
}
enum ExactResult {
    Unique {
        point: HPoint,
        inputs: Box<[HPoint; 5]>,
    },
    Parallel,
    Contained,
    DegenerateLine,
    DegeneratePlane,
}
fn solve(inputs: [HPoint; 5], ctx: &mut PredicateContext<'_>) -> Result<ExactResult, Reason> {
    let [a, b, c, d, e] = &inputs;
    let direction = delta(b, a, ctx)?;
    if zero(&direction) {
        return Ok(ExactResult::DegenerateLine);
    }
    let support = plane(c, d, e, ctx)?;
    if zero(&support) {
        return Ok(ExactResult::DegeneratePlane);
    }
    let ra = dot(&support, a, ctx)?;
    let rb = dot(&support, b, ctx)?;
    if ra.sign() == Sign::Zero && rb.sign() == Sign::Zero {
        return Ok(ExactResult::Contained);
    }
    let mut point = normalize(
        [
            rb.mul(&a[0], ctx)?.sub(&ra.mul(&b[0], ctx)?, ctx)?,
            rb.mul(&a[1], ctx)?.sub(&ra.mul(&b[1], ctx)?, ctx)?,
            rb.mul(&a[2], ctx)?.sub(&ra.mul(&b[2], ctx)?, ctx)?,
            rb.mul(&a[3], ctx)?.sub(&ra.mul(&b[3], ctx)?, ctx)?,
        ],
        ctx,
    )?;
    if point[3].sign() == Sign::Zero {
        return Ok(ExactResult::Parallel);
    }
    if point[3].sign() == Sign::Negative {
        point = point.map(|x| x.negated());
    }
    if dot(&support, &point, ctx)?.sign() != Sign::Zero {
        return Err(Reason::PrecisionExhausted);
    }
    let displacement = delta(&point, a, ctx)?;
    // Exact collinearity in every coordinate, including non-axis-aligned lines.
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        if displacement[i]
            .mul(&direction[j], ctx)?
            .sub(&displacement[j].mul(&direction[i], ctx)?, ctx)?
            .sign()
            != Sign::Zero
        {
            return Err(Reason::PrecisionExhausted);
        }
    }
    Ok(ExactResult::Unique {
        point,
        inputs: Box::new(inputs),
    })
}
fn quotient(
    n: &Expansion,
    d: &Expansion,
    ctx: &mut PredicateContext<'_>,
) -> Result<Interval, Reason> {
    n.enclosure(ctx)?.div_nonzero(&d.enclosure(ctx)?, ctx)
}
fn line_parameter_fraction(
    point: &HPoint,
    a: &HPoint,
    b: &HPoint,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Expansion; 2], Reason> {
    let direction = delta(b, a, ctx)?;
    let displacement = delta(point, a, ctx)?;
    for i in 0..3 {
        if direction[i].sign() != Sign::Zero {
            return Ok([
                displacement[i].mul(&b[3], ctx)?,
                direction[i].mul(&point[3], ctx)?,
            ]);
        }
    }
    Err(Reason::MissingProof)
}
fn plane_parameter_fractions(
    point: &HPoint,
    c: &HPoint,
    d: &HPoint,
    e: &HPoint,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Expansion; 3], Reason> {
    let u = delta(d, c, ctx)?;
    let v = delta(e, c, ctx)?;
    let p = delta(point, c, ctx)?;
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let det = u[i].mul(&v[j], ctx)?.sub(&u[j].mul(&v[i], ctx)?, ctx)?;
        if det.sign() != Sign::Zero {
            let den = det.mul(&point[3], ctx)?;
            let un = p[i]
                .mul(&v[j], ctx)?
                .sub(&p[j].mul(&v[i], ctx)?, ctx)?
                .mul(&d[3], ctx)?;
            let vn = u[i]
                .mul(&p[j], ctx)?
                .sub(&u[j].mul(&p[i], ctx)?, ctx)?
                .mul(&e[3], ctx)?;
            return Ok([un, vn, den]);
        }
    }
    Err(Reason::MissingProof)
}
#[derive(Default)]
struct Evaluation {
    cache: HashMap<*const Node3, HPoint>,
    terms: usize,
}
impl Evaluation {
    fn point(
        &mut self,
        input: Point3Input<'_>,
        ctx: &mut PredicateContext<'_>,
    ) -> Result<HPoint, Reason> {
        ctx.charge(1)?;
        match input {
            Point3Input::Authored(p) => {
                authored(&values(ctx, p).map_err(|_| Reason::MissingProof)?, ctx)
            }
            Point3Input::Constructed(recipe) => {
                if let Some(point) = self.cache.get(&recipe.key()) {
                    ctx.charge(point.iter().map(Expansion::term_count).sum::<usize>() as u64)?;
                    return Ok(point.clone());
                }
                let [a, b, c, d, e] = recipe.inputs();
                let inputs = [
                    self.point(a, ctx)?,
                    self.point(b, ctx)?,
                    self.point(c, ctx)?,
                    self.point(d, ctx)?,
                    self.point(e, ctx)?,
                ];
                let point = match recipe.node.recipe {
                    ConstructionKind3::LinePlane => {
                        let ExactResult::Unique { point, .. } = solve(inputs, ctx)? else {
                            return Err(Reason::MissingProof);
                        };
                        point
                    }
                    ConstructionKind3::CoplanarEndpoint(endpoint) => {
                        clip_endpoint(&inputs, endpoint, ctx)?
                    }
                };
                let terms = point.iter().map(Expansion::term_count).sum::<usize>();
                ctx.charge(terms as u64)?;
                if self.terms + terms > MAX_CACHED_TERMS
                    || self.cache.len() >= MAX_CONSTRUCTION_NODES
                {
                    return Err(Reason::ResourceLimit);
                }
                self.terms += terms;
                self.cache.insert(recipe.key(), point.clone());
                Ok(point)
            }
        }
    }
}
fn publish_point3(
    ctx: &mut PredicateContext<'_>,
    inputs: [Point3Input<'_>; 5],
    inventory: &Inventory,
    point: &HPoint,
    exact: &[HPoint; 5],
    recipe: ConstructionKind3,
) -> Result<ConstructedPoint3, Reason> {
    let bounds = [
        quotient(&point[0], &point[3], ctx)?,
        quotient(&point[1], &point[3], ctx)?,
        quotient(&point[2], &point[3], ctx)?,
    ];
    let mut diameter = Interval::point(0.);
    for bound in bounds {
        diameter = diameter.add(&Interval::point(bound.width()), ctx)?;
    }
    let diameter = inputs.iter().fold(diameter.hi, |bound, input| match input {
        Point3Input::Authored(_) => bound,
        Point3Input::Constructed(p) => bound.max(p.diameter_bound()),
    });
    if !diameter.is_finite() || diameter > ctx.tolerance.spec.max_entity_error {
        return Err(Reason::PrecisionExhausted);
    }
    let [tn, td] = line_parameter_fraction(point, &exact[0], &exact[1], ctx)?;
    let t = quotient(&tn, &td, ctx)?;
    let [un, vn, den] = plane_parameter_fractions(point, &exact[2], &exact[3], &exact[4], ctx)?;
    let uv = [quotient(&un, &den, ctx)?, quotient(&vn, &den, ctx)?];
    Ok(ConstructedPoint3 {
        node: Arc::new(Node3 {
            recipe,
            inputs: inputs.map(StoredPoint3::owned),
            context: ctx.identity(),
            enclosure: bounds.map(|b| [b.lo, b.hi]),
            line_parameter: [t.lo, t.hi],
            plane_parameters: uv.map(|b| [b.lo, b.hi]),
            diameter,
            depth: inventory.depth + 1,
            node_count: inventory.nodes + 1,
        }),
    })
}
/// A/B define the infinite line; C/D/E define the oriented infinite plane.
pub fn intersect_line_plane3d(
    ctx: &mut PredicateContext<'_>,
    a: Point3Input<'_>,
    b: Point3Input<'_>,
    c: Point3Input<'_>,
    d: Point3Input<'_>,
    e: Point3Input<'_>,
) -> Result<Construction3Report, InputError> {
    let inputs = [a, b, c, d, e];
    let inventory = inventory(ctx, &inputs)?;
    let outcome = (|| {
        ctx.charge(inventory.work)?;
        if inventory.depth >= MAX_CONSTRUCTION_DEPTH || inventory.nodes >= MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        let mut evaluation = Evaluation::default();
        let exact = [
            evaluation.point(a, ctx)?,
            evaluation.point(b, ctx)?,
            evaluation.point(c, ctx)?,
            evaluation.point(d, ctx)?,
            evaluation.point(e, ctx)?,
        ];
        let result = match solve(exact, ctx)? {
            ExactResult::Parallel => LinePlaneIntersection3::Parallel,
            ExactResult::Contained => LinePlaneIntersection3::Contained,
            ExactResult::DegenerateLine => LinePlaneIntersection3::DegenerateLine,
            ExactResult::DegeneratePlane => LinePlaneIntersection3::DegeneratePlane,
            ExactResult::Unique {
                point,
                inputs: exact,
            } => LinePlaneIntersection3::Unique(Box::new(publish_point3(
                ctx,
                inputs,
                &inventory,
                &point,
                &exact,
                ConstructionKind3::LinePlane,
            )?)),
        };
        ctx.charge(0)?;
        Ok(result)
    })();
    Ok(Construction3Report {
        outcome: outcome.unwrap_or_else(LinePlaneIntersection3::Indeterminate),
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}
fn interval_point(
    point: Point3Input<'_>,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Interval; 3], Reason> {
    match point {
        Point3Input::Authored(p) => {
            let v = values(ctx, p).map_err(|_| Reason::MissingProof)?;
            Ok([
                Interval::authored(&v[0], ctx)?,
                Interval::authored(&v[1], ctx)?,
                Interval::authored(&v[2], ctx)?,
            ])
        }
        Point3Input::Constructed(p) => {
            ctx.charge(3)?;
            Ok(p.enclosure().map(|[lo, hi]| Interval { lo, hi }))
        }
    }
}
/// Plane-side / tetrahedral orientation with exact recipe fallback.
pub fn orient3d_points(
    ctx: &mut PredicateContext<'_>,
    a: Point3Input<'_>,
    b: Point3Input<'_>,
    c: Point3Input<'_>,
    d: Point3Input<'_>,
) -> Result<Decision, InputError> {
    if let (
        Point3Input::Authored(a),
        Point3Input::Authored(b),
        Point3Input::Authored(c),
        Point3Input::Authored(d),
    ) = (a, b, c, d)
    {
        return crate::orient3d(ctx, a, b, c, d);
    }
    let inventory = inventory(ctx, &[a, b, c, d])?;
    let mut stage = Stage::Admission;
    let result = (|| {
        ctx.charge(inventory.work)?;
        if inventory.depth > MAX_CONSTRUCTION_DEPTH || inventory.nodes > MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        stage = Stage::Interval;
        let points = [
            interval_point(a, ctx)?,
            interval_point(b, ctx)?,
            interval_point(c, ctx)?,
            interval_point(d, ctx)?,
        ];
        let flattened: Vec<_> = points.into_iter().flatten().collect();
        if let Some(sign) = crate::expression(crate::Predicate::Orient3, &flattened, ctx)?.sign() {
            ctx.charge(0)?;
            return Ok(sign);
        }
        stage = Stage::ExactExpansion;
        let mut evaluation = Evaluation::default();
        let a = evaluation.point(a, ctx)?;
        let b = evaluation.point(b, ctx)?;
        let c = evaluation.point(c, ctx)?;
        let d = evaluation.point(d, ctx)?;
        let sign = dot(&plane(&a, &b, &c, ctx)?, &d, ctx)?.sign();
        ctx.charge(0)?;
        Ok(sign)
    })();
    Ok(Decision {
        outcome: match result {
            Ok(sign) => Outcome::Sign(sign),
            Err(reason) => Outcome::Indeterminate(reason),
        },
        stage,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

/// Exact algebraic location relative to the recipe's original finite segment.
/// This is independent of modelling tolerances and does not merge nearby points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentLocation3 {
    BeforeStart,
    Start,
    Interior,
    End,
    AfterEnd,
}
/// Original triangle vertices are C/D/E. Edge indices are opposite C/D/E.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriangleLocation3 {
    Outside,
    Interior,
    Edge(usize),
    Vertex(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FiniteLocation3 {
    pub segment: SegmentLocation3,
    pub triangle: TriangleLocation3,
}
impl FiniteLocation3 {
    /// Includes exact endpoints, edges and vertices of both closed domains.
    pub fn is_closed_domain_hit(self) -> bool {
        !matches!(
            self.segment,
            SegmentLocation3::BeforeStart | SegmentLocation3::AfterEnd
        ) && self.triangle != TriangleLocation3::Outside
    }
}
#[derive(Clone, Debug)]
pub struct FiniteLocation3Report {
    pub outcome: Result<FiniteLocation3, Reason>,
    pub context: ContextIdentity,
    pub work_used: u64,
}
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
fn barycentric_fraction(
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
fn clip_endpoint(
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

fn triangle_location_from_signs(signs: [Sign; 3]) -> Result<TriangleLocation3, Reason> {
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
