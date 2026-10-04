//! Homogeneous Gordon interpolation of a compatible rational curve network.
use crate::{
    Result, check,
    curve::{Curve, basis},
    surface::{Axis, Surface},
};

pub(crate) fn normalized(c: &Curve) -> Result<Curve> {
    c.validate()?;
    check(
        c.control_points[0].len() == 3 && !c.periodic,
        "Gordon needs nonperiodic 3D curves",
    )?;
    let [a, b] = c.domain();
    let mut c = c.trim(a, b)?;
    c.knots.iter_mut().for_each(|k| *k = (*k - a) / (b - a));
    c.validate()?;
    Ok(c)
}
pub(crate) fn stations(p: &[f64], n: usize) -> Result<Vec<f64>> {
    check(
        p.len() == n && p.iter().all(|x| x.is_finite()) && p.windows(2).all(|x| x[0] < x[1]),
        "Gordon needs finite increasing matching stations",
    )?;
    let span = p[n - 1] - p[0];
    check(
        span.is_finite() && span > 0.,
        "Gordon station span overflow",
    )?;
    let mut u: Vec<f64> = p.iter().map(|x| (x - p[0]) / span).collect();
    u[0] = 0.;
    u[n - 1] = 1.;
    check(
        u.windows(2).all(|x| x[0] < x[1]),
        "Gordon normalized stations collapse",
    )?;
    Ok(u)
}
pub(crate) fn homogeneous(c: &Curve, u: f64) -> Result<[f64; 4]> {
    let b = basis(c.degree, &c.knots, c.control_points.len(), u, false)?;
    let mut h = [0.; 4];
    for ((p, w), b) in c.control_points.iter().zip(&c.weights).zip(b.basis) {
        for k in 0..3 {
            h[k] += p[k] * w * b;
        }
        h[3] += w * b;
    }
    check(
        h.iter().all(|x| x.is_finite()) && h[3] > 0.,
        "Gordon homogeneous crossing overflow",
    )?;
    Ok(h)
}
pub(crate) fn transpose(s: Surface) -> Surface {
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v,
        knots_v: s.knots_u,
        control_points: (0..nv)
            .map(|j| (0..nu).map(|i| s.control_points[i][j].clone()).collect())
            .collect(),
        weights: (0..nv)
            .map(|j| (0..nu).map(|i| s.weights[i][j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn align(s: &mut [Surface], axis: Axis) -> Result<()> {
    let degree = s
        .iter()
        .map(|s| {
            if matches!(axis, Axis::U) {
                s.degree_u
            } else {
                s.degree_v
            }
        })
        .max()
        .unwrap();
    for x in s.iter_mut() {
        *x = x.edit_axis(axis, |c| c.elevate(degree))?;
    }
    let knots = |s: &Surface| {
        if matches!(axis, Axis::U) {
            s.knots_u.clone()
        } else {
            s.knots_v.clone()
        }
    };
    let mut unique: Vec<f64> = s
        .iter()
        .flat_map(knots)
        .filter(|k| *k > 0. && *k < 1.)
        .collect();
    unique.sort_by(f64::total_cmp);
    unique.dedup();
    for k in unique {
        let target = s
            .iter()
            .map(|x| knots(x).iter().filter(|&&x| x == k).count())
            .max()
            .unwrap();
        for x in s.iter_mut() {
            let count = knots(x).iter().filter(|&&x| x == k).count();
            if count < target {
                *x = x.edit_axis(axis, |c| c.insert(k, target - count))?;
            }
        }
    }
    Ok(())
}
/// U curves are stationed along V; V curves along U. Each family has 2..11
/// members. Aligned homogeneous crossings must match exactly, without snapping.
/// Natural cubic interpolation combines Su + Sv - Suv in homogeneous space.
pub fn patch(
    u_curves: &[Curve],
    v_curves: &[Curve],
    parameters_u: &[f64],
    parameters_v: &[f64],
) -> Result<Surface> {
    check(
        (2..=11).contains(&u_curves.len()) && (2..=11).contains(&v_curves.len()),
        "Gordon needs 2..11 curves per family",
    )?;
    let u: Vec<Curve> = u_curves.iter().map(normalized).collect::<Result<_>>()?;
    let v: Vec<Curve> = v_curves.iter().map(normalized).collect::<Result<_>>()?;
    let pu = stations(parameters_u, v.len())?;
    let pv = stations(parameters_v, u.len())?;
    let mut xyz = vec![vec![[0.; 3]; u.len()]; v.len()];
    let mut weights = xyz.clone();
    for i in 0..v.len() {
        for j in 0..u.len() {
            let h = homogeneous(&u[j], pu[i])?;
            let other = homogeneous(&v[i], pv[j])?;
            check(
                h == other,
                "Gordon homogeneous crossings must match exactly; align profiles and weights",
            )?;
            xyz[i][j] = [h[0], h[1], h[2]];
            weights[i][j] = [h[3], 0., 0.];
        }
    }
    let mut grid = crate::grid_spline::interpolate(&xyz, parameters_u, parameters_v)?;
    let wg = crate::grid_spline::interpolate(&weights, parameters_u, parameters_v)?;
    for i in 0..grid.control_points.len() {
        for j in 0..grid.control_points[0].len() {
            let w = wg.control_points[i][j][0];
            check(
                w.is_finite() && w > 0.,
                "Gordon grid needs positive control weights",
            )?;
            grid.weights[i][j] = w;
            for x in &mut grid.control_points[i][j] {
                *x /= w;
            }
        }
    }
    grid.validate()?;
    let s = [
        crate::natural_loft::interpolate(&u, parameters_v)?,
        transpose(crate::natural_loft::interpolate(&v, parameters_u)?),
        grid,
    ];
    combine(s)
}

pub(crate) fn combine(mut s: [Surface; 3]) -> Result<Surface> {
    align(&mut s, Axis::U)?;
    align(&mut s, Axis::V)?;
    let mut result = s[0].clone();
    for i in 0..result.control_points.len() {
        for j in 0..result.control_points[0].len() {
            let w = s[0].weights[i][j] + s[1].weights[i][j] - s[2].weights[i][j];
            check(
                w.is_finite() && w > 0.,
                "Gordon result needs positive control weights",
            )?;
            for k in 0..3 {
                let h = s[0].control_points[i][j][k] * s[0].weights[i][j]
                    + s[1].control_points[i][j][k] * s[1].weights[i][j]
                    - s[2].control_points[i][j][k] * s[2].weights[i][j];
                result.control_points[i][j][k] = h / w;
            }
            result.weights[i][j] = w;
        }
    }
    result.validate()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "transport")]
    #[test]
    fn json_constructor_keeps_network_family_orientation() {
        let u = [
            line([0., 0., 0.], [1., 0., 0.]),
            line([0., 1., 0.], [1., 1., 1.]),
        ];
        let v = [
            line([0., 0., 0.], [0., 1., 0.]),
            line([1., 0., 0.], [1., 1., 1.]),
        ];
        let result=crate::transport::dispatch(value_codec::json!({"op":"surface_gordon","u_curves":u,"v_curves":v,"parameters_u":[0.,1.],"parameters_v":[0.,1.]})).unwrap();
        let s: Surface = value_codec::from_value(result).unwrap();
        let p = s.evaluate(0.25, 0.75).unwrap().point;
        for k in 0..3 {
            assert!((p[k] - [0.25, 0.75, 0.1875][k]).abs() < 1e-10);
        }
    }
    #[test]
    fn preserves_nonlinear_network_and_matches_independent_gordon_formula() {
        let u: Vec<_> = [0., 0.5, 1.]
            .into_iter()
            .map(|v| {
                crate::paths::bezier(
                    vec![
                        vec![0., v, 0.],
                        vec![0.5, v, (v + v * v) / 2.],
                        vec![1., v, v],
                    ],
                    None,
                )
                .unwrap()
            })
            .collect();
        let v: Vec<_> = [0., 0.5, 1.]
            .into_iter()
            .map(|u| {
                crate::paths::bezier(
                    vec![
                        vec![u, 0., 0.],
                        vec![u, 0.5, u / 2.],
                        vec![u, 1., u + u * (1. - u)],
                    ],
                    None,
                )
                .unwrap()
            })
            .collect();
        let s = patch(&u, &v, &[0., 0.5, 1.], &[0., 0.5, 1.]).unwrap();
        // Independent natural interpolation of sites (0,0),(.5,.25),(1,1), moment M1=3.
        let n = |x: f64| {
            let (a, b, p0, p1, m0, m1) = if x < 0.5 {
                (0., 0.5, 0., 0.25, 0., 3.)
            } else {
                (0.5, 1., 0.25, 1., 3., 0.)
            };
            let h = b - a;
            let t = (x - a) / h;
            let l = 1. - t;
            l * p0 + t * p1 + h * h * ((l * l * l - l) * m0 + (t * t * t - t) * m1) / 6.
        };
        for x in [0., 0.13, 0.5, 0.87, 1.] {
            for y in [0., 0.17, 0.5, 0.83, 1.] {
                let ng = x - n(x);
                let expected = x * y + x * (1. - x) * n(y) + y * y * ng - ng * n(y);
                assert!((s.evaluate(x, y).unwrap().point[2] - expected).abs() < 1e-10);
            }
        }
        for j in 0..3 {
            for t in [0., 0.13, 0.37, 0.83, 1.] {
                for (p, q) in [
                    (
                        s.evaluate(t, j as f64 / 2.).unwrap().point,
                        u[j].evaluate(t).unwrap().point,
                    ),
                    (
                        s.evaluate(j as f64 / 2., t).unwrap().point,
                        v[j].evaluate(t).unwrap().point,
                    ),
                ] {
                    for k in 0..3 {
                        assert!((p[k] - q[k]).abs() < 1e-10);
                    }
                }
            }
        }
    }
    #[test]
    fn retains_rational_arc_boundaries_between_crossings() {
        let a = crate::primitives::circle_arc([0., 0., 0.], [0., 0., 1.], 2., 0., 90.).unwrap();
        let mut b = a.clone();
        for p in &mut b.control_points {
            p[2] = 2.;
        }
        let left = line(
            std::array::from_fn(|k| a.control_points[0][k]),
            std::array::from_fn(|k| b.control_points[0][k]),
        );
        let last = a.control_points.len() - 1;
        let right = line(
            std::array::from_fn(|k| a.control_points[last][k]),
            std::array::from_fn(|k| b.control_points[last][k]),
        );
        let s = patch(&[a, b], &[left, right], &[0., 1.], &[0., 1.]).unwrap();
        for u in [0., 0.13, 0.37, 0.83, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[0] * p[0] + p[1] * p[1] - 4.).abs() < 1e-10);
                assert!((p[2] - 2. * v).abs() < 1e-10);
            }
        }
    }
    fn line(a: [f64; 3], b: [f64; 3]) -> Curve {
        crate::primitives::line(a, b).unwrap()
    }
    #[test]
    fn reproduces_independent_bilinear_network() {
        let u: Vec<_> = [0., 0.5, 1.]
            .into_iter()
            .map(|v| line([0., v, 0.], [1., v, v]))
            .collect();
        let v: Vec<_> = [0., 0.5, 1.]
            .into_iter()
            .map(|u| line([u, 0., 0.], [u, 1., u]))
            .collect();
        let s = patch(&u, &v, &[2., 3., 4.], &[-1., 0., 1.]).unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - [u, v, u * v][k]).abs() < 1e-10);
                }
            }
        }
    }
    #[test]
    fn refuses_incompatible_crossings_and_station_arrays() {
        let u = [
            line([0., 0., 0.], [1., 0., 0.]),
            line([0., 1., 0.], [1., 1., 0.]),
        ];
        let mut v = [
            line([0., 0., 0.], [0., 1., 0.]),
            line([1., 0., 0.], [1., 1., 0.]),
        ];
        assert!(patch(&u, &v, &[0., 0.], &[0., 1.]).is_err());
        v[0].weights[0] = 2.;
        assert!(patch(&u, &v, &[0., 1.], &[0., 1.]).is_err());
        assert!(patch(&[], &v, &[0., 1.], &[]).is_err());
    }
}
