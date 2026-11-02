//! Exact normalized identity of ORIGINAL rational B-spline tensors.
//! Different degrees, control nets and knot partitions are permitted. Complete
//! common knot rectangles are restricted with a single homogeneous gauge per
//! tensor; Bernstein numerator cross-products then vanish coefficient by
//! coefficient. This supplies no regularity, injectivity, trim or Solid credit.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign,
};
mod blossom;
mod continuity;
pub use continuity::rational_bspline_tensor_knot_continuity;

/// Original Cartesian XYZ and positive weight per control. No rounded knot
/// insertion, degree elevation or caller-authored normalized surrogate belongs
/// here. Each axis has degree 1..8, at most 64 controls and its full knot vector.
#[derive(Clone, Copy)]
pub struct RationalTensorSource<'a> {
    pub degrees: [usize; 2],
    pub controls: &'a [Vec<[LeafRef; 4]>],
    pub knots: [&'a [LeafRef]; 2],
}
impl RationalTensorSource<'_> {
    fn shape(&self) -> Result<[usize; 2], InputError> {
        let n = [
            self.controls.len(),
            self.controls.first().map_or(0, Vec::len),
        ];
        if (0..2).any(|i| {
            !(1..=8).contains(&self.degrees[i])
                || n[i] <= self.degrees[i]
                || n[i] > 64
                || self.knots[i].len() != n[i] + self.degrees[i] + 1
        }) || self.controls.iter().any(|r| r.len() != n[1])
        {
            return Err(InputError::InvalidInput(
                "Invalid original tensor identity basis census",
            ));
        }
        Ok(n)
    }
    fn values(&self, ctx: &PredicateContext<'_>) -> Result<Vec<AuthoredScalar>, InputError> {
        self.controls
            .iter()
            .flatten()
            .flatten()
            .chain(self.knots[0])
            .chain(self.knots[1])
            .map(|&r| ctx.resolve(r).cloned())
            .collect()
    }
}
struct Tensor {
    degrees: [usize; 2],
    points: Vec<Vec<[Expansion; 4]>>,
    knots: [Vec<Expansion>; 2],
}
fn decode(
    values: &[AuthoredScalar],
    n: [usize; 2],
    degrees: [usize; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<Tensor, Reason> {
    let values = values
        .iter()
        .map(|v| match v {
            AuthoredScalar::Binary64Bits(b) => Expansion::scalar(f64::from_bits(*b)),
            _ => unreachable!("validated literal leaves"),
        })
        .collect::<Vec<_>>();
    let mut points = Vec::new();
    for i in 0..n[0] {
        let mut row = Vec::new();
        for j in 0..n[1] {
            let p = &values[4 * (i * n[1] + j)..4 * (i * n[1] + j) + 4];
            if p[3].sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            row.push([
                p[0].mul(&p[3], ctx)?,
                p[1].mul(&p[3], ctx)?,
                p[2].mul(&p[3], ctx)?,
                p[3].clone(),
            ]);
        }
        points.push(row);
    }
    let start = 4 * n[0] * n[1];
    let split = start + n[0] + degrees[0] + 1;
    let knots = [values[start..split].to_vec(), values[split..].to_vec()];
    for axis in 0..2 {
        let k = &knots[axis];
        let p = degrees[axis];
        if k[n[axis]].sub(&k[p], ctx)?.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        for pair in k.windows(2) {
            if pair[1].sub(&pair[0], ctx)?.sign() == Sign::Negative {
                return Err(Reason::MissingProof);
            }
        }
        // Equality on all closed cells extends across original C0 joins.
        for i in p + 1..n[axis] {
            if i + p < n[axis] && k[i + p].sub(&k[i], ctx)?.sign() == Sign::Zero {
                return Err(Reason::MissingProof);
            }
        }
    }
    Ok(Tensor {
        degrees,
        points,
        knots,
    })
}
struct Piece {
    spans: [usize; 2],
    range: [Expansion; 2],
}
fn partition(
    a: &mut Tensor,
    b: &mut Tensor,
    axis: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<Piece>, Reason> {
    let n = [a.points.len(), b.points.len()];
    let n = if axis == 0 {
        n
    } else {
        [a.points[0].len(), b.points[0].len()]
    };
    let p = [a.degrees[axis], b.degrees[axis]];
    let mut k = [Vec::new(), Vec::new()];
    for (i, s) in [&*a, &*b].into_iter().enumerate() {
        let original = &s.knots[axis];
        k[i] = original
            .iter()
            .map(|x| x.sub(&original[p[i]], ctx))
            .collect::<Result<Vec<_>, _>>()?;
        Expansion::normalize_common(&mut k[i], ctx)?;
    }
    // Exact common normalized coordinate t * widthA * widthB. No division,
    // rounded normalized knot or newly authored leaf is introduced.
    let widths = [k[0][n[0]].clone(), k[1][n[1]].clone()];
    for i in 0..2 {
        for x in &mut k[i] {
            *x = x.mul(&widths[1 - i], ctx)?;
        }
    }
    let mut at = p;
    let mut cursor = Expansion::scalar(0.);
    let mut pieces = Vec::new();
    while at[0] < n[0] && at[1] < n[1] {
        ctx.charge(1)?;
        let lo = if k[0][at[0]].sub(&k[1][at[1]], ctx)?.sign() == Sign::Positive {
            &k[0][at[0]]
        } else {
            &k[1][at[1]]
        };
        let hi = if k[0][at[0] + 1].sub(&k[1][at[1] + 1], ctx)?.sign() == Sign::Negative {
            &k[0][at[0] + 1]
        } else {
            &k[1][at[1] + 1]
        };
        if hi.sub(lo, ctx)?.sign() == Sign::Positive {
            if lo.sub(&cursor, ctx)?.sign() != Sign::Zero {
                return Err(Reason::MissingProof);
            }
            pieces.push(Piece {
                spans: at,
                range: [lo.clone(), hi.clone()],
            });
            cursor = hi.clone();
        }
        let order = k[0][at[0] + 1].sub(&k[1][at[1] + 1], ctx)?.sign();
        if order != Sign::Positive {
            at[0] += 1;
        }
        if order != Sign::Negative {
            at[1] += 1;
        }
    }
    if pieces.is_empty() || cursor.sub(&k[0][n[0]], ctx)?.sign() != Sign::Zero {
        return Err(Reason::MissingProof);
    }
    let [ka, kb] = k;
    a.knots[axis] = ka;
    b.knots[axis] = kb;
    Ok(pieces)
}
type Net = Vec<Vec<[Expansion; 4]>>;
fn restrict(
    s: &Tensor,
    u: &Piece,
    v: &Piece,
    which: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Net, Reason> {
    let rows = s
        .points
        .iter()
        .map(|r| r.iter().flatten().cloned().collect())
        .collect::<Vec<_>>();
    let rows = blossom::restrict(
        &rows,
        &s.knots[0],
        s.degrees[0],
        u.spans[which],
        [&u.range[0], &u.range[1]],
        ctx,
    )?;
    let columns = (0..s.points[0].len())
        .map(|j| {
            rows.iter()
                .flat_map(|r| r[4 * j..4 * j + 4].iter().cloned())
                .collect()
        })
        .collect::<Vec<_>>();
    let columns = blossom::restrict(
        &columns,
        &s.knots[1],
        s.degrees[1],
        v.spans[which],
        [&v.range[0], &v.range[1]],
        ctx,
    )?;
    Ok((0..rows.len())
        .map(|i| {
            columns
                .iter()
                .map(|c| std::array::from_fn(|k| c[4 * i + k].clone()))
                .collect()
        })
        .collect())
}
fn compare(a: &Net, b: &Net, ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    let (p, q) = (a.len() - 1, a[0].len() - 1);
    let (r, s) = (b.len() - 1, b[0].len() - 1);
    let choose = crate::binomial_u64;
    for u in 0..=p + r {
        for v in 0..=q + s {
            for axis in 0..3 {
                let mut coefficient = Expansion::scalar(0.);
                for i in u.saturating_sub(r)..=p.min(u) {
                    for j in v.saturating_sub(s)..=q.min(v) {
                        let aa = &a[i][j];
                        let bb = &b[u - i][v - j];
                        let term = aa[axis]
                            .mul(&bb[3], ctx)?
                            .sub(&bb[axis].mul(&aa[3], ctx)?, ctx)?;
                        let factor = Expansion::integer(
                            choose(p, i) * choose(r, u - i) * choose(q, j) * choose(s, v - j),
                            ctx,
                        )?;
                        coefficient = coefficient.add(&term.mul(&factor, ctx)?, ctx)?;
                    }
                }
                // The common positive binomial product for this output coefficient
                // is omitted from BOTH cross-products, never separately normalized.
                if coefficient.sign() != Sign::Zero {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}
/// Equality of complete original maps in independently normalized UV domains.
/// Positive weights and complete C0 knot coverage are checked here; periodic
/// quotient ownership, original regularity and global embedding are separate.
/// Every input leaf must be literal Binary64Bits. The explicit context budget,
/// cancellation, deadline and expansion envelope apply to all failed trials.
pub fn rational_bspline_tensor_normalized_identity(
    ctx: &mut PredicateContext<'_>,
    a: RationalTensorSource<'_>,
    b: RationalTensorSource<'_>,
) -> Result<BezierIdentityDecision, InputError> {
    let an = a.shape()?;
    let bn = b.shape()?;
    // Resolve ALL source references before a shortcut or zero-work refusal.
    let av = a.values(ctx)?;
    let bv = b.values(ctx)?;
    if av
        .iter()
        .chain(&bv)
        .any(|x| !matches!(x, AuthoredScalar::Binary64Bits(_)))
    {
        return Err(InputError::InvalidInput(
            "Tensor identity requires original binary64 leaves",
        ));
    }
    let computed = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge((av.len() + bv.len()) as u64)?;
        let mut a = decode(&av, an, a.degrees, ctx)?;
        let mut b = decode(&bv, bn, b.degrees, ctx)?;
        let u = partition(&mut a, &mut b, 0, ctx)?;
        let v = partition(&mut a, &mut b, 1, ctx)?;
        for u in &u {
            for v in &v {
                ctx.charge(1)?;
                let aa = restrict(&a, u, v, 0, ctx)?;
                let bb = restrict(&b, u, v, 1, ctx)?;
                if !compare(&aa, &bb, ctx)? {
                    return Ok(BezierIdentity::Different);
                }
            }
        }
        Ok(BezierIdentity::Equal)
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(BezierIdentityDecision {
        outcome: computed.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests;
