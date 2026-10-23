//! Source fragments against an interior coordinate fiber, with support side.
use crate::{
    source_boundary_fragment::{Endpoint, Fragment, Role},
    source_fiber_boundary::Locus,
    source_interior_fiber::Certificate,
};
use nurbs_core::{
    interval_eval::{self, Interval},
    Error, Result,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Below,
    Above,
    On,
}
pub struct Report {
    pub locus: Locus,
    pub side: Option<Side>,
    pub driver_cells: usize,
}
pub fn inspect(f: &Fragment, fiber: &Certificate, max_driver: usize) -> Result<Report> {
    if f.surface() != fiber.surface() || max_driver > 100000 {
        return Err(Error::new(
            "BREP_SOURCE_INTERIOR_BOUNDARY",
            "Use the original certified surface and bounded driver work",
        ));
    }
    let (axis, level) = fiber.coordinate();
    let c = f.curve();
    let d = c.domain();
    let mut out = Report {
        locus: Locus::Unresolved,
        side: None,
        driver_cells: 0,
    };
    let emit = |out: &mut Report, locus, side| {
        out.locus = locus;
        out.side = Some(side);
    };
    if c.control_points.iter().all(|p| p[axis] == level) {
        emit(&mut out, Locus::EntireFragment, Side::On);
        return Ok(out);
    }
    for (below, side) in [(true, Side::Below), (false, Side::Above)] {
        if c.control_points.iter().all(|p| {
            if below {
                p[axis] < level
            } else {
                p[axis] > level
            }
        }) {
            emit(&mut out, Locus::Away, side);
            return Ok(out);
        }
        let bezier = c.control_points.len() == c.degree + 1
            && c.knots[..=c.degree].iter().all(|&t| t == d[0])
            && c.knots[c.control_points.len()..].iter().all(|&t| t == d[1]);
        if bezier
            && c.control_points.iter().all(|p| {
                if below {
                    p[axis] <= level
                } else {
                    p[axis] >= level
                }
            })
        {
            let mut ends = [false; 2];
            for i in 0..2 {
                let bounds = match &f.endpoints()[i] {
                    Endpoint::Crossing { point, .. } => point.uv_box()[axis],
                    Endpoint::Parameter(t) => {
                        let v = interval_eval::evaluate_interval(c, Interval::point(*t))?[axis];
                        [v.lo, v.hi]
                    }
                };
                ends[i] = bounds[0] <= level && level <= bounds[1];
            }
            emit(
                &mut out,
                if ends.iter().any(|x| *x) {
                    Locus::Endpoints(ends)
                } else {
                    Locus::Away
                },
                side,
            );
            return Ok(out);
        }
    }
    // A source-owned implicit root is exactly on this coordinate line. Strict
    // monotonicity and the immutable endpoint order put its full fragment on
    // one side; the outward root box is not substituted for the endpoint.
    let roots = f
        .endpoints()
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match e {
            Endpoint::Crossing { point, role } => {
                let other = match role {
                    Role::Boundary if point.boundary() == c => Some(point.contact()),
                    Role::Contact if point.contact() == c => Some(point.boundary()),
                    _ => None,
                }?;
                other
                    .control_points
                    .iter()
                    .all(|p| p[axis] == level)
                    .then_some(i)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if roots.len() != 1
        || !matches!(f.endpoints()[1 - roots[0]], Endpoint::Parameter(_))
        || max_driver == 0
    {
        return Ok(out);
    }
    let driver = nurbs_core::curve_axis_driver::certify(c, axis, max_driver)?;
    out.driver_cells = driver.visited;
    if !driver.monotonic_proven {
        return Ok(out);
    }
    let above = (driver.increasing != f.reversed()) != (roots[0] == 1);
    let mut ends = [false; 2];
    ends[roots[0]] = true;
    emit(
        &mut out,
        Locus::Endpoints(ends),
        if above { Side::Above } else { Side::Below },
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn root_owned_restrictions_prove_support_and_crossing_fragments_remain_unknown() {
        let s = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.], vec![0., 1., 1.]],
            ],
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let fiber = crate::source_interior_fiber::certify(
            &s,
            [[0., 0.859375, 0.], [1., 0.859375, 0.], [0., 0.859375, 1.]],
            1,
            0.625,
            100_000_000,
            10,
        )
        .unwrap()
        .certificate
        .unwrap();
        let main =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0.2, 0.], vec![0.2, 1.]]).unwrap();
        let cut = nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0.625], vec![1., 0.625]])
            .unwrap();
        let point = crate::source_contact_point::qualify(
            &s,
            &main,
            &cut,
            [[0.6, 0.65], [0.15, 0.25]],
            10000,
        )
        .unwrap()
        .point
        .unwrap();
        let root = Endpoint::Crossing {
            point,
            role: Role::Boundary,
        };
        let below = Fragment::new(&s, &main, Endpoint::Parameter(0.), root.clone()).unwrap();
        let r = inspect(&below, &fiber, 1).unwrap();
        assert_eq!(r.side, Some(Side::Below));
        assert_eq!(r.locus, Locus::Endpoints([false, true]));
        assert_eq!(r.driver_cells, 1);
        let reversed = Fragment::new(&s, &main, root.clone(), Endpoint::Parameter(0.)).unwrap();
        assert_eq!(
            inspect(&reversed, &fiber, 1).unwrap().side,
            Some(Side::Below)
        );
        let above = Fragment::new(&s, &main, root, Endpoint::Parameter(1.)).unwrap();
        assert_eq!(inspect(&above, &fiber, 1).unwrap().side, Some(Side::Above));
        assert_eq!(inspect(&below, &fiber, 0).unwrap().locus, Locus::Unresolved);
        let crossing =
            Fragment::new(&s, &main, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        assert_eq!(
            inspect(&crossing, &fiber, 10).unwrap().locus,
            Locus::Unresolved
        );
        let on = Fragment::new(&s, &cut, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        assert_eq!(
            inspect(&on, &fiber, 0).unwrap().locus,
            Locus::EntireFragment
        );
        for y in [0.3, 0.25] {
            // Fold crosses the interior level twice (0.3) or touches it
            // tangentially (0.25), although both endpoints are below it.
            let folded = nurbs_core::curve::Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![vec![0., y], vec![0.5, 1.], vec![1., y]],
                weights: vec![1.; 3],
                periodic: false,
            };
            let fragment = Fragment::new(
                &s,
                &folded,
                Endpoint::Parameter(0.),
                Endpoint::Parameter(1.),
            )
            .unwrap();
            assert_eq!(
                inspect(&fragment, &fiber, 10).unwrap().locus,
                Locus::Unresolved
            );
        }
        let mut changed = s.clone();
        changed.control_points[0][0][0] += 1e-12;
        let fragment = Fragment::new(
            &changed,
            &cut,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        assert!(inspect(&fragment, &fiber, 10).is_err());
    }
}
