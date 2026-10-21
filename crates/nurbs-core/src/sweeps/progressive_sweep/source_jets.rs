//! Immutable original homogeneous XYZ jets with one shared weight recurrence.
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
#[derive(Clone, Debug)]
struct Span {
    domain: [f64; 2],
    anchor: [f64; 3],
    fields: [Vec<[I; 4]>; 4],
}
#[derive(Clone, Debug)]
pub(super) struct SourceJets {
    domain: [f64; 2],
    spans: Vec<Span>,
    order: usize,
}
// Only fields through `order` are certified; higher slots are placeholders.
pub(super) struct Report {
    pub cells: usize,
    pub jets: Option<[[[f64; 2]; 3]; 4]>,
    pub single_span: bool,
    pub order: usize,
}
fn derivative(values: &[[I; 4]], degree: usize, width: I) -> Result<Vec<[I; 4]>> {
    if degree == 0 {
        return Ok(vec![[I::point(0.); 4]]);
    }
    let factor = I::point(degree as f64).div(width)?;
    values
        .windows(2)
        .map(|p| {
            let mut value = [I::point(0.); 4];
            for k in 0..4 {
                value[k] = p[1][k].sub(p[0][k])?.mul(factor)?;
            }
            Ok(value)
        })
        .collect()
}
fn restrict(values: &[[I; 4]], local: I) -> Result<[I; 4]> {
    let degree = values.len() - 1;
    let mut hull = [I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 4];
    let mut controls = [[I::point(0.); 4]; 33];
    for pole in 0..=degree {
        controls[..=degree].copy_from_slice(values);
        for r in 1..=degree {
            let t = I::point(if r - 1 < degree - pole {
                local.lo
            } else {
                local.hi
            });
            let beta = I::point(1.).sub(t)?;
            for i in 0..=degree - r {
                for axis in 0..4 {
                    controls[i][axis] = controls[i][axis]
                        .mul(beta)?
                        .add(controls[i + 1][axis].mul(t)?)?;
                }
            }
        }
        for k in 0..4 {
            hull[k].lo = hull[k].lo.min(controls[0][k].lo);
            hull[k].hi = hull[k].hi.max(controls[0][k].hi);
        }
    }
    Ok(hull)
}
impl SourceJets {
    #[cfg(test)]
    pub(super) fn new(curve: &Curve, budget: usize) -> Result<(usize, Option<Self>)> {
        Self::new_order(curve, 3, budget)
    }
    pub(super) fn new_order(
        curve: &Curve,
        order: usize,
        budget: usize,
    ) -> Result<(usize, Option<Self>)> {
        check((1..=3).contains(&order), "Invalid original XYZ jet order")?;
        curve.validate()?;
        check(
            budget <= 100000 && curve.control_points.iter().all(|p| p.len() == 3),
            "Invalid original XYZ jet snapshot",
        )?;
        let mut cells = 0;
        let mut spans = Vec::with_capacity(curve.control_points.len().saturating_sub(curve.degree));
        for span in curve.degree..curve.control_points.len() {
            let domain = [curve.knots[span], curve.knots[span + 1]];
            if domain[0] >= domain[1] {
                continue;
            }
            if budget - cells < order + 1 {
                return Ok((cells, None));
            }
            cells += order + 1;
            let built = (|| -> Result<Span> {
                let width = I::point(domain[1]).sub(I::point(domain[0]))?;
                let h = crate::curve_distance::restricted_controls(
                    curve,
                    span,
                    I::new(domain[0], domain[1])?,
                )?
                .iter()
                .map(|p| std::array::from_fn(|k| p[k]))
                .collect::<Vec<_>>();
                let first = derivative(&h, curve.degree, width)?;
                let second = if order >= 2 {
                    derivative(&first, curve.degree.saturating_sub(1), width)?
                } else {
                    vec![[I::point(0.); 4]]
                };
                let third = if order >= 3 {
                    derivative(&second, curve.degree.saturating_sub(2), width)?
                } else {
                    vec![[I::point(0.); 4]]
                };
                Ok(Span {
                    domain,
                    anchor: std::array::from_fn(|k| curve.control_points[span - curve.degree][k]),
                    fields: [h, first, second, third],
                })
            })();
            let Ok(span) = built else {
                return Ok((cells, None));
            };
            spans.push(span);
        }
        Ok((
            cells,
            Some(Self {
                domain: curve.domain(),
                spans,
                order,
            }),
        ))
    }
    pub(super) fn query(&self, traversal: [f64; 2], budget: usize) -> Result<Report> {
        check(
            budget <= 100000
                && traversal.iter().all(|v| v.is_finite())
                && traversal[0] >= 0.
                && traversal[1] <= 1.
                && traversal[0] <= traversal[1],
            "Invalid original XYZ jet query",
        )?;
        let mut cells = 0;
        let mut count = 0;
        let result = (|| -> Result<Option<[[[f64; 2]; 3]; 4]>> {
            let [a, b] = self.domain;
            let source = I::point(a)
                .add(
                    I::point(b)
                        .sub(I::point(a))?
                        .mul(I::new(traversal[0], traversal[1])?)?,
                )?
                .intersect(a, b)?;
            let mut jets = [[[f64::INFINITY, f64::NEG_INFINITY]; 3]; 4];
            for span in &self.spans {
                let lo = source.lo.max(span.domain[0]);
                let hi = source.hi.min(span.domain[1]);
                if lo > hi {
                    continue;
                }
                if cells == budget {
                    return Ok(None);
                }
                cells += 1;
                count += 1;
                let width = I::point(span.domain[1]).sub(I::point(span.domain[0]))?;
                let local = I::new(lo, hi)?
                    .sub(I::point(span.domain[0]))?
                    .div(width)?
                    .intersect(0., 1.)?;
                let mut h = [[I::point(0.); 4]; 4];
                for order in 0..=self.order {
                    h[order] = restrict(&span.fields[order], local)?;
                }
                for axis in 0..3 {
                    let value = h[0][axis].div(h[0][3])?;
                    let first = h[1][axis].sub(value.mul(h[1][3])?)?.div(h[0][3])?;
                    let second = if self.order >= 2 {
                        h[2][axis]
                            .sub(first.mul(h[1][3])?.mul(I::point(2.))?)?
                            .sub(value.mul(h[2][3])?)?
                            .div(h[0][3])?
                    } else {
                        I::point(0.)
                    };
                    let third = if self.order >= 3 {
                        h[3][axis]
                            .sub(second.mul(h[1][3])?.mul(I::point(3.))?)?
                            .sub(first.mul(h[2][3])?.mul(I::point(3.))?)?
                            .sub(value.mul(h[3][3])?)?
                            .div(h[0][3])?
                    } else {
                        I::point(0.)
                    };
                    for (order, value) in [
                        value.add(I::point(span.anchor[axis]))?,
                        first,
                        second,
                        third,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        jets[order][axis][0] = jets[order][axis][0].min(value.lo);
                        jets[order][axis][1] = jets[order][axis][1].max(value.hi);
                    }
                }
            }
            Ok((count > 0).then_some(jets))
        })();
        Ok(Report {
            cells,
            jets: result.unwrap_or(None),
            single_span: count == 1,
            order: self.order,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_joint_xyz_jets_preserve_domain_derivatives_and_partial_refusal() {
        let polynomial = Curve {
            degree: 3,
            knots: vec![7., 7., 7., 7., 11., 11., 11., 11.],
            control_points: vec![
                vec![0.; 3],
                vec![1., 0., 0.],
                vec![2., 1., 0.],
                vec![3., 3., 1.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        for weights in [vec![1.; 4], vec![1., 0.5, 2., 1.]] {
            let curve = Curve {
                weights,
                ..polynomial.clone()
            };
            assert!(SourceJets::new(&curve, 3).unwrap().1.is_none());
            let (work, owner) = SourceJets::new(&curve, 10000).unwrap();
            assert_eq!(work, 4);
            let owner = owner.unwrap();
            for interval in [[0.; 2], [0.125, 0.1875], [0.5; 2], [0., 1.], [1.; 2]] {
                let proved = owner.query(interval, 10000).unwrap();
                assert!(proved.single_span);
                assert_eq!(proved.cells, 1);
                let jets = proved.jets.unwrap();
                for budget in [0, proved.cells - 1] {
                    let refused = owner.query(interval, budget).unwrap();
                    assert!(refused.jets.is_none());
                    assert!(refused.cells <= budget);
                }
                for f in [0., 0.5, 1.] {
                    let t = interval[0] + f * (interval[1] - interval[0]);
                    let sample = curve.evaluate(7. + 4. * t).unwrap();
                    for (order, field) in [sample.point, sample.d1.unwrap(), sample.d2.unwrap()]
                        .iter()
                        .enumerate()
                    {
                        for k in 0..3 {
                            assert!(
                                jets[order][k][0] - 1e-11 <= field[k]
                                    && field[k] <= jets[order][k][1] + 1e-11
                            );
                        }
                    }
                }
                if curve.weights.iter().all(|w| *w == 1.) {
                    for (axis, expected) in [0., 0., 6. / 64.].into_iter().enumerate() {
                        assert!(jets[3][axis][0] <= expected && expected <= jets[3][axis][1]);
                    }
                }
            }
        }
        let corner = Curve {
            degree: 1,
            knots: vec![2., 2., 3., 5., 5.],
            control_points: vec![vec![0.; 3], vec![1., 0., 0.], vec![1., 2., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let (_, owner) = SourceJets::new(&corner, 10000).unwrap();
        let report = owner.unwrap().query([1. / 3.; 2], 10000).unwrap();
        assert!(!report.single_span);
        assert_eq!(report.cells, 2);
        assert!(report.jets.is_some());
        let tiny = Curve {
            knots: vec![0., 0., 0., 0., 1e-130, 1e-130, 1e-130, 1e-130],
            ..polynomial
        };
        let (_, lower) = SourceJets::new_order(&tiny, 2, 10000).unwrap();
        let report = lower.unwrap().query([0.25, 0.5], 10000).unwrap();
        assert_eq!(report.order, 2);
        assert!(report.jets.is_some());
        assert!(SourceJets::new_order(&tiny, 3, 10000).unwrap().1.is_none());
    }
}
