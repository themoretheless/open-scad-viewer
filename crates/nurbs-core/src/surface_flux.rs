//! Outward flux bounds for F_axis = S_axis - origin, whose divergence is one.
use crate::{Result, check, interval_eval::Interval as I, surface::Surface};
pub struct Bound {
    pub density: [f64; 2],
    pub integral: [f64; 2],
}
pub struct Report {
    pub bound: Option<Bound>,
    pub spans: usize,
}
fn magnitude(x: I) -> f64 {
    x.lo.abs().max(x.hi.abs())
}
// Derivatives in normalized original knot coordinates. Product rule keeps
// the original rational chart; only the integration enclosure is tightened.
fn flux_derivative(
    j: &crate::surface_measure::jets::Jets,
    axis: usize,
    offset: I,
    derivative: [usize; 2],
) -> Result<I> {
    let mut result = I::point(0.);
    for a in 0..=derivative[0] {
        for b in 0..=derivative[1] {
            for c in 0..=derivative[0] - a {
                for d in 0..=derivative[1] - b {
                    let e = derivative[0] - a - c;
                    let f = derivative[1] - b - d;
                    let factorial = |n: usize| if n == 2 { 2. } else { 1. };
                    let coefficient = factorial(derivative[0]) * factorial(derivative[1])
                        / (factorial(a)
                            * factorial(c)
                            * factorial(e)
                            * factorial(b)
                            * factorial(d)
                            * factorial(f));
                    let mut position = j[a][b][axis];
                    if a + b == 0 {
                        position = position.add(offset)?;
                    }
                    let k = (axis + 1) % 3;
                    let l = (axis + 2) % 3;
                    let normal = j[c + 1][d][k]
                        .mul(j[e][f + 1][l])?
                        .sub(j[c + 1][d][l].mul(j[e][f + 1][k])?)?;
                    result = result.add(position.mul(normal)?.mul(I::point(coefficient))?)?;
                }
            }
        }
    }
    Ok(result)
}
fn taylor_integral(
    s: &Surface,
    span: [usize; 2],
    section: [[f64; 2]; 2],
    axis: usize,
    origin: f64,
) -> Result<I> {
    let full = std::array::from_fn::<_, 2, _>(|a| {
        let knots = if a == 0 { &s.knots_u } else { &s.knots_v };
        I::point(knots[span[a] + 1]).sub(I::point(knots[span[a]]))
    });
    let full = [full[0].clone()?, full[1].clone()?];
    let mut widths = [I::point(0.); 2];
    let mut midpoint = [[0.; 2]; 2];
    for a in 0..2 {
        widths[a] = I::point(section[a][1])
            .sub(I::point(section[a][0]))?
            .div(full[a])?;
        let m = I::point(section[a][0])
            .add(I::point(section[a][1]))?
            .div(I::point(2.))?;
        midpoint[a] = [m.lo.max(section[a][0]), m.hi.min(section[a][1])];
    }
    let jets = crate::surface_measure::jets::calculate_partial_stable(s, span, section, [None; 2])?;
    let center =
        crate::surface_measure::jets::calculate_partial_stable(s, span, midpoint, [None; 2])?;
    let offset = I::point(s.control_points[span[0] - s.degree_u][span[1] - s.degree_v][axis])
        .sub(I::point(origin))?;
    let f = flux_derivative(&center, axis, offset, [0, 0])?;
    let hessian = [[2, 0], [1, 1], [0, 2]].map(|d| flux_derivative(&jets, axis, offset, d));
    let error = I::point(magnitude(hessian[0].clone()?))
        .mul(widths[0])?
        .mul(widths[0])?
        .div(I::point(24.))?
        .add(
            I::point(magnitude(hessian[1].clone()?))
                .mul(widths[0])?
                .mul(widths[1])?
                .div(I::point(16.))?,
        )?
        .add(
            I::point(magnitude(hessian[2].clone()?))
                .mul(widths[1])?
                .mul(widths[1])?
                .div(I::point(24.))?,
        )?;
    f.add(I::new(-error.hi, error.hi)?)?
        .mul(widths[0])?
        .mul(widths[1])
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
            let coarse = f.mul(area)?;
            let refined = taylor_integral(s, indices, section, axis, origin)?;
            total = total.add(coarse.intersect(refined.lo, refined.hi)?)?;
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
    fn taylor_bounds_enclose_independent_polynomial_integrals_on_nonunit_domains() {
        let mut s = crate::polynomial::graph(
            [0., 1., 0., 1.],
            &[
                vec![0., 0., 0., 1.],
                vec![0.; 4],
                vec![0., 0., 1., 0.],
                vec![2., 0., 0., 0.],
            ],
        )
        .unwrap();
        s.knots_u.iter_mut().for_each(|x| *x = 2. + 3. * *x);
        s.knots_v.iter_mut().for_each(|x| *x = -4. + 2. * *x);
        let expected = 31. / 36.;
        let integrate = |n: usize| {
            let mut total = I::point(0.);
            for u in 0..n {
                for v in 0..n {
                    let section = [
                        [
                            2. + 3. * u as f64 / n as f64,
                            2. + 3. * (u + 1) as f64 / n as f64,
                        ],
                        [
                            -4. + 2. * v as f64 / n as f64,
                            -4. + 2. * (v + 1) as f64 / n as f64,
                        ],
                    ];
                    let value = bound(&s, section, 2, 0., 1).unwrap().bound.unwrap();
                    total = total
                        .add(I::new(value.integral[0], value.integral[1]).unwrap())
                        .unwrap();
                }
            }
            assert!(total.lo <= expected && expected <= total.hi, "{total:?}");
            total.hi - total.lo
        };
        let coarse = integrate(4);
        let fine = integrate(8);
        assert!(fine < coarse / 2.);
        let mut reversed = s.clone();
        reversed.control_points.reverse();
        let value = bound(&reversed, [[2., 5.], [-4., -2.]], 2, 0., 1)
            .unwrap()
            .bound
            .unwrap();
        assert!(value.integral[0] <= -expected && -expected <= value.integral[1]);
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
        assert!(
            bound(&s, [[2., 4.], [3., 7.]], 2, 0., 0)
                .unwrap()
                .bound
                .is_none()
        );
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
        assert!(
            bound(&s, [[2., 4.], [3., 7.]], 2, 0., 1)
                .unwrap()
                .bound
                .is_none()
        );
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
