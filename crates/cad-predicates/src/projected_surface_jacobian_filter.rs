//! Outward interval filter for factorized projected Jacobian coefficients.
//! A bound containing zero is unresolved; it never means an exact zero.
use super::{Decision, choose};
use crate::arithmetic::Interval;
use crate::{Algebra, AuthoredScalar, InputError, LeafRef, PredicateContext, Reason, Sign};
type IPoly = Vec<Vec<Interval>>;
fn integer(value: u64, ctx: &mut PredicateContext<'_>) -> Result<Interval, Reason> {
    Interval::authored(
        &AuthoredScalar::RationalConstant {
            numerator: i64::try_from(value).map_err(|_| Reason::MissingProof)?,
            denominator: 1,
        },
        ctx,
    )
}
fn zeros(u: usize, v: usize) -> IPoly {
    vec![vec![Interval::point(0.); v]; u]
}
// Products retain Bernstein form, scaled by the same positive integer for
// every coefficient. Equal bidegrees therefore share the scale in a minor.
fn factors(degree: usize, ctx: &mut PredicateContext<'_>) -> Result<Vec<Interval>, Reason> {
    let gcd = |mut a: u64, mut b: u64| {
        while b != 0 {
            let r = a % b;
            a = b;
            b = r;
        }
        a
    };
    let mut common = 1u64;
    for k in 0..=degree {
        let c = choose(degree, k);
        common = (common / gcd(common, c))
            .checked_mul(c)
            .ok_or(Reason::MissingProof)?;
    }
    (0..=degree)
        .map(|k| integer(common / choose(degree, k), ctx))
        .collect()
}
fn derivative(p: &IPoly, axis: usize, ctx: &mut PredicateContext<'_>) -> Result<IPoly, Reason> {
    let degree = if axis == 0 {
        p.len() - 1
    } else {
        p[0].len() - 1
    };
    let multiplier = integer(degree as u64, ctx)?;
    let mut out = zeros(
        p.len() - usize::from(axis == 0),
        p[0].len() - usize::from(axis == 1),
    );
    for (u, row) in out.iter_mut().enumerate() {
        for (v, x) in row.iter_mut().enumerate() {
            *x = p[u + usize::from(axis == 0)][v + usize::from(axis == 1)]
                .sub(&p[u][v], ctx)?
                .mul(&multiplier, ctx)?;
        }
    }
    Ok(out)
}
fn mul(
    a: &IPoly,
    b: &IPoly,
    selected_cell: [Option<usize>; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<IPoly, Reason> {
    let degrees = [a.len() + b.len() - 2, a[0].len() + b[0].len() - 2];
    let window =
        std::array::from_fn(|axis| selected_cell[axis].map_or([0, degrees[axis]], |i| [i, i]));
    mul_window(a, b, window, ctx)
}
fn mul_window(
    a: &IPoly,
    b: &IPoly,
    window: [[usize; 2]; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<IPoly, Reason> {
    let (p, q, r, s) = (a.len() - 1, a[0].len() - 1, b.len() - 1, b[0].len() - 1);
    let mut out = zeros(p + r + 1, q + s + 1);
    let fu = factors(p + r, ctx)?;
    let fv = factors(q + s, ctx)?;
    for (i, row) in a.iter().enumerate() {
        for (j, x) in row.iter().enumerate() {
            if x.lo == 0. && x.hi == 0. {
                continue;
            }
            for (k, row) in b.iter().enumerate() {
                for (l, y) in row.iter().enumerate() {
                    if i + k < window[0][0]
                        || i + k > window[0][1]
                        || j + l < window[1][0]
                        || j + l > window[1][1]
                        || (y.lo == 0. && y.hi == 0.)
                    {
                        continue;
                    }
                    let scale = choose(p, i) * choose(q, j) * choose(r, k) * choose(s, l);
                    let term = x.mul(y, ctx)?.mul(&integer(scale, ctx)?, ctx)?;
                    out[i + k][j + l] = out[i + k][j + l].add(&term, ctx)?;
                }
            }
        }
    }
    for (i, row) in out.iter_mut().enumerate() {
        for (j, x) in row.iter_mut().enumerate() {
            *x = x.mul(&fu[i], ctx)?.mul(&fv[j], ctx)?;
        }
    }
    Ok(out)
}
fn difference(a: IPoly, b: IPoly, ctx: &mut PredicateContext<'_>) -> Result<IPoly, Reason> {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| a.into_iter().zip(b).map(|(a, b)| a.sub(&b, ctx)).collect())
        .collect()
}

/// Whole coefficient bounds of the exact factorized chart, computed by outward
/// interval arithmetic. Unresolved signs require an exact fallback. Positive
/// pivot cancellation and chart validation remain caller obligations.
pub fn factorized_surface_projected_jacobian_filter(
    ctx: &mut PredicateContext<'_>,
    points: &[Vec<[LeafRef; 3]>],
    rows: &[LeafRef],
    columns: &[LeafRef],
    axes: [usize; 2],
) -> Result<Decision, InputError> {
    if !(2..=9).contains(&points.len())
        || rows.len() != points.len()
        || !(2..=9).contains(&columns.len())
        || points.iter().any(|r| r.len() != columns.len())
        || axes[0] > 2
        || axes[1] > 2
        || axes[0] == axes[1]
    {
        return Err(InputError::InvalidInput(
            "Invalid factorized interval chart",
        ));
    }
    let values = points
        .iter()
        .flatten()
        .flatten()
        .chain(rows)
        .chain(columns)
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let result = (|| -> Result<Vec<Vec<[f64; 2]>>, Reason> {
        let x = values
            .iter()
            .map(|v| Interval::authored(v, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let count = rows.len() * columns.len() * 3;
        if x[count..].iter().any(|w| w.lo <= 0. || !w.hi.is_finite()) {
            return Err(Reason::MissingProof);
        }
        let mut h = (0..3)
            .map(|_| zeros(rows.len(), columns.len()))
            .collect::<Vec<_>>();
        for u in 0..rows.len() {
            for v in 0..columns.len() {
                let at = (u * columns.len() + v) * 3;
                let weight = x[count + u].mul(&x[count + rows.len() + v], ctx)?;
                if weight.lo <= 0. || !weight.hi.is_finite() {
                    return Err(Reason::PrecisionExhausted);
                }
                h[0][u][v] = x[at + axes[0]].sub(&x[axes[0]], ctx)?.mul(&weight, ctx)?;
                h[1][u][v] = x[at + axes[1]].sub(&x[axes[1]], ctx)?.mul(&weight, ctx)?;
                h[2][u][v] = weight;
            }
        }
        let u = h
            .iter()
            .map(|p| derivative(p, 0, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let v = h
            .iter()
            .map(|p| derivative(p, 1, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let mut determinant = zeros(3 * (rows.len() - 1), 3 * (columns.len() - 1));
        for axis in 0..3 {
            let a = (axis + 1) % 3;
            let b = (axis + 2) % 3;
            let minor = difference(
                mul(&u[a], &v[b], [None, None], ctx)?,
                mul(&u[b], &v[a], [None, None], ctx)?,
                ctx,
            )?;
            let term = mul(&h[axis], &minor, [None, None], ctx)?;
            for (i, row) in term.iter().enumerate() {
                for (j, value) in row.iter().enumerate() {
                    determinant[i][j] = determinant[i][j].add(value, ctx)?;
                }
            }
        }
        if determinant
            .iter()
            .flatten()
            .any(|x| !x.lo.is_finite() || !x.hi.is_finite())
        {
            return Err(Reason::PrecisionExhausted);
        }
        ctx.charge(0)?;
        Ok(determinant
            .into_iter()
            .map(|row| row.into_iter().map(|x| [x.lo, x.hi]).collect())
            .collect())
    })();
    let (bounds, reason) = match result {
        Ok(b) => (Some(b), None),
        Err(r) => (None, Some(r)),
    };
    let signs = bounds.as_ref().and_then(|b| {
        b.iter()
            .map(|row| {
                row.iter()
                    .map(|x| {
                        if x[0] > 0. {
                            Some(Sign::Positive)
                        } else if x[1] < 0. {
                            Some(Sign::Negative)
                        } else {
                            None
                        }
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .collect::<Option<Vec<_>>>()
    });
    Ok(Decision {
        signs,
        coefficient_bounds: bounds,
        coefficient_scale: None,
        reason,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn compare(points: Vec<Vec<[f64; 3]>>, factors: [Vec<f64>; 2], axes: [usize; 2]) {
        let n = points.len();
        let m = points[0].len();
        let arena = SourceArena::authored(
            "interval-jacobian-test",
            1,
            points
                .iter()
                .flatten()
                .flatten()
                .chain(&factors[0])
                .chain(&factors[1])
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let points: Vec<Vec<_>> = (0..n)
            .map(|u| {
                (0..m)
                    .map(|v| std::array::from_fn(|a| arena.leaf((u * m + v) * 3 + a).unwrap()))
                    .collect()
            })
            .collect();
        let rows: Vec<_> = (0..n).map(|u| arena.leaf(n * m * 3 + u).unwrap()).collect();
        let columns: Vec<_> = (0..m)
            .map(|v| arena.leaf(n * m * 3 + n + v).unwrap())
            .collect();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let exact = super::super::factorized_surface_projected_jacobian(
            &mut ctx, &points, &rows, &columns, axes, None,
        )
        .unwrap()
        .signs
        .unwrap();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let filtered =
            factorized_surface_projected_jacobian_filter(&mut ctx, &points, &rows, &columns, axes)
                .unwrap();
        let bounds = filtered.coefficient_bounds.unwrap();
        assert_eq!(bounds.len(), exact.len());
        let mut strict = 0;
        for (u, row) in bounds.iter().enumerate() {
            for (v, b) in row.iter().enumerate() {
                assert!(b[0] <= b[1]);
                if b[0] > 0. {
                    assert_eq!(exact[u][v], Sign::Positive);
                    strict += 1;
                } else if b[1] < 0. {
                    assert_eq!(exact[u][v], Sign::Negative);
                    strict += 1;
                }
                if exact[u][v] == Sign::Zero {
                    assert!(b[0] <= 0. && b[1] >= 0.);
                }
            }
        }
        assert!(strict > 0);
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: 1,
                ..Limits::default()
            },
            None,
        );
        let denied =
            factorized_surface_projected_jacobian_filter(&mut ctx, &points, &rows, &columns, axes)
                .unwrap();
        assert!(denied.coefficient_bounds.is_none() && denied.signs.is_none());
    }
    #[test]
    fn outward_filter_agrees_with_exact_signs_and_never_rounds_zero() {
        for axes in [[0, 1], [1, 0]] {
            compare(
                vec![
                    vec![[128., -256., 0.], [128., -255., 0.]],
                    vec![[129., -256., 0.], [129., -255., 0.]],
                ],
                [vec![1., 1.1], vec![1., 1.1]],
                axes,
            );
            compare(
                vec![
                    vec![[0., 0., 0.], [0., 0., 0.], [0., 0., 0.]],
                    vec![[1., 0., 0.], [1., 1., 0.], [1., 1., 0.]],
                ],
                [vec![1., 1.1], vec![1., 0.75, 1.]],
                axes,
            );
            compare(
                vec![
                    vec![[0., 0., 0.], [1., 1., 0.]],
                    vec![[1., 0., 0.], [0., 1., 0.]],
                ],
                [vec![1., 1.1], vec![1., 1.1]],
                axes,
            );
        }
    }
}
