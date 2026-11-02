//! Exact predicates on ORIGINAL 3D points projected orthogonally to a chosen
//! nonzero direction. No rounded projected point is admitted as source data.
use crate::{
    Algebra, Decision, Expansion, InputError, LeafRef, Outcome, PredicateContext, Reason, Sign,
    Stage,
};
type E = [Expansion; 3];
fn cross(a: &E, b: &E, c: &mut PredicateContext<'_>) -> Result<E, Reason> {
    Ok([
        a[1].mul(&b[2], c)?.sub(&a[2].mul(&b[1], c)?, c)?,
        a[2].mul(&b[0], c)?.sub(&a[0].mul(&b[2], c)?, c)?,
        a[0].mul(&b[1], c)?.sub(&a[1].mul(&b[0], c)?, c)?,
    ])
}
fn delta(a: &E, b: &E, c: &mut PredicateContext<'_>) -> Result<E, Reason> {
    Ok([
        a[0].sub(&b[0], c)?,
        a[1].sub(&b[1], c)?,
        a[2].sub(&b[2], c)?,
    ])
}
fn dot(a: &E, b: &E, c: &mut PredicateContext<'_>) -> Result<Expansion, Reason> {
    let mut r = Expansion::scalar(0.);
    for k in 0..3 {
        r = r.add(&a[k].mul(&b[k], c)?, c)?;
    }
    Ok(r)
}
fn evaluate(
    ctx: &mut PredicateContext<'_>,
    points: &[[LeafRef; 3]],
    normal: [LeafRef; 3],
    kind: u8,
) -> Result<Decision, InputError> {
    let refs = points
        .iter()
        .flatten()
        .chain(normal.iter())
        .copied()
        .collect::<Vec<_>>();
    let values = refs
        .into_iter()
        .map(|r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let computed = (|| -> Result<Sign, Reason> {
        let values = crate::exact_inputs(&values, ctx)?;
        let p = values
            .chunks_exact(3)
            .map(|p| [p[0].clone(), p[1].clone(), p[2].clone()])
            .collect::<Vec<_>>();
        let n = &p[points.len()];
        let value = match kind {
            0 => dot(
                &cross(&delta(&p[1], &p[0], ctx)?, &delta(&p[2], &p[0], ctx)?, ctx)?,
                n,
                ctx,
            )?,
            1 => dot(
                &cross(&delta(&p[1], &p[0], ctx)?, n, ctx)?,
                &cross(&delta(&p[3], &p[2], ctx)?, n, ctx)?,
                ctx,
            )?,
            _ => {
                let mut sum = Expansion::scalar(0.);
                for i in 1..points.len() - 1 {
                    sum = sum.add(
                        &dot(
                            &cross(
                                &delta(&p[i], &p[0], ctx)?,
                                &delta(&p[i + 1], &p[0], ctx)?,
                                ctx,
                            )?,
                            n,
                            ctx,
                        )?,
                        ctx,
                    )?;
                }
                sum
            }
        };
        ctx.charge(0)?;
        Ok(value.sign())
    })();
    Ok(Decision {
        outcome: match computed {
            Ok(s) => Outcome::Sign(s),
            Err(r) => Outcome::Indeterminate(r),
        },
        stage: Stage::ExactExpansion,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
/// Sign of n·((b-a)×(c-a)). The projection direction is an operator parameter,
/// not an assertion that a rounded frame was constructed exactly from points.
pub fn projected_orient3d(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 3],
    b: [LeafRef; 3],
    c: [LeafRef; 3],
    normal: [LeafRef; 3],
) -> Result<Decision, InputError> {
    evaluate(ctx, &[a, b, c], normal, 0)
}
/// Sign of ((b-a)×n)·((d-c)×n); multiplying the true projected dot product by
/// |n|² preserves its sign. Used for exact projected between/overlap tests.
pub fn projected_direction_dot3d(
    ctx: &mut PredicateContext<'_>,
    a: [LeafRef; 3],
    b: [LeafRef; 3],
    c: [LeafRef; 3],
    d: [LeafRef; 3],
    normal: [LeafRef; 3],
) -> Result<Decision, InputError> {
    evaluate(ctx, &[a, b, c, d], normal, 1)
}
/// Signed projected polygon area, without relabeling rounded projected points.
pub fn projected_polygon_orientation(
    ctx: &mut PredicateContext<'_>,
    points: &[[LeafRef; 3]],
    normal: [LeafRef; 3],
) -> Result<Decision, InputError> {
    if !(3..=512).contains(&points.len()) {
        return Err(InputError::InvalidInput(
            "Choose 3..512 original projected polygon vertices",
        ));
    }
    evaluate(ctx, points, normal, 2)
}
