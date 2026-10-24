//! Candidate finite predicates over explicit authored source coordinates.
//!
//! This crate has no B-rep dependency and issues no solid/operation certificate.
//! Authored admission is a trusted boundary: callers must not relabel rounded
//! construction results as authored input. Opaque references prevent accidental
//! cross-source reuse; they do not authenticate arbitrary caller assertions.
mod arithmetic;
mod fibered_affine;
pub use fibered_affine::*;
mod bezier_quadric_identity;
pub use bezier_quadric_identity::*;
mod bezier_composition_identity;
pub use bezier_composition_identity::*;
mod line_chart_identity;
pub use line_chart_identity::*;
mod line_crossing_parameter;
pub use line_crossing_parameter::*;
mod bspline_restriction_identity;
pub use bspline_restriction_identity::*;
mod bspline_projective_correspondence;
pub use bspline_projective_correspondence::*;
mod bezier_projective_correspondence;
pub use bezier_projective_correspondence::*;
mod bezier_identity;
pub use bezier_identity::*;
mod projective_strip_jets;
pub use projective_strip_jets::*;
mod curve_geometric_jets;
pub use curve_geometric_jets::*;
mod bezier_strip_jets;
pub use bezier_strip_jets::*;
mod bspline_internal_jets;
pub use bspline_internal_jets::*;
mod bspline_knot_jets;
pub use bspline_knot_jets::*;
mod bspline_endpoint_jets;
pub use bspline_endpoint_jets::*;
mod construction;
mod construction3;
mod context;
mod recipe_codec3;
pub use recipe_codec3::*;

use arithmetic::{Approx, Expansion, Interval};
pub use construction::*;
pub use construction3::*;
pub use context::*;

