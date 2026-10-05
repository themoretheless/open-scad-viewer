//! Immutable original source use on an exact affine portion of a canonical curve.
//! This does not prove cross-face endpoint ownership or admit a shell pair.
use crate::{source_allowed_contact, source_boundary_fragment::Fragment};
use cad_predicates::{BezierIdentity, Sign};
use nurbs_core::{curve::Curve, interval_eval::Interval as I, Error, Result};
#[derive(Clone)]
pub struct MappedUse {
    world: Curve,
    fragment: Fragment,
    range: [[f64; 2]; 2],
    reversed: bool,
}
impl MappedUse {
    pub fn world(&self) -> &Curve {
        &self.world
    }
    pub fn fragment(&self) -> &Fragment {
        &self.fragment
    }
    /// Exact normalized rational endpoints for the forward original pcurve.
    pub fn range(&self) -> [[f64; 2]; 2] {
        self.range
    }
    pub fn reversed(&self) -> bool {
        self.reversed
    }
    /// Outward enclosures only: root expressions and original domains remain
    /// authoritative. These bounds must never replace a root by a fixed vertex.
    pub fn parameter_bounds(&self) -> Result<[[f64; 2]; 2]> {
        let source = self.fragment.curve().domain();
        let target = self.world.domain();
        let first = I::point(self.range[0][0]).div(I::point(self.range[0][1]))?;
        let last = I::point(self.range[1][0]).div(I::point(self.range[1][1]))?;
        let span = last.sub(first)?;
        let map = |r: [f64; 2]| -> Result<[f64; 2]> {
            let normalized = I::new(r[0], r[1])?
                .sub(I::point(source[0]))?
                .div(I::point(source[1]).sub(I::point(source[0]))?)?
                .intersect(0., 1.)?;
            let q = first.add(normalized.mul(span)?)?.intersect(0., 1.)?;
            let t = I::point(target[0])
                .add(q.mul(I::point(target[1]).sub(I::point(target[0]))?)?)?
                .intersect(target[0], target[1])?;
            Ok([t.lo, t.hi])
        };
        let b = self.fragment.parameter_bounds();
        Ok([map(b[0])?, map(b[1])?])
    }
}
pub struct Report {
    pub mapped: Option<MappedUse>,
    pub exact_work: u64,
    pub reason: &'static str,
}
/// Fragment authority proves actual chart membership. Fresh exact composition
/// then binds its unchanged source curve to the supplied canonical subinterval.
pub fn qualify(
    world: &Curve,
    fragment: &Fragment,
    range: [[f64; 2]; 2],
    max_work: u64,
) -> Result<Report> {
    if !(1..=100_000_000).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_AFFINE_USE",
            "Choose bounded exact mapping work",
        ));
    }
    let mut out = Report {
        mapped: None,
        exact_work: 0,
        reason: "source-affine-layout-unproven",
    };
    let Some(r) = nurbs_core::curve_surface_affine::verify_algebraic(
        world,
        fragment.curve(),
        fragment.surface(),
        range,
        max_work.min(cad_predicates::MAX_WORK),
    )?
    else {
        return Ok(out);
    };
    out.exact_work += r.work_used;
    out.reason = "source-affine-composition-unproven";
    if r.outcome != BezierIdentity::Equal {
        return Ok(out);
    }
    out.reason = "source-affine-orientation-unproven";
    let points = [
        [0., 0., 0.],
        [range[0][0], range[0][1], 0.],
        [range[1][0], range[1][1], 0.],
    ];
    let Some(sign) =
        source_allowed_contact::orient(&points, Some([0, 1]), &mut out.exact_work, max_work)?
    else {
        return Ok(out);
    };
    if sign == Sign::Zero {
        return Ok(out);
    }
    out.mapped = Some(MappedUse {
        world: world.clone(),
        fragment: fragment.clone(),
        range,
        reversed: fragment.reversed() ^ (sign == Sign::Positive),
    });
    out.reason = "source-affine-use-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_boundary_fragment::Endpoint;
    use nurbs_core::surface::Surface;
    fn plane(fixed: bool) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if fixed {
                                vec![0.609375, u as f64, v as f64]
                            } else {
                                vec![u as f64, v as f64, 0.]
                            }
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn partial_closing_line_keeps_sources_and_bounds_exact_rational_map() {
        let s = plane(true);
        let world = Curve {
            degree: 1,
            knots: vec![-5., -5., 3., 3.],
            control_points: vec![vec![0.609375, -0.25, 0.], vec![0.609375, 1.25, 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let pc = Curve {
            degree: 1,
            knots: vec![10., 10., 18., 18.],
            control_points: vec![vec![0., 0.], vec![0.859375, 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let f = Fragment::new(&s, &pc, Endpoint::Parameter(10.), Endpoint::Parameter(18.)).unwrap();
        let range = [[1., 6.], [71., 96.]];
        let r = qualify(&world, &f, range, 100_000_000).unwrap();
        assert!(r.mapped.is_some(), "{}", r.reason);
        let m = r.mapped.unwrap();
        assert_eq!(m.world(), &world);
        assert_eq!(m.fragment().curve(), &pc);
        assert_eq!(m.range(), range);
        assert!(!m.reversed());
        let bounds = m.parameter_bounds().unwrap();
        assert!(bounds[0][0] <= -11. / 3. && -11. / 3. <= bounds[0][1]);
        assert!(bounds[1][0] <= 11. / 12. && 11. / 12. <= bounds[1][1]);
        let reversed =
            Fragment::new(&s, &pc, Endpoint::Parameter(18.), Endpoint::Parameter(10.)).unwrap();
        assert!(qualify(&world, &reversed, range, 100_000_000)
            .unwrap()
            .mapped
            .unwrap()
            .reversed());
        assert!(
            qualify(&world, &f, [[1., 6.], [71. + 1e-12, 96.]], 100_000_000)
                .unwrap()
                .mapped
                .is_none()
        );
        assert!(qualify(&world, &f, range, 1).unwrap().mapped.is_none());
    }
    #[test]
    fn curved_source_subcoverage_and_reverse_map_are_bound_without_trimming() {
        let s = plane(false);
        let world = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let pc = Curve {
            degree: 2,
            knots: world.knots.clone(),
            control_points: vec![
                vec![0.9375, 0.4375],
                vec![0.8125, 0.8125],
                vec![0.4375, 0.9375],
            ],
            weights: vec![1.; 3],
            periodic: false,
        };
        let f = Fragment::new(&s, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        assert!(qualify(&world, &f, [[1., 4.], [3., 4.]], 100_000_000)
            .unwrap()
            .mapped
            .is_some());
        let cut = Curve::from_polyline(vec![vec![0.75, 1.25], vec![0.75, -0.25]]).unwrap();
        let root =
            crate::source_contact_point::qualify(&s, &pc, &cut, [[0.4, 0.6], [0.25, 0.4]], 10000)
                .unwrap();
        assert!(root.point.is_some(), "{}", root.reason);
        let clipped = Fragment::new(
            &s,
            &pc,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: root.point.unwrap(),
                role: crate::source_boundary_fragment::Role::Boundary,
            },
        )
        .unwrap();
        let mapped = qualify(&world, &clipped, [[1., 4.], [3., 4.]], 100_000_000)
            .unwrap()
            .mapped
            .unwrap();
        assert!(matches!(
            mapped.fragment().endpoints()[1],
            Endpoint::Crossing { .. }
        ));
        assert_eq!(mapped.fragment().curve(), &pc);
        let b = mapped.parameter_bounds().unwrap();
        assert!(b[1][0] <= 0.5 && 0.5 <= b[1][1]);
        let mut reverse = pc.clone();
        reverse.control_points.reverse();
        let f = Fragment::new(
            &s,
            &reverse,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let m = qualify(&world, &f, [[3., 4.], [1., 4.]], 100_000_000)
            .unwrap()
            .mapped
            .unwrap();
        assert!(m.reversed());
        assert_eq!(m.fragment().curve(), &reverse);
        let b = m.parameter_bounds().unwrap();
        assert!(b[0][0] <= 0.75 && 0.75 <= b[0][1]);
        assert!(b[1][0] <= 0.25 && 0.25 <= b[1][1]);
    }
}
