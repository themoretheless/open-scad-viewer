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
mod bspline_strip_jets;
pub use bspline_strip_jets::*;
mod bspline_trace_identity;
pub use bspline_trace_identity::*;
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
    InSphere3,
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
        Predicate::InSphere3 => {
            let aex = values[0].sub(&values[12], ctx)?;
            let aey = values[1].sub(&values[13], ctx)?;
            let aez = values[2].sub(&values[14], ctx)?;
            let bex = values[3].sub(&values[12], ctx)?;
            let bey = values[4].sub(&values[13], ctx)?;
            let bez = values[5].sub(&values[14], ctx)?;
            let cex = values[6].sub(&values[12], ctx)?;
            let cey = values[7].sub(&values[13], ctx)?;
            let cez = values[8].sub(&values[14], ctx)?;
            let dex = values[9].sub(&values[12], ctx)?;
            let dey = values[10].sub(&values[13], ctx)?;
            let dez = values[11].sub(&values[14], ctx)?;

            let ab = aex.mul(&bey, ctx)?.sub(&bex.mul(&aey, ctx)?, ctx)?;
            let bc = bex.mul(&cey, ctx)?.sub(&cex.mul(&bey, ctx)?, ctx)?;
            let cd = cex.mul(&dey, ctx)?.sub(&dex.mul(&cey, ctx)?, ctx)?;
            let da = dex.mul(&aey, ctx)?.sub(&aex.mul(&dey, ctx)?, ctx)?;
            let ac = aex.mul(&cey, ctx)?.sub(&cex.mul(&aey, ctx)?, ctx)?;
            let bd = bex.mul(&dey, ctx)?.sub(&dex.mul(&bey, ctx)?, ctx)?;

            let abc = aez
                .mul(&bc, ctx)?
                .sub(&bez.mul(&ac, ctx)?, ctx)?
                .add(&cez.mul(&ab, ctx)?, ctx)?;
            let bcd = bez
                .mul(&cd, ctx)?
                .sub(&cez.mul(&bd, ctx)?, ctx)?
                .add(&dez.mul(&bc, ctx)?, ctx)?;
            let cda = cez
                .mul(&da, ctx)?
                .add(&dez.mul(&ac, ctx)?, ctx)?
                .add(&aez.mul(&cd, ctx)?, ctx)?;
            let dab = dez
                .mul(&ab, ctx)?
                .add(&aez.mul(&bd, ctx)?, ctx)?
                .add(&bez.mul(&da, ctx)?, ctx)?;

            let alift = aex
                .mul(&aex, ctx)?
                .add(&aey.mul(&aey, ctx)?, ctx)?
                .add(&aez.mul(&aez, ctx)?, ctx)?;
            let blift = bex
                .mul(&bex, ctx)?
                .add(&bey.mul(&bey, ctx)?, ctx)?
                .add(&bez.mul(&bez, ctx)?, ctx)?;
            let clift = cex
                .mul(&cex, ctx)?
                .add(&cey.mul(&cey, ctx)?, ctx)?
                .add(&cez.mul(&cez, ctx)?, ctx)?;
            let dlift = dex
                .mul(&dex, ctx)?
                .add(&dey.mul(&dey, ctx)?, ctx)?
                .add(&dez.mul(&dez, ctx)?, ctx)?;

            alift
                .mul(&bcd, ctx)?
                .sub(&blift.mul(&cda, ctx)?, ctx)?
                .add(&clift.mul(&dab, ctx)?, ctx)?
                .sub(&dlift.mul(&abc, ctx)?, ctx)
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


/// Authored 3D insphere predicate: positive when `e` lies strictly inside the
/// sphere through `(a, b, c, d)` when `orient3d(a, b, c, d)` is positive,
/// negative when strictly outside, and zero when cospherical.
pub fn insphere3d_authored(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 3],
    b: [LeafRef; 3],
    c: [LeafRef; 3],
    d: [LeafRef; 3],
    e: [LeafRef; 3],
) -> Result<Decision, InputError> {
    evaluate(
        ctx,
        Predicate::InSphere3,
        &[
            a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2], e[0], e[1],
            e[2],
        ],
    )
}