const BINOMIAL_TABLE: [[u64; 33]; 33] = {
    let mut table = [[0u64; 33]; 33];
    let mut n = 0;
    while n < 33 {
        table[n][0] = 1;
        let mut k = 1;
        while k <= n {
            table[n][k] = table[n - 1][k - 1] + table[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    table
};

#[inline(always)]
pub(crate) fn binomial_u64(n: usize, k: usize) -> u64 {
    if k > n {
        0
    } else if n < 33 {
        BINOMIAL_TABLE[n][k]
    } else {
        (0..k).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
    }
}

pub const IMPLEMENTATION_VERSION: &str = "cad-predicates-candidate-15";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    Negative,
    Zero,
    Positive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    PrecisionExhausted,
    ResourceLimit,
    Cancelled,
    DeadlineExceeded,
    MissingProof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Admission,
    Binary64,
    Interval,
    ExactExpansion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Sign(Sign),
    Indeterminate(Reason),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub outcome: Outcome,
    pub stage: Stage,
    /// Cumulative charged work in the immutable source/tolerance context.
    pub work_used: u64,
    pub context: ContextIdentity,
}

#[derive(Clone, Copy)]
enum Predicate {
    Orient2,
    Orient3,
    InCircle2,
    DirectionDot3,
    Distance(usize),
}

trait Algebra: Sized + Clone {
    fn add(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason>;
    fn sub(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason>;
    fn mul(&self, other: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason>;
}

fn expression<T: Algebra>(
    kind: Predicate,
    values: &[T],
    ctx: &mut PredicateContext<'_>,
) -> Result<T, Reason> {
    match kind {
        Predicate::Orient2 => {
            let x = values[2].sub(&values[0], ctx)?;
            let y = values[3].sub(&values[1], ctx)?;
            let u = values[4].sub(&values[0], ctx)?;
            let v = values[5].sub(&values[1], ctx)?;
            x.mul(&v, ctx)?.sub(&y.mul(&u, ctx)?, ctx)
        }
        Predicate::InCircle2 => {
            let adx = values[0].sub(&values[6], ctx)?;
            let ady = values[1].sub(&values[7], ctx)?;
            let bdx = values[2].sub(&values[6], ctx)?;
            let bdy = values[3].sub(&values[7], ctx)?;
            let cdx = values[4].sub(&values[6], ctx)?;
            let cdy = values[5].sub(&values[7], ctx)?;
            let alift = adx.mul(&adx, ctx)?.add(&ady.mul(&ady, ctx)?, ctx)?;
            let blift = bdx.mul(&bdx, ctx)?.add(&bdy.mul(&bdy, ctx)?, ctx)?;
            let clift = cdx.mul(&cdx, ctx)?.add(&cdy.mul(&cdy, ctx)?, ctx)?;
            let bc = bdx.mul(&cdy, ctx)?.sub(&bdy.mul(&cdx, ctx)?, ctx)?;
            let ca = cdx.mul(&ady, ctx)?.sub(&cdy.mul(&adx, ctx)?, ctx)?;
            let ab = adx.mul(&bdy, ctx)?.sub(&ady.mul(&bdx, ctx)?, ctx)?;
            alift
                .mul(&bc, ctx)?
                .add(&blift.mul(&ca, ctx)?, ctx)?
                .add(&clift.mul(&ab, ctx)?, ctx)
        }
        Predicate::Orient3 => {
            let mut vectors = Vec::with_capacity(9);
            for i in 3..12 {
                vectors.push(values[i].sub(&values[i % 3], ctx)?);
            }
            let x = vectors[4]
                .mul(&vectors[8], ctx)?
                .sub(&vectors[5].mul(&vectors[7], ctx)?, ctx)?;
            let y = vectors[5]
                .mul(&vectors[6], ctx)?
                .sub(&vectors[3].mul(&vectors[8], ctx)?, ctx)?;
            let z = vectors[3]
                .mul(&vectors[7], ctx)?
                .sub(&vectors[4].mul(&vectors[6], ctx)?, ctx)?;
            vectors[0]
                .mul(&x, ctx)?
                .add(&vectors[1].mul(&y, ctx)?, ctx)?
                .add(&vectors[2].mul(&z, ctx)?, ctx)
        }
        Predicate::DirectionDot3 => {
            let product = |k: usize, ctx: &mut PredicateContext<'_>| {
                values[3 + k].sub(&values[k], ctx)?.mul(
                    &values[9 + k].sub(&values[6 + k], ctx)?, ctx,
                )
            };
            product(0, ctx)?.add(&product(1, ctx)?, ctx)?.add(&product(2, ctx)?, ctx)
        }
        Predicate::Distance(dimension) => {
            let first = values[0].sub(&values[dimension], ctx)?;
            let mut result = first.mul(&first, ctx)?;
            for i in 1..dimension {
                let difference = values[i].sub(&values[dimension + i], ctx)?;
                result = result.add(&difference.mul(&difference, ctx)?, ctx)?;
            }
            result.sub(
                &values[2 * dimension].mul(&values[2 * dimension], ctx)?,
                ctx,
            )
        }
    }
}

fn exact_inputs(
    values: &[AuthoredScalar],
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<Expansion>, Reason> {
    // All coordinates share a positive denominator product. No division or
    // rounded rational center enters the exact expression. Products may refuse
    // if the finite expansion envelope cannot contain this particular matrix.
    let mut numerators = Vec::with_capacity(values.len());
    let mut denominators = Vec::with_capacity(values.len());
    for value in values {
        ctx.charge(1)?;
        match *value {
            AuthoredScalar::Binary64Bits(bits) => {
                numerators.push(Expansion::scalar(f64::from_bits(bits)));
                denominators.push(Expansion::scalar(1.));
            }
            AuthoredScalar::RationalConstant {
                numerator,
                denominator,
            } => {
                let n = Expansion::integer(numerator.unsigned_abs(), ctx)?;
                numerators.push(if numerator < 0 { n.negated() } else { n });
                denominators.push(Expansion::integer(denominator, ctx)?);
            }
        }
    }
    for (i, numerator) in numerators.iter_mut().enumerate() {
        for (j, denominator) in denominators.iter().enumerate() {
            if i != j && !denominator.is_one() {
                *numerator = numerator.mul(denominator, ctx)?;
            }
        }
    }
    Expansion::normalize_common(&mut numerators, ctx)?;
    Ok(numerators)
}

fn evaluate(
    ctx: &mut PredicateContext<'_>,
    kind: Predicate,
    references: &[LeafRef],
) -> Result<Decision, InputError> {
    // Validate every reference before either a shortcut or resource refusal.
    let values = references
        .iter()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let mut stage = Stage::Admission;
    let computed = (|| {
        ctx.charge(references.len() as u64)?;
        stage = Stage::Binary64;
        let fast = values
            .iter()
            .map(|v| match v {
                AuthoredScalar::Binary64Bits(bits) => Some(Approx::leaf(f64::from_bits(*bits))),
                AuthoredScalar::RationalConstant { .. } => None,
            })
            .collect::<Option<Vec<_>>>();
        if let Some(fast) = fast
            && let Some(sign) = expression(kind, &fast, ctx)?.sign()
        {
            return Ok(sign);
        }
        stage = Stage::Interval;
        let intervals = values
            .iter()
            .map(|v| Interval::authored(v, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(sign) = expression(kind, &intervals, ctx)?.sign() {
            return Ok(sign);
        }
        stage = Stage::ExactExpansion;
        let expanded = exact_inputs(&values, ctx)?;
        let sign = expression(kind, &expanded, ctx)?.sign();
        ctx.charge(0)?;
        Ok(sign)
    })();
    let computed = computed.and_then(|sign| ctx.charge(0).map(|_| sign));
    Ok(Decision {
        outcome: match computed {
            Ok(sign) => Outcome::Sign(sign),
            Err(reason) => Outcome::Indeterminate(reason),
        },
        stage,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

pub fn orient2d(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 2],
    b: [LeafRef; 2],
    c: [LeafRef; 2],
) -> Result<Decision, InputError> {
    evaluate(
        ctx,
        Predicate::Orient2,
        &[a[0], a[1], b[0], b[1], c[0], c[1]],
    )
}

/// Sign of (b-a) dot (d-c), evaluated from original coordinates without
/// rounding either direction first. Zero certifies perpendicular directions;
/// callers must establish nondegeneracy separately.
pub fn direction_dot3d(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 3], b: [LeafRef; 3], c: [LeafRef; 3], d: [LeafRef; 3],
) -> Result<Decision, InputError> {
    evaluate(ctx, Predicate::DirectionDot3,
        &[a[0],a[1],a[2],b[0],b[1],b[2],c[0],c[1],c[2],d[0],d[1],d[2]])
}

pub fn orient3d(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 3],
    b: [LeafRef; 3],
    c: [LeafRef; 3],
    d: [LeafRef; 3],
) -> Result<Decision, InputError> {
    evaluate(
        ctx,
        Predicate::Orient3,
        &[
            a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2],
        ],
    )
}

/// Authored 2D incircle predicate: positive when `d` lies strictly inside the
/// counter-clockwise circle through `(a, b, c)`, negative when strictly outside,
/// and zero when cocircular.
pub fn incircle2d_authored(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 2],
    b: [LeafRef; 2],
    c: [LeafRef; 2],
    d: [LeafRef; 2],
) -> Result<Decision, InputError> {
    evaluate(
        ctx,
        Predicate::InCircle2,
        &[a[0], a[1], b[0], b[1], c[0], c[1], d[0], d[1]],
    )
}

/// Adaptive exact 2D incircle predicate over finite binary64 coordinates.
pub fn incircle2d(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Sign {
    let coords = [a[0], a[1], b[0], b[1], c[0], c[1], d[0], d[1]];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Zero;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("incircle2d-ephemeral", 0, scalars) else {
        return Sign::Zero;
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let pair = |offset: usize| [source.leaf(offset).unwrap(), source.leaf(offset + 1).unwrap()];
    match incircle2d_authored(&mut ctx, pair(0), pair(2), pair(4), pair(6)) {
        Ok(Decision {
            outcome: Outcome::Sign(sign),
            ..
        }) => sign,
        _ => Sign::Zero,
    }
}

/// Return true when `d` lies within signed distance `tol` of the plane spanned by
/// `(a, b, c)`, or when `(a, b, c)` are collinear within `tol`. Uses exact
/// orientation when `tol <= 0.0`.
pub fn coplanar3d(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3], tol: f64) -> bool {
    let coords = [
        a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) || !tol.is_finite() || tol < 0.0 {
        return false;
    }
    if tol == 0.0 {
        let scalars = coords
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect();
        let Ok(source) = SourceArena::authored("coplanar3d-ephemeral", 0, scalars) else {
            return false;
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        let pt = |offset: usize| {
            [
                source.leaf(offset).unwrap(),
                source.leaf(offset + 1).unwrap(),
                source.leaf(offset + 2).unwrap(),
            ]
        };
        return matches!(
            orient3d(&mut ctx, pt(0), pt(3), pt(6), pt(9)),
            Ok(Decision {
                outcome: Outcome::Sign(Sign::Zero),
                ..
            })
        );
    }
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let ad = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
    let n = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let n_len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if n_len <= tol * tol {
        return true;
    }
    let det = ad[0] * n[0] + ad[1] * n[1] + ad[2] * n[2];
    (det.abs() / n_len) <= tol
}

/// Return true when `(a, b, c)` are collinear within perpendicular distance `tol`
/// (exact cross-product zero check when `tol <= 0.0`).
pub fn collinear3d(a: [f64; 3], b: [f64; 3], c: [f64; 3], tol: f64) -> bool {
    let coords = [a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]];
    if coords.iter().any(|v| !v.is_finite()) || !tol.is_finite() || tol < 0.0 {
        return false;
    }
    if tol == 0.0 {
        let scalars = [
            a[0], a[1], b[0], b[1], c[0], c[1],
            a[1], a[2], b[1], b[2], c[1], c[2],
            a[2], a[0], b[2], b[0], c[2], c[0],
        ]
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
        let Ok(source) = SourceArena::authored("collinear3d-ephemeral", 0, scalars) else {
            return false;
        };
        let tolerance = ToleranceContext::default_valid();
        for plane in 0..3 {
            let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
            let base = plane * 6;
            let p = |k: usize| [source.leaf(base + k).unwrap(), source.leaf(base + k + 1).unwrap()];
            let Ok(decision) = orient2d(&mut ctx, p(0), p(2), p(4)) else {
                return false;
            };
            if decision.outcome != Outcome::Sign(Sign::Zero) {
                return false;
            }
        }
        return true;
    }
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let bc = [c[0] - b[0], c[1] - b[1], c[2] - b[2]];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let area2 = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    let max_len = (ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2])
        .max(ac[0] * ac[0] + ac[1] * ac[1] + ac[2] * ac[2])
        .max(bc[0] * bc[0] + bc[1] * bc[1] + bc[2] * bc[2])
        .sqrt();
    if max_len <= tol {
        return true;
    }
    (area2 / max_len) <= tol
}

fn distance_references(
    ctx: &PredicateContext<'_>,
    p: &[LeafRef],
    q: &[LeafRef],
) -> Result<Vec<LeafRef>, InputError> {
    if p.len() != q.len() || !(2..=3).contains(&p.len()) {
        return Err(InputError::InvalidInput(
            "Expected matching 2D or 3D points",
        ));
    }
    let references: Vec<_> = p.iter().chain(q).copied().collect();
    for reference in &references {
        ctx.resolve(*reference)?;
    }
    Ok(references)
}

pub fn compare_squared_distance(
    ctx: &mut PredicateContext<'_>,
    p: &[LeafRef],
    q: &[LeafRef],
    radius: LeafRef,
) -> Result<Decision, InputError> {
    let mut references = distance_references(ctx, p, q)?;
    let nonnegative = match *ctx.resolve(radius)? {
        AuthoredScalar::Binary64Bits(bits) => f64::from_bits(bits) >= 0.,
        AuthoredScalar::RationalConstant { numerator, .. } => numerator >= 0,
    };
    if !nonnegative {
        return Err(InputError::InvalidInput("Radius must be nonnegative"));
    }
    references.push(radius);
    evaluate(ctx, Predicate::Distance(p.len()), &references)
}

/// An enclosure issued only by this crate's distance recipe, bound to its
/// source snapshot and tolerance context. No public arbitrary-bound constructor.
#[derive(Debug)]
pub struct ProvenResidual {
    squared: Interval,
    context: ContextIdentity,
}

#[derive(Debug)]
pub enum ResidualOutcome {
    Proven(ProvenResidual),
    Indeterminate(Reason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelClass {
    Coincident,
    Separate,
    Indeterminate(Reason),
}

pub fn distance_enclosure(
    ctx: &mut PredicateContext<'_>,
    p: &[LeafRef],
    q: &[LeafRef],
) -> Result<ResidualOutcome, InputError> {
    let references = distance_references(ctx, p, q)?;
    let values = references
        .iter()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let computed = (|| {
        ctx.charge(references.len() as u64)?;
        let values = values
            .iter()
            .map(|v| Interval::authored(v, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let mut sum = Interval::point(0.);
        for i in 0..p.len() {
            let difference = values[i].sub(&values[p.len() + i], ctx)?;
            sum = sum.add(&difference.mul(&difference, ctx)?, ctx)?;
        }
        // Intersect with the independently known nonnegativity of a sum of
        // squares. This is a mathematical enclosure refinement, not snapping.
        sum.lo = sum.lo.max(0.);
        ctx.charge(8)?;
        if sum.distance_width() > ctx.tolerance.spec.max_entity_error {
            return Err(Reason::PrecisionExhausted);
        }
        ctx.charge(0)?;
        Ok(sum)
    })();
    Ok(match computed {
        Ok(squared) => ResidualOutcome::Proven(ProvenResidual {
            squared,
            context: ctx.identity(),
        }),
        Err(reason) => ResidualOutcome::Indeterminate(reason),
    })
}

pub fn classify_residual(
    ctx: &mut PredicateContext<'_>,
    residual: Option<&ProvenResidual>,
) -> Result<ModelClass, InputError> {
    if residual.is_some_and(|r| r.context != ctx.identity()) {
        return Err(InputError::ContextMismatch);
    }
    let computed = (|| {
        ctx.charge(1)?;
        let residual = residual.ok_or(Reason::MissingProof)?;
        let on = Interval::point(ctx.tolerance.spec.on_tol);
        let clear = Interval::point(ctx.tolerance.spec.clear_tol);
        let on_squared = on.mul(&on, ctx)?;
        let clear_squared = clear.mul(&clear, ctx)?;
        ctx.charge(0)?;
        if residual.squared.hi < on_squared.lo {
            Ok(ModelClass::Coincident)
        } else if residual.squared.lo > clear_squared.hi {
            Ok(ModelClass::Separate)
        } else {
            Ok(ModelClass::Indeterminate(Reason::PrecisionExhausted))
        }
    })();
    Ok(computed.unwrap_or_else(ModelClass::Indeterminate))
}

#[cfg(test)]
mod tests;

mod bezier_point_identity;
pub use bezier_point_identity::*;

mod quadratic_shear_identity;
pub use quadratic_shear_identity::*;

mod projected_surface_jacobian;
pub use projected_surface_jacobian::Decision as SurfaceDerivativeDecision;
pub use projected_surface_jacobian::*;

mod quadratic_curve_separator;
pub use quadratic_curve_separator::*;
mod bilinear_graph_side;
pub use bilinear_graph_side::rational_bilinear_graph_side;

mod planar_bezier_roots;
pub use planar_bezier_roots::*;
