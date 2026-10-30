//! Exact scalar identities for homogeneous polynomial construction only.
//! Public Interval outward-rounding and frozen binary contracts stay unchanged.
use super::Interval as I;
use crate::Result;
pub(super) trait ExactIdentities {
    fn exact_add(self, b: Self) -> Result<Self>
    where
        Self: Sized;
    fn exact_sub(self, b: Self) -> Result<Self>
    where
        Self: Sized;
    fn exact_mul(self, b: Self) -> Result<Self>
    where
        Self: Sized;
    fn exact_div(self, b: Self) -> Result<Self>
    where
        Self: Sized;
}
fn point(a: I, x: f64) -> bool {
    a.lo == x && a.hi == x
}
impl ExactIdentities for I {
    fn exact_add(self, b: Self) -> Result<Self> {
        if point(self, 0.) || point(b, 0.) {
            return I::new(self.lo + b.lo, self.hi + b.hi);
        }
        self.add(b)
    }
    fn exact_sub(self, b: Self) -> Result<Self> {
        if point(b, 0.) {
            return I::new(self.lo - b.lo, self.hi - b.hi);
        }
        if point(self, 0.) {
            return I::new(-b.hi, -b.lo);
        }
        if self.lo == self.hi && b.lo == b.hi && self.lo == b.lo {
            return I::new(self.lo - b.lo, self.hi - b.hi);
        }
        self.sub(b)
    }
    fn exact_mul(self, b: Self) -> Result<Self> {
        if (point(self, 0.) || point(b, 0.))
            && [self.lo, self.hi, b.lo, b.hi].iter().all(|x| x.is_finite())
        {
            return I::new(self.lo * b.lo, self.hi * b.hi);
        }
        if point(self, 1.) {
            return I::new(b.lo, b.hi);
        }
        if point(b, 1.) {
            return I::new(self.lo, self.hi);
        }
        self.mul(b)
    }
    fn exact_div(self, b: Self) -> Result<Self> {
        if b.lo > 0. && b.hi.is_finite() && b.lo <= b.hi {
            if point(self, 0.) || point(b, 1.) {
                return I::new(self.lo / b.lo, self.hi / b.lo);
            }
            if self.lo == self.hi && b.lo == b.hi && self.lo == b.lo {
                return Ok(I::point(1.));
            }
        }
        self.div(b)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scalar_identities_keep_true_zero_but_do_not_cancel_equal_uncertain_ranges() {
        let z = I::point(0.);
        let one = I::point(1.);
        let a = I::new(1., 2.).unwrap();
        assert_eq!(z.exact_mul(a).unwrap(), z);
        assert_eq!(z.exact_div(a).unwrap(), z);
        assert_eq!(one.exact_mul(a).unwrap(), a);
        assert_eq!(a.exact_div(one).unwrap(), a);
        assert_eq!(a.exact_add(z).unwrap(), a);
        assert_eq!(one.exact_sub(one).unwrap(), z);
        assert_ne!(a.exact_sub(a).unwrap(), z);
        let tiny = I::point(0_f64.next_up());
        assert_eq!(tiny.exact_mul(one).unwrap(), tiny);
        assert!(
            z.exact_mul(I {
                lo: f64::INFINITY,
                hi: f64::INFINITY
            })
            .is_err()
        );
        assert!(z.exact_div(z).is_err());
    }
    #[test]
    fn polynomial_proposals_preserve_literal_flat_end_rows_at_zero_and_shifted_planes() {
        for height in [0., 10.] {
            for reverse in [false, true] {
                let mut s = super::super::Surface {
                    degree_u: 1,
                    degree_v: 3,
                    knots_u: vec![0., 0., 1., 1.],
                    knots_v: [vec![0.; 4], vec![1.; 4]].concat(),
                    control_points: (0..2)
                        .map(|i| {
                            (0..4)
                                .map(|j| {
                                    vec![
                                        i as f64,
                                        j as f64 / 3.,
                                        height + if j == 3 { 1. } else { 0. },
                                    ]
                                })
                                .collect()
                        })
                        .collect(),
                    weights: vec![vec![1.; 4]; 2],
                    periodic_u: false,
                    periodic_v: false,
                };
                if reverse {
                    for row in &mut s.control_points {
                        row.reverse();
                    }
                }
                let p = super::super::Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: vec![vec![0.3, 0.], vec![0.3, 1.]],
                    weights: vec![1., 1.000000000000001],
                    periodic: false,
                };
                let before = s.clone();
                let curve = super::super::cell_trace::candidate(&p, &s)
                    .unwrap()
                    .unwrap();
                let rows = if reverse {
                    &curve.control_points[curve.control_points.len() - 3..]
                } else {
                    &curve.control_points[..3]
                };
                assert!(
                    rows.iter().all(|r| r[2] == height),
                    "height={height} reverse={reverse}: {rows:?}"
                );
                assert_eq!(s, before);
            }
        }
    }
    #[test]
    fn original_zero_rows_survive_nonuniform_multispan_extraction_and_oblique_trace() {
        let s = super::super::Surface {
            degree_u: 5,
            degree_v: 3,
            knots_u: [vec![0.; 6], vec![1.; 6]].concat(),
            knots_v: vec![0., 0., 0., 0., 0.25, 0.5, 0.75, 1., 1., 1., 1.],
            control_points: (0..6)
                .map(|i| {
                    (0..7)
                        .map(|j| vec![i as f64 / 5., j as f64, if i < 3 { 0. } else { 1. }])
                        .collect()
                })
                .collect(),
            weights: vec![
                vec![
                    1.,
                    1.0000000000000002,
                    1.,
                    1.0000000000000004,
                    0.9999999999999997,
                    1.0000000000000002,
                    1.
                ];
                6
            ],
            periodic_u: false,
            periodic_v: false,
        };
        let pc = super::super::Curve::from_polyline(vec![vec![0., 0.95], vec![1., 0.950000000001]])
            .unwrap();
        let saved = s.clone();
        let c = super::super::trace_candidate(&pc, &s).unwrap().unwrap();
        assert!(c.control_points[..3].iter().all(|p| p[2] == 0.));
        assert_eq!(s, saved);
        let mut tiny = s.clone();
        tiny.control_points[1][6][2] = 0_f64.next_up();
        let cell = [5, 6];
        let domain = [[0., 1.], [0.75, 1.]];
        let net = super::super::surface_net_on_centered(&tiny, cell, domain, [0.; 3]).unwrap();
        assert!(net[1].iter().any(|p| p[2].lo != 0. || p[2].hi != 0.));
    }
}
