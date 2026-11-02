//! Exact quadratic graph jets at an ORIGINAL collapsed Bezier boundary.
//! This algebra alone supplies no graph, regularity, G2 or body authority.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, ContextIdentity, Expansion, InputError, LeafRef,
    PredicateContext, Reason, Sign, exact_inputs,
};
type P = Vec<Expansion>;
pub struct PoleQuadraticJet {
    dropped_axis: usize,
    gradient: [Expansion; 2],
    linear_den: Expansion,
    hessian: [Expansion; 3],
    quadratic_den: Expansion,
    bounds: [[f64; 2]; 5],
}
impl PoleQuadraticJet {
    pub fn dropped_axis(&self) -> usize {
        self.dropped_axis
    }
    pub fn coefficient_bounds(&self) -> [[f64; 2]; 5] {
        self.bounds
    }
}
pub struct PoleQuadraticJetDecision {
    pub jet: Option<PoleQuadraticJet>,
    pub outcome: BezierIdentity,
    pub work_used: u64,
    pub context: ContextIdentity,
}
fn zero() -> Expansion {
    Expansion::scalar(0.)
}
fn choose(n: usize, k: usize) -> u64 {
    (0..k).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
}
fn to_power(b: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let n = b.len() - 1;
    let mut out = vec![zero(); b.len()];
    for (i, a) in b.iter().enumerate() {
        for k in i..=n {
            let t = a.mul(
                &Expansion::integer(choose(n, i) * choose(n - i, k - i), ctx)?,
                ctx,
            )?;
            out[k] = if (k - i) % 2 == 0 {
                out[k].add(&t, ctx)?
            } else {
                out[k].sub(&t, ctx)?
            };
        }
    }
    Ok(out)
}
fn add(a: &P, b: &P, subtract: bool, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    (0..a.len().max(b.len()))
        .map(|i| {
            let z = zero();
            let x = a.get(i).unwrap_or(&z);
            let y = b.get(i).unwrap_or(&z);
            if subtract {
                x.sub(y, ctx)
            } else {
                x.add(y, ctx)
            }
        })
        .collect()
}
fn scale(a: &P, b: &Expansion, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    a.iter().map(|x| x.mul(b, ctx)).collect()
}
fn mul(a: &P, b: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let mut out = vec![zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] = out[i + j].add(&x.mul(y, ctx)?, ctx)?;
        }
    }
    Ok(out)
}
fn eval(a: &P, t: f64, ctx: &mut PredicateContext<'_>) -> Result<Expansion, Reason> {
    let mut out = zero();
    for x in a.iter().rev() {
        out = out.mul(&Expansion::scalar(t), ctx)?.add(x, ctx)?;
    }
    Ok(out)
}
fn det(m: &[[Expansion; 3]; 3], ctx: &mut PredicateContext<'_>) -> Result<Expansion, Reason> {
    let mut r = zero();
    for (a, b, c, positive) in [
        (0, 1, 2, true),
        (1, 2, 0, true),
        (2, 0, 1, true),
        (2, 1, 0, false),
        (1, 0, 2, false),
        (0, 2, 1, false),
    ] {
        let x = m[0][a].mul(&m[1][b], ctx)?.mul(&m[2][c], ctx)?;
        r = if positive {
            r.add(&x, ctx)?
        } else {
            r.sub(&x, ctx)?
        };
    }
    Ok(r)
}
fn positive_pair(data: &mut [Expansion], ctx: &mut PredicateContext<'_>) -> Result<(), Reason> {
    let sign = data.last().unwrap().sign();
    if sign == Sign::Zero {
        return Err(Reason::MissingProof);
    }
    if sign == Sign::Negative {
        for x in data.iter_mut() {
            *x = zero().sub(x, ctx)?;
        }
    }
    Expansion::reduce_projective(data, ctx)?;
    Expansion::normalize_common(data, ctx)
}
fn ratio_bound(
    n: &Expansion,
    d: &Expansion,
    ctx: &mut PredicateContext<'_>,
) -> Result<[f64; 2], Reason> {
    let mut v = [n.clone(), d.clone()];
    positive_pair(&mut v, ctx)?;
    let r = v[0]
        .enclosure(ctx)?
        .div_nonzero(&v[1].enclosure(ctx)?, ctx)?;
    Ok([r.lo, r.hi])
}
/// Rows are inward normalized U, columns original normalized V. The caller
/// permutes/reverses indices exactly when another natural boundary is used.
/// Returned coefficients are gx,gy,Hxx,Hxy,Hyy in cyclic world coordinates
/// x=(drop+1)%3,y=(drop+2)%3,z=drop, relative to the original pole point.
/// Coefficients are solved as exact rational expansions, never rounded fits.
pub fn rational_surface_pole_quadratic_jet(
    ctx: &mut PredicateContext<'_>,
    s: &[Vec<[LeafRef; 4]>],
    drop: usize,
) -> Result<PoleQuadraticJetDecision, InputError> {
    if drop > 2
        || !(2..=9).contains(&s.len())
        || s.first().is_none_or(|r| !(2..=9).contains(&r.len()))
        || s.iter().any(|r| r.len() != s[0].len())
    {
        return Err(InputError::InvalidInput(
            "Choose an original degree 1..8 pole tensor and world graph axis",
        ));
    }
    let mut values = s
        .iter()
        .flatten()
        .flatten()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<Option<PoleQuadraticJet>, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        let p = s.len() - 1;
        let q = s[0].len() - 1;
        if x[..x.len() - 1]
            .chunks_exact(4)
            .any(|v| v[3].sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        for v in 0..=q {
            for k in 0..3 {
                if x[4 * v + k].sub(&x[k], ctx)?.sign() != Sign::Zero {
                    return Err(Reason::MissingProof);
                }
            }
        }
        let axes = [(drop + 1) % 3, (drop + 2) % 3, drop];
        let mut w0 = Vec::new();
        let mut h1: [P; 3] = std::array::from_fn(|_| Vec::new());
        let mut h2 = h1.clone();
        let degree = Expansion::integer(p as u64, ctx)?;
        let twice = Expansion::scalar(2.);
        let second = Expansion::integer(choose(p, 2.min(p)), ctx)?;
        for v in 0..=q {
            w0.push(unit.mul(&x[4 * v + 3], ctx)?);
            for k in 0..3 {
                let row = |u: usize, ctx: &mut PredicateContext<'_>| -> Result<Expansion, Reason> {
                    let at = (u * (q + 1) + v) * 4;
                    x[at + axes[k]].sub(&x[axes[k]], ctx)?.mul(&x[at + 3], ctx)
                };
                let a = row(1, ctx)?;
                h1[k].push(a.mul(&degree, ctx)?);
                h2[k].push(if p >= 2 {
                    row(2, ctx)?
                        .sub(&a.mul(&twice, ctx)?, ctx)?
                        .mul(&second, ctx)?
                } else {
                    zero()
                });
            }
        }
        w0 = to_power(&w0, ctx)?;
        for k in 0..3 {
            h1[k] = to_power(&h1[k], ctx)?;
            h2[k] = to_power(&h2[k], ctx)?;
        }
        let a = [
            eval(&h1[0], 0., ctx)?,
            eval(&h1[1], 0., ctx)?,
            eval(&h1[2], 0., ctx)?,
        ];
        let b = [
            eval(&h1[0], 1., ctx)?,
            eval(&h1[1], 1., ctx)?,
            eval(&h1[2], 1., ctx)?,
        ];
        let mut linear = [
            a[2].mul(&b[1], ctx)?.sub(&a[1].mul(&b[2], ctx)?, ctx)?,
            a[0].mul(&b[2], ctx)?.sub(&a[2].mul(&b[0], ctx)?, ctx)?,
            a[0].mul(&b[1], ctx)?.sub(&a[1].mul(&b[0], ctx)?, ctx)?,
        ];
        positive_pair(&mut linear, ctx)?;
        let [gx, gy, gd] = linear;
        let residual = |h: &[P; 3], ctx: &mut PredicateContext<'_>| -> Result<P, Reason> {
            let z = scale(&h[2], &gd, ctx)?;
            let x = scale(&h[0], &gx, ctx)?;
            let y = scale(&h[1], &gy, ctx)?;
            add(&add(&z, &x, true, ctx)?, &y, true, ctx)
        };
        if residual(&h1, ctx)?.iter().any(|x| x.sign() != Sign::Zero) {
            return Ok(None);
        }
        let rhs = scale(&mul(&residual(&h2, ctx)?, &w0, ctx)?, &twice, ctx)?;
        let terms = [
            scale(&mul(&h1[0], &h1[0], ctx)?, &gd, ctx)?,
            scale(&mul(&h1[0], &h1[1], ctx)?, &gd.mul(&twice, ctx)?, ctx)?,
            scale(&mul(&h1[1], &h1[1], ctx)?, &gd, ctx)?,
        ];
        let mut matrix: [[Expansion; 3]; 3] =
            std::array::from_fn(|_| std::array::from_fn(|_| zero()));
        let mut r = [zero(), zero(), zero()];
        for (i, t) in [0., 0.5, 1.].iter().enumerate() {
            for k in 0..3 {
                matrix[i][k] = eval(&terms[k], *t, ctx)?;
            }
            r[i] = eval(&rhs, *t, ctx)?;
        }
        let denominator = det(&matrix, ctx)?;
        let mut quad = Vec::new();
        for k in 0..3 {
            let mut m = matrix.clone();
            for i in 0..3 {
                m[i][k] = r[i].clone();
            }
            quad.push(det(&m, ctx)?);
        }
        quad.push(denominator);
        positive_pair(&mut quad, ctx)?;
        let mut check = scale(&rhs, &quad[3], ctx)?;
        for k in 0..3 {
            check = add(&check, &scale(&terms[k], &quad[k], ctx)?, true, ctx)?;
        }
        if check.iter().any(|x| x.sign() != Sign::Zero) {
            return Ok(None);
        }
        let mut bounds = [[0.; 2]; 5];
        bounds[0] = ratio_bound(&gx, &gd, ctx)?;
        bounds[1] = ratio_bound(&gy, &gd, ctx)?;
        for k in 0..3 {
            bounds[k + 2] = ratio_bound(&quad[k], &quad[3], ctx)?;
        }
        Ok(Some(PoleQuadraticJet {
            dropped_axis: drop,
            gradient: [gx, gy],
            linear_den: gd,
            hessian: std::array::from_fn(|k| quad[k].clone()),
            quadratic_den: quad[3].clone(),
            bounds,
        }))
    })();
    let (jet, outcome) = match result {
        Ok(Some(j)) => (Some(j), BezierIdentity::Equal),
        Ok(None) => (None, BezierIdentity::Different),
        Err(e) => (None, BezierIdentity::Indeterminate(e)),
    };
    Ok(PoleQuadraticJetDecision {
        jet,
        outcome,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
/// Compare values carried by two private exact jet objects, including their
/// gradients. No rounded coefficient enclosure can authorize equality.
pub fn compare_pole_quadratic_jets(
    ctx: &mut PredicateContext<'_>,
    a: &PoleQuadraticJet,
    b: &PoleQuadraticJet,
) -> Result<bool, Reason> {
    if a.dropped_axis != b.dropped_axis {
        return Err(Reason::MissingProof);
    }
    for k in 0..2 {
        if a.gradient[k]
            .mul(&b.linear_den, ctx)?
            .sub(&b.gradient[k].mul(&a.linear_den, ctx)?, ctx)?
            .sign()
            != Sign::Zero
        {
            return Ok(false);
        }
    }
    for k in 0..3 {
        if a.hessian[k]
            .mul(&b.quadratic_den, ctx)?
            .sub(&b.hessian[k].mul(&a.quadratic_den, ctx)?, ctx)?
            .sign()
            != Sign::Zero
        {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
