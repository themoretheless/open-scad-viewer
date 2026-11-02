//! Exact natural-side ownership and outward transport of collar cuts.
use super::*;
use crate::source_boundary_fragment::{Endpoint, Fragment};
use nurbs_core::interval_eval::Interval as I;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Side {
    pub axis: usize,
    pub upper: bool,
}
pub(super) fn pole_side(b: Boundary) -> Side {
    match b {
        Boundary::UMin => Side {
            axis: 0,
            upper: false,
        },
        Boundary::UMax => Side {
            axis: 0,
            upper: true,
        },
        Boundary::VMin => Side {
            axis: 1,
            upper: false,
        },
        Boundary::VMax => Side {
            axis: 1,
            upper: true,
        },
    }
}
pub(super) fn natural_side(f: &Fragment) -> Option<Side> {
    let c = f.curve();
    let d = c.domain();
    let b = f.parameter_bounds();
    let full = (b == [[d[0], d[0]], [d[1], d[1]]]) || (b == [[d[1], d[1]], [d[0], d[0]]]);
    if !full
        || c.degree != 1
        || c.periodic
        || c.control_points.len() != 2
        || c.weights.len() != 2
        || c.knots.len() != 4
        || c.knots[..2].iter().any(|&k| k != d[0])
        || c.knots[2..].iter().any(|&k| k != d[1])
        || f.endpoints()
            .iter()
            .any(|e| !matches!(e, Endpoint::Parameter(_)))
    {
        return None;
    }
    let s = f.surface();
    let domains = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    for axis in 0..2 {
        for upper in [false, true] {
            let fixed = domains[axis][usize::from(upper)];
            let free = 1 - axis;
            let a = c.control_points[0][free];
            let b = c.control_points[1][free];
            if c.control_points.iter().all(|p| p[axis] == fixed)
                && ((a == domains[free][0] && b == domains[free][1])
                    || (a == domains[free][1] && b == domains[free][0]))
            {
                return Some(Side { axis, upper });
            }
        }
    }
    None
}
/// A complete literal fragment on PART of a natural side. Only the outer
/// radial side may use this in graph admission, with a separate native proof
/// that the full material UV rectangle is covered. Meridian cuts stay whole.
pub(super) fn natural_side_piece(f: &Fragment) -> Option<Side> {
    let c = f.curve();
    let d = c.domain();
    let b = f.parameter_bounds();
    if !((b == [[d[0]; 2], [d[1]; 2]]) || (b == [[d[1]; 2], [d[0]; 2]]))
        || c.degree != 1
        || c.periodic
        || c.control_points.len() != 2
        || c.weights.len() != 2
        || c.weights.iter().any(|w| !w.is_finite() || *w <= 0.)
        || c.control_points.iter().any(|p| p.len() != 2)
        || c.knots.len() != 4
        || c.knots[..2].iter().any(|k| *k != d[0])
        || c.knots[2..].iter().any(|k| *k != d[1])
        || f.endpoints()
            .iter()
            .any(|e| !matches!(e, Endpoint::Parameter(_)))
    {
        return None;
    }
    let s = f.surface();
    let domains = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    for axis in 0..2 {
        for upper in [false, true] {
            let free = 1 - axis;
            let (a, b) = (&c.control_points[0], &c.control_points[1]);
            if a[axis] == domains[axis][usize::from(upper)]
                && b[axis] == a[axis]
                && a[free] != b[free]
                && [a[free], b[free]]
                    .iter()
                    .all(|v| *v >= domains[free][0] && *v <= domains[free][1])
            {
                return Some(Side { axis, upper });
            }
        }
    }
    None
}
#[derive(Clone, Copy)]
pub(super) struct Meridian {
    domain: [f64; 2],
    weights: [f64; 2],
    pole_first: bool,
    pole_end: usize,
}
impl Meridian {
    pub fn from_fragment(f: &Fragment, pole: Boundary) -> Option<Self> {
        let side = natural_side(f)?;
        let p = pole_side(pole);
        if side.axis == p.axis {
            return None;
        }
        let s = f.surface();
        let d = if p.axis == 0 {
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]]
        } else {
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]]
        };
        let c = f.curve();
        let pole_first = c.control_points[0][p.axis] == d[usize::from(p.upper)];
        let wire_first = pole_first ^ f.reversed();
        Some(Self {
            domain: c.domain(),
            weights: [c.weights[0], c.weights[1]],
            pole_first,
            pole_end: usize::from(!wire_first),
        })
    }
    pub fn pole_end(&self) -> usize {
        self.pole_end
    }
    /// Solve the positive rational-linear UV map, not t=extent.
    pub fn cut(&self, extent: f64) -> Result<I> {
        let [w0, w1] = if self.pole_first {
            self.weights
        } else {
            [self.weights[1], self.weights[0]]
        };
        let a = I::point(extent).mul(I::point(w0))?;
        let b = I::point(1.).sub(I::point(extent))?.mul(I::point(w1))?;
        let q = a.div(a.add(b)?)?.intersect(0., 1.)?;
        let q = if self.pole_first {
            q
        } else {
            I::point(1.).sub(q)?.intersect(0., 1.)?
        };
        I::point(self.domain[0])
            .add(
                I::point(self.domain[1])
                    .sub(I::point(self.domain[0]))?
                    .mul(q)?,
            )?
            .intersect(self.domain[0], self.domain[1])
    }
    pub fn inward(&self, t: I) -> Result<I> {
        let q = t
            .sub(I::point(self.domain[0]))?
            .div(I::point(self.domain[1]).sub(I::point(self.domain[0]))?)?
            .intersect(0., 1.)?;
        let a = I::point(1.)
            .sub(q)?
            .intersect(0., 1.)?
            .mul(I::point(self.weights[0]))?;
        let b = q.mul(I::point(self.weights[1]))?;
        (if self.pole_first { b } else { a })
            .div(a.add(b)?)?
            .intersect(0., 1.)
    }
}
/// The sealed carrier owns the exact common endpoint and strictly monotone
/// canonical parameter maps. Transport yields the other chart's INWARD
/// coordinate at this cut; no root or world point is replaced by a sample.
pub(super) fn transported_cut<E: Carrier>(
    edge: &E,
    from: usize,
    meridians: [Meridian; 2],
    extent: f64,
) -> Result<I> {
    let t = meridians[from].cut(extent)?;
    let canonical = edge.map_parameter(from, t)?;
    let other = edge.inverse_parameter(1 - from, canonical)?;
    meridians[1 - from].inward(other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::{curve::Curve, surface::Surface};
    #[test]
    fn weighted_nonunit_cut_is_inverted_before_transport_and_partial_edges_refuse() {
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0.; 3]; 2],
                vec![vec![0.5, 0., 0.], vec![0.5, 0.5, 0.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let c = Curve {
            degree: 1,
            knots: vec![-2., -2., 5., 5.],
            control_points: vec![vec![0., 0.], vec![1., 0.]],
            weights: vec![0.5, 2.],
            periodic: false,
        };
        for reversed in [false, true] {
            let ends = if reversed { [5., -2.] } else { [-2., 5.] };
            let f = Fragment::new(
                &s,
                &c,
                Endpoint::Parameter(ends[0]),
                Endpoint::Parameter(ends[1]),
            )
            .unwrap();
            let m = Meridian::from_fragment(&f, Boundary::UMin).unwrap();
            let cut = m.cut(0.25).unwrap();
            let expected = -2. + 7. / 13.;
            assert!(cut.lo <= expected && expected <= cut.hi);
            let u = m.inward(cut).unwrap();
            assert!(u.lo <= 0.25 && u.hi >= 0.25);
            assert_eq!(m.pole_end(), usize::from(reversed));
            let partial = f.split_at_parameter(0.).unwrap();
            assert!(natural_side(&partial[0]).is_none());
        }
    }
}
