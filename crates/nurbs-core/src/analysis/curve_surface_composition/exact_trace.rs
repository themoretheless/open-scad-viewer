//! Exact preprocessing of authored binary64 spline coefficients. In particular,
//! subtract a narrow chart origin BEFORE conversion back to binary64 intervals.
//! This module issues no geometric certificate by itself.
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};

pub(super) struct Work {
    pub used: usize,
    pub limit: usize,
}
impl Work {
    fn operations(&mut self, n: usize) -> Result<()> {
        if n > self.limit.saturating_sub(self.used) {
            self.used = self.limit;
            return Err(crate::numeric_err(
                "Exact trace coefficient budget exhausted",
            ));
        }
        self.used += n;
        Ok(())
    }
    fn charge(&mut self, q: &Q) -> Result<()> {
        self.operations(1)?;
        check(
            q.numer().bits() <= 8192 && q.denom().bits() <= 8192,
            "Exact trace coefficient bit budget exhausted",
        )
    }
}
fn authored(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::numeric_err("Non-finite exact trace input"))
}
/// Conversion is verified against the rational itself, rather than assuming
/// that a library's nearest-float conversion has a particular rounding mode.
fn enclosure(q: &Q) -> Result<I> {
    let x = q
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| crate::numeric_err("Exact trace outside binary64 range"))?;
    if authored(x)? == *q {
        return Ok(I::point(x));
    }
    let mut lo = x;
    let mut hi = x;
    for _ in 0..4 {
        if authored(lo)? > *q {
            lo = lo.next_down();
        }
        if authored(hi)? < *q {
            hi = hi.next_up();
        }
        if lo.is_finite() && hi.is_finite() && authored(lo)? <= *q && authored(hi)? >= *q {
            return I::new(lo, hi);
        }
    }
    Err(crate::numeric_err(
        "Could not enclose exact trace coefficient",
    ))
}
type Grid = Vec<Vec<[Q; 4]>>;
struct Jet {
    degrees: [usize; 2],
    knots: [Vec<f64>; 2],
    poles: Grid,
}
/// All homogeneous derivatives, projected onto the exact integer orthogonal
/// chart (x-y, x+y-2z, x+y+z). Projection precedes differencing.
type Net = Vec<Vec<[I; 4]>>;
pub(super) struct Support {
    jets: Vec<Jet>,
    degrees: [usize; 2],
    nets: std::cell::RefCell<std::collections::BTreeMap<(usize, usize, usize), Net>>,
}
const ORDERS: [[usize; 2]; 6] = [[0, 0], [1, 0], [0, 1], [2, 0], [1, 1], [0, 2]];
impl Support {
    pub(super) fn hulls(&self) -> Result<Vec<[I; 4]>> {
        self.jets
            .iter()
            .map(|jet| {
                let mut out = [I::point(0.); 4];
                for k in 0..4 {
                    let mut lo = f64::INFINITY;
                    let mut hi = f64::NEG_INFINITY;
                    for row in &jet.poles {
                        for cp in row {
                            let i = enclosure(&cp[k])?;
                            lo = lo.min(i.lo);
                            hi = hi.max(i.hi);
                        }
                    }
                    out[k] = I::new(lo, hi)?;
                }
                Ok(out)
            })
            .collect()
    }
    pub(super) fn new(s: &crate::surface::Surface, w: &mut Work) -> Result<Self> {
        s.validate()?;
        check(
            !s.periodic_u && !s.periodic_v && s.degree_u <= 12 && s.degree_v <= 12,
            "Exact graph support degree or periodicity unsupported",
        )?;
        check(
            s.control_points
                .len()
                .saturating_mul(s.control_points[0].len())
                <= 4096,
            "Exact graph support pole budget exhausted",
        )?;
        let mut poles = Vec::new();
        for (i, row) in s.control_points.iter().enumerate() {
            let mut output = Vec::new();
            for (j, p) in row.iter().enumerate() {
                let weight = authored(s.weights[i][j])?;
                let x = authored(p[0])?;
                let y = authored(p[1])?;
                let z = authored(p[2])?;
                let cp = [
                    (&x - &y) * &weight,
                    (&x + &y - Q::from_integer(2.into()) * &z) * &weight,
                    (x + y + z) * &weight,
                    weight,
                ];
                for q in &cp {
                    w.charge(q)?;
                }
                output.push(cp);
            }
            poles.push(output);
        }
        let degrees = [s.degree_u, s.degree_v];
        let mut jets = Vec::new();
        for orders in ORDERS {
            let mut jet = Jet {
                degrees,
                knots: [s.knots_u.clone(), s.knots_v.clone()],
                poles: poles.clone(),
            };
            for axis in 0..2 {
                for _ in 0..orders[axis] {
                    jet = differentiate(jet, axis, w)?;
                }
            }
            jets.push(jet);
        }
        Ok(Self {
            jets,
            degrees,
            nets: std::cell::RefCell::new(Default::default()),
        })
    }
    /// Each derivative uses the same homogeneous pcurve denominator power.
    /// Thus rational source jet quotient formulas can cancel it algebraically.
    pub(super) fn compose(
        &self,
        pc: &Curve,
        range: [f64; 2],
        spans: [usize; 2],
        w: &mut Work,
    ) -> Result<Vec<Vec<Vec<I>>>> {
        let root = &self.jets[0];
        let domains =
            std::array::from_fn(|a| [root.knots[a][spans[a]], root.knots[a][spans[a] + 1]]);
        let h = pcurve(pc, range, domains, w)?;
        let mut weight_powers = vec![vec![I::point(1.)]];
        for n in 1..=4 {
            w.operations(12 * weight_powers[n - 1].len() * h[2].len())?;
            weight_powers.push(super::mul(&weight_powers[n - 1], &h[2])?);
        }
        let mut output = Vec::new();
        for (index, jet) in self.jets.iter().enumerate() {
            let [a, b] = ORDERS[index];
            let zero = a > self.degrees[0] || b > self.degrees[1];
            if zero {
                output.push(vec![
                    vec![
                        I::point(0.);
                        pc.degree * (self.degrees[0] + self.degrees[1]) + 1
                    ];
                    4
                ]);
                continue;
            }
            // Precharge an upper bound for every interval convolution and
            // coefficient accumulation in lift_coefficients. Big-rational
            // preprocessing is charged separately, operation by operation.
            let n = pc.degree;
            let [du, dv] = jet.degrees;
            let mut cost = 0usize;
            for d in [du, dv] {
                for r in 1..=d {
                    cost = cost.saturating_add(24 * (n * (r - 1) + 1) * (n + 1));
                }
                for r in 0..=d {
                    cost = cost.saturating_add(12 * (n * r + 1) * (n * (d - r) + 1));
                }
            }
            cost = cost.saturating_add(
                (du + 1) * (dv + 1) * (12 * (n * du + 1) * (n * dv + 1) + 8 * (n * (du + dv) + 1)),
            );
            cost = cost.saturating_add(4 * 12 * (n * (du + dv) + 1) * (n * (a + b) + 1));
            w.operations(cost)?;
            let span = [spans[0] - a, spans[1] - b];
            let key = (index, span[0], span[1]);
            let cached = self.nets.borrow().get(&key).cloned();
            let net = if let Some(net) = cached {
                net
            } else {
                let u = extraction(&jet.knots[0], jet.degrees[0], span[0], w)?;
                let v = extraction(&jet.knots[1], jet.degrees[1], span[1], w)?;
                let mut net = vec![vec![[I::point(0.); 4]; v.len()]; u.len()];
                for i in 0..u.len() {
                    for j in 0..v.len() {
                        for k in 0..4 {
                            let mut q = Q::zero();
                            for r in 0..u.len() {
                                for t in 0..v.len() {
                                    q += &u[i][r]
                                        * &v[j][t]
                                        * &jet.poles[span[0] - jet.degrees[0] + r]
                                            [span[1] - jet.degrees[1] + t][k];
                                    w.charge(&q)?;
                                }
                            }
                            net[i][j][k] = enclosure(&q)?;
                        }
                    }
                }
                // The cache belongs to this immutable authored support, never to
                // a receipt or a rounded construction proxy. Eviction drops only
                // an optimization; the next visit recomputes original coefficients.
                let mut cache = self.nets.borrow_mut();
                if cache.len() >= 256 {
                    cache.clear();
                }
                cache.insert(key, net.clone());
                net
            };
            let composed = super::lift_coefficients(&h, &net, jet.degrees, [[0., 1.], [0., 1.]])?;
            output.push(
                composed
                    .iter()
                    .map(|poly| super::mul(poly, &weight_powers[a + b]))
                    .collect::<Result<Vec<_>>>()?,
            );
        }
        Ok(output)
    }
}
fn differentiate(jet: Jet, axis: usize, w: &mut Work) -> Result<Jet> {
    let p = jet.degrees[axis];
    if p == 0 {
        let mut jet = jet;
        for row in &mut jet.poles {
            for cp in row {
                *cp = std::array::from_fn(|_| Q::zero());
            }
        }
        return Ok(jet);
    }
    let n = jet.poles.len() - usize::from(axis == 0);
    let m = jet.poles[0].len() - usize::from(axis == 1);
    let mut poles = vec![vec![std::array::from_fn(|_| Q::zero()); m]; n];
    for i in 0..n {
        for j in 0..m {
            let index = if axis == 0 { i } else { j };
            let den =
                authored(jet.knots[axis][index + p + 1])? - authored(jet.knots[axis][index + 1])?;
            // Disconnected supports are rejected by the caller's continuity gate.
            check(!den.is_zero(), "Disconnected exact derivative support")?;
            for k in 0..4 {
                poles[i][j][k] = (&jet.poles[i + usize::from(axis == 0)]
                    [j + usize::from(axis == 1)][k]
                    - &jet.poles[i][j][k])
                    * Q::from_integer(p.into())
                    / &den;
                w.charge(&poles[i][j][k])?;
            }
        }
    }
    let mut degrees = jet.degrees;
    degrees[axis] -= 1;
    let mut knots = jet.knots;
    knots[axis] = knots[axis][1..knots[axis].len() - 1].to_vec();
    Ok(Jet {
        degrees,
        knots,
        poles,
    })
}
fn mix(a: &Q, b: &Q, t: &Q, w: &mut Work) -> Result<Q> {
    let q = a + t * (b - a);
    w.charge(&q)?;
    Ok(q)
}
fn blossom(mut row: Vec<Q>, arguments: &[Q], w: &mut Work) -> Result<Q> {
    check(
        row.len() == arguments.len() + 1,
        "Invalid exact blossom degree",
    )?;
    for t in arguments {
        row = row
            .windows(2)
            .map(|p| mix(&p[0], &p[1], t, w))
            .collect::<Result<Vec<_>>>()?;
    }
    Ok(row.remove(0))
}

