//! Local C1 cubic curve interpolation (The NURBS Book, section 9.3.4).
//!
//! Each segment depends only on its two sites and their neighbours, so moving
//! one site changes at most the four segments around it. This is the
//! difference from global interpolation ([`super::interpolation`]), where one
//! site influences the whole curve.
//!
//! The derivative at each site is the Bessel estimate: the derivative of the
//! parabola through the site and its two neighbours. The curve is a cubic
//! Hermite spline written as a B-spline with double interior knots, so it is
//! C1 at every site and its parameter at site `k` is exactly `parameters[k]`.
//!
//! [`interpolate_surface_local`] is the bicubic analogue (section 9.3.5):
//! bicubic Hermite patches whose corner derivatives and twists are Bessel
//! estimates along the grid lines, so one moved site changes at most the
//! nine patches around it (the twist estimate reaches one more patch).
use super::interpolation::{Parameterization, parameters};
use super::{Curve, Result, Surface, check};

/// Bessel derivative estimates of `values` sampled at strictly increasing
/// `u`, with end-parabola derivatives at both ends.
fn bessel(values: &[[f64; 3]], u: &[f64]) -> Vec<[f64; 3]> {
    let n = values.len();
    let steps: Vec<f64> = u.windows(2).map(|w| w[1] - w[0]).collect();
    let slopes: Vec<[f64; 3]> = (0..n - 1)
        .map(|k| difference(values[k + 1], values[k], steps[k]))
        .collect();
    let mut d = vec![[0.; 3]; n];
    if n == 2 {
        return vec![slopes[0]; 2];
    }
    for k in 1..n - 1 {
        let alpha = steps[k - 1] / (steps[k - 1] + steps[k]);
        d[k] = std::array::from_fn(|i| (1. - alpha) * slopes[k - 1][i] + alpha * slopes[k][i]);
    }
    d[0] = std::array::from_fn(|i| 2. * slopes[0][i] - d[1][i]);
    d[n - 1] = std::array::from_fn(|i| 2. * slopes[n - 2][i] - d[n - 2][i]);
    d
}

/// Knot vector with a triple knot at every interior site.
fn triple_knots(u: &[f64]) -> Vec<f64> {
    let mut knots = vec![0.; 4];
    for &site in &u[1..u.len() - 1] {
        knots.extend([site; 3]);
    }
    knots.extend([1.; 4]);
    knots
}

/// Parameters of a grid direction: the average over all grid lines of the
/// per-line parameters (as in global surface interpolation).
fn averaged(lines: &[Vec<[f64; 3]>], param: Parameterization) -> Vec<f64> {
    let mut sum = vec![0.; lines[0].len()];
    for line in lines {
        for (s, u) in sum.iter_mut().zip(parameters(line, param)) {
            *s += u;
        }
    }
    sum.iter().map(|s| s / lines.len() as f64).collect()
}

/// Interpolate a rectangular grid `points[i][j]` (`i` along u, `j` along v)
/// with a C1 bicubic. The surface passes through site `(i, j)` at the
/// averaged parameters `(u[i], v[j])`. Each direction allows 2..86 sites
/// (`3 * sites - 2` control points, at most 256).
pub fn interpolate_surface_local(
    points: &[Vec<[f64; 3]>],
    param: Parameterization,
) -> Result<Surface> {
    let nu = points.len();
    let nv = points.first().map_or(0, Vec::len);
    check(
        (2..=86).contains(&nu) && (2..=86).contains(&nv) && points.iter().all(|r| r.len() == nv),
        "Local bicubic interpolation needs a rectangular 2..86 by 2..86 grid",
    )?;
    check(
        points
            .iter()
            .flatten()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= 1e9),
        "Interpolation sites must be finite and bounded by 1e9",
    )?;
    let columns: Vec<Vec<[f64; 3]>> = (0..nv).map(|j| points.iter().map(|r| r[j]).collect()).collect();
    let u = averaged(&columns, param);
    let v = averaged(points, param);
    check(
        u.windows(2).all(|w| w[1] > w[0]) && v.windows(2).all(|w| w[1] > w[0]),
        "Consecutive grid lines must be distinct",
    )?;
    // Corner derivatives: d/du along columns, d/dv along rows, and the twist
    // as the u-derivative estimate of the v-derivatives.
    let du_cols: Vec<Vec<[f64; 3]>> = columns.iter().map(|c| bessel(c, &u)).collect();
    let dv: Vec<Vec<[f64; 3]>> = points.iter().map(|r| bessel(r, &v)).collect();
    let duv_cols: Vec<Vec<[f64; 3]>> = (0..nv)
        .map(|j| bessel(&dv.iter().map(|r| r[j]).collect::<Vec<_>>(), &u))
        .collect();
    let hu: Vec<f64> = u.windows(2).map(|w| w[1] - w[0]).collect();
    let hv: Vec<f64> = v.windows(2).map(|w| w[1] - w[0]).collect();
    let (cu, cv) = (3 * nu - 2, 3 * nv - 2);
    let mut control_points = vec![vec![Vec::new(); cv]; cu];
    for i in 0..nu {
        for j in 0..nv {
            for a in -1i32..=1 {
                for b in -1i32..=1 {
                    let (Some(ri), Some(rj)) = (
                        (3 * i as i32 + a).try_into().ok().filter(|&r: &usize| r < cu),
                        (3 * j as i32 + b).try_into().ok().filter(|&r: &usize| r < cv),
                    ) else {
                        continue;
                    };
                    // Signed thirds of the neighbouring step on each side.
                    let su = match a {
                        -1 => -hu[i - 1] / 3.,
                        1 => hu[i] / 3.,
                        _ => 0.,
                    };
                    let sv = match b {
                        -1 => -hv[j - 1] / 3.,
                        1 => hv[j] / 3.,
                        _ => 0.,
                    };
                    control_points[ri][rj] = (0..3)
                        .map(|k| {
                            points[i][j][k]
                                + su * du_cols[j][i][k]
                                + sv * dv[i][j][k]
                                + su * sv * duv_cols[j][i][k]
                        })
                        .collect();
                }
            }
        }
    }
    let surface = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: triple_knots(&u),
        knots_v: triple_knots(&v),
        control_points,
        weights: vec![vec![1.; cv]; cu],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(surface)
}

