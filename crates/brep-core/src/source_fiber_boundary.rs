//! Source restrictions classified against an immutable natural plane fiber.
//! Potential contacts remain explicit; this module never grants shared ownership.
use crate::{
    source_boundary_fragment::{Endpoint, Fragment},
    source_plane_fiber::Certificate,
};
use nurbs_core::{Error, Result};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locus {
    EntireFragment,
    /// Interior is away; true means that endpoint may lie on the fiber.
    Endpoints([bool; 2]),
    Away,
    Unresolved,
}
pub struct Report {
    pub locus: Locus,
    pub driver_cells: usize,
}
/// Chart containment has already been established by Fragment construction.
/// Strict source-coordinate monotonicity therefore excludes an interior
/// extremum at the natural chart boundary, even for root-valued restrictions.
pub fn inspect(fragment: &Fragment, fiber: &Certificate, max_cells: usize) -> Result<Report> {
    if fragment.surface() != fiber.surface() || !(1..=100000).contains(&max_cells) {
        return Err(Error::new(
            "BREP_SOURCE_FIBER_BOUNDARY",
            "Use the certified source surface and bounded driver work",
        ));
    }
    let (axis, upper) = fiber.boundary();
    let s = fiber.surface();
    let (knots, degree, count) = if axis == 0 {
        (&s.knots_u, s.degree_u, s.control_points.len())
    } else {
        (&s.knots_v, s.degree_v, s.control_points[0].len())
    };
    let fixed = knots[if upper { count } else { degree }];
    let c = fragment.curve();
    let mut out = Report {
        locus: Locus::Unresolved,
        driver_cells: 0,
    };
    if c.control_points.iter().all(|p| p[axis] == fixed) {
        out.locus = Locus::EntireFragment;
        return Ok(out);
    }
    if c.control_points.iter().all(|p| {
        if upper {
            p[axis] < fixed
        } else {
            p[axis] > fixed
        }
    }) {
        out.locus = Locus::Away;
        return Ok(out);
    }
    // Positive single-chart Bernstein basis is strictly positive inside.
    // This is valid even when the coordinate is not monotone.
    let d = c.domain();
    let bezier = c.control_points.len() == c.degree + 1
        && c.knots[..=c.degree].iter().all(|&t| t == d[0])
        && c.knots[c.control_points.len()..].iter().all(|&t| t == d[1]);
    let one_side = c.control_points.iter().all(|p| {
        if upper {
            p[axis] <= fixed
        } else {
            p[axis] >= fixed
        }
    });
    if !(bezier && one_side) {
        if c.knots[..=c.degree].iter().any(|&t| t != d[0])
            || c.knots[c.control_points.len()..].iter().any(|&t| t != d[1])
        {
            return Ok(out);
        }
        let driver = nurbs_core::curve_axis_driver::certify(c, axis, max_cells)?;
        out.driver_cells = driver.visited;
        if !driver.monotonic_proven {
            return Ok(out);
        }
    }
    let endpoint_may_contact = |end: &Endpoint| -> Result<bool> {
        match end {
            Endpoint::Crossing { point, .. } => {
                let b = point.uv_box()[axis];
                Ok(b[0] <= fixed && fixed <= b[1])
            }
            Endpoint::Parameter(t) => {
                let exact = if *t == d[0] && c.knots[..=c.degree].iter().all(|&k| k == *t) {
                    Some(c.control_points[0][axis])
                } else if *t == d[1] && c.knots[c.control_points.len()..].iter().all(|&k| k == *t) {
                    Some(c.control_points.last().unwrap()[axis])
                } else {
                    None
                };
                if let Some(x) = exact {
                    return Ok(x == fixed);
                }
                let b = nurbs_core::interval_eval::evaluate_interval(
                    c,
                    nurbs_core::interval_eval::Interval::new(*t, *t)?,
                )?[axis];
                Ok(b.lo <= fixed && fixed <= b.hi)
            }
        }
    };
    let ends = [
        endpoint_may_contact(&fragment.endpoints()[0])?,
        endpoint_may_contact(&fragment.endpoints()[1])?,
    ];
    out.locus = if ends.iter().any(|&x| x) {
        Locus::Endpoints(ends)
    } else {
        Locus::Away
    };
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_plane_fiber;
    use nurbs_core::{curve::Curve, surface::Surface};
    #[test]
    fn original_paths_report_owned_candidates_without_proximity() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let fiber = source_plane_fiber::certify(
            &s,
            [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            0,
            false,
            100000,
        )
        .unwrap()
        .certificate
        .unwrap();
        let path = |p: Vec<Vec<f64>>| {
            let c = Curve::from_polyline(p).unwrap();
            Fragment::new(&s, &c, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap()
        };
        assert_eq!(
            inspect(&path(vec![vec![0., 0.], vec![0., 1.]]), &fiber, 100)
                .unwrap()
                .locus,
            Locus::EntireFragment
        );
        assert_eq!(
            inspect(&path(vec![vec![0., 0.], vec![1., 0.]]), &fiber, 100)
                .unwrap()
                .locus,
            Locus::Endpoints([true, false])
        );
        assert_eq!(
            inspect(&path(vec![vec![1., 1.], vec![0., 1.]]), &fiber, 100)
                .unwrap()
                .locus,
            Locus::Endpoints([false, true])
        );
        assert_eq!(
            inspect(&path(vec![vec![1e-12, 0.], vec![1e-12, 1.]]), &fiber, 100)
                .unwrap()
                .locus,
            Locus::Away
        );
        let mut c = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        c.degree = 2;
        c.knots = vec![0., 0., 0., 1., 1., 1.];
        c.control_points = vec![vec![0., 0.], vec![0.75, 0.5], vec![0., 1.]];
        c.weights = vec![1.; 3];
        let f = Fragment::new(&s, &c, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        assert_eq!(
            inspect(&f, &fiber, 100).unwrap().locus,
            Locus::Endpoints([true, true])
        );
        let part =
            Fragment::new(&s, &c, Endpoint::Parameter(0.25), Endpoint::Parameter(0.75)).unwrap();
        assert_eq!(inspect(&part, &fiber, 100).unwrap().locus, Locus::Away);
    }
}