/// Restrict a clamped Bezier pcurve to normalized traversal [lo,hi], then
/// express it in the surface knot cell. Original poles/weights are untouched.
pub(super) fn pcurve(
    c: &Curve,
    range: [f64; 2],
    cell: [[f64; 2]; 2],
    w: &mut Work,
) -> Result<Vec<Vec<I>>> {
    c.validate()?;
    let p = c.degree;
    check(
        c.control_points.len() == p + 1
            && p <= 16
            && !c.periodic
            && c.control_points[0].len() == 2
            && c.knots[..=p].iter().all(|x| *x == c.knots[p])
            && c.knots[p + 1..].iter().all(|x| *x == c.knots[p + 1]),
        "Exact trace requires a clamped Bezier pcurve",
    )?;
    check(
        range[0] >= 0. && range[1] <= 1. && range[0] < range[1],
        "Invalid exact trace traversal range",
    )?;
    let ends = [authored(range[0])?, authored(range[1])?];
    let mut h: Vec<Vec<Q>> = vec![Vec::new(); 3];
    for (i, cp) in c.control_points.iter().enumerate() {
        let weight = authored(c.weights[i])?;
        for k in 0..2 {
            h[k].push(authored(cp[k])? * &weight);
        }
        h[2].push(weight);
    }
    let mut restricted = vec![Vec::new(); 3];
    for j in 0..=p {
        let args: Vec<_> = (0..p)
            .map(|i| ends[usize::from(i >= p - j)].clone())
            .collect();
        for k in 0..3 {
            restricted[k].push(blossom(h[k].clone(), &args, w)?);
        }
    }
    for k in 0..2 {
        let a = authored(cell[k][0])?;
        let width = authored(cell[k][1])? - &a;
        check(width > Q::zero(), "Empty exact trace chart cell")?;
        for j in 0..=p {
            restricted[k][j] = (&restricted[k][j] - &a * &restricted[2][j]) / &width;
            w.charge(&restricted[k][j])?;
        }
    }
    restricted
        .iter()
        .map(|row| row.iter().map(enclosure).collect())
        .collect()
}

