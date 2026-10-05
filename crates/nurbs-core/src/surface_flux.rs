//! Outward flux bounds for F_axis = S_axis - origin, whose divergence is one.
use crate::{check, interval_eval::Interval as I, surface::Surface, Result};
pub struct Bound {
    pub density: [f64; 2],
    pub integral: [f64; 2],
}
pub struct Report {
    pub bound: Option<Bound>,
    pub spans: usize,
}
/// Derivatives are with respect to the original surface parameters. Physical
/// UV section area is applied exactly once, including non-unit knot domains.
pub fn bound(
    s: &Surface,
    rectangle: [[f64; 2]; 2],
    axis: usize,
    origin: f64,
    max_spans: usize,
) -> Result<Report> {
    s.validate()?;
    check(
        axis < 3
            && origin.is_finite()
            && max_spans <= 100000
            && s.control_points[0][0].len() == 3
            && !s.periodic_u
            && !s.periodic_v,
        "Choose an original nonperiodic 3D chart and finite flux inputs",
    )?;
    let knots = [&s.knots_u, &s.knots_v];
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    check(
        (0..2).all(|a| {
            rectangle[a].iter().all(|x| x.is_finite())
                && rectangle[a][0] < rectangle[a][1]
                && rectangle[a][0] >= knots[a][degrees[a]]
                && rectangle[a][1] <= knots[a][counts[a]]
        }),
        "Flux rectangle must have positive area inside the original chart",
    )?;
    let mut out = Report {
        bound: None,
        spans: 0,
    };
    let mut total = I::point(0.);
    let mut density = [f64::INFINITY, f64::NEG_INFINITY];
    for u in crate::sweep_support::audit::nonempty_spans(knots[0], degrees[0], counts[0]) {
        for v in crate::sweep_support::audit::nonempty_spans(knots[1], degrees[1], counts[1]) {
            let indices = [u, v];
            let section = std::array::from_fn(|a| {
                [
                    rectangle[a][0].max(knots[a][indices[a]]),
                    rectangle[a][1].min(knots[a][indices[a] + 1]),
                ]
            });
            if section.iter().any(|a| a[0] >= a[1]) {
                continue;
            }
            if out.spans == max_spans {
                return Ok(out);
            }
            out.spans += 1;
            let position = crate::surface_distance::rectangle_bounds(s, section)?;
            let j = crate::surface_injectivity::section_jacobian(s, indices, section)?;
            let b = (axis + 1) % 3;
            let c = (axis + 2) % 3;
            let normal = j[b][0].mul(j[c][1])?.sub(j[c][0].mul(j[b][1])?)?;
            let f = I::new(position[axis][0], position[axis][1])?
                .sub(I::point(origin))?
                .mul(normal)?;
            let area = I::point(section[0][1])
                .sub(I::point(section[0][0]))?
                .mul(I::point(section[1][1]).sub(I::point(section[1][0]))?)?;
            total = total.add(f.mul(area)?)?;
            density[0] = density[0].min(f.lo);
            density[1] = density[1].max(f.hi);
        }
    }
    if out.spans > 0 {
        out.bound = Some(Bound {
            density,
            integral: [total.lo, total.hi],
        });
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![2., 2., 4., 4.],
            knots_v: vec![3., 3., 7., 7.],
            control_points: vec![
                vec![vec![0., 0., 5.], vec![0., 3., 5.]],
                vec![vec![2., 0., 5.], vec![2., 3., 5.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn original_domain_area_and_flux_orientation_are_not_normalized_twice() {
        let s = plane();
        let r = bound(&s, [[2., 4.], [3., 7.]], 2, 0., 10)
            .unwrap()
            .bound
            .unwrap();
        assert!(r.integral[0] <= 30. && 30. <= r.integral[1]);
        assert!(r.integral[1] - r.integral[0] < 1e-10);
        let r = bound(&s, [[2., 3.], [3., 5.]], 2, 1., 10)
            .unwrap()
            .bound
            .unwrap();
        assert!(r.integral[0] <= 6. && 6. <= r.integral[1]);
        let mut reversed = s.clone();
        reversed.control_points.reverse();
        let r = bound(&reversed, [[2., 4.], [3., 7.]], 2, 0., 10)
            .unwrap()
            .bound
            .unwrap();
        assert!(r.integral[0] <= -30. && -30. <= r.integral[1]);
        assert!(bound(&s, [[2., 4.], [3., 7.]], 2, 0., 0)
            .unwrap()
            .bound
            .is_none());
        assert!(bound(&s, [[2., 4.], [3., 7.]], 3, 0., 10).is_err());
    }
    #[test]
    fn span_partition_and_rational_density_enclose_original_point_samples() {
        let mut s = plane();
        s.knots_u = vec![2., 2., 3., 4., 4.];
        s.control_points
            .insert(1, vec![vec![1., 0., 5.], vec![1., 3., 5.]]);
        s.weights.insert(1, vec![1.; 2]);
        let r = bound(&s, [[2., 4.], [3., 7.]], 2, 0., 10).unwrap();
        assert_eq!(r.spans, 2);
        let r = r.bound.unwrap();
        assert!(r.integral[0] <= 30. && 30. <= r.integral[1]);
        assert!(bound(&s, [[2., 4.], [3., 7.]], 2, 0., 1)
            .unwrap()
            .bound
            .is_none());
        let mut s = plane();
        s.weights = vec![vec![1., 2.], vec![3., 6.]];
        let r = bound(&s, [[2., 4.], [3., 7.]], 2, 0., 10)
            .unwrap()
            .bound
            .unwrap();
        for (u, v) in [(2.1, 3.1), (3., 5.), (3.9, 6.9)] {
            let p = s.evaluate(u, v).unwrap();
            // Independent finite differences only check sampled inclusion;
            // certificate authority is the outward original derivative bound.
            let h = 1e-5;
            let a = s.evaluate(u + h, v).unwrap().point;
            let b = s.evaluate(u - h, v).unwrap().point;
            let c = s.evaluate(u, v + h).unwrap().point;
            let d = s.evaluate(u, v - h).unwrap().point;
            let value = p.point[2]
                * ((a[0] - b[0]) * (c[1] - d[1]) - (a[1] - b[1]) * (c[0] - d[0]))
                / (4. * h * h);
            assert!(r.density[0] <= value && value <= r.density[1]);
        }
    }
}
