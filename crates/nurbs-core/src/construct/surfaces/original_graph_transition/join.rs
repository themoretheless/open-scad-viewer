//! Whole-rim geometric G2 using exact coefficient differences and correlated
//! interval finite differences. Pointwise equality is never used as authority.
use super::curvature::{derivative, enclose};
use super::*;
use crate::distance_bounds::Interval as I;
use std::cell::Cell;
#[derive(Debug)]
pub struct JoinReport {
    pub within: bool,
    pub position_exact: bool,
    pub normal_difference_upper: Option<f64>,
    pub tensor_difference_upper: Option<f64>,
    pub coefficient_work: usize,
    pub reason: &'static str,
}
struct Counter {
    used: Cell<usize>,
    limit: usize,
}
impl Counter {
    fn charge(&self) -> Result<()> {
        check(
            self.used.get() < self.limit,
            "Correlated join budget exhausted",
        )?;
        self.used.set(self.used.get() + 1);
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct P<'a> {
    a: I,
    d: I,
    c: &'a Counter,
}
impl<'a> P<'a> {
    fn constant(x: f64, c: &'a Counter) -> Self {
        Self {
            a: I::point(x),
            d: I::point(0.),
            c,
        }
    }
    fn add(self, b: Self) -> Result<Self> {
        self.c.charge()?;
        Ok(Self {
            a: self.a.add(b.a)?,
            d: self.d.add(b.d)?,
            ..self
        })
    }
    fn sub(self, b: Self) -> Result<Self> {
        self.c.charge()?;
        Ok(Self {
            a: self.a.sub(b.a)?,
            d: self.d.sub(b.d)?,
            ..self
        })
    }
    fn mul(self, b: Self) -> Result<Self> {
        self.c.charge()?;
        Ok(Self {
            a: self.a.mul(b.a)?,
            d: self
                .d
                .mul(b.a)?
                .add(self.a.mul(b.d)?)?
                .add(self.d.mul(b.d)?)?,
            ..self
        })
    }
    fn div(self, b: Self) -> Result<Self> {
        self.c.charge()?;
        let other = b.a.add(b.d)?;
        check(
            b.a.lo > 0. && other.lo > 0.,
            "Join denominator is not proved positive",
        )?;
        Ok(Self {
            a: self.a.div(b.a)?,
            d: self
                .d
                .mul(b.a)?
                .sub(self.a.mul(b.d)?)?
                .div(b.a.mul(other)?)?,
            ..self
        })
    }
    fn sqrt(self) -> Result<Self> {
        self.c.charge()?;
        let other = self.a.add(self.d)?;
        check(
            self.a.lo > 0. && other.lo > 0.,
            "Join square root is not proved regular",
        )?;
        let sqrt = |x: I| I::new(x.lo.sqrt().next_down().max(0.), x.hi.sqrt().next_up());
        let a = sqrt(self.a)?;
        let b = sqrt(other)?;
        Ok(Self {
            a,
            d: self.d.div(a.add(b)?)?,
            ..self
        })
    }
}
fn boundary(p: &Patch, end: bool, b: &mut Budget) -> Result<Vec<Vec<[Q; 3]>>> {
    let u = derivative(&p.coefficients, 0, b)?;
    let v = derivative(&p.coefficients, 1, b)?;
    let uu = derivative(&u, 0, b)?;
    let uv = derivative(&u, 1, b)?;
    let vv = derivative(&v, 1, b)?;
    let d = p.width.domain();
    let hi = b.authored(d[1])?;
    let lo = b.authored(d[0])?;
    let span = b.keep(hi - lo)?;
    let mut out = Vec::new();
    for (net, order) in [(&u, 1), (&v, 0), (&uu, 2), (&uv, 1), (&vv, 0)] {
        let row = &net[if end { net.len() - 1 } else { 0 }];
        let factor = b.keep(Q::one() / span.pow(order))?;
        let mut values = Vec::new();
        for p in row {
            let mut q = std::array::from_fn(|_| Q::zero());
            for k in 0..3 {
                q[k] = b.keep(&p[k] * &factor)?;
            }
            values.push(q);
        }
        out.push(values);
    }
    // This specialized graph chart is justified by actual exact jets, not a
    // floating resemblance to X=U or a guessed inverse parameterization.
    for (i, net) in out.iter().enumerate() {
        for p in net {
            check(
                p[0] == if i == 0 { Q::one() } else { Q::zero() },
                "Join physical X clock changed",
            )?;
            if i == 4 {
                check(p[1].is_zero(), "Join transverse Y is not linear")?;
            }
        }
    }
    Ok(out)
}
fn hull(values: &[I]) -> Result<I> {
    I::new(
        values.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
        values
            .iter()
            .map(|x| x.hi)
            .fold(f64::NEG_INFINITY, f64::max),
    )
}
fn paired<'a>(
    a: &[Vec<[Q; 3]>],
    b: &[Vec<[Q; 3]>],
    budget: &mut Budget,
    c: &'a Counter,
) -> Result<[[P<'a>; 3]; 5]> {
    let zero = P::constant(0., c);
    let mut jets = [[zero; 3]; 5];
    for i in 0..5 {
        check(a[i].len() == b[i].len(), "Join transverse degrees differ")?;
        for k in 0..3 {
            let base = a[i]
                .iter()
                .map(|p| enclose(&p[k], budget))
                .collect::<Result<Vec<_>>>()?;
            let delta = a[i]
                .iter()
                .zip(&b[i])
                .map(|(p, q)| {
                    let d = budget.keep(&q[k] - &p[k])?;
                    enclose(&d, budget)
                })
                .collect::<Result<Vec<_>>>()?;
            jets[i][k] = P {
                a: hull(&base)?,
                d: hull(&delta)?,
                c,
            };
        }
    }
    Ok(jets)
}
fn shape<'a>(j: [[P<'a>; 3]; 5], c: &'a Counter) -> Result<([P<'a>; 3], [[P<'a>; 3]; 3])> {
    let zero = P::constant(0., c);
    let one = P::constant(1., c);
    let two = P::constant(2., c);
    let gy = j[1][2].div(j[1][1])?;
    let gx = j[0][2].sub(gy.mul(j[0][1])?)?;
    let l = zero.sub(j[0][1])?.div(j[1][1])?;
    let m = one.div(j[1][1])?;
    let ruu = j[2][2].sub(gy.mul(j[2][1])?)?;
    let ruv = j[3][2].sub(gy.mul(j[3][1])?)?;
    let rvv = j[4][2];
    let hxx = ruu.add(two.mul(l)?.mul(ruv)?)?.add(l.mul(l)?.mul(rvv)?)?;
    let hxy = m.mul(ruv)?.add(l.mul(m)?.mul(rvv)?)?;
    let hyy = m.mul(m)?.mul(rvv)?;
    let determinant = one.add(gx.mul(gx)?)?.add(gy.mul(gy)?)?;
    let norm = determinant.sqrt()?;
    let normal = [
        zero.sub(gx)?.div(norm)?,
        zero.sub(gy)?.div(norm)?,
        one.div(norm)?,
    ];
    let off = zero.sub(gx.mul(gy)?)?.div(determinant)?;
    let inv = [
        [one.add(gy.mul(gy)?)?.div(determinant)?, off],
        [off, one.add(gx.mul(gx)?)?.div(determinant)?],
    ];
    let second = [
        [hxx.div(norm)?, hxy.div(norm)?],
        [hxy.div(norm)?, hyy.div(norm)?],
    ];
    let mut coefficients = [[zero; 2]; 2];
    for a in 0..2 {
        for b in 0..2 {
            for r in 0..2 {
                for s in 0..2 {
                    coefficients[a][b] =
                        coefficients[a][b].add(inv[a][r].mul(second[r][s])?.mul(inv[s][b])?)?;
                }
            }
        }
    }
    let vectors = [[one, zero, gx], [zero, one, gy]];
    let mut tensor = [[zero; 3]; 3];
    for a in 0..3 {
        for b in 0..3 {
            for r in 0..2 {
                for s in 0..2 {
                    tensor[a][b] = tensor[a][b]
                        .add(vectors[r][a].mul(coefficients[r][s])?.mul(vectors[s][b])?)?;
                }
            }
        }
    }
    Ok((normal, tensor))
}
fn abs(x: I) -> f64 {
    x.lo.abs().max(x.hi.abs())
}
impl Patch {
    /// Exact C0 and correlated whole-rim normal/tensor differences in world
    /// coordinates. Independent field curvature/solid proofs remain mandatory.
    pub fn inspect_join_g2(
        &self,
        next: &Patch,
        normal_tolerance: f64,
        tensor_tolerance: f64,
        budget: &mut Budget,
    ) -> Result<JoinReport> {
        check(
            normal_tolerance.is_finite()
                && normal_tolerance > 0.
                && tensor_tolerance.is_finite()
                && tensor_tolerance > 0.,
            "Choose positive finite join tolerances",
        )?;
        let start = budget.used();
        let mut report = JoinReport {
            within: false,
            position_exact: false,
            normal_difference_upper: None,
            tensor_difference_upper: None,
            coefficient_work: 0,
            reason: "unproven",
        };
        let checked = (|| -> Result<(f64, f64)> {
            report.position_exact = self.joins_position_exact(next, budget)?;
            check(report.position_exact, "Exact join position does not agree")?;
            let a = boundary(self, true, budget)?;
            let b = boundary(next, false, budget)?;
            // The paired jets only allocate coefficients; the fixed graph
            // expression below has a separately charged bounded evaluation.
            let dummy = Counter {
                used: Cell::new(0),
                limit: usize::MAX,
            };
            let jets = paired(&a, &b, budget, &dummy)?;
            let counter = Counter {
                used: Cell::new(0),
                limit: budget.limit - budget.used,
            };
            let jets = jets.map(|r| r.map(|p| P { c: &counter, ..p }));
            let outcome = shape(jets, &counter);
            budget.charge(counter.used.get())?;
            let (normal, tensor) = outcome?;
            let mut n = I::point(0.);
            let mut t = 0f64;
            for p in normal {
                n = n.add(I::point(abs(p.d)))?;
            }
            for row in tensor {
                let mut sum = I::point(0.);
                for p in row {
                    sum = sum.add(I::point(abs(p.d)))?;
                }
                t = t.max(sum.hi);
            }
            Ok((n.hi, t))
        })();
        report.coefficient_work = budget.used() - start;
        match checked {
            Ok((n, t)) => {
                report.normal_difference_upper = Some(n);
                report.tensor_difference_upper = Some(t);
                report.within = n <= normal_tolerance && t <= tensor_tolerance;
                report.reason = if report.within {
                    "complete-correlated-original-two-jet-bound"
                } else {
                    "difference-bound-not-within-tolerance"
                };
            }
            Err(_) => {
                report.reason = "position-budget-or-arithmetic-unproven";
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn correlated_world_shape_matches_existing_kernel_for_nontrivial_jets() {
        let a = [
            [1., 0.25, 0.5],
            [0., 0.5, 0.25],
            [0., 0.125, 0.75],
            [0., 0.25, 0.125],
            [0., 0., -0.5],
        ];
        let mut b = a;
        b[0][1] += 0.03125;
        b[2][2] += 0.0625;
        b[3][2] -= 0.03125;
        let c = Counter {
            used: Cell::new(0),
            limit: 10000,
        };
        let jets = std::array::from_fn(|i| {
            std::array::from_fn(|k| P {
                a: I::point(a[i][k]),
                d: I::point(b[i][k] - a[i][k]),
                c: &c,
            })
        });
        let (normal, tensor) = shape(jets, &c).unwrap();
        let reference = |values: [[f64; 3]; 5]| {
            let mut j = [[[I::point(0.); 3]; 4]; 4];
            for (p, (u, v)) in values
                .into_iter()
                .zip([(1, 0), (0, 1), (2, 0), (1, 1), (0, 2)])
            {
                j[u][v] = p.map(I::point);
            }
            crate::surface_shape_operator::from_graph_jets(&j, ([0, 1, 2], [0., 0.]))
                .unwrap()
                .unwrap()
        };
        let first = reference(a);
        let second = reference(b);
        let agrees = |pair: P, a: I, b: I| {
            let delta = b.sub(a).unwrap();
            assert!(pair.a.lo <= a.hi && a.lo <= pair.a.hi);
            assert!(
                pair.d.lo <= delta.hi && delta.lo <= pair.d.hi,
                "{:?} {:?}",
                pair.d,
                delta
            );
            assert!(pair.d.hi - pair.d.lo < 1e-10);
        };
        for i in 0..3 {
            agrees(normal[i], first.normal[i], second.normal[i]);
            for k in 0..3 {
                agrees(tensor[i][k], first.tensor[i][k], second.tensor[i][k]);
            }
        }
        assert!(c.used.get() < 10000);
    }
}
