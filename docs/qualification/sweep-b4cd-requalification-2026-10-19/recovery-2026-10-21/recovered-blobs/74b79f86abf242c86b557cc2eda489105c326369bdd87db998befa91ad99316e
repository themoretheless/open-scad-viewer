//! Finite projective construction recipes; rounded centers are never inputs.
//!
//! In homogeneous 2D coordinates a line is A×B and its unique intersection
//! with C×D is (A×B)×(C×D). All products/sums use bounded exact expansions.
//! Positive common power-of-two normalization preserves each homogeneous object.
//! These are infinite straight lines, not general curve or segment intersections.
use crate::arithmetic::{Expansion, Interval};
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Decision, InputError, LeafRef, Outcome,
    PredicateContext, Reason, Sign, Stage,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

type HPoint = [Expansion; 3];
type SourcePoints = [[LeafRef; 2]; 4];
pub const MAX_CONSTRUCTION_DEPTH: usize = 32;
pub const MAX_CONSTRUCTION_NODES: usize = 256;
pub(crate) const MAX_CACHED_TERMS: usize = 65_536;

/// Only the construction function can mint this context-bound recipe.
/// Enclosures are observations of the exact recipe, never substitute source leaves.
#[derive(Clone)]
pub struct ConstructedPoint2 {
    node: Arc<ConstructionNode2>,
}
impl std::fmt::Debug for ConstructedPoint2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not recursively expand shared ancestors into exponentially large logs.
        f.debug_struct("ConstructedPoint2")
            .field("context", &self.context())
            .field("enclosure", &self.enclosure())
            .field("depth", &self.recipe_depth())
            .field("nodes", &self.recipe_node_count())
            .finish_non_exhaustive()
    }
}
#[derive(Debug)]
struct ConstructionNode2 {
    inputs: [StoredPoint2; 4],
    context: ContextIdentity,
    enclosure: [[f64; 2]; 2],
    parameters: [[f64; 2]; 2],
    diameter_bound: f64,
    depth: usize,
    node_count: usize,
}
#[derive(Clone, Debug)]
enum StoredPoint2 {
    Authored([LeafRef; 2]),
    Constructed(ConstructedPoint2),
}
impl StoredPoint2 {
    fn borrowed(&self) -> Point2Input<'_> {
        match self {
            Self::Authored(point) => Point2Input::Authored(*point),
            Self::Constructed(point) => Point2Input::Constructed(point),
        }
    }
    fn from_input(input: Point2Input<'_>) -> Self {
        match input {
            Point2Input::Authored(point) => Self::Authored(point),
            Point2Input::Constructed(point) => Self::Constructed(point.clone()),
        }
    }
}
impl ConstructedPoint2 {
    pub fn context(&self) -> ContextIdentity {
        self.node.context
    }
    /// Authored endpoints when this is a first-generation recipe; otherwise None.
    pub fn source_points(&self) -> Option<SourcePoints> {
        match self.inputs() {
            [
                Point2Input::Authored(a),
                Point2Input::Authored(b),
                Point2Input::Authored(c),
                Point2Input::Authored(d),
            ] => Some([a, b, c, d]),
            _ => None,
        }
    }
    pub fn inputs(&self) -> [Point2Input<'_>; 4] {
        self.node.inputs.each_ref().map(StoredPoint2::borrowed)
    }
    pub fn recipe_depth(&self) -> usize {
        self.node.depth
    }
    /// Unique shared construction nodes reachable from this recipe, including it.
    pub fn recipe_node_count(&self) -> usize {
        self.node.node_count
    }
    fn key(&self) -> *const ConstructionNode2 {
        Arc::as_ptr(&self.node)
    }
    /// Outward x/y bounds of the unique intersection of the two infinite lines.
    pub fn enclosure(&self) -> [[f64; 2]; 2] {
        self.node.enclosure
    }
    /// Outward t/s bounds in A+t(B-A), C+s(D-C); these need not lie in [0,1].
    pub fn parameter_enclosures(&self) -> [[f64; 2]; 2] {
        self.node.parameters
    }
    /// Outward L1 box diameter bound, never smaller than an ancestor's bound.
    pub fn diameter_bound(&self) -> f64 {
        self.node.diameter_bound
    }
}

