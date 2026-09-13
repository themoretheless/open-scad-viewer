//! Candidate finite predicates over explicit authored source coordinates.
//!
//! This crate has no B-rep dependency and issues no solid/operation certificate.
//! Authored admission is a trusted boundary: callers must not relabel rounded
//! construction results as authored input. Opaque references prevent accidental
//! cross-source reuse; they do not authenticate arbitrary caller assertions.
mod arithmetic;
mod construction;
mod construction3;
mod context;
mod recipe_codec3;
pub use recipe_codec3::*;

use arithmetic::{Approx, Expansion, Interval};
pub use construction::*;
pub use construction3::*;
pub use context::*;

pub const IMPLEMENTATION_VERSION: &str = "cad-predicates-candidate-10";

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
