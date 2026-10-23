//! Exact discriminant admission, outward location, no midpoint assumption.
use super::{ExactEndpoints, project};
use crate::{Result, curve::Curve, distance_bounds::Interval as I};
pub(super) enum QuadraticProof {
    Excluded,
    DoubleRoot([f64; 2]),
}
pub(super) fn inspect(
    c: &Curve,
    span: usize,
    d: [f64; 2],
    n: [f64; 3],
    o: f64,
    exact: &mut ExactEndpoints<'_>,
) -> Option<QuadraticProof> {
    if c.degree != 2
        || d != [c.knots[span], c.knots[span + 1]]
        || c.knots[span - 1] != d[0]
        || c.knots[span + 2] != d[1]
    {
        return None;
    }
    let calculate = || -> Result<Option<QuadraticProof>> {
        let scale = c.weights[span - 2..=span]
            .iter()
            .copied()
            .fold(0_f64, f64::max);
        let mut h = Vec::new();
        for i in span - 2..=span {
            let p: Vec<_> = c.control_points[i].iter().map(|x| I::point(*x)).collect();
            h.push(project(&p, n, o)?.mul(I::point(c.weights[i]).div(I::point(scale))?)?);
        }
        let positive = h[0].lo > 0. && h[1].hi < 0. && h[2].lo > 0.;
        let negative = h[0].hi < 0. && h[1].lo > 0. && h[2].hi < 0.;
        if !positive && !negative {
            return Ok(None);
        }
        match exact.discriminant(span) {
            Some(cad_predicates::Sign::Negative) => Ok(Some(QuadraticProof::Excluded)),
            Some(cad_predicates::Sign::Zero) => {
                if negative {
                    for v in &mut h {
                        *v = I::new(-v.hi, -v.lo)?;
                    }
                }
                let q = h[0].div(h[0].sub(h[1])?)?.intersect(0., 1.)?;
                let t = I::point(d[0])
                    .add(I::point(d[1]).sub(I::point(d[0]))?.mul(q)?)?
                    .intersect(d[0], d[1])?;
                Ok(Some(QuadraticProof::DoubleRoot([t.lo, t.hi])))
            }
            _ => Ok(None),
        }
    };
    let mut calculate = calculate;
    calculate().ok().flatten()
}