fn difference(a: [f64; 3], b: [f64; 3], scale: f64) -> [f64; 3] {
    std::array::from_fn(|k| (a[k] - b[k]) / scale)
}

/// Interpolate `points` with a C1 cubic. `end_tangents`, when given, are the
/// derivatives with respect to the normalized parameter at the first and last
/// site; otherwise the end derivatives come from the end parabolas.
///
/// Consecutive sites must be distinct under the chosen parameterization.
/// The result has `3 * points.len() - 2` control points, so at most 86 sites
/// fit the 256 control point limit.
pub fn interpolate_curve_local(
    points: &[[f64; 3]],
    param: Parameterization,
    end_tangents: Option<([f64; 3], [f64; 3])>,
) -> Result<Curve> {
    let n = points.len();
    check(
        (2..=86).contains(&n),
        "Local cubic interpolation needs 2..86 data sites",
    )?;
    check(
        points
            .iter()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= 1e9),
        "Interpolation sites must be finite and bounded by 1e9",
    )?;
    if let Some((a, b)) = end_tangents {
        check(
            a.iter().chain(&b).all(|x| x.is_finite()),
            "End tangents must be finite",
        )?;
    }
    let u = parameters(points, param);
    let steps: Vec<f64> = u.windows(2).map(|w| w[1] - w[0]).collect();
    check(
        steps.iter().all(|&h| h > 0.)
            && points.windows(2).all(|w| w[0] != w[1]),
        "Consecutive interpolation sites must be distinct",
    )?;
    let mut tangents = bessel(points, &u);
    if let Some((first, last)) = end_tangents {
        tangents[0] = first;
        tangents[n - 1] = last;
    }
    let mut control_points = vec![points[0].to_vec()];
    for k in 0..n - 1 {
        let third = steps[k] / 3.;
        control_points.push((0..3).map(|i| points[k][i] + third * tangents[k][i]).collect());
        control_points.push(
            (0..3)
                .map(|i| points[k + 1][i] - third * tangents[k + 1][i])
                .collect(),
        );
        control_points.push(points[k + 1].to_vec());
    }
    let knots = triple_knots(&u);
    // Triple interior knots make each segment a separate Bezier piece; the
    // shared tangents make adjacent pieces meet with equal derivatives.
    let curve = Curve {
        degree: 3,
        knots,
        control_points,
        weights: vec![1.; 3 * n - 2],
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sites() -> Vec<[f64; 3]> {
        vec![
            [0., 0., 0.],
            [1., 2., 0.5],
            [3., 2.5, 1.],
            [4., 0., 0.],
            [6., -1., 2.],
            [7., 1., 2.5],
        ]
    }

    fn close(a: &[f64], b: &[f64], tolerance: f64) {
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() <= tolerance, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn passes_through_every_site_at_its_parameter() {
        for param in [
            Parameterization::Uniform,
            Parameterization::ChordLength,
            Parameterization::Centripetal,
        ] {
            let points = sites();
            let curve = interpolate_curve_local(&points, param, None).unwrap();
            for (point, &u) in points.iter().zip(&parameters(&points, param)) {
                close(&curve.evaluate(u).unwrap().point, point, 1e-12);
            }
        }
    }

    #[test]
    fn is_c1_at_interior_sites() {
        let points = sites();
        let curve = interpolate_curve_local(&points, Parameterization::ChordLength, None).unwrap();
        for &u in &parameters(&points, Parameterization::ChordLength)[1..points.len() - 1] {
            let left = curve.derivatives(u - 1e-9, 1).unwrap();
            let right = curve.derivatives(u, 1).unwrap();
            close(&left[1], &right[1], 1e-6);
        }
    }

    #[test]
    fn reproduces_a_parabola_and_honours_end_tangents() {
        // Bessel tangents are exact for a quadratic sampled uniformly.
        let points: Vec<[f64; 3]> = (0..7)
            .map(|i| {
                let t = i as f64 / 6.;
                [t, t * t, 0.]
            })
            .collect();
        let curve = interpolate_curve_local(&points, Parameterization::Uniform, None).unwrap();
        for step in 0..=60 {
            let t = step as f64 / 60.;
            close(&curve.evaluate(t).unwrap().point, &[t, t * t, 0.], 1e-12);
        }
        let tangents = ([0., 3., 0.], [1., 0., 1.]);
        let pinned =
            interpolate_curve_local(&points, Parameterization::Uniform, Some(tangents)).unwrap();
        close(&pinned.derivatives(0., 1).unwrap()[1], &tangents.0, 1e-12);
        close(&pinned.derivatives(1., 1).unwrap()[1], &tangents.1, 1e-12);
    }

    #[test]
    fn moving_one_site_changes_only_nearby_segments() {
        let points = sites();
        let mut moved = points.clone();
        moved[1][1] += 3.;
        let param = Parameterization::Uniform;
        let a = interpolate_curve_local(&points, param, None).unwrap();
        let b = interpolate_curve_local(&moved, param, None).unwrap();
        let u = parameters(&points, param);
        // Site 1 influences segments 0..=2; segments 3 and 4 are untouched.
        for step in 0..=20 {
            let t = u[3] + (u[5] - u[3]) * step as f64 / 20.;
            close(&a.evaluate(t).unwrap().point, &b.evaluate(t).unwrap().point, 0.);
        }
    }

    fn grid(f: impl Fn(f64, f64) -> f64, nu: usize, nv: usize) -> Vec<Vec<[f64; 3]>> {
        (0..nu)
            .map(|i| {
                (0..nv)
                    .map(|j| {
                        let (x, y) = (i as f64 / (nu - 1) as f64, j as f64 / (nv - 1) as f64);
                        [x, y, f(x, y)]
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn local_surface_interpolates_and_reproduces_biquadratics() {
        // Bessel estimates are exact for quadratics on uniform grids, and the
        // twist estimate of a biquadratic is exact as well.
        let f = |x: f64, y: f64| x * x - 2. * x * y + 0.5 * y * y + x;
        let points = grid(f, 6, 5);
        let s = interpolate_surface_local(&points, Parameterization::Uniform).unwrap();
        for i in 0..=30 {
            for j in 0..=30 {
                let (x, y) = (i as f64 / 30., j as f64 / 30.);
                close(&s.evaluate(x, y).unwrap().point, &[x, y, f(x, y)], 1e-12);
            }
        }
        // Wavy data, chord-length: passes through every site at its
        // averaged parameters.
        let g = |x: f64, y: f64| (3. * x).sin() * (2. * y).cos();
        let points = grid(g, 7, 9);
        let param = Parameterization::ChordLength;
        let s = interpolate_surface_local(&points, param).unwrap();
        let columns: Vec<Vec<[f64; 3]>> = (0..9).map(|j| points.iter().map(|r| r[j]).collect()).collect();
        let (u, v) = (averaged(&columns, param), averaged(&points, param));
        for i in 0..7 {
            for j in 0..9 {
                close(&s.evaluate(u[i], v[j]).unwrap().point, &points[i][j], 1e-12);
            }
        }
    }

    #[test]
    fn local_surface_changes_only_near_a_moved_site() {
        let g = |x: f64, y: f64| (3. * x).sin() * (2. * y).cos();
        let points = grid(g, 8, 8);
        let mut moved = points.clone();
        moved[1][1][2] += 1.;
        let p = Parameterization::Uniform;
        let a = interpolate_surface_local(&points, p).unwrap();
        let b = interpolate_surface_local(&moved, p).unwrap();
        // Uniform: the moved site sits at (1/7, 1/7); beyond three grid
        // steps in either direction nothing changes.
        for i in 0..=10 {
            for j in 0..=10 {
                let (x, y) = (4. / 7. + 3. / 7. * i as f64 / 10., 3. / 7. * j as f64 / 10.);
                close(&a.evaluate(x, y).unwrap().point, &b.evaluate(x, y).unwrap().point, 0.);
            }
        }
        assert!(interpolate_surface_local(&points[..1], p).is_err());
        let mut ragged = points.clone();
        ragged[2].pop();
        assert!(interpolate_surface_local(&ragged, p).is_err());
    }

    #[test]
    fn invalid_sites_are_refused() {
        let p = Parameterization::ChordLength;
        assert!(interpolate_curve_local(&[[0.; 3]], p, None).is_err());
        assert!(interpolate_curve_local(&[[0.; 3], [0.; 3], [1.; 3]], p, None).is_err());
        assert!(interpolate_curve_local(&[[0.; 3], [f64::NAN, 0., 0.]], p, None).is_err());
        assert!(interpolate_curve_local(&vec![[0.; 3]; 87], p, None).is_err());
        let two = interpolate_curve_local(&[[0.; 3], [1., 2., 3.]], p, None).unwrap();
        close(&two.evaluate(0.5).unwrap().point, &[0.5, 1., 1.5], 1e-14);
    }
}
