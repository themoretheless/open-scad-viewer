//! Exact tensor Bernstein signs of a rational Bezier XY Jacobian numerator.
//! Consistent coefficients imply strict orientation in the open chart; this
//! alone proves neither global injectivity nor boundary ownership.
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Expansion, InputError, LeafRef, PredicateContext,
    Reason, Sign, exact_inputs,
};
type Poly = Vec<Vec<Expansion>>;
pub struct Decision {
    pub signs: Option<Vec<Vec<Sign>>>,
    pub reason: Option<Reason>,
    pub work_used: u64,
    pub context: ContextIdentity,
}
fn choose(n: usize, k: usize) -> u64 {
    (0..k).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
}
fn zeros(u: usize, v: usize) -> Poly {
    vec![vec![Expansion::scalar(0.); v]; u]
}
fn power(b: &Poly, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let (p, q) = (b.len() - 1, b[0].len() - 1);
    let mut out = zeros(p + 1, q + 1);
    for i in 0..=p {
        for j in 0..=q {
            for u in i..=p {
                for v in j..=q {
                    if b[i][j].sign() == Sign::Zero {
                        continue;
                    }
                    let scale =
                        choose(p, i) * choose(p - i, u - i) * choose(q, j) * choose(q - j, v - j);
                    let term = b[i][j].mul(&Expansion::integer(scale, ctx)?, ctx)?;
                    out[u][v] = if (u - i + v - j) % 2 == 0 {
                        out[u][v].add(&term, ctx)?
                    } else {
                        out[u][v].sub(&term, ctx)?
                    };
                }
            }
        }
    }
    Ok(out)
}
fn derivative(p: &Poly, axis: usize, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let mut out = zeros(
        p.len() - usize::from(axis == 0),
        p[0].len() - usize::from(axis == 1),
    );
    for (u, row) in out.iter_mut().enumerate() {
        for (v, x) in row.iter_mut().enumerate() {
            let i = u + usize::from(axis == 0);
            let j = v + usize::from(axis == 1);
            *x = p[i][j].mul(
                &Expansion::integer(if axis == 0 { i as u64 } else { j as u64 }, ctx)?,
                ctx,
            )?;
        }
    }
    Ok(out)
}
fn mul(a: &Poly, b: &Poly, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let mut out = zeros(a.len() + b.len() - 1, a[0].len() + b[0].len() - 1);
    for (i, row) in a.iter().enumerate() {
        for (j, x) in row.iter().enumerate() {
            if x.sign() == Sign::Zero {
                continue;
            }
            for (k, row) in b.iter().enumerate() {
                for (l, y) in row.iter().enumerate() {
                    if y.sign() == Sign::Zero {
                        continue;
                    }
                    out[i + k][j + l] = out[i + k][j + l].add(&x.mul(y, ctx)?, ctx)?;
                }
            }
        }
    }
    Ok(out)
}
fn difference(a: Poly, b: Poly, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| a.into_iter().zip(b).map(|(a, b)| a.sub(&b, ctx)).collect())
        .collect()
}
/// Multiply power-to-Bernstein coefficients by a common positive integer
/// denominator. Every factor is exact; there is no rounded division.
fn signs(p: &Poly, ctx: &mut PredicateContext<'_>) -> Result<Vec<Vec<Sign>>, Reason> {
    let (n, m) = (p.len() - 1, p[0].len() - 1);
    let denominators =
        |degree: usize, ctx: &mut PredicateContext<'_>| -> Result<Vec<Expansion>, Reason> {
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
                .map(|k| Expansion::integer(common / choose(degree, k), ctx))
                .collect()
        };
    let du = denominators(n, ctx)?;
    let dv = denominators(m, ctx)?;
    let mut out = vec![vec![Sign::Zero; m + 1]; n + 1];
    for i in 0..=n {
        for j in 0..=m {
            let mut coefficient = Expansion::scalar(0.);
            for k in 0..=i {
                for l in 0..=j {
                    if p[k][l].sign() == Sign::Zero {
                        continue;
                    }
                    let factor = Expansion::integer(choose(i, k) * choose(j, l), ctx)?;
                    let term = p[k][l]
                        .mul(&factor, ctx)?
                        .mul(&du[k], ctx)?
                        .mul(&dv[l], ctx)?;
                    coefficient = coefficient.add(&term, ctx)?;
                }
            }
            out[i][j] = coefficient.sign();
        }
    }
    Ok(out)
}
/// XYZ+weight leaves; caller separately validates positive chart spans,
/// clamped Bezier layout, positive weights and original domain containment.
pub fn rational_surface_projected_jacobian(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    axes: [usize; 2],
) -> Result<Decision, InputError> {
    if !(2..=9).contains(&surface.len())
        || !(2..=9).contains(&surface[0].len())
        || surface.iter().any(|r| r.len() != surface[0].len())
        || axes[0] > 2
        || axes[1] > 2
        || axes[0] == axes[1]
    {
        return Err(InputError::InvalidInput("Invalid projected surface chart"));
    }
    let mut values = surface
        .iter()
        .flatten()
        .flat_map(|p| p.iter())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<Vec<Vec<Sign>>, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        if x[..x.len() - 1]
            .chunks_exact(4)
            .any(|p| p[3].sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        let mut h = (0..3)
            .map(|_| zeros(surface.len(), surface[0].len()))
            .collect::<Vec<_>>();
        for (u, row) in surface.iter().enumerate() {
            for (v, _) in row.iter().enumerate() {
                let at = (u * row.len() + v) * 4;
                let w = &x[at + 3];
                h[0][u][v] = x[at + axes[0]].mul(w, ctx)?;
                h[1][u][v] = x[at + axes[1]].mul(w, ctx)?;
                h[2][u][v] = unit.mul(w, ctx)?;
            }
        }
        let h = h
            .iter()
            .map(|p| power(p, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let u = h
            .iter()
            .map(|p| derivative(p, 0, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let v = h
            .iter()
            .map(|p| derivative(p, 1, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let mut determinant = zeros(3 * (surface.len() - 1), 3 * (surface[0].len() - 1));
        for axis in 0..3 {
            let a = (axis + 1) % 3;
            let b = (axis + 2) % 3;
            let minors = difference(mul(&u[a], &v[b], ctx)?, mul(&u[b], &v[a], ctx)?, ctx)?;
            let term = mul(&h[axis], &minors, ctx)?;
            for (i, row) in term.iter().enumerate() {
                for (j, value) in row.iter().enumerate() {
                    determinant[i][j] = determinant[i][j].add(value, ctx)?;
                }
            }
        }
        let signs = signs(&determinant, ctx)?;
        ctx.charge(0)?;
        Ok(signs)
    })();
    let (signs, reason) = match result {
        Ok(s) => (Some(s), None),
        Err(r) => (None, Some(r)),
    };
    Ok(Decision {
        signs,
        reason,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};

    fn check(points: Vec<Vec<[f64; 4]>>, axes: [usize; 2], work: u64) -> Decision {
        let arena = SourceArena::authored(
            "projected-jacobian-test",
            1,
            points
                .iter()
                .flatten()
                .flatten()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap();
        let leaves = points
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, _)| {
                        std::array::from_fn(|a| arena.leaf((i * row.len() + j) * 4 + a).unwrap())
                    })
                    .collect()
            })
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: work,
                ..Limits::default()
            },
            None,
        );
        rational_surface_projected_jacobian(&mut ctx, &leaves, axes).unwrap()
    }
    fn has_orientation(d: &Decision, wanted: Sign) -> bool {
        d.signs.as_ref().is_some_and(|rows| {
            rows.iter()
                .flatten()
                .all(|s| *s == Sign::Zero || *s == wanted)
                && rows.iter().flatten().any(|s| *s == wanted)
        })
    }
    #[test]
    fn plane_orientation_and_axis_reversal() {
        let p = vec![
            vec![[0., 0., 0., 1.], [0., 1., 0., 1.]],
            vec![[1., 0., 0., 1.], [1., 1., 0., 1.]],
        ];
        assert!(has_orientation(
            &check(p.clone(), [0, 1], 1_000_000),
            Sign::Positive
        ));
        assert!(has_orientation(
            &check(p, [1, 0], 1_000_000),
            Sign::Negative
        ));
    }
    #[test]
    fn collapsed_boundary_and_flat_endpoint_preserve_interior_sign() {
        // (u, u*(2v-v²)): Jacobian 2u*(1-v), zero only on chart boundaries.
        let p = vec![
            vec![[0., 0., 0., 1.]; 3],
            vec![[1., 0., 0., 1.], [1., 1., 0., 1.], [1., 1., 0., 1.]],
        ];
        let d = check(p, [0, 1], 1_000_000);
        assert!(has_orientation(&d, Sign::Positive));
        assert!(d.signs.unwrap().iter().flatten().any(|s| *s == Sign::Zero));
    }
    #[test]
    fn interior_fold_has_both_signs() {
        // (u, u*(v-v²)): orientation reverses at v=1/2.
        let p = vec![
            vec![[0., 0., 0., 1.]; 3],
            vec![[1., 0., 0., 1.], [1., 0.5, 0., 1.], [1., 0., 0., 1.]],
        ];
        let d = check(p, [0, 1], 1_000_000);
        let signs = d.signs.unwrap();
        assert!(signs.iter().flatten().any(|s| *s == Sign::Positive));
        assert!(signs.iter().flatten().any(|s| *s == Sign::Negative));
    }
    #[test]
    fn rational_weights_and_work_refusal() {
        let p = vec![
            vec![[0., 0., 0., 1.], [0., 1., 0., 2.]],
            vec![[1., 0., 0., 1.], [1., 1., 0., 2.]],
        ];
        assert!(has_orientation(
            &check(p.clone(), [0, 1], 1_000_000),
            Sign::Positive
        ));
        let d = check(p, [0, 1], 1);
        assert!(d.signs.is_none());
        assert!(d.reason.is_some());
    }
    #[test]
    fn rank_deficient_projection_has_no_strict_sign() {
        let p = vec![
            vec![[0., 0., 0., 1.], [0., 0., 1., 1.]],
            vec![[1., 0., 0., 1.], [1., 0., 1., 1.]],
        ];
        let d = check(p, [0, 1], 1_000_000);
        assert!(d.signs.unwrap().iter().flatten().all(|s| *s == Sign::Zero));
    }
}