/// Adaptive exact 3D insphere predicate over finite binary64 coordinates.
/// Returns `Sign::Positive` when `e` is strictly inside the sphere through `(a, b, c, d)`
/// (assuming `orient3d(a, b, c, d) > 0`), `Sign::Negative` when strictly outside, and `Sign::Zero` when cospherical.
pub fn insphere3d(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
    e: [f64; 3],
) -> Sign {
    let coords = [
        a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2], e[0], e[1], e[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Zero;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("insphere3d-ephemeral", 0, scalars) else {
        return Sign::Zero;
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
    match insphere3d_authored(&mut ctx, pt(0), pt(3), pt(6), pt(9), pt(12)) {
        Ok(Decision {
            outcome: Outcome::Sign(sign),
            ..
        }) => sign,
        _ => Sign::Zero,
    }
}

/// Fast 2D axis-aligned bounding-box rejection filter: returns `true` when closed
/// boxes `[min_a, max_a]` and `[min_b, max_b]` do not overlap.
#[inline]
pub fn aabb2_disjoint(min_a: [f64; 2], max_a: [f64; 2], min_b: [f64; 2], max_b: [f64; 2]) -> bool {
    max_a[0] < min_b[0] || max_b[0] < min_a[0] || max_a[1] < min_b[1] || max_b[1] < min_a[1]
}

/// Fast 3D axis-aligned bounding-box rejection filter: returns `true` when closed
/// boxes `[min_a, max_a]` and `[min_b, max_b]` do not overlap.
#[inline]
pub fn aabb3_disjoint(min_a: [f64; 3], max_a: [f64; 3], min_b: [f64; 3], max_b: [f64; 3]) -> bool {
    max_a[0] < min_b[0]
        || max_b[0] < min_a[0]
        || max_a[1] < min_b[1]
        || max_b[1] < min_a[1]
        || max_a[2] < min_b[2]
        || max_b[2] < min_a[2]
}

/// Exact 2D closed segment intersection test using adaptive `orient2d` predicates.
/// Returns `true` if closed segments `[a0, a1]` and `[b0, b1]` intersect (either by proper
/// crossing or by endpoint/collinear touch).
pub fn segment_intersects_segment_2d(
    a0: [f64; 2],
    a1: [f64; 2],
    b0: [f64; 2],
    b1: [f64; 2],
) -> bool {
    let coords = [a0[0], a0[1], a1[0], a1[1], b0[0], b0[1], b1[0], b1[1]];
    if coords.iter().any(|v| !v.is_finite()) {
        return false;
    }
    let min_a = [a0[0].min(a1[0]), a0[1].min(a1[1])];
    let max_a = [a0[0].max(a1[0]), a0[1].max(a1[1])];
    let min_b = [b0[0].min(b1[0]), b0[1].min(b1[1])];
    let max_b = [b0[0].max(b1[0]), b0[1].max(b1[1])];
    if aabb2_disjoint(min_a, max_a, min_b, max_b) {
        return false;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("seg-inter-2d-ephemeral", 0, scalars) else {
        return false;
    };
    let tolerance = ToleranceContext::default_valid();
    let pair = |offset: usize| [source.leaf(offset).unwrap(), source.leaf(offset + 1).unwrap()];
    let orient = |p: usize, q: usize, r: usize| -> Option<Sign> {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient2d(&mut ctx, pair(p), pair(q), pair(r)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => Some(s),
            _ => None,
        }
    };
    let (Some(o1), Some(o2), Some(o3), Some(o4)) = (
        orient(0, 2, 4),
        orient(0, 2, 6),
        orient(4, 6, 0),
        orient(4, 6, 2),
    ) else {
        return false;
    };
    if o1 != o2 && o3 != o4 && o1 != Sign::Zero && o2 != Sign::Zero && o3 != Sign::Zero && o4 != Sign::Zero {
        return true;
    }
    let on_segment = |p: [f64; 2], s0: [f64; 2], s1: [f64; 2]| -> bool {
        p[0] >= s0[0].min(s1[0])
            && p[0] <= s0[0].max(s1[0])
            && p[1] >= s0[1].min(s1[1])
            && p[1] <= s0[1].max(s1[1])
    };
    (o1 == Sign::Zero && on_segment(b0, a0, a1))
        || (o2 == Sign::Zero && on_segment(b1, a0, a1))
        || (o3 == Sign::Zero && on_segment(a0, b0, b1))
        || (o4 == Sign::Zero && on_segment(a1, b0, b1))
}

/// Exact 2D point-in-triangle classification using adaptive `orient2d` predicates.
/// Returns `Sign::Positive` when `p` is strictly inside triangle `(a, b, c)`,
/// `Sign::Zero` when `p` lies on a boundary edge or vertex, and `Sign::Negative`
/// when `p` is outside (or if `(a, b, c)` is degenerate), independent of whether
/// `(a, b, c)` is ordered counter-clockwise or clockwise.
pub fn point_in_triangle_2d(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Sign {
    let coords = [p[0], p[1], a[0], a[1], b[0], b[1], c[0], c[1]];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Negative;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("pt-in-tri-2d-ephemeral", 0, scalars) else {
        return Sign::Negative;
    };
    let tolerance = ToleranceContext::default_valid();
    let pair = |offset: usize| [source.leaf(offset).unwrap(), source.leaf(offset + 1).unwrap()];
    let orient = |q0: usize, q1: usize, q2: usize| -> Option<Sign> {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient2d(&mut ctx, pair(q0), pair(q1), pair(q2)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => Some(s),
            _ => None,
        }
    };
    let Some(tri_orient) = orient(2, 4, 6) else {
        return Sign::Negative;
    };
    if tri_orient == Sign::Zero {
        return Sign::Negative;
    }
    let (Some(s0), Some(s1), Some(s2)) = (orient(2, 4, 0), orient(4, 6, 0), orient(6, 2, 0)) else {
        return Sign::Negative;
    };
    if [s0, s1, s2].iter().all(|&s| s == tri_orient) {
        return Sign::Positive;
    }
    if [s0, s1, s2]
        .iter()
        .all(|&s| s == tri_orient || s == Sign::Zero)
    {
        return Sign::Zero;
    }
    Sign::Negative
}

/// Exact 3D point-in-tetrahedron classification using adaptive `orient3d` predicates.
/// Returns `Sign::Positive` when `p` is strictly inside tetrahedron `(a, b, c, d)`,
/// `Sign::Zero` when `p` lies on a boundary face, edge, or vertex, and `Sign::Negative`
/// when `p` is outside (or if `(a, b, c, d)` is coplanar/degenerate), independent of
/// the orientation of `(a, b, c, d)`.
pub fn point_in_tetrahedron_3d(
    p: [f64; 3],
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
) -> Sign {
    let coords = [
        p[0], p[1], p[2], a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Negative;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("pt-in-tet-3d-ephemeral", 0, scalars) else {
        return Sign::Negative;
    };
    let tolerance = ToleranceContext::default_valid();
    let pt = |offset: usize| {
        [
            source.leaf(offset).unwrap(),
            source.leaf(offset + 1).unwrap(),
            source.leaf(offset + 2).unwrap(),
        ]
    };
    let orient = |q0: usize, q1: usize, q2: usize, q3: usize| -> Option<Sign> {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient3d(&mut ctx, pt(q0), pt(q1), pt(q2), pt(q3)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => Some(s),
            _ => None,
        }
    };
    let Some(tet_orient) = orient(3, 6, 9, 12) else {
        return Sign::Negative;
    };
    if tet_orient == Sign::Zero {
        return Sign::Negative;
    }
    let (Some(s0), Some(s1), Some(s2), Some(s3)) = (
        orient(0, 6, 9, 12),
        orient(3, 0, 9, 12),
        orient(3, 6, 0, 12),
        orient(3, 6, 9, 0),
    ) else {
        return Sign::Negative;
    };
    if [s0, s1, s2, s3].iter().all(|&s| s == tet_orient) {
        return Sign::Positive;
    }
    if [s0, s1, s2, s3]
        .iter()
        .all(|&s| s == tet_orient || s == Sign::Zero)
    {
        return Sign::Zero;
    }
    Sign::Negative
}

/// Exact 3D segment–triangle intersection classification using adaptive `orient3d` predicates
/// with a fast [`aabb3_disjoint`] pre-rejection filter.
///
/// Returns:
/// - `Sign::Positive` when the open segment `(seg_a, seg_b)` pierces the strict interior of
///   triangle `(tri_a, tri_b, tri_c)`,
/// - `Sign::Zero` when the closed segment `[seg_a, seg_b]` touches the triangle boundary
///   (edge or vertex) or when a segment endpoint lies on the closed triangle,
/// - `Sign::Negative` when the segment and triangle are disjoint (or if the triangle is degenerate).
pub fn segment_triangle_intersect_3d_exact(
    seg_a: [f64; 3],
    seg_b: [f64; 3],
    tri_a: [f64; 3],
    tri_b: [f64; 3],
    tri_c: [f64; 3],
) -> Sign {
    let coords = [
        seg_a[0], seg_a[1], seg_a[2], seg_b[0], seg_b[1], seg_b[2], tri_a[0], tri_a[1], tri_a[2],
        tri_b[0], tri_b[1], tri_b[2], tri_c[0], tri_c[1], tri_c[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) {
        return Sign::Negative;
    }
    let min_seg = [
        seg_a[0].min(seg_b[0]),
        seg_a[1].min(seg_b[1]),
        seg_a[2].min(seg_b[2]),
    ];
    let max_seg = [
        seg_a[0].max(seg_b[0]),
        seg_a[1].max(seg_b[1]),
        seg_a[2].max(seg_b[2]),
    ];
    let min_tri = [
        tri_a[0].min(tri_b[0]).min(tri_c[0]),
        tri_a[1].min(tri_b[1]).min(tri_c[1]),
        tri_a[2].min(tri_b[2]).min(tri_c[2]),
    ];
    let max_tri = [
        tri_a[0].max(tri_b[0]).max(tri_c[0]),
        tri_a[1].max(tri_b[1]).max(tri_c[1]),
        tri_a[2].max(tri_b[2]).max(tri_c[2]),
    ];
    if aabb3_disjoint(min_seg, max_seg, min_tri, max_tri) {
        return Sign::Negative;
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("seg-tri-3d-ephemeral", 0, scalars) else {
        return Sign::Negative;
    };
    let tolerance = ToleranceContext::default_valid();
    let pt = |offset: usize| {
        [
            source.leaf(offset).unwrap(),
            source.leaf(offset + 1).unwrap(),
            source.leaf(offset + 2).unwrap(),
        ]
    };
    let orient = |q0: usize, q1: usize, q2: usize, q3: usize| -> Option<Sign> {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient3d(&mut ctx, pt(q0), pt(q1), pt(q2), pt(q3)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => Some(s),
            _ => None,
        }
    };
    let (Some(oa), Some(ob)) = (orient(6, 9, 12, 0), orient(6, 9, 12, 3)) else {
        return Sign::Negative;
    };
    if oa != Sign::Zero && oa == ob {
        return Sign::Negative;
    }
    let coplanar_pt_in_tri = |p: [f64; 3]| -> Sign {
        let ab = [tri_b[0] - tri_a[0], tri_b[1] - tri_a[1], tri_b[2] - tri_a[2]];
        let ac = [tri_c[0] - tri_a[0], tri_c[1] - tri_a[1], tri_c[2] - tri_a[2]];
        let nx = (ab[1] * ac[2] - ab[2] * ac[1]).abs();
        let ny = (ab[2] * ac[0] - ab[0] * ac[2]).abs();
        let nz = (ab[0] * ac[1] - ab[1] * ac[0]).abs();
        let (i0, i1) = if nx >= ny && nx >= nz {
            (1, 2)
        } else if ny >= nz {
            (2, 0)
        } else {
            (0, 1)
        };
        point_in_triangle_2d(
            [p[i0], p[i1]],
            [tri_a[i0], tri_a[i1]],
            [tri_b[i0], tri_b[i1]],
            [tri_c[i0], tri_c[i1]],
        )
    };
    if oa == Sign::Zero && ob == Sign::Zero {
        let ab = [tri_b[0] - tri_a[0], tri_b[1] - tri_a[1], tri_b[2] - tri_a[2]];
        let ac = [tri_c[0] - tri_a[0], tri_c[1] - tri_a[1], tri_c[2] - tri_a[2]];
        let nx = (ab[1] * ac[2] - ab[2] * ac[1]).abs();
        let ny = (ab[2] * ac[0] - ab[0] * ac[2]).abs();
        let nz = (ab[0] * ac[1] - ab[1] * ac[0]).abs();
        let (i0, i1) = if nx >= ny && nx >= nz {
            (1, 2)
        } else if ny >= nz {
            (2, 0)
        } else {
            (0, 1)
        };
        let sa2 = [seg_a[i0], seg_a[i1]];
        let sb2 = [seg_b[i0], seg_b[i1]];
        let ta2 = [tri_a[i0], tri_a[i1]];
        let tb2 = [tri_b[i0], tri_b[i1]];
        let tc2 = [tri_c[i0], tri_c[i1]];
        if point_in_triangle_2d(sa2, ta2, tb2, tc2) != Sign::Negative
            || point_in_triangle_2d(sb2, ta2, tb2, tc2) != Sign::Negative
            || segment_intersects_segment_2d(sa2, sb2, ta2, tb2)
            || segment_intersects_segment_2d(sa2, sb2, tb2, tc2)
            || segment_intersects_segment_2d(sa2, sb2, tc2, ta2)
        {
            return Sign::Zero;
        }
        return Sign::Negative;
    }
    if oa == Sign::Zero {
        return if coplanar_pt_in_tri(seg_a) != Sign::Negative {
            Sign::Zero
        } else {
            Sign::Negative
        };
    }
    if ob == Sign::Zero {
        return if coplanar_pt_in_tri(seg_b) != Sign::Negative {
            Sign::Zero
        } else {
            Sign::Negative
        };
    }
    // oa and ob have strictly opposite signs: segment straddles the triangle plane.
    let (Some(s0), Some(s1), Some(s2)) = (
        orient(0, 3, 6, 9),
        orient(0, 3, 9, 12),
        orient(0, 3, 12, 6),
    ) else {
        return Sign::Negative;
    };
    // Because orient3d(tri_a, tri_b, tri_c, seg_a) = oa and orient3d(tri_a, tri_b, tri_c, seg_b) = ob = -oa,
    // all three edge-tetrahedra (seg_a, seg_b, e0, e1) must match `ob` (or be Zero on an edge/vertex).
    if [s0, s1, s2].iter().all(|&s| s == ob) {
        return Sign::Positive;
    }
    if [s0, s1, s2].iter().all(|&s| s == ob || s == Sign::Zero) {
        return Sign::Zero;
    }
    Sign::Negative
}

/// Exact adaptive 3D tetrahedron orientation sign alongside the signed determinant magnitude `6 * V`.
///
/// The sign is certified by the exact adaptive `orient3d(a, b, c, d)` predicate, while the
/// scalar `6 * V = ((b - a) × (c - a)) · (d - a)` is evaluated with exact expansion arithmetic
/// (falling back to the binary64 determinant if finite, or `0.0` when exact `orient3d` is `Sign::Zero`).
pub fn tetrahedron_signed_volume_6x(
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
) -> (Sign, f64) {
    let coords = [
        a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2],
    ];
    if coords.iter().any(|v| !v.is_finite()) {
        return (Sign::Zero, 0.0);
    }
    let scalars: Vec<_> = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("tet-vol-6x-ephemeral", 0, scalars) else {
        return (Sign::Zero, 0.0);
    };
    let tolerance = ToleranceContext::default_valid();
    let pt = |offset: usize| {
        [
            source.leaf(offset).unwrap(),
            source.leaf(offset + 1).unwrap(),
            source.leaf(offset + 2).unwrap(),
        ]
    };
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let sign = match orient3d(&mut ctx, pt(0), pt(3), pt(6), pt(9)) {
        Ok(Decision {
            outcome: Outcome::Sign(s),
            ..
        }) => s,
        _ => Sign::Zero,
    };
    if sign == Sign::Zero {
        return (Sign::Zero, 0.0);
    }
    let mut exp_ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let inputs: Vec<Expansion> = coords.iter().map(|&v| Expansion::scalar(v)).collect();
    let det_6v = if let Ok(exp) = expression(Predicate::Orient3, &inputs, &mut exp_ctx)
        && let Ok(interval) = exp.enclosure(&mut exp_ctx)
    {
        let mid = 0.5 * (interval.lo + interval.hi);
        if mid.is_finite() && mid != 0.0 {
            mid
        } else {
            let bx = b[0] - a[0];
            let by = b[1] - a[1];
            let bz = b[2] - a[2];
            let cx = c[0] - a[0];
            let cy = c[1] - a[1];
            let cz = c[2] - a[2];
            let dx = d[0] - a[0];
            let dy = d[1] - a[1];
            let dz = d[2] - a[2];
            bx * (cy * dz - cz * dy) + by * (cz * dx - cx * dz) + bz * (cx * dy - cy * dx)
        }
    } else {
        let bx = b[0] - a[0];
        let by = b[1] - a[1];
        let bz = b[2] - a[2];
        let cx = c[0] - a[0];
        let cy = c[1] - a[1];
        let cz = c[2] - a[2];
        let dx = d[0] - a[0];
        let dy = d[1] - a[1];
        let dz = d[2] - a[2];
        bx * (cy * dz - cz * dy) + by * (cz * dx - cx * dz) + bz * (cx * dy - cy * dx)
    };
    (sign, det_6v)
}

/// Exact 2D barycentric sub-triangle orientation signs `(orient2d(b, c, p), orient2d(c, a, p), orient2d(a, b, p))`
/// for exact topological classification of point `p` with respect to triangle `(a, b, c)`
/// (interior, edge 0/1/2, vertex 0/1/2, or exterior).
pub fn barycentric_in_triangle_2d_exact(
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
    p: [f64; 2],
) -> (Sign, Sign, Sign) {
    let coords = [a[0], a[1], b[0], b[1], c[0], c[1], p[0], p[1]];
    if coords.iter().any(|v| !v.is_finite()) {
        return (Sign::Zero, Sign::Zero, Sign::Zero);
    }
    let scalars = coords
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("bary-tri-2d-ephemeral", 0, scalars) else {
        return (Sign::Zero, Sign::Zero, Sign::Zero);
    };
    let tolerance = ToleranceContext::default_valid();
    let pair = |offset: usize| [source.leaf(offset).unwrap(), source.leaf(offset + 1).unwrap()];
    let orient = |q0: usize, q1: usize, q2: usize| -> Sign {
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        match orient2d(&mut ctx, pair(q0), pair(q1), pair(q2)) {
            Ok(Decision {
                outcome: Outcome::Sign(s),
                ..
            }) => s,
            _ => Sign::Zero,
        }
    };
    (orient(2, 4, 6), orient(4, 0, 6), orient(0, 2, 6))
}