/// Exact Bezier extraction matrix for one original B-spline span. Rows are
/// output Bezier poles, columns are the p+1 original active coefficients.
pub(super) fn extraction(
    knots: &[f64],
    p: usize,
    span: usize,
    w: &mut Work,
) -> Result<Vec<Vec<Q>>> {
    check(
        p <= 24 && span >= p && span + p + 1 < knots.len() && knots[span] < knots[span + 1],
        "Invalid exact spline span",
    )?;
    let k = knots
        .iter()
        .copied()
        .map(authored)
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::new();
    for r in 0..=p {
        let args: Vec<_> = (0..p)
            .map(|i| k[span + usize::from(i >= p - r)].clone())
            .collect();
        let mut d: Vec<Vec<Q>> = (0..=p)
            .map(|i| {
                (0..=p)
                    .map(|j| if i == j { Q::one() } else { Q::zero() })
                    .collect()
            })
            .collect();
        for level in 1..=p {
            for j in (level..=p).rev() {
                let left = span - p + j;
                let den = &k[span + 1 + j - level] - &k[left];
                check(!den.is_zero(), "Singular exact spline extraction")?;
                let alpha = (&args[level - 1] - &k[left]) / den;
                for i in 0..=p {
                    d[j][i] = mix(&d[j - 1][i], &d[j][i], &alpha, w)?;
                }
            }
        }
        out.push(d.remove(p));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn work() -> Work {
        Work {
            used: 0,
            limit: 100_000,
        }
    }
    #[test]
    fn directed_conversion_including_subnormal_and_rational_thirds() {
        for q in [
            Q::new(1.into(), 3.into()),
            -Q::new(1.into(), 3.into()),
            authored(f64::from_bits(1)).unwrap() / Q::from_integer(2.into()),
            Q::zero(),
        ] {
            let i = enclosure(&q).unwrap();
            assert!(authored(i.lo).unwrap() <= q && authored(i.hi).unwrap() >= q);
            assert!(i.hi - i.lo <= 2. * i.hi.abs().max(f64::MIN_POSITIVE) * f64::EPSILON);
        }
    }
    #[test]
    fn exact_narrow_cell_subtraction_retains_zero_and_one() {
        let a = 0.5f64;
        let b = a.next_up();
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![a, a], vec![b, b]],
            weights: vec![1., 1.],
            periodic: false,
        };
        let h = pcurve(&c, [0., 1.], [[a, b], [a, b]], &mut work()).unwrap();
        for k in 0..2 {
            assert_eq!(h[k], vec![I::point(0.), I::point(1.)]);
        }
    }
    #[test]
    fn extraction_of_quadratic_c2_span_is_partition_of_unity() {
        let k = [0., 0., 0., 0.3, 0.8, 1., 1., 1.];
        for span in 2..5 {
            let matrix = extraction(&k, 2, span, &mut work()).unwrap();
            for row in matrix {
                assert_eq!(row.iter().cloned().sum::<Q>(), Q::one());
                assert!(row.iter().all(|x| *x >= Q::zero()));
            }
        }
    }
    #[test]
    fn resource_exhaustion_does_not_return_coefficients() {
        let mut w = Work { used: 0, limit: 1 };
        assert!(extraction(&[0., 0., 0., 1., 1., 1.], 2, 2, &mut w).is_err());
        assert!(w.used <= w.limit);
    }
    #[test]
    fn composed_original_multispan_derivatives_reproduce_quadratic() {
        // S=(u,v,u^2), quadratic clamped u with one simple knot.
        // u^2 has B-spline coefficients product of its two Greville knots.
        let knots = vec![0., 0., 0., 0.25, 1., 1., 1.];
        let gu = [0., 0.125, 0.625, 1.];
        let z = [0., 0., 0.25, 1.];
        let s = crate::surface::Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: knots,
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..4)
                .map(|i| (0..2).map(|j| vec![gu[i], j as f64, z[i]]).collect())
                .collect(),
            weights: vec![vec![1.; 2]; 4],
            periodic_u: false,
            periodic_v: false,
        };
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0.1, 0.25], vec![0.2, 0.75]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let mut w = work();
        let support = Support::new(&s, &mut w).unwrap();
        let hulls = support.hulls().unwrap();
        assert_eq!(hulls[3][2], I::point(2.));
        let jets = support.compose(&c, [0., 1.], [2, 1], &mut w).unwrap();
        let initial_work = w.used;
        let replay = support.compose(&c, [0., 1.], [2, 1], &mut w).unwrap();
        assert_eq!(
            jets, replay,
            "A cached span must retain original coefficient enclosures"
        );
        assert!(
            w.used > initial_work,
            "Cache hits still consume composition work"
        );
        let mut exhausted = Work {
            used: w.used,
            limit: w.used,
        };
        assert!(
            support
                .compose(&c, [0., 1.], [2, 1], &mut exhausted)
                .is_err()
        );
        // Z=x+y+z, so Z_uu=2 and Z_uv=Z_vv=0, W=1.
        for i in &jets[3][2] {
            assert!(i.lo <= 2. && i.hi >= 2., "{i:?}");
        }
        for index in [4, 5] {
            for k in 0..4 {
                for i in &jets[index][k] {
                    assert!(i.lo <= 0. && i.hi >= 0.);
                }
            }
        }
        // An independent evaluation of composed first derivative at endpoints.
        for (endpoint, u) in [0.1, 0.2].into_iter().enumerate() {
            let i = if endpoint == 0 {
                0
            } else {
                jets[1][2].len() - 1
            };
            assert!(jets[1][2][i].lo <= 1. + 2. * u && jets[1][2][i].hi >= 1. + 2. * u);
        }
    }
}
