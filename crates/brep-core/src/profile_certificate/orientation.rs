//! Exact Green orientation independently of chord replacement. Rational range
//! bounds are rounded *outward* to dyadic rationals by integer division, keeping
//! a bounded denominator when thousands of cells are summed.
use super::*;
use num_bigint::BigInt;
use num_traits::Signed;
#[derive(Clone)]
struct Range {
    lo: Q,
    hi: Q,
}
impl Range {
    fn hull(values: impl Iterator<Item = Q>) -> Self {
        let v: Vec<_> = values.collect();
        Self {
            lo: v.iter().min().unwrap().clone(),
            hi: v.iter().max().unwrap().clone(),
        }
    }
    fn mul(&self, b: &Self) -> Self {
        Self::hull(
            [
                &self.lo * &b.lo,
                &self.lo * &b.hi,
                &self.hi * &b.lo,
                &self.hi * &b.hi,
            ]
            .into_iter(),
        )
    }
    fn sub(&self, b: &Self) -> Self {
        Self {
            lo: &self.lo - &b.hi,
            hi: &self.hi - &b.lo,
        }
    }
}
fn dyadic(q: &Q, up: bool) -> Q {
    let denominator = BigInt::one() << 256usize;
    let numerator = q.numer() * &denominator;
    let mut integer = &numerator / q.denom();
    if &numerator % q.denom() != BigInt::zero() {
        if up && numerator.is_positive() {
            integer += 1;
        }
        if !up && numerator.is_negative() {
            integer -= 1;
        }
    }
    Q::new(integer, denominator)
}
fn green(net: &[[Q; 3]]) -> Result<Range> {
    let h: Vec<_> = (0..3)
        .map(|k| Range::hull(net.iter().map(|p| p[k].clone())))
        .collect();
    if h[2].lo <= Q::zero() {
        return Err(refused("Exact orientation denominator is not positive"));
    }
    let n = Q::from_integer((net.len() - 1).into());
    let d: Vec<_> = (0..2)
        .map(|k| Range::hull(net.windows(2).map(|p| (&p[1][k] - &p[0][k]) * &n)))
        .collect();
    let cross = h[0].mul(&d[1]).sub(&h[1].mul(&d[0]));
    let half = Q::new(1.into(), 2.into());
    let reciprocal = Range {
        lo: &half / (&h[2].hi * &h[2].hi),
        hi: &half / (&h[2].lo * &h[2].lo),
    };
    let r = cross.mul(&reciprocal);
    Ok(Range {
        lo: dyadic(&r.lo, false),
        hi: dyadic(&r.hi, true),
    })
}
pub(super) fn check(loops: &[Vec<Curve>], depth: u8) -> Result<Vec<i8>> {
    let rational = loops
        .iter()
        .flatten()
        .filter(|c| c.weights.iter().any(|w| *w != c.weights[0]))
        .count();
    if depth > 12 || rational * (1usize << depth) > 8192 {
        return Err(refused("Exact Green orientation exceeds its work budget"));
    }
    let mut signs = Vec::new();
    for wire in loops {
        let origin = std::array::from_fn(|k| Q::from_float(wire[0].control_points[0][k]).unwrap());
        let (mut lo, mut hi) = (Q::zero(), Q::zero());
        for c in wire {
            let mut net: Vec<[Q; 3]> = c
                .control_points
                .iter()
                .zip(&c.weights)
                .map(|(p, w)| {
                    let w = Q::from_float(*w).unwrap();
                    [
                        Q::from_float(p[0]).unwrap() * &w,
                        Q::from_float(p[1]).unwrap() * &w,
                        w,
                    ]
                })
                .collect();
            if c.weights.iter().all(|w| *w == c.weights[0]) || c.degree == 1 {
                let area = arithmetic::polynomial_area(&net, &origin);
                lo += &area;
                hi += area;
            } else {
                for h in &mut net {
                    h[0] -= &origin[0] * &h[2];
                    h[1] -= &origin[1] * &h[2];
                }
                let mut nets = vec![net];
                for _ in 0..depth {
                    nets = nets
                        .into_iter()
                        .flat_map(|n| {
                            let (a, b) = split(&n);
                            [a, b]
                        })
                        .collect();
                }
                for n in nets {
                    let range = green(&n)?;
                    lo += range.lo;
                    hi += range.hi;
                }
            }
        }
        if lo > Q::zero() {
            signs.push(1);
        } else if hi < Q::zero() {
            signs.push(-1);
        } else {
            return Err(refused("Exact Green orientation interval contains zero"));
        }
    }
    Ok(signs)
}