#[derive(Clone, Debug)]
pub enum LineIntersection2 {
    Unique(Box<ConstructedPoint2>),
    Parallel,
    Coincident,
    /// Index 0 or 1 of the line whose two endpoints are exactly equal.
    DegenerateLine(usize),
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct Construction2Report {
    pub outcome: LineIntersection2,
    pub context: ContextIdentity,
    pub work_used: u64,
}

#[derive(Clone, Copy)]
pub enum Point2Input<'a> {
    Authored([LeafRef; 2]),
    Constructed(&'a ConstructedPoint2),
}

fn values(
    ctx: &PredicateContext<'_>,
    point: [LeafRef; 2],
) -> Result<[AuthoredScalar; 2], InputError> {
    Ok([
        ctx.resolve(point[0])?.clone(),
        ctx.resolve(point[1])?.clone(),
    ])
}
struct Inventory {
    nodes: usize,
    depth: usize,
    work: u64,
}
fn inventory(
    ctx: &PredicateContext<'_>,
    inputs: &[Point2Input<'_>],
) -> Result<Inventory, InputError> {
    // Each admitted root owns at most 256 nodes; at most four roots enter a call.
    // Admission is therefore bounded even before cancellation/resource shortcuts.
    let mut seen = HashSet::new();
    let mut pending = inputs.to_vec();
    let mut depth = 0;
    let mut work = 0;
    while let Some(input) = pending.pop() {
        work += 1;
        match input {
            Point2Input::Authored(point) => {
                values(ctx, point)?;
                work += 2;
            }
            Point2Input::Constructed(point) => {
                if point.context() != ctx.identity() {
                    return Err(InputError::ContextMismatch);
                }
                depth = depth.max(point.recipe_depth());
                if seen.insert(point.key()) {
                    pending.extend(point.inputs());
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
pub(crate) fn fraction(
    value: &AuthoredScalar,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Expansion; 2], Reason> {
    ctx.charge(1)?;
    Ok(match *value {
        AuthoredScalar::Binary64Bits(bits) => [
            Expansion::scalar(f64::from_bits(bits)),
            Expansion::scalar(1.),
        ],
        AuthoredScalar::RationalConstant {
            numerator,
            denominator,
        } => {
            let n = Expansion::integer(numerator.unsigned_abs(), ctx)?;
            [
                if numerator < 0 { n.negated() } else { n },
                Expansion::integer(denominator, ctx)?,
            ]
        }
    })
}
pub(crate) fn normalize<const N: usize>(
    mut point: [Expansion; N],
    ctx: &mut PredicateContext<'_>,
) -> Result<[Expansion; N], Reason> {
    Expansion::reduce_projective(&mut point, ctx)?;
    Expansion::normalize_common(&mut point, ctx)?;
    Ok(point)
}
fn authored(point: &[AuthoredScalar; 2], ctx: &mut PredicateContext<'_>) -> Result<HPoint, Reason> {
    let [xn, xd] = fraction(&point[0], ctx)?;
    let [yn, yd] = fraction(&point[1], ctx)?;
    // x=X/W, y=Y/W. Denominators are multiplied, never divided and rounded.
    normalize(
        [xn.mul(&yd, ctx)?, yn.mul(&xd, ctx)?, xd.mul(&yd, ctx)?],
        ctx,
    )
}
fn cross(a: &HPoint, b: &HPoint, ctx: &mut PredicateContext<'_>) -> Result<HPoint, Reason> {
    normalize(
        [
            a[1].mul(&b[2], ctx)?.sub(&a[2].mul(&b[1], ctx)?, ctx)?,
            a[2].mul(&b[0], ctx)?.sub(&a[0].mul(&b[2], ctx)?, ctx)?,
            a[0].mul(&b[1], ctx)?.sub(&a[1].mul(&b[0], ctx)?, ctx)?,
        ],
        ctx,
    )
}
fn dot(a: &HPoint, b: &HPoint, ctx: &mut PredicateContext<'_>) -> Result<Expansion, Reason> {
    a[0].mul(&b[0], ctx)?
        .add(&a[1].mul(&b[1], ctx)?, ctx)?
        .add(&a[2].mul(&b[2], ctx)?, ctx)
}
fn zero(a: &HPoint) -> bool {
    a.iter().all(|v| v.sign() == Sign::Zero)
}

enum ExactIntersection {
    Unique {
        point: HPoint,
        sources: Box<[HPoint; 4]>,
    },
    Parallel,
    Coincident,
    Degenerate(usize),
}
fn solve(points: [HPoint; 4], ctx: &mut PredicateContext<'_>) -> Result<ExactIntersection, Reason> {
    let first = cross(&points[0], &points[1], ctx)?;
    let second = cross(&points[2], &points[3], ctx)?;
    if zero(&first) {
        return Ok(ExactIntersection::Degenerate(0));
    }
    if zero(&second) {
        return Ok(ExactIntersection::Degenerate(1));
    }
    let mut point = cross(&first, &second, ctx)?;
    if point[2].sign() == Sign::Zero {
        return Ok(if zero(&point) {
            ExactIntersection::Coincident
        } else {
            ExactIntersection::Parallel
        });
    }
    if point[2].sign() == Sign::Negative {
        point = point.map(|value| value.negated());
    }
    // Verify exact incidence in both source lines before issuing a recipe.
    if dot(&first, &point, ctx)?.sign() != Sign::Zero
        || dot(&second, &point, ctx)?.sign() != Sign::Zero
    {
        return Err(Reason::PrecisionExhausted);
    }
    Ok(ExactIntersection::Unique {
        point,
        sources: Box::new(points),
    })
}
fn quotient(
    numerator: &Expansion,
    denominator: &Expansion,
    ctx: &mut PredicateContext<'_>,
) -> Result<Interval, Reason> {
    numerator
        .enclosure(ctx)?
        .div_nonzero(&denominator.enclosure(ctx)?, ctx)
}
fn parameter(
    point: &HPoint,
    a: &HPoint,
    b: &HPoint,
    ctx: &mut PredicateContext<'_>,
) -> Result<Interval, Reason> {
    for axis in 0..2 {
        let delta = b[axis]
            .mul(&a[2], ctx)?
            .sub(&a[axis].mul(&b[2], ctx)?, ctx)?;
        if delta.sign() != Sign::Zero {
            let numerator = point[axis]
                .mul(&a[2], ctx)?
                .sub(&a[axis].mul(&point[2], ctx)?, ctx)?
                .mul(&b[2], ctx)?;
            let denominator = delta.mul(&point[2], ctx)?;
            return quotient(&numerator, &denominator, ctx);
        }
    }
    Err(Reason::PrecisionExhausted)
}

pub fn intersect_authored_lines2d(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 2],
    b: [LeafRef; 2],
    c: [LeafRef; 2],
    d: [LeafRef; 2],
) -> Result<Construction2Report, InputError> {
    intersect_lines2d(
        ctx,
        Point2Input::Authored(a),
        Point2Input::Authored(b),
        Point2Input::Authored(c),
        Point2Input::Authored(d),
    )
}

/// Intersect infinite lines whose endpoints may themselves be exact recipes.
/// Shared nodes remain shared; no input handle or prior construction is mutated.
pub fn intersect_lines2d(
    ctx: &mut PredicateContext<'_>,
    a: Point2Input<'_>,
    b: Point2Input<'_>,
    c: Point2Input<'_>,
    d: Point2Input<'_>,
) -> Result<Construction2Report, InputError> {
    let inputs = [a, b, c, d];
    let inventory = inventory(ctx, &inputs)?;
    let outcome = (|| {
        ctx.charge(inventory.work)?;
        if inventory.depth >= MAX_CONSTRUCTION_DEPTH || inventory.nodes >= MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        let mut evaluation = Evaluation::default();
        let points = [
            evaluation.point(a, ctx)?,
            evaluation.point(b, ctx)?,
            evaluation.point(c, ctx)?,
            evaluation.point(d, ctx)?,
        ];
        let result = match solve(points, ctx)? {
            ExactIntersection::Parallel => LineIntersection2::Parallel,
            ExactIntersection::Coincident => LineIntersection2::Coincident,
            ExactIntersection::Degenerate(line) => LineIntersection2::DegenerateLine(line),
            ExactIntersection::Unique { point, sources } => {
                let x = quotient(&point[0], &point[2], ctx)?;
                let y = quotient(&point[1], &point[2], ctx)?;
                let measured_diameter = Interval::point(x.width())
                    .add(&Interval::point(y.width()), ctx)?
                    .hi;
                // Keep the reported bound monotone through a chain. Exact recipe
                // re-evaluation establishes the new box; this maximum is not an
                // uncertainty-amplification or topology certificate.
                let diameter = inputs
                    .iter()
                    .fold(measured_diameter, |bound, input| match input {
                        Point2Input::Authored(_) => bound,
                        Point2Input::Constructed(point) => bound.max(point.diameter_bound()),
                    });
                if !diameter.is_finite() || diameter > ctx.tolerance.spec.max_entity_error {
                    return Err(Reason::PrecisionExhausted);
                }
                let t = parameter(&point, &sources[0], &sources[1], ctx)?;
                let s = parameter(&point, &sources[2], &sources[3], ctx)?;
                LineIntersection2::Unique(Box::new(ConstructedPoint2 {
                    node: Arc::new(ConstructionNode2 {
                        inputs: inputs.map(StoredPoint2::from_input),
                        context: ctx.identity(),
                        enclosure: [[x.lo, x.hi], [y.lo, y.hi]],
                        parameters: [[t.lo, t.hi], [s.lo, s.hi]],
                        diameter_bound: diameter,
                        depth: inventory.depth + 1,
                        node_count: inventory.nodes + 1,
                    }),
                }))
            }
        };
        ctx.charge(0)?;
        Ok(result)
    })();
    Ok(Construction2Report {
        outcome: outcome.unwrap_or_else(LineIntersection2::Indeterminate),
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}

#[derive(Default)]
struct Evaluation {
    // Arc addresses are only live, per-query lookup keys. They are never portable
    // identities, decision tie-breakers or cache keys surviving this query.
    cache: HashMap<*const ConstructionNode2, HPoint>,
    terms: usize,
}
impl Evaluation {
    fn point(
        &mut self,
        input: Point2Input<'_>,
        ctx: &mut PredicateContext<'_>,
    ) -> Result<HPoint, Reason> {
        ctx.charge(1)?;
        match input {
            Point2Input::Authored(point) => {
                authored(&values(ctx, point).map_err(|_| Reason::MissingProof)?, ctx)
            }
            Point2Input::Constructed(recipe) => {
                if let Some(cached) = self.cache.get(&recipe.key()) {
                    ctx.charge(cached.iter().map(Expansion::term_count).sum::<usize>() as u64)?;
                    return Ok(cached.clone());
                }
                let [a, b, c, d] = recipe.inputs();
                let points = [
                    self.point(a, ctx)?,
                    self.point(b, ctx)?,
                    self.point(c, ctx)?,
                    self.point(d, ctx)?,
                ];
                let ExactIntersection::Unique { point, .. } = solve(points, ctx)? else {
                    return Err(Reason::MissingProof);
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
fn interval_point(
    point: Point2Input<'_>,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Interval; 2], Reason> {
    match point {
        Point2Input::Authored(point) => {
            let value = values(ctx, point).map_err(|_| Reason::MissingProof)?;
            Ok([
                Interval::authored(&value[0], ctx)?,
                Interval::authored(&value[1], ctx)?,
            ])
        }
        Point2Input::Constructed(point) => {
            ctx.charge(2)?;
            Ok(point.enclosure().map(|[lo, hi]| Interval { lo, hi }))
        }
    }
}

/// Strict signs may use issued outward enclosures; ambiguous signs re-evaluate
/// the exact recipes. No enclosure center is promoted to an authored point;
/// exhaustion returns Indeterminate, never Zero.
pub fn orient2d_points(
    ctx: &mut PredicateContext<'_>,
    a: Point2Input<'_>,
    b: Point2Input<'_>,
    c: Point2Input<'_>,
) -> Result<Decision, InputError> {
    if let (Point2Input::Authored(a), Point2Input::Authored(b), Point2Input::Authored(c)) =
        (a, b, c)
    {
        return crate::orient2d(ctx, a, b, c);
    }
    let inventory = inventory(ctx, &[a, b, c])?;
    let mut stage = Stage::Admission;
    let result = (|| {
        ctx.charge(inventory.work)?;
        if inventory.nodes > MAX_CONSTRUCTION_NODES || inventory.depth > MAX_CONSTRUCTION_DEPTH {
            return Err(Reason::ResourceLimit);
        }
        // A previously issued outward enclosure can accelerate a strict sign.
        // Ambiguity (including exact incidence) always re-evaluates the recipe.
        stage = Stage::Interval;
        let intervals = [
            interval_point(a, ctx)?,
            interval_point(b, ctx)?,
            interval_point(c, ctx)?,
        ];
        let determinant = crate::expression(
            crate::Predicate::Orient2,
            &[
                intervals[0][0],
                intervals[0][1],
                intervals[1][0],
                intervals[1][1],
                intervals[2][0],
                intervals[2][1],
            ],
            ctx,
        )?;
        if let Some(sign) = determinant.sign() {
            ctx.charge(0)?;
            return Ok(sign);
        }
        stage = Stage::ExactExpansion;
        let mut evaluation = Evaluation::default();
        let a = evaluation.point(a, ctx)?;
        let b = evaluation.point(b, ctx)?;
        let c = evaluation.point(c, ctx)?;
        // All finite homogeneous weights are positive, so det(A,B,C) has
        // precisely the affine orient2d sign, without any quotient rounding.
        let sign = dot(&a, &cross(&b, &c, ctx)?, ctx)?.sign();
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
