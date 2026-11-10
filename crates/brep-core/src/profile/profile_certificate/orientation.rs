//! Exact Green orientation independently of chord replacement. Rational range
//! bounds are rounded *outward* to dyadic rationals by integer division, keeping
//! a bounded denominator when thousands of cells are summed.
use super::*;
use num_bigint::BigInt;
use num_traits::Signed;
#[derive(Clone)]
pub(super) struct Range {
    pub lo: Q,
    pub hi: Q,
}
impl Range {
    fn hull(mut values: impl Iterator<Item = Q>) -> Self {
        let first = values.next().unwrap();
        let mut lo = first.clone();
        let mut hi = first;
        for v in values {
            if v < lo {
                lo = v;
            } else if v > hi {
                hi = v;
            }
        }
        Self { lo, hi }
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
    let bound = cross.lo.abs().max(cross.hi.abs());
    let (lo, hi) = arithmetic::rational_area_bounds(net, bound)?;
    Ok(Range {
        lo: dyadic(&lo, false),
        hi: dyadic(&hi, true),
    })
}
pub(super) fn bounds(loops: &[Vec<Curve>], depth: u8) -> Result<Vec<Range>> {
    let extracted = extraction::nets(loops)?;
    let rational = extracted
        .iter()
        .filter(|(_, net)| net.iter().any(|h| h[2] != net[0][2]))
        .count();
    if depth > 12 || rational * (1usize << depth) > 8192 {
        return Err(refused("Exact Green orientation exceeds its work budget"));
    }
    let mut ranges = Vec::new();
    for (wire_index, wire) in loops.iter().enumerate() {
        let origin = std::array::from_fn(|k| Q::from_float(wire[0].control_points[0][k]).unwrap());
        let (mut lo, mut hi) = (Q::zero(), Q::zero());
        for (_, source_net) in extracted.iter().filter(|(w, _)| *w == wire_index) {
            let mut net = source_net.clone();
            if net.iter().all(|h| h[2] == net[0][2]) || net.len() == 2 {
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
        ranges.push(Range { lo, hi });
    }
    Ok(ranges)
}

pub(super) fn check(loops: &[Vec<Curve>], depth: u8) -> Result<Vec<i8>> {
    bounds(loops, depth)?
        .into_iter()
        .map(|r| {
            if r.lo > Q::zero() {
                Ok(1)
            } else if r.hi < Q::zero() {
                Ok(-1)
            } else {
                Err(refused("Exact Green orientation interval contains zero"))
            }
        })
        .collect()
}
